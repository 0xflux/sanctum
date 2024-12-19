#![allow(dead_code)]

use shared_std::{driver_manager::DriverState, file_scanner::{FileScannerState, ScanningLiveInfo}, settings::SanctumSettings};
use std::{fs, path::PathBuf, sync::{Arc, Mutex}};
use crate::{driver_manager::SanctumDriverManager, settings::SanctumSettingsImpl, utils::{env::get_logged_in_username, log::{Log, LogLevel}}};
use crate::filescanner::FileScanner;
use crate::settings::get_setting_paths;

// todo - decommission UsermodeAPI and split any functionality into the modules.
pub struct UsermodeAPI {
    pub driver_manager: Arc<Mutex<SanctumDriverManager>>,   // the interface for managing the driver
    pub file_scanner: FileScanner,
    pub sanctum_settings: Arc<Mutex<SanctumSettings>>,
    pub log: Log, // for logging events
}

impl UsermodeAPI {

    /// Initialises the usermode engine, ensuring the driver file exists in the image directory.
    pub async fn new() -> Self {

        let log = Log::new();

        log.log(LogLevel::Info, "Sanctum usermode engine staring..");

        //
        // Config setup
        //

         // settings and environment
         let sanctum_settings = Arc::new(Mutex::new(SanctumSettings::load()));

        // driver manager
        let driver_manager = Arc::new(Mutex::new(SanctumDriverManager::new()));

        // scanner module
        let scanner = FileScanner::new().await;
        if let Err(e) = scanner {
            panic!("[-] Failed to initialise scanner: {e}.");
        }
        let file_scanner = scanner.unwrap();

        UsermodeAPI{
            driver_manager,
            file_scanner,
            sanctum_settings,
            log,
        }
    }


    /// Public entrypoint for scanning, taking in a target file / folder, and the scan type.
    /// 
    /// This function ensures all state is accurate for whether a scan is in progress etc.
    /// 
    /// # Returns
    /// 
    /// The function will return the enum ScanResult which 'genericifies' the return type to give flexibility to 
    /// allowing the function to conduct different types of scan. This will need checking in the calling function.
    pub fn scanner_start_scan(&self, target: Vec<PathBuf>) -> FileScannerState {
        
        // check whether a scan is active
        if self.file_scanner.is_scanning() {
            return FileScannerState::Scanning;
        }

        self.file_scanner.scan_started(); // update state

        // send the job for a scan
        let result = self.file_scanner.begin_scan(target);

        self.file_scanner.end_scan(); // update state

        let result = match result {
            Ok(state) => state,
            Err(e) => {
                FileScannerState::FinishedWithError(e.to_string())
            },
        };

        result
    }


    /// Instructs the scanner to cancel its scan, returning information about the results
    pub fn scanner_cancel_scan(&self) {
        self.file_scanner.cancel_scan();
    }


    /// Gets the state of the scanner module
    pub fn scanner_get_state(&self) -> FileScannerState {
        self.file_scanner.get_state()
    }


    pub fn scanner_get_scan_data(&self) -> ScanningLiveInfo {
        self.file_scanner.scanning_info.lock().unwrap().clone()
    }


    //
    // Settings
    // 

    pub fn settings_get_common_scan_areas(&self) -> Vec<PathBuf> {
        let lock = self.sanctum_settings.lock().unwrap();
        lock.common_scan_areas.clone()
    }

    pub fn settings_update_settings(&self, settings: SanctumSettings) {
        // change the live state
        let mut lock = self.sanctum_settings.lock().unwrap();
        *lock = settings.clone();

        // write the new file
        let settings_str = serde_json::to_string(&settings).unwrap();
        let path = get_setting_paths(&get_logged_in_username().unwrap()).1;
        fs::write(path, settings_str).unwrap();
    }


    //
    // Driver controls
    //

    /// Public API for installing the driver on the host machine
    /// 
    /// # Returns
    /// 
    /// The state of the driver after initialisation
    pub fn driver_install_driver(&self) -> DriverState {
        let mut lock = self.driver_manager.lock().unwrap();
        lock.install_driver();
        lock.get_state()
    }
    
    pub fn driver_uninstall_driver(&self) -> DriverState {
        let mut lock = self.driver_manager.lock().unwrap();
        lock.uninstall_driver();
        lock.get_state()
    }

    pub fn driver_start_driver(&self) -> DriverState {
        let mut lock = self.driver_manager.lock().unwrap();
        lock.start_driver();
        lock.get_state()
    }

    pub fn driver_stop_driver(&self) -> DriverState {
        let mut lock = self.driver_manager.lock().unwrap();
        lock.stop_driver();
        lock.get_state()
    }

    pub fn driver_get_state(&self) -> DriverState {
        let lock = self.driver_manager.lock().unwrap();
        lock.get_state()
    }


    //
    // IOCTLS
    //
    pub fn ioctl_ping_driver(&self) -> String {
        let mut lock = self.driver_manager.lock().unwrap();
        lock.ioctl_ping_driver()
    }
}