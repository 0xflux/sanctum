// Constant literals (or types not part of the Windows API) for use across the project

// these should end with the same name
pub static NT_DEVICE_NAME: &str = "\\Device\\SanctumEDR";
pub static DOS_DEVICE_NAME: &str = "\\??\\SanctumEDR";
pub static DRIVER_UM_NAME: &str = "\\\\.\\SanctumEDR"; // \\.\ sets device namespace

pub static SYS_INSTALL_RELATIVE_LOC: &str = "sanctum.sys";
pub static SVC_NAME: &str = "Sanctum";

// version info
pub struct SanctumVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

pub static RELEASE_NAME: &str = "Sanctify";
pub static VERSION_DRIVER: SanctumVersion = SanctumVersion { major: 0, minor: 0, patch: 1 }; // 0.0.1 etc
pub static VERSION_CLIENT: SanctumVersion = SanctumVersion { major: 0, minor: 0, patch: 1 }; // 0.0.1 etc