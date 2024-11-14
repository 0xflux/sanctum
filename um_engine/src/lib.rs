#![feature(io_error_uncategorized)]

pub use filescanner::FileScannerState;
pub use driver_manager::DriverState;
pub use filescanner::{MatchedIOC, ScanResult, ScanType};
pub use settings::SanctumSettings;
pub use filescanner::ScanningLiveInfo;

mod engine;
mod driver_manager;
mod strings;
mod settings;
mod filescanner;
mod utils;
mod ipc_handler;