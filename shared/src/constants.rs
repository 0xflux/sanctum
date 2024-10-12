// Constant literals (or types not part of the Windows API) for use across the project

pub static DEVICE_NAME_PATH: &str = "\\Device\\SanctumEDR";
pub static SYMBOLIC_NAME_PATH: &str = "\\??\\SanctumEDR";
pub static SYS_INSTALL_RELATIVE_LOC: &str = "sanctum.sys";
pub static SVC_NAME: &str = "Sanctum";

// version info
pub static RELEASE_NAME: &str = "Sanctify";
pub static USERMODE_VER: &str = "0.0.1";
pub static DRIVER_VER: &str = "0.0.1";