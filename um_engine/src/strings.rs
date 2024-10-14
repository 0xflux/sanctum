// usermode implementation of utilities

use std::error::Error;

use windows::{
    core::{PCWSTR, PWSTR},
    Win32::Foundation::UNICODE_STRING,
};

pub trait ToUnicodeString {
    fn to_u16_vec(&self) -> Vec<u16>;
}

impl ToUnicodeString for &str {
    fn to_u16_vec(&self) -> Vec<u16> {
        // reserve space for null terminator
        let mut buf = Vec::with_capacity(self.len() + 1);

        // iterate over each char and push the UTF-16 to the buf
        for c in self.chars() {
            let mut c_buf = [0; 2];
            let encoded = c.encode_utf16(&mut c_buf);
            buf.extend_from_slice(encoded);
        }

        buf.push(0); // add null terminator
        buf
    }
}

pub trait ToWindowsUnicodeString {
    fn to_windows_unicode_string(&self) -> Option<UNICODE_STRING>;
}

impl ToWindowsUnicodeString for Vec<u16> {
    fn to_windows_unicode_string(&self) -> Option<UNICODE_STRING> {
        create_unicode_string(self)
    }
}

/// Creates a Windows API compatible
/// unicode string from a u16 slice.
///
///
/// <h1>Returns</h1>
/// Returns an option UNICODE_STRING, if the len of the input string is 0 then
/// the function will return None.
pub fn create_unicode_string(s: &Vec<u16>) -> Option<UNICODE_STRING> {
    //
    // Check the length of the input string is greater than 0, if it isn't,
    // we will return none
    //
    let len = if s.len() > 0 {
        s.len()
    } else {
        return None;
    };

    //
    // Windows docs specifies for UNICODE_STRING:
    //
    // param 1 - length, Specifies the length, in bytes, of the string pointed to by the Buffer member,
    // not including the terminating NULL character, if any.
    //
    // param 2 - max len, Specifies the total size, in bytes, of memory allocated for Buffer. Up to
    // MaximumLength bytes may be written into the buffer without trampling memory.
    //
    // param 3 - buffer, Pointer to a wide-character string
    //
    // Therefore, we will do the below check to remove the null terminator from the len

    let len_checked = if len > 0 && s[len - 1] == 0 {
        len - 1
    } else {
        len
    };

    Some(UNICODE_STRING {
        Length: (len_checked * 2) as u16,
        MaximumLength: (len * 2) as u16,
        Buffer: PWSTR::from_raw(s.as_ptr() as *mut u16),
    })
}

/// Converts a PCWSTR to a String
///
/// # Safety
///
/// Dereferences the raw pointer of the PCWSTR
pub unsafe fn pcwstr_to_string(s: PCWSTR) -> Result<String, Box<dyn Error>> {
    if s.is_null() {
        return Err("PCWSTR was null.".into());
    }

    let mut len = 0;
    while *s.0.add(len) != 0 {
        len += 1;
    }

    Ok(String::from_utf16_lossy(std::slice::from_raw_parts(
        s.0, len,
    )))
}

/// Converts a PWSTR to a String
///
/// # Safety
///
/// Dereferences the raw pointer of the PWSTR
pub unsafe fn pwstr_to_string(s: PWSTR) -> Result<String, Box<dyn Error>> {
    if s.is_null() {
        return Err("PCWSTR was null.".into());
    }

    let mut len = 0;
    while *s.0.add(len) != 0 {
        len += 1;
    }

    Ok(String::from_utf16_lossy(std::slice::from_raw_parts(
        s.0, len,
    )))
}
