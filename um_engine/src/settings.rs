use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::utils::get_logged_in_username;

#[derive(Serialize, Deserialize)]
pub struct SanctumSettings {
    pub common_scan_areas: Vec<PathBuf>,
}

impl SanctumSettings {
    pub fn load() -> Self {
        let username = get_logged_in_username().unwrap();
        let base_path = format!("C:\\Users\\{username}\\AppData\\Roaming\\Sanctum\\");
        let dir = PathBuf::from(&base_path);
        let path = PathBuf::from(format!("{}\\config.cfg", base_path));

        // if the path doesn't exist, the app is likely running for the first time, so configure any app defaults
        let settings = if !dir.exists() {
            let settings = SanctumSettings {
                common_scan_areas: vec![
                    PathBuf::from(format!("C:\\Users\\{}", username)),
                    PathBuf::from("C:\\ProgramData"),
                    PathBuf::from("C:\\Temp"),
                    PathBuf::from("C:\\temp"),
                ],
            };

            let settings_string = serde_json::to_string(&settings).unwrap();
            fs::create_dir_all(&dir).expect("[-] Unable to create directory file.");
            fs::write(path, settings_string).expect("[-] Unable to write file.");

            settings
        } else {
            let settings = fs::read_to_string(path).expect("[-] Could not read settings file.");
            serde_json::from_str(&settings).unwrap()
        };

        settings
    }
}