use std::ffi::CString;

use ::windows::Win32::Foundation::*;
use ::windows::Win32::Storage::FileSystem::*;
use ::windows::Win32::System::IO::*;
use ::windows::Win32::System::Ioctl::*;
use ::windows::core::*;
use ext4_rs::BlockDevice;
use log::debug;
use log::info;

pub struct DriveBlockDevice {
    handle: HANDLE,
    super_block: DiskSuperBlock,
}

#[derive(Debug)]
struct DiskSuperBlock {
    block_size: u32,
    block_count: u32,
}

unsafe impl Send for DriveBlockDevice {}
unsafe impl Sync for DriveBlockDevice {}

impl DriveBlockDevice {
    pub fn open(path: &str) -> Result<Self> {
        let path_cstr = CString::new(path).unwrap();
        let handle = unsafe {
            CreateFileA(
                PCSTR(path_cstr.as_ptr() as *const u8),
                GENERIC_READ.0 | GENERIC_WRITE.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_FLAGS_AND_ATTRIBUTES(0),
                None,
            )?
        };

        assert!(!handle.is_invalid(), "handle is invalid");

        let mut geo: Option<DISK_GEOMETRY_EX> = None;
        let mut bytes = 0u32;
        unsafe {
            let _ = DeviceIoControl(
                handle,
                IOCTL_DISK_GET_DRIVE_GEOMETRY_EX,
                None,
                0,
                Some(&mut geo as *mut _ as *mut _),
                size_of::<DISK_GEOMETRY_EX>() as u32,
                Some(&mut bytes),
                None,
            );
        }

        let sector_size = match geo {
            Some(geo) => geo.Geometry.BytesPerSector,
            None => 512u32,
        };
        let sector_count = match geo {
            Some(geo) => geo.DiskSize as u64 / sector_size as u64,
            None => {
                let mut disk_size = 0i64;
                unsafe {
                    GetFileSizeEx(handle, &mut disk_size)?;
                }
                disk_size as u64 / sector_size as u64
            }
        };

        info!(
            "using {}; sector size: {}, sector count: {}",
            if let Some(_) = geo { "disk" } else { "img" },
            sector_size,
            sector_count
        );

        let super_block = DriveBlockDevice::get_super_block(&handle);
        info!(
            "block size: {}, block count: {}",
            super_block.block_size, super_block.block_count
        );

        Ok(Self {
            handle,
            super_block,
        })
    }

    fn get_super_block(handle: &HANDLE) -> DiskSuperBlock {
        let mut buf = vec![0u8; 1024];
        let mut bytes_read = 0u32;
        unsafe {
            SetFilePointerEx(*handle, 1024, None, FILE_BEGIN)
                .ok()
                .unwrap();
            ReadFile(*handle, Some(&mut buf), Some(&mut bytes_read), None)
                .ok()
                .unwrap();
        }

        DiskSuperBlock {
            block_size: 1024 << u32::from_le_bytes(buf[24..28].try_into().unwrap()),
            block_count: u32::from_le_bytes(buf[4..8].try_into().unwrap()),
        }
    }

    fn flush(&self) -> Result<()> {
        unsafe {
            FlushFileBuffers(self.handle).ok().unwrap();
            Ok(())
        }
    }
}

impl BlockDevice for DriveBlockDevice {
    fn read_offset(&self, offset: usize) -> Vec<u8> {
        let sector_size = 512usize;
        let aligned_offset = (offset / sector_size) * sector_size;
        let delta = offset - aligned_offset;
        let read_size = ((delta + self.super_block.block_size as usize + sector_size - 1)
            / sector_size)
            * sector_size;

        let mut buf = vec![0u8; read_size];
        let mut bytes_read = 0u32;

        debug!("reading offset={} aligned={}", offset, aligned_offset);

        unsafe {
            SetFilePointerEx(self.handle, aligned_offset as i64, None, FILE_BEGIN)
                .expect("failed to set file pointer");
            ReadFile(self.handle, Some(&mut buf), Some(&mut bytes_read), None)
                .expect("failed to read file");
        }
        debug!(
            "read_offset={}, bytes_read={}, first8={:02x?}",
            offset,
            bytes_read,
            &buf[..8]
        );

        let result = buf[delta..delta + self.super_block.block_size as usize].to_vec();
        result
    }

    fn write_offset(&self, offset: usize, data: &[u8]) {
        let mut bytes_written = 0u32;

        debug!("reading offset={}", offset);

        unsafe {
            SetFilePointerEx(self.handle, offset as i64, None, FILE_BEGIN).unwrap();
            WriteFile(self.handle, Some(data), Some(&mut bytes_written), None).unwrap();
        }

        self.flush().unwrap();
    }
}
