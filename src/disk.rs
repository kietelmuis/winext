use std::ffi::CString;

use windows::Win32::Foundation::*;
use windows::Win32::Storage::FileSystem::*;
use windows::Win32::System::IO::*;
use windows::Win32::System::Ioctl::*;

use ext4_lwext4::BlockDevice;
use windows::core::PCSTR;

pub struct DriveBlockDevice {
    handle: HANDLE,
    block_size: u32,
    block_count: u64,
}
unsafe impl Send for DriveBlockDevice {}
unsafe impl Sync for DriveBlockDevice {}

impl DriveBlockDevice {
    pub fn open(path: &str) -> windows::core::Result<Self> {
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

        let block_size = geo.Geometry.BytesPerSector;
        let block_count = geo.DiskSize as u64 / block_size as u64;

        Ok(Self {
            handle,
            block_size,
            block_count,
        })
    }
}

impl BlockDevice for DriveBlockDevice {
    fn read_blocks(&self, block_id: u64, buf: &mut [u8]) -> ext4_lwext4::Result<u32> {
        let byte_offset = block_id * buf.len() as u64;
        unsafe {
            SetFilePointerEx(self.handle, byte_offset as i64, None, FILE_BEGIN)
                .ok()
                .unwrap();
            ReadFile(self.handle, Some(buf), None, None).ok().unwrap();

            Ok(buf.len() as u32)
        }
    }

    fn write_blocks(&mut self, block_id: u64, buf: &[u8]) -> ext4_lwext4::Result<u32> {
        let offset = block_id * self.block_size as u64;
        unsafe {
            SetFilePointerEx(self.handle, offset as i64, None, FILE_BEGIN)
                .ok()
                .unwrap();

            let mut bytes_written = 0u32;
            WriteFile(self.handle, Some(buf), Some(&mut bytes_written), None)
                .ok()
                .unwrap();

            Ok(bytes_written)
        }
    }

    fn flush(&mut self) -> ext4_lwext4::Result<()> {
        unsafe {
            FlushFileBuffers(self.handle).ok().unwrap();
            Ok(())
        }
    }

    fn block_size(&self) -> u32 {
        self.block_size
    }

    fn block_count(&self) -> u64 {
        self.block_count
    }
}
