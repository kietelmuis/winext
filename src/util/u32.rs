pub trait U32Ext {
    fn to_windows_time(self) -> u64;
}

impl U32Ext for u32 {
    fn to_windows_time(self) -> u64 {
        (self as u64 + 11644473600) * 10_000_000
    }
}
