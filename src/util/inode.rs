use ext4_rs::InodeFileType;
use windows::Win32::Storage::FileSystem::*;

pub fn inode_type_to_windows(file_type: InodeFileType) -> FILE_FLAGS_AND_ATTRIBUTES {
    let bits = file_type.bits();

    if bits & 0x4000 != 0 {
        // S_IFDIR
        FILE_ATTRIBUTE_DIRECTORY
    } else if bits & 0x8000 != 0 {
        // S_IFREG
        FILE_ATTRIBUTE_NORMAL
    } else if bits & 0x2000 != 0 {
        // S_IFCHR
        FILE_ATTRIBUTE_NORMAL
    } else {
        FILE_ATTRIBUTE_NORMAL // fallback
    }
}

pub fn windows_to_inode_type(attr: u32) -> InodeFileType {
    if attr & FILE_ATTRIBUTE_DIRECTORY.0 != 0 {
        InodeFileType::S_IFDIR
    } else {
        InodeFileType::S_IFREG
    }
}
