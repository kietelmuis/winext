use winfsp::U16CStr;

pub trait U16CStrExt {
    fn sanitize(&self) -> String;
}

impl U16CStrExt for U16CStr {
    fn sanitize(&self) -> String {
        let raw = self.to_string_lossy();
        let mut s = raw.replace('\\', "/");
        s = s.trim_end_matches('\0').to_string();
        while s.contains("//") {
            s = s.replace("//", "/");
        }
        if s.is_empty() || s == "/" {
            return "/".to_string();
        }
        if !s.starts_with('/') {
            s.insert(0, '/');
        }
        if s.len() > 1 {
            s = s.trim_end_matches('/').to_string();
        }
        s
    }
}
