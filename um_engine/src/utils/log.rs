use std::{fs::{self, OpenOptions}, io::Write, path::PathBuf};

use shared_no_std::constants::LOG_PATH;

use crate::settings::get_setting_paths;

use super::env::get_logged_in_username;

#[derive(Debug)]
pub struct Log {
    log_file_path: PathBuf,
}

pub enum LogLevel {
    Info,
    Warning,
    Success,
    Error,
}

impl Log {
    pub fn new() -> Self {
        //
        // check for log file
        //
        let username = get_logged_in_username().unwrap();
        let mut log_path = get_setting_paths(&username).0;
        log_path.push(LOG_PATH);
        let log_dir = match log_path.parent() {
            Some(dir) => dir,
            None => panic!("[fatal] Could not get parent of log paths"),
        };

        if !log_dir.exists() {
            fs::create_dir_all(log_dir).expect("[-] Unable to create directory file.");
            fs::write(&log_path, "").expect("[-] Unable to write log file.");
        }
        
        Log {
            log_file_path: get_log_file_path(),
        }
    }

    /// Logs the message and panics.
    /// 
    /// # Warning
    /// This function does not return and will panic.
    #[track_caller]
    pub fn panic(&self, msg: &str) -> ! {
        // open the file
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file_path)
            .expect(format!("[fatal] Unable to open log file: {}", self.log_file_path.display()).as_str());

        // write to the file
        writeln!(file, "{}", msg)
                .expect("Unable to write to log file");
            
        panic!("[fatal] {}", msg);
    }

    /// Log messages to the log file defined in the applications constant strings.
    pub fn log(&self, level: LogLevel, msg: &str) {
        // open the file
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file_path)
            .expect(format!("[fatal] Unable to open log file: {}", self.log_file_path.display()).as_str());

        // write to the file
        writeln!(file, "{}", msg)
                .expect("Unable to write to log file");

        // console log the message
        match level {
            LogLevel::Info => println!("[i] {}", msg),
            LogLevel::Warning => println!("[w] {}", msg),
            LogLevel::Success => println!("[+] {}", msg),
            LogLevel::Error => println!("[e] {}", msg),
        }
    }
}

/// Gets the fully qualified path to the log file for Sanctum
pub fn get_log_file_path() -> PathBuf {
    let username = get_logged_in_username().unwrap();
    let mut sanctum_app_data_path = get_setting_paths(&username).0; // .0 will give us the folder
    sanctum_app_data_path.push(LOG_PATH);
    sanctum_app_data_path
}