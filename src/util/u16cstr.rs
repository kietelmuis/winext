use winfsp::U16CStr;

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
