use ext4_rs::{BlockDevice, Ext4, InodeFileType};
use log::{debug, error, info};
use std::{
    ffi::c_void,
    io::ErrorKind,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use windows::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL};
use winfsp::{
    FspError, Result, U16CStr,
    filesystem::{
        DirInfo, DirMarker, FileInfo, FileSecurity, FileSystemContext, OpenFileInfo, VolumeInfo,
        WideNameInfo,
    },
    host::{FileSystemHost, VolumeParams},
};

use crate::util::{inode::inode_type_to_windows, u16cstr::U16CStrExt, u32::U32Ext};
use crate::{fs::file::WinExtFile, util::inode::windows_to_inode_type};

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
        volume_params.filesystem_name("NTFS");
        volume_params.volume_creation_time(windows_filetime);
        volume_params.volume_serial_number(0x6f910e5b);
        volume_params.read_only_volume(false);
        volume_params.case_sensitive_search(false);
        volume_params.case_preserved_names(true);
        volume_params.unicode_on_disk(true);
        volume_params.persistent_acls(true);

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

        let path = file_name.sanitize();

        let inode_num = self
            .fs
            .ext4_file_open(&path, "r")
            .map_err(|_| FspError::IO(ErrorKind::NotFound))?;

        let inode = self.fs.get_inode_ref(inode_num);
        let file_type = inode_type_to_windows(inode.inode.file_type());

        Result::Ok(FileSecurity {
            reparse: false,
            sz_security_descriptor: 0,
            attributes: file_type.0,
        })
    }

    fn create(
        &self,
        file_name: &U16CStr,
        _create_options: u32,
        _granted_access: u32,
        file_attributes: u32,
        _security_descriptor: Option<&[c_void]>,
        _allocation_size: u64,
        _extra_buffer: Option<&[u8]>,
        _extra_buffer_is_reparse_point: bool,
        _file_info: &mut OpenFileInfo,
    ) -> Result<Self::FileContext> {
        debug!("create: {:?}", file_name);
        debug!(
            "opt: {}, access: {}, attrs: {}",
            _create_options, _granted_access, file_attributes
        );

        let path = file_name.sanitize();
        debug!("create path: {}", path);

        let inode = if file_attributes & FILE_ATTRIBUTE_DIRECTORY.0 != 0 {
            debug!("creating directory: {}", path);
            self.fs.ext4_dir_mk(&path)
        } else {
            debug!("creating file: {}", path);
            self.fs.ext4_file_open(&path, "w")
        }
        .map_err(|e| {
            error!("create failed: {:?}", e);
            FspError::IO(ErrorKind::Other)
        })?;

        debug!("created file: {}", path);

        Ok(WinExtFile {
            file: file_name.to_string_lossy(),
            inode: inode as u64,
        })
    }

    fn read(&self, context: &Self::FileContext, buffer: &mut [u8], offset: u64) -> Result<u32> {
        match self
            .fs
            .ext4_file_read(context.inode, buffer.len() as u32, offset as i64)
        {
            Ok(buf) => {
                buffer.copy_from_slice(&buf);
                Ok(buf.len() as u32)
            }
            Err(e) => {
                error!("read failed: {:?}", e);
                Err(FspError::IO(ErrorKind::Other))
            }
        }
    }

    fn write(
        &self,
        context: &Self::FileContext,
        buffer: &[u8],
        offset: u64,
        _write_to_eof: bool,
        _constrained_io: bool,
        _file_info: &mut FileInfo,
    ) -> Result<u32> {
        match self
            .fs
            .ext4_file_write(context.inode, offset as i64, buffer)
        {
            Ok(w) => Ok(w as u32),
            Err(e) => {
                error!("write failed: {:?}", e);
                Err(FspError::IO(ErrorKind::Other))
            }
        }
    }

    fn open(
        &self,
        file_name: &U16CStr,
        create_options: u32,
        _granted_access: u32,
        file_info: &mut winfsp::filesystem::OpenFileInfo,
    ) -> Result<Self::FileContext> {
        debug!("open: {:?}, create_options: {}", file_name, create_options);

        let path = file_name.sanitize();
        debug!("open path: {}", path);

        let inode_num = self.fs.ext4_file_open(&path, "r").map_err(|e| {
            error!("open error: {:?}", e);
            FspError::IO(ErrorKind::NotFound)
        })?;

        debug!("open inode: {:?}", inode_num);
        let inode = self.fs.get_inode_ref(inode_num);

        let ext_type = inode.inode.file_type();
        info!("open ext type: {:?}", ext_type);

        let file_type = inode_type_to_windows(ext_type);
        info!("open type: {:?}", file_type);

        let info = file_info.as_mut();
        info.file_attributes = file_type.0;
        info.reparse_tag = 0;
        info.file_size = 0;
        info.allocation_size = 0;

        info.creation_time = inode.inode.i_crtime().to_windows_time();
        info.last_access_time = inode.inode.atime().to_windows_time();
        info.last_write_time = inode.inode.mtime().to_windows_time();
        info.change_time = inode.inode.ctime().to_windows_time();

        Ok(WinExtFile {
            file: path.clone(),
            inode: inode_num as u64,
        })
    }

    fn close(&self, context: Self::FileContext) {
        debug!("close: {}", context.file);
    }

    fn get_file_info(&self, context: &Self::FileContext, info: &mut FileInfo) -> Result<()> {
        debug!("file info: {}", context.file);

        let inode = self.fs.get_inode_ref(context.inode as u32);

        let inode_type = inode.inode.file_type();
        info!("file info ext type: {:?}", inode_type);

        let file_type = inode_type_to_windows(inode_type);
        info!("file info type: {:?}", file_type);

        info.file_attributes = file_type.0;
        info.reparse_tag = 0;
        info.file_size = inode.inode.size();
        info.allocation_size = ((inode.inode.size() + 4095) / 4096) * 4096;

        info.creation_time = inode.inode.i_crtime().to_windows_time();
        info.last_access_time = inode.inode.atime().to_windows_time();
        info.last_write_time = inode.inode.mtime().to_windows_time();
        info.change_time = inode.inode.ctime().to_windows_time();

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
        out_volume_info.set_volume_label("NTFS");

        Ok(())
    }

    fn read_directory(
        &self,
        context: &Self::FileContext,
        _pattern: Option<&U16CStr>,
        marker: DirMarker<'_>,
        buffer: &mut [u8],
    ) -> Result<u32> {
        debug!("read_directory");
        debug!("directory is on inode {}", context.inode);

        let mut directories = self.fs.dir_get_entries(context.inode as u32);
        directories.sort_by(|a, b| a.get_name().cmp(&b.get_name()));

        let marker_name = marker.inner_as_cstr().map(|m| m.to_string_lossy());
        let mut bytes_transferred: u32 = 0;

        let start_index = if let Some(ref m) = marker_name {
            directories
                .iter()
                .position(|d| d.get_name() == *m)
                .map(|i| i + 1)
                .unwrap_or(0)
        } else {
            0
        };

        for dir in directories.iter().skip(start_index) {
            let name = dir.get_name();
            debug!("appending directory {}", name);

            if name == "." || name == ".." {
                continue;
            }

            let mut dirinfo: DirInfo<255> = DirInfo::new();
            if dirinfo.set_name(name.clone()).is_err() {
                debug!("failed to get name for directory {}", name);
                continue;
            }

            let fileinfo = dirinfo.file_info_mut();
            let attributes = match dir.get_de_type() {
                0 => {
                    let inode = self.fs.get_inode_ref(dir.inode);
                    inode_type_to_windows(inode.inode.file_type())
                }
                1 | 7 => FILE_ATTRIBUTE_NORMAL,
                2 => FILE_ATTRIBUTE_DIRECTORY,
                t => panic!("cannot handle de type {}", t),
            }
            .0;
            debug!("directory {} has attributes {}", name, attributes);

            fileinfo.file_attributes = attributes;

            if !dirinfo.append_to_buffer(buffer, &mut bytes_transferred) {
                break;
            }
            debug!("appended directory {}", name);
        }

        debug!("transferred {} bytes", bytes_transferred);
        Ok(bytes_transferred)
    }
}
