use ext4_lwext4::OpenFlags;

pub struct WinExtFile {
    pub path: String,
    pub flags: OpenFlags,
}

impl WinExtFile {
    pub fn new(path: &str, flags: OpenFlags) -> Self {
        WinExtFile {
            path: path.to_string(),
            flags,
        }
    }
}
