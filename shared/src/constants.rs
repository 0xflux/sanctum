// Constant literals (or types not part of the Windows API) for use across the project

// these should end with the same name
pub static NT_DEVICE_NAME: &str = "\\Device\\SanctumEDR";
pub static DOS_DEVICE_NAME: &str = "\\DosDevices\\SanctumEDR";
pub static DRIVER_UM_NAME: &str = "\\\\.\\SanctumEDR"; // \\.\ sets device namespace

pub static SYS_INSTALL_RELATIVE_LOC: &str = "sanctum.sys";
pub static SVC_NAME: &str = "Sanctum";

// version info
pub static RELEASE_NAME: &str = "Sanctify";
pub static USERMODE_VER: &str = "0.0.1";
pub static DRIVER_VER: &str = "0.0.1";