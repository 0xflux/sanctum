use std::{io, path::PathBuf, time::Duration};

use serde::{Deserialize, Serialize};

/// The state of the scanner either Scanning or Inactive. If the scanner is scanning, then it contains
/// further information about the live-time information such as how many files have been scanned and time taken so far.
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum FileScannerState {
    Scanning,
    Finished,
    FinishedWithError(String),
    Inactive,
    Cancelled,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum ScanType {
    File,
    Folder,
}

pub enum ScanResult {
    Results(Result<Vec<MatchedIOC>, io::Error>),
    ScanInProgress,
}


/// Structure for containing results pertaining to an IOC match
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct MatchedIOC {
    pub hash: String,
    pub file: PathBuf,
}

/// Live time information about the current scan
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct ScanningLiveInfo {
    pub num_files_scanned: u128,
    pub time_taken: Duration,
    pub scan_results: Vec<MatchedIOC>,
}