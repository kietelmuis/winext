use ext4_lwext4::{Ext4Fs, FileBlockDevice, OpenFlags};
use log::{debug, info};
use std::{
    ffi::c_void,
    marker::PhantomData,
    time::{SystemTime, UNIX_EPOCH},
};
use windows::Win32::Storage::FileSystem::{
    FILE_ACCESS_RIGHTS, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL, FILE_FLAGS_AND_ATTRIBUTES,
};
use winfsp::{
    FspError, Result, U16CStr,
    filesystem::{
        DirBuffer, DirInfo, DirMarker, FileInfo, FileSecurity, FileSystemContext, OpenFileInfo,
        VolumeInfo, WideNameInfo,
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
        security_descriptor: Option<&mut [std::ffi::c_void]>,
        reparse_point_resolver: impl FnOnce(&U16CStr) -> Option<FileSecurity>,
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
        create_options: u32,
        granted_access: u32,
        file_info: &mut winfsp::filesystem::OpenFileInfo,
    ) -> Result<Self::FileContext> {
        debug!("open: {:?}", file_name);

        let path = file_name
            .to_string_lossy()
            .replace('\\', "/")
            .trim_end_matches('\0')
            .to_string();

        let flags = OpenFlags::all();
        let meta = self.fs.metadata(&path).unwrap();

        let file_type = match meta.file_type {
            ext4_lwext4::FileType::RegularFile => FILE_ATTRIBUTE_NORMAL,
            ext4_lwext4::FileType::Directory => FILE_ATTRIBUTE_DIRECTORY,
            t => panic!("{:?} support is todo", t),
        }
        .0;

        info!("type: {:?}", file_type);

        let info = file_info.as_mut();
        info.file_attributes = file_type;
        info.reparse_tag = 0;
        info.file_size = 0;
        info.allocation_size = 0;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let windows_time = (now + 11644473600) * 10000000;

        info.creation_time = windows_time;
        info.last_access_time = windows_time;
        info.last_write_time = windows_time;
        info.change_time = windows_time;

        Ok(WinExtFile::new(&path, flags))
    }

    fn close(&self, _context: Self::FileContext) {
        debug!("close");
    }

    fn get_file_info(&self, context: &Self::FileContext, file_info: &mut FileInfo) -> Result<()> {
        debug!("get_file_info");

        let meta = self.fs.metadata(&context.path).unwrap();

        let file_type = match meta.file_type {
            ext4_lwext4::FileType::RegularFile => FILE_ATTRIBUTE_NORMAL,
            ext4_lwext4::FileType::Directory => FILE_ATTRIBUTE_DIRECTORY,
            t => panic!("{:?} support is todo", t),
        }
        .0;

        info!("type: {:?}", file_type);

        file_info.file_attributes = file_type;
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

        Ok(0)
    }
}
