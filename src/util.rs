use winfsp::U16CStr;

pub trait U32Ext {
    fn to_windows_time(self) -> u64;
}

impl U32Ext for u32 {
    fn to_windows_time(self) -> u64 {
        (self as u64 + 11644473600) * 10_000_000
    }
}

pub trait U16CStrExt {
    fn sanitize(&self) -> String;
}

impl U16CStrExt for U16CStr {
    fn sanitize(&self) -> String {
        self.to_string_lossy()
            .replace('\\', "/")
            .trim_end_matches('\0')
            .to_string()
    }
}
