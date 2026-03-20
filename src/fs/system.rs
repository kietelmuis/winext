use ext4_lwext4::{BlockDevice, BlockDeviceExt, Ext4Fs, FileBlockDevice, OpenFlags};
use log::debug;
use std::{
    io::{Write, stderr},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use winfsp::{
    Result, U16CStr,
    filesystem::{
        DirBuffer, DirInfo, DirMarker, FileInfo, FileSecurity, FileSystemContext, VolumeInfo,
        WideNameInfo,
    },
    host::{FileSystemHost, VolumeParams},
};

use crate::fs::file::WinExtFile;

pub struct WinExtFs {
    pub host: FileSystemHost<WinExtContext>,
}

impl WinExtFs {
    pub fn new(context: WinExtContext, serial_number: u32) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let windows_filetime = (now + 11644473600) * 10000000;

        let mut volume_params = VolumeParams::new();
        volume_params.filesystem_name("ext4");
        volume_params.volume_creation_time(windows_filetime);
        volume_params.volume_serial_number(serial_number);

        WinExtFs {
            host: FileSystemHost::new(volume_params, context).expect("failed to create filesystem"),
        }
    }
}

pub struct WinExtContext {
    pub fs: Ext4Fs,
}

impl WinExtContext {
    pub fn new(device: FileBlockDevice) -> Self {
        let fs = Ext4Fs::mount(device, false).unwrap();
        WinExtContext { fs }
    }
}

impl FileSystemContext for WinExtContext {
    type FileContext = WinExtFile;

    fn get_security_by_name(
        &self,
        file_name: &U16CStr,
        _security_descriptor: Option<&mut [std::ffi::c_void]>,
        _reparse_point_resolver: impl FnOnce(&U16CStr) -> Option<FileSecurity>,
    ) -> Result<FileSecurity> {
        debug!("get_security_by_name: {:?}", file_name);

        Result::Ok(FileSecurity {
            reparse: false,
            sz_security_descriptor: 0,
            attributes: 0x10,
        })
    }

    fn open(
        &self,
        file_name: &U16CStr,
        _create_options: u32,
        _granted_access: u32,
        file_info: &mut winfsp::filesystem::OpenFileInfo,
    ) -> Result<Self::FileContext> {
        debug!("open: {:?}", file_name);

        let file = self
            .fs
            .open(&file_name.to_string_lossy(), OpenFlags::all())
            .unwrap();

        Result::Ok(WinExtFile {})
    }

    fn close(&self, _context: Self::FileContext) {
        debug!("close");
    }

    fn get_file_info(&self, _context: &Self::FileContext, file_info: &mut FileInfo) -> Result<()> {
        debug!("get_file_info");

        file_info.file_attributes = 0x10;
        file_info.reparse_tag = 0;
        file_info.file_size = 0;
        file_info.allocation_size = 0;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let windows_time = (now + 11644473600) * 10000000;

        file_info.creation_time = windows_time;
        file_info.last_access_time = windows_time;
        file_info.last_write_time = windows_time;
        file_info.change_time = windows_time;

        Ok(())
    }

    fn get_volume_info(&self, out_volume_info: &mut VolumeInfo) -> Result<()> {
        debug!("get_volume_info");

        let stats = self.fs.stat().unwrap();
        out_volume_info.total_size = stats.total_size();
        out_volume_info.free_size = stats.free_size();
        out_volume_info.set_volume_label(&stats.volume_name);

        Ok(())
    }

    fn read_directory(
        &self,
        context: &Self::FileContext,
        pattern: Option<&U16CStr>,
        marker: DirMarker<'_>,
        buffer: &mut [u8],
    ) -> Result<u32> {
        debug!("read_directory");

        let mut directory: DirInfo<1> = DirInfo::new();

        let buf = DirBuffer::new();
        buf.acquire(false, None)
            .unwrap()
            .write(&mut directory)
            .unwrap();

        let mut bytes_transferred: u32 = 0;
        directory.append_to_buffer(buffer, &mut bytes_transferred);

        Ok(bytes_transferred)
    }
}
