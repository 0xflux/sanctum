// IOCTL definitions

extern crate alloc;

// definitions to prevent importing the windows crate
const FILE_DEVICE_UNKNOWN: u32 = 34u32;
const METHOD_NEITHER: u32 = 3u32;
const METHOD_BUFFERED: u32 = 0u32;
const FILE_ANY_ACCESS: u32 = 0u32;

/// A macro to generate a control code. 
macro_rules! CTL_CODE {
    ($DeviceType:expr, $Function:expr, $Method:expr, $Access:expr) => {
        ($DeviceType << 16) | ($Access << 14) | ($Function << 2) | $Method
    }
}

// general communication
pub const SANC_IOCTL_PING: u32 = CTL_CODE!(FILE_DEVICE_UNKNOWN, 0x800, METHOD_NEITHER, FILE_ANY_ACCESS);