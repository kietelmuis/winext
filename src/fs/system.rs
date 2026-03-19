use std::{
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use winfsp::{
    Result, U16CStr,
    filesystem::{DirMarker, FileInfo, FileSecurity, FileSystemContext, VolumeInfo},
    host::FileSystemHost,
};

use crate::fs::file::GoonFile;

pub struct GoonFs {
    host: Arc<Mutex<FileSystemHost<GoonContext>>>,
}

pub struct GoonContext;

impl GoonFs {
    pub fn new(context: FileSystemHost<GoonContext>) -> Self {
        GoonFs {
            host: Arc::new(Mutex::new(context)),
        }
    }
}

impl FileSystemContext for GoonContext {
    type FileContext = GoonFile;

    fn get_security_by_name(
        &self,
        file_name: &U16CStr,
        _security_descriptor: Option<&mut [std::ffi::c_void]>,
        _reparse_point_resolver: impl FnOnce(&U16CStr) -> Option<FileSecurity>,
    ) -> Result<FileSecurity> {
        eprintln!("get_security_by_name: {:?}", file_name);

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
        eprintln!("open: {:?}", file_name);

        Result::Ok(GoonFile {})
    }

    fn close(&self, _context: Self::FileContext) {
        eprintln!("close");
    }

    fn get_file_info(&self, _context: &Self::FileContext, file_info: &mut FileInfo) -> Result<()> {
        eprintln!("get_file_info");

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
        eprintln!("get_volume_info");

        out_volume_info.total_size = 10 * 1024 * 1024 * 1024;
        out_volume_info.free_size = 5 * 1024 * 1024 * 1024;

        Ok(())
    }

    fn read_directory(
        &self,
        _context: &Self::FileContext,
        _pattern: Option<&U16CStr>,
        _marker: DirMarker<'_>,
        _buffer: &mut [u8],
    ) -> Result<u32> {
        eprintln!("read_directory");
        Ok(0)
    }
}
