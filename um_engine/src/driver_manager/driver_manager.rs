//! The main setup and more general functions for the driver manager module for the usermode engine

use std::{os::windows::ffi::OsStrExt, path::PathBuf};
use shared_no_std::constants::{DRIVER_UM_NAME, SANC_SYS_FILE_LOCATION, SVC_NAME};
use shared_std::driver_manager::DriverState;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{CloseHandle, GetLastError, HANDLE},
        Storage::FileSystem::{GetFileAttributesW, INVALID_FILE_ATTRIBUTES},
    },
};

use crate::{strings::ToUnicodeString, utils::log::Log};

/// The SanctumDriverManager holds key information to be shared between
/// modules which relates to uniquely identifiable attributes such as its name
/// and other critical settings.
/// 
/// # Safety
/// 
/// The structure implements Send and Sync for the Handle stored in DriverHandleRaii. This should be safe as all accesses to the driver handle
/// will live for the lifetime of the object. If the handle could be null, the wrapping Option **should** be None.
pub struct SanctumDriverManager {
    pub device_um_symbolic_link_name: Vec<u16>,
    pub(super) svc_path: Vec<u16>,
    pub(super) svc_name: Vec<u16>,
    pub handle_via_path: DriverHandleRaii,
    pub state: DriverState,
    pub log: Log,
}


impl SanctumDriverManager {
    /// Generate a new instance of the driver manager, which initialises the device name path and symbolic link path
    pub fn new() -> SanctumDriverManager {
        //
        // Generate the UNICODE_STRING values for the device and symbolic name
        //
        let device_um_symbolic_link_name = DRIVER_UM_NAME.to_u16_vec();
        let log = Log::new();

        let appdata = match std::env::var("APPDATA") {
            Ok(a) => a,
            Err(e) => log.panic(&format!("Could not find App Data folder in environment variables. {e}")),
        };
        let sys_file_path: Vec<u16> = PathBuf::from(appdata).join(SANC_SYS_FILE_LOCATION).as_os_str().encode_wide().chain(std::iter::once(0)).collect();

        let svc_name = SVC_NAME.to_u16_vec();

        // check the sys file exists
        // todo this eventually should be in the actual install directory under Windows
        let x = unsafe { GetFileAttributesW(PCWSTR::from_raw(sys_file_path.as_ptr())) };
        if x == INVALID_FILE_ATTRIBUTES {
            panic!("[-] Cannot find sanctum.sys. Err: {}. Ensure the driver file is at: {:?}", unsafe {
                GetLastError().0
            }, sys_file_path);
        }

        let mut instance = SanctumDriverManager {
            device_um_symbolic_link_name,
            svc_path: sys_file_path,
            svc_name,
            handle_via_path: DriverHandleRaii::default(), // sets to None
            state: DriverState::Uninstalled("".to_string()),
            log,
        };

        // attempt an install of the driver
        instance.install_driver();

        // attempt to initialise a handle to the driver, this may silently fail - and will do so in the case
        // where the driver is not yet installed (or has been uninstalled)
        if instance.init_handle_via_registry() {
            instance.state = DriverState::Started("".to_string());
        }

        instance
    }

    pub fn get_state(&self) -> DriverState {
        self.state.clone()
    }
}

impl Default for SanctumDriverManager {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Send for SanctumDriverManager {}
unsafe impl Sync for SanctumDriverManager {}

pub struct DriverHandleRaii {
    pub handle: Option<HANDLE>,
}

impl Default for DriverHandleRaii {
    fn default() -> Self {
        Self { handle: None }
    }
}

impl Drop for DriverHandleRaii {
    fn drop(&mut self) {
        if self.handle.is_some() && !self.handle.unwrap().is_invalid() {
            let _ = unsafe { CloseHandle(self.handle.unwrap()) };
            self.handle = None;
        }
    }
}


// /// Gets the path to the .sys file on the target device, for the time being this needs to be
// /// located in the same folder as where this usermode exe is run from.
// fn get_sys_file_path() -> Vec<u16> {
//     //
//     // A little long winded, but construct the path as a PCWSTR to where the sys driver is
//     // this should be bundled into the same location as where the usermode exe is.
//     //
//     let mut svc_path: Vec<u16> = vec![0u16; MAX_PATH as usize];
//     let len = unsafe { GetModuleFileNameW(None, &mut svc_path) };
//     if len == 0 {
//         eprintln!(
//             "[-] Error getting path of module. Win32 Error: {}",
//             unsafe { GetLastError().0 }
//         );
//     } else if len >= MAX_PATH {
//         panic!("[-] Path of module is too long. Run from a location with a shorter path.");
//     }

//     svc_path.truncate(len as usize - 11); // remove um_engine.sys\0
//     svc_path.append(&mut SYS_INSTALL_RELATIVE_LOC.to_u16_vec()); // append the .sys file

//     svc_path
// }
