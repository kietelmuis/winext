use std::collections::HashMap;
use std::ffi::CString;
use std::sync::Mutex;

use ::windows::Win32::Foundation::*;
use ::windows::Win32::Storage::FileSystem::*;
use ::windows::Win32::System::IO::*;
use ::windows::Win32::System::Ioctl::*;
use ::windows::core::*;
use ext4_rs::BlockDevice;
use log::info;

pub struct DriveBlockDevice {
    handle: HANDLE,
    super_block: DiskSuperBlock,
    cache: Mutex<HashMap<usize, Vec<u8>>>,
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
                GENERIC_READ.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_FLAGS_AND_ATTRIBUTES(0),
                None,
            )?
        };

        assert!(!handle.is_invalid(), "handle is invalid");

        let mut geo = DISK_GEOMETRY_EX::default();
        let mut bytes = 0u32;
        unsafe {
            DeviceIoControl(
                handle,
                IOCTL_DISK_GET_DRIVE_GEOMETRY_EX,
                None,
                0,
                Some(&mut geo as *mut _ as *mut _),
                size_of::<DISK_GEOMETRY_EX>() as u32,
                Some(&mut bytes),
                None,
            )?;
        }

        let sector_size = geo.Geometry.BytesPerSector;
        let sector_count = geo.DiskSize as u64 / sector_size as u64;

        info!(
            "sector size: {}, sector count: {}",
            sector_size, sector_count
        );

        let super_block = DriveBlockDevice::get_super_block(&handle);
        info!(
            "block size: {}, block count: {}",
            super_block.block_size, super_block.block_count
        );

        Ok(Self {
            handle,
            super_block,
            cache: Mutex::new(HashMap::new()),
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
        {
            let cache = self.cache.lock().unwrap();
            if let Some(cached) = cache.get(&offset) {
                return cached.clone();
            }
        }

        let mut buf = vec![0u8; self.super_block.block_size as usize];
        let mut bytes_read = 0u32;
        let mut overlapped = OVERLAPPED::default();
        overlapped.Anonymous.Anonymous.Offset = (offset & 0xFFFF_FFFF) as u32;
        overlapped.Anonymous.Anonymous.OffsetHigh = (offset >> 32) as u32;

        unsafe {
            SetFilePointerEx(self.handle, offset as i64, None, FILE_BEGIN)
                .expect("failed to set file pointer");
            ReadFile(
                self.handle,
                Some(&mut buf),
                Some(&mut bytes_read),
                Some(&mut overlapped),
            )
            .expect("failed to read file");
        }
        println!("read_offset({}) first8={:02x?}", offset, &buf[..8]);

        buf.truncate(bytes_read as usize);
        self.cache.lock().unwrap().insert(offset, buf.clone());
        buf
    }

    fn write_offset(&self, offset: usize, data: &[u8]) {
        let mut bytes_written = 0u32;

        unsafe {
            SetFilePointerEx(self.handle, offset as i64, None, FILE_BEGIN).unwrap();
            WriteFile(self.handle, Some(data), Some(&mut bytes_written), None).unwrap();
        }

        self.flush().unwrap();
    }
}
