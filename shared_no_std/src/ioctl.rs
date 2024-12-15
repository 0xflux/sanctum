// This file will contain definitions of IOCTLs and definitions of any structures related directly
// to IOCTL message passing

use serde::{Deserialize, Serialize};

use crate::driver_ipc::{ProcessStarted, ProcessTerminated};

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
    };
}

// ****************** IOCTL DEFINITIONS ******************

// general communication
pub const SANC_IOCTL_PING: u32 =
    CTL_CODE!(FILE_DEVICE_UNKNOWN, 0x800, METHOD_BUFFERED, FILE_ANY_ACCESS);

pub const SANC_IOCTL_PING_WITH_STRUCT: u32 =
    CTL_CODE!(FILE_DEVICE_UNKNOWN, 0x801, METHOD_BUFFERED, FILE_ANY_ACCESS);

pub const SANC_IOCTL_CHECK_COMPATIBILITY: u32 =
    CTL_CODE!(FILE_DEVICE_UNKNOWN, 0x802, METHOD_BUFFERED, FILE_ANY_ACCESS);

pub const SANC_IOCTL_DRIVER_GET_MESSAGES: u32 =
    CTL_CODE!(FILE_DEVICE_UNKNOWN, 0x803, METHOD_BUFFERED, FILE_ANY_ACCESS);

pub const SANC_IOCTL_DRIVER_GET_MESSAGE_LEN: u32 =
    CTL_CODE!(FILE_DEVICE_UNKNOWN, 0x804, METHOD_BUFFERED, FILE_ANY_ACCESS);


// ****************** IOCTL MSG STRUCTS ******************

/// Response to a hello ping from usermode, indicates whether the data was received, and the driver
/// will respond with its current version.
pub struct SancIoctlPing {
    pub received: bool,
    pub version: [u8; SANC_IOCTL_PING_CAPACITY],
    pub str_len: usize,
    pub capacity: usize,
}

/// The capacity maximum for the u8 buffer for the ping protocol
const SANC_IOCTL_PING_CAPACITY: usize = 256;

impl SancIoctlPing<> {
    /// Create aa new instance of the object with default values
    pub fn new() -> SancIoctlPing {
        SancIoctlPing {
            received: false,
            version: [0; SANC_IOCTL_PING_CAPACITY],
            str_len: 0,
            capacity: SANC_IOCTL_PING_CAPACITY,
        }
    }
}

impl Default for SancIoctlPing<> {
     fn default() -> Self {
         Self::new()
     }
 }

 /// The actual type within DriverMessagesWithMutex which contains the data.
/// 
/// # Warning
/// This struct definition is NOT shared between the driver and usermode code due to
/// the requirement for Vec (alloc vs std). Therefore, this should be manually defined and
/// updated in the usermode code as it will use a different allocator.
/// 
/// # Warning
/// When adding new fields to this, ensure you also update BOTH the .append sections and 
/// serde_json::to_vec in `add_existing_queue` for the data to properly be sent to userland.
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct DriverMessages {
    pub messages: alloc::vec::Vec<alloc::string::String>,
    pub process_creations: alloc::vec::Vec<ProcessStarted>,
    pub process_terminations: alloc::vec::Vec<ProcessTerminated>,
}