use std::iter::once;

/// Build a NUL-terminated UTF-16 string (windows-sys only accepts PCWSTR = *const u16).
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(once(0)).collect()
}

/// Scale a 96-dpi logical pixel value to the actual DPI (rounded).
pub fn scale(v: i32, dpi: u32) -> i32 {
    (v * dpi as i32 + 48) / 96
}
