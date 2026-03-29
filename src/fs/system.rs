use ext4_rs::{BlockDevice, Ext4, InodeFileType};
use log::{debug, info};
use std::{
    ffi::c_void,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use windows::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL};
use winfsp::{
    Result, U16CStr,
    filesystem::{
        DirInfo, DirMarker, FileInfo, FileSecurity, FileSystemContext, OpenFileInfo, VolumeInfo,
        WideNameInfo,
    },
    host::{FileSystemHost, VolumeParams},
};

use crate::fs::file::WinExtFile;

pub struct WinExtFs {
    pub host: FileSystemHost<WinExtContext>,
}

impl WinExtFs {
    pub fn new(context: WinExtContext) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let windows_filetime = (now + 11644473600) * 10000000;

        let mut volume_params = VolumeParams::new();
        volume_params.sector_size(512);
        volume_params.sectors_per_allocation_unit(8);
        volume_params.max_component_length(255);
        volume_params.filesystem_name("ext4");
        volume_params.volume_creation_time(windows_filetime);
        volume_params.volume_serial_number(0x6f910e5b);
        volume_params.read_only_volume(true);

        WinExtFs {
            host: FileSystemHost::new(volume_params, context).expect("failed to create filesystem"),
        }
    }
}

pub struct WinExtContext {
    pub fs: Ext4,
}

impl WinExtContext {
    pub fn new(device: Arc<dyn BlockDevice + 'static>) -> Self {
        let fs = ext4_rs::Ext4::open(device);
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
            attributes: FILE_ATTRIBUTE_DIRECTORY.0,
        })
    }

    fn create(
        &self,
        file_name: &U16CStr,
        _create_options: u32,
        _granted_access: u32,
        _file_attributes: u32,
        _security_descriptor: Option<&[c_void]>,
        _allocation_size: u64,
        _extra_buffer: Option<&[u8]>,
        _extra_buffer_is_reparse_point: bool,
        _file_info: &mut OpenFileInfo,
    ) -> Result<Self::FileContext> {
        Ok(WinExtFile(file_name.to_string_lossy()))
    }

    fn open(
        &self,
        file_name: &U16CStr,
        _create_options: u32,
        _granted_access: u32,
        file_info: &mut winfsp::filesystem::OpenFileInfo,
    ) -> Result<Self::FileContext> {
        debug!("open: {:?}", file_name);

        let path = file_name
            .to_string_lossy()
            .replace('\\', "/")
            .trim_end_matches('\0')
            .to_string();
        debug!("path: {}", path);

        let inode = self
            .fs
            .generic_open(&path, &mut 2, false, InodeFileType::all().bits(), &mut 0)
            .unwrap();

        debug!("inode: {:?}", inode);

        let inoderef = self.fs.get_inode_ref(inode);

        let file_type = match inoderef.inode.file_type() {
            InodeFileType::S_IFREG => FILE_ATTRIBUTE_NORMAL,
            InodeFileType::S_IFDIR => FILE_ATTRIBUTE_DIRECTORY,
            t => {
                if t.bits() == 0 {
                    FILE_ATTRIBUTE_DIRECTORY
                } else {
                    panic!("no support for file attribute {}", t.bits())
                }
            }
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

        Ok(WinExtFile(path))
    }

    fn close(&self, context: Self::FileContext) {
        debug!("close: {}", &context.0);
    }

    fn get_file_info(&self, context: &Self::FileContext, file_info: &mut FileInfo) -> Result<()> {
        debug!("get_file_info: {}", &context.0);

        let inode = self.fs.ext4_file_open(&context.0, "r").unwrap();
        let inoderef = self.fs.get_inode_ref(inode);

        let file_type = match inoderef.inode.file_type() {
            InodeFileType::S_IFREG => FILE_ATTRIBUTE_NORMAL,
            InodeFileType::S_IFDIR => FILE_ATTRIBUTE_DIRECTORY,
            t => {
                if t.bits() == 0 {
                    FILE_ATTRIBUTE_DIRECTORY
                } else {
                    panic!("no support for file attribute {}", t.bits())
                }
            }
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
        debug!(
            "get_volume_info: {} inodes",
            self.fs.super_block.inodes_count
        );

        let free_blocks = self.fs.super_block.free_blocks_count();
        let total_blocks = self.fs.super_block.blocks_count() as u64;
        let block_size = self.fs.super_block.block_size() as u64;

        out_volume_info.total_size = total_blocks * block_size;
        out_volume_info.free_size = free_blocks * block_size;
        out_volume_info.set_volume_label("ext4");

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

        let dir = context.0.clone();
        debug!("doing dir {}", dir);

        let inode = self.fs.ext4_file_open(&dir, "r").unwrap();
        debug!("dir has inode {}", inode);

        let directories = self.fs.dir_get_entries(inode);
        let mut bytes_transferred: u32 = 0;

        let mut marker_passed = marker.is_none();
        let marker_name = match marker.inner_as_cstr() {
            Some(m) => Some(m.to_string_lossy()),
            None => None,
        };

        for dir in directories {
            let name = dir.get_name();
            debug!("doing dir {}", name);

            if !marker_passed {
                if marker_name == Some(name) {
                    marker_passed = true;
                }
                continue;
            }

            let mut dirinfo: DirInfo<255> = DirInfo::new();
            if dirinfo.set_name(name.clone()).is_err() {
                debug!("we were too stupid for dir {}", name);
                continue;
            }

            let fileinfo = dirinfo.file_info_mut();
            let attributes = match dir.get_de_type() {
                1 => FILE_ATTRIBUTE_NORMAL,
                7 => FILE_ATTRIBUTE_NORMAL,
                2 => FILE_ATTRIBUTE_DIRECTORY,
                t => panic!("cannot handle de type {}", t),
            }
            .0;
            debug!("dir {} has attributes {:x}", name, attributes);

            fileinfo.file_attributes = attributes;

            dirinfo.append_to_buffer(buffer, &mut bytes_transferred);
            debug!("appended dir {}", name);
        }

        debug!("appended {} bytes", bytes_transferred);
        Ok(bytes_transferred)
    }
}
