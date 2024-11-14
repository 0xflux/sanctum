use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::utils::get_logged_in_username;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct SanctumSettings {
    pub common_scan_areas: Vec<PathBuf>,
}

impl SanctumSettings {
    pub fn load() -> Self {
        let username = get_logged_in_username().unwrap();
        let paths = get_setting_paths(&username);
        let dir = paths.0;
        let path = paths.1;

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


    /// Update the settings fields in place
    pub fn update_settings(&mut self, settings: SanctumSettings) -> Self{
        // update self fields in memory
        self.common_scan_areas = settings.clone().common_scan_areas;

        // write new file to disk
        let settings_str = serde_json::to_string(&settings).unwrap();
        let path = get_setting_paths(&get_logged_in_username().unwrap()).1;
        fs::write(path, settings_str).unwrap();

        self.clone()
    }
}

 /// Get the base path and file name of the settings file, from the AppData folder.
 pub fn get_setting_paths(username: &String) -> (PathBuf, PathBuf) {
    let base_path = format!("C:\\Users\\{username}\\AppData\\Roaming\\Sanctum\\");
    let dir = PathBuf::from(&base_path);
    let path = PathBuf::from(format!("{}\\config.cfg", base_path));

    (dir, path)
}