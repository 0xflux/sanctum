use core::{iter::once, ptr::null_mut};

use alloc::{vec, format, string::{String, ToString}, vec::Vec};
use shared_no_std::constants::SanctumVersion;
use wdk::println;
use wdk_sys::{ntddk::{KeGetCurrentIrql, RtlInitUnicodeString, RtlUnicodeStringToAnsiString, ZwClose, ZwCreateFile, ZwWriteFile}, FALSE, FILE_APPEND_DATA, FILE_ATTRIBUTE_NORMAL, FILE_OPEN_IF, FILE_SYNCHRONOUS_IO_NONALERT, GENERIC_WRITE, IO_STATUS_BLOCK, OBJECT_ATTRIBUTES, OBJ_CASE_INSENSITIVE, OBJ_KERNEL_HANDLE, PASSIVE_LEVEL, PHANDLE, POBJECT_ATTRIBUTES, STATUS_SUCCESS, STRING, UNICODE_STRING};

use crate::ffi::InitializeObjectAttributes;

#[derive(Debug)]
/// A custom error enum for the Sanctum driver
pub enum DriverError {
    NullPtr,
    DriverMessagePtrNull,
    LengthTooLarge,
    CouldNotDecodeUnicode,
    CouldNotEncodeUnicode,
    CouldNotSerialize,
    NoDataToSend,
    Unknown(String),
}

pub trait ToUnicodeString {
    fn to_unicode_string(&self) -> Option<UNICODE_STRING>;
}

impl ToUnicodeString for Vec<u16> {
    fn to_unicode_string(&self) -> Option<UNICODE_STRING> {
        create_unicode_string(self)
    }
}

impl ToUnicodeString for &str {
    fn to_unicode_string(&self) -> Option<UNICODE_STRING> {
        let v = self.to_u16_vec();
        create_unicode_string(&v)
    }
}

/// Creates a Windows API compatible unicode string from a u16 slice.
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
        Buffer: s.as_ptr() as *mut u16,
    })
}


pub trait ToU16Vec {
    fn to_u16_vec(&self) -> Vec<u16>;
}

impl ToU16Vec for &str {
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


/// Checks the compatibility of the driver and client versions based on major.minor.patch fields.
/// 
/// # Returns
/// 
/// True if compatible, false otherwise.
pub fn check_driver_version(client_version: &SanctumVersion) -> bool {

    // only compatible with versions less than 1
    if client_version.major >= 1 {
        return false;
    }

    true
}

/// Converts a UNICODE_STRING into a `String` (lossy) for printing.
///
/// # Errors
/// - `DriverError::NullPtr` if the input is null.
/// - `DriverError::LengthTooLarge` if the input exceeds `MAX_LEN`.
/// - `DriverError::Unknown` if the conversion fails.
pub fn unicode_to_string(input: *const UNICODE_STRING) -> Result<String, DriverError> {

    if input.is_null() {
        println!("[sanctum] [-] Null pointer passed to unicode_to_string.");
        return Err(DriverError::NullPtr);
    }

    let unicode = unsafe { &*input };

    // Allocate a heap buffer for the ANSI string with a size based on `unicode.Length`.
    let mut buf: Vec<i8> = vec![0; (unicode.Length + 1) as usize];
    let mut ansi = STRING {
        Length: 0,
        MaximumLength: (buf.len() + 1) as u16,
        Buffer: buf.as_mut_ptr(),
    };

    // convert the UNICODE_STRING to an ANSI string.
    let status = unsafe { RtlUnicodeStringToAnsiString(&mut ansi, unicode, FALSE as u8) };
    if status != STATUS_SUCCESS {
        println!("[sanctum] [-] RtlUnicodeStringToAnsiString failed with status {status}.");
        return Err(DriverError::Unknown(format!(
            "Conversion failed with status code: {status}"
        )));
    }

    // create the String
    let slice = unsafe { core::slice::from_raw_parts(ansi.Buffer as *const u8, ansi.Length as usize) };
    Ok(String::from_utf8_lossy(slice).to_string())
}

/// The interface for message logging. This includes both logging to a file in \SystemRoot\ and an interface
/// for logging to userland (for example, in the event where the system log fails, the userland logger may want to
/// log that event fail)
pub struct Log<'a> {
    log_path: &'a str,
}

