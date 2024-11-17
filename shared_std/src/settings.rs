use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct SanctumSettings {
    pub common_scan_areas: Vec<PathBuf>,
}