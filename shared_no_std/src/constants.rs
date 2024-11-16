//! Constant literals (or types not part of the Windows API) for use across the project

use core::fmt::Display;

// these should end with the same name
pub static NT_DEVICE_NAME: &str = "\\Device\\SanctumEDR";
pub static DOS_DEVICE_NAME: &str = "\\??\\SanctumEDR";
pub static DRIVER_UM_NAME: &str = "\\\\.\\SanctumEDR"; // \\.\ sets device namespace

pub static SYS_INSTALL_RELATIVE_LOC: &str = "sanctum.sys";
pub static SVC_NAME: &str = "Sanctum";
pub static PIPE_NAME: &'static str = r"\\.\pipe\sanctum_um_engine_pipe";
pub static PIPE_NAME_FOR_DRIVER: &str = r"\??\pipe\sanctum_um_engine_pipe";

//
// version info
//
pub struct SanctumVersion<'a> {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub name: &'a str,
}

pub static RELEASE_NAME: &str = "Sanctify";
pub static VERSION_DRIVER: SanctumVersion = SanctumVersion { major: 0, minor: 0, patch: 2, name: "Light's Resolve" };
pub static VERSION_CLIENT: SanctumVersion = SanctumVersion { major: 0, minor: 0, patch: 2, name: "Light's Resolve"};

impl<'a> Display for SanctumVersion<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}.{} - {}", self.major, self.minor, self.patch, self.name)
    }
}

//
// Usermode specific constants
//
pub static SANC_SYS_FILE_LOCATION: &str = "Sanctum\\sanctum.sys";
pub static IOC_LIST_LOCATION: &str = "Sanctum\\ioc_list.txt";