pub enum LogLevel {
    Info,
    Warning,
    Success,
    Error,
}

impl<'a> Log<'a> {
    pub fn new() -> Self {
        Log {
            log_path: r"\SystemRoot\sanctum\sanctum_driver.log"
        }
    }

    /// Log kernel events / debug messages directly to the sanctum_driver.log file in
    /// \SystemRoot\sanctum\. This will not send any log messages to userland, other than when an error
    /// occurs writing to sanctum_driver.log
    /// 
    /// # Args
    /// - level: LogLevel - the level of logging required for the event
    /// - msg: &str - a formatted str to be logged
    pub fn log(&self, level: LogLevel, msg: &str) {
        //
        // Cast the log path as a Unicode string.
        // TODO: Move this to the constructor if InitializeObjectAttributes
        // doesn't modify the string. Consider RefCell for interior mutability.
        //
        let mut log_path_unicode = UNICODE_STRING::default();
        let src = self.log_path.encode_utf16().chain(once(0)).collect::<Vec<_>>();
        unsafe { RtlInitUnicodeString(&mut log_path_unicode, src.as_ptr()) };

        //
        // Initialise OBJECT_ATTRIBUTES
        //
        let mut oa: OBJECT_ATTRIBUTES = OBJECT_ATTRIBUTES::default();
        let result = unsafe {
            InitializeObjectAttributes(
                &mut oa,
                &mut log_path_unicode,
                OBJ_CASE_INSENSITIVE | OBJ_KERNEL_HANDLE,
                null_mut(),
                null_mut(),      
            )
        };
        if result.is_err() {
            println!("[sanctum] [-] Error calling InitializeObjectAttributes. No log event taking place..");
            return;
        }

        //
        // Do not perform file operations at higher IRQL levels
        //
        unsafe {
            if KeGetCurrentIrql() as u32 != PASSIVE_LEVEL {
                println!("[sanctum] [-] IRQL level too high to log event.");
                return;
            }
        }

        //
        // Create the driver log file if it doesn't already exist
        //
        let mut handle: PHANDLE = null_mut();
        let mut io_status_block = IO_STATUS_BLOCK::default();

        let result = unsafe {
            ZwCreateFile(
                &mut handle as *mut _ as *mut _,
                GENERIC_WRITE | FILE_APPEND_DATA,
                &mut oa,
                &mut io_status_block,
                null_mut(),
                FILE_ATTRIBUTE_NORMAL,
                0,
                FILE_OPEN_IF,
                FILE_SYNCHRONOUS_IO_NONALERT,
                null_mut(),
                0
            )
        };

        if result != STATUS_SUCCESS || handle.is_null() {
            println!("[sanctum] [-] result of ZwCreateFile was not success - result: {result}. Returning.");
            unsafe {
                if !handle.is_null() {
                    let _ = ZwClose(*handle);
                    println!("[sanctum] [+] Closed file handle");
                }
            }
            return;
        }
        
        //
        // Write data to the file
        //

        // convert the input message to a vector we can pass into the write file
        // heap allocating as the ZwWriteFile requires us to have a mutable pointer, so we
        // cannot use a &str.as_mut_ptr()
        let buf: Vec<u8> = msg.as_bytes().iter().cloned().collect();

        let result = unsafe {
            ZwWriteFile(
                handle as *mut _ as *mut _, 
                null_mut(),
                None, 
                null_mut(), 
                &mut io_status_block, 
                buf.as_ptr() as *mut _, 
                buf.len() as u32,
                null_mut(), // should be ignored due to flag FILE_APPEND_DATA
                null_mut(),
            )
        };

        if result != STATUS_SUCCESS {
            println!("[sanctum] [-] Failed writing file. Code: {result}");
            unsafe {
                if !handle.is_null() {
                    let _ = ZwClose(*handle);
                    println!("[sanctum] [+] Closed file handle");
                }
            }

            return;
        }

        // close the file handle
        unsafe {
            if !handle.is_null() {
                let _ = ZwClose(handle as *mut _);
                println!("[sanctum] [+] Closed file handle");
            }
        }

    }

    /// Send a message to userland from the kernel, via the DriverMessages feature
    pub fn log_to_userland() {
        
    }
}