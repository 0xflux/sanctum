#![feature(io_error_uncategorized)]
use std::{fs, io, path::PathBuf, sync::{Arc, Mutex}};

use driver_manager::SanctumDriverManager;
use filescanner::{FileScanner, ScanningLiveInfo};
pub use filescanner::{MatchedIOC, ScanResult, ScanType, State};
use settings::get_setting_paths;
pub use settings::SanctumSettings;
use utils::get_logged_in_username;

mod driver_manager;
mod strings;
mod settings;
mod filescanner;
mod utils;

/// The public API for the usermode engine which will run inside the Tauri GUI application.
/// At present this interface does not hold state, and is used as a singleton in order to instruct the 
/// engine to conduct actions on behalf of the user.
/// 
/// This interface also blocks, and is not async yet but the plan will be either to make it async or
/// run certain functions in their own threads. 
/// 
/// # API naming conventions
/// 
/// - scanner_ => Any functionality for file scanning etc shall be prefixed with scanner_
/// - driver_ => Any functionality for driver interaction shall be prefixed with driver_
pub struct UmEngine {
    pub driver_manager: SanctumDriverManager,   // the interface for managing the driver
    pub file_scanner: FileScanner,
    pub sanctum_settings: Arc<Mutex<SanctumSettings>>,
}

impl UmEngine {

    /// Initialises the usermode engine, ensuring the driver file exists in the image directory.
    pub fn new(sanctum_settings: SanctumSettings) -> Self {

        println!("[i] Sanctum usermode engine staring..");

        //
        // Config setup
        //

        // driver manager
        let driver_manager: SanctumDriverManager = SanctumDriverManager::new();

        // scanner module
        let scanner = FileScanner::new();
        if let Err(e) = scanner {
            panic!("[-] Failed to initialise scanner: {e}.");
        }
        let file_scanner = scanner.unwrap();

        // settings
        let sanctum_settings = Arc::new(Mutex::new(sanctum_settings));

        UmEngine{
            driver_manager,
            file_scanner,
            sanctum_settings,
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
    pub fn scanner_start_scan(&self, target: Vec<PathBuf>) -> State {
        
        // check whether a scan is active
        if self.file_scanner.is_scanning() {
            return State::Scanning;
        }

        self.file_scanner.scan_started(); // update state

        // send the job for a scan
        let result = self.file_scanner.begin_scan(target);

        self.file_scanner.end_scan(); // update state

        let result = match result {
            Ok(state) => state,
            Err(e) => {
                State::FinishedWithError(e.to_string())
            },
        };

        result
    }


    /// Instructs the scanner to cancel its scan, returning information about the results
    pub fn scanner_cancel_scan(&self) {
        self.file_scanner.cancel_scan();
    }


    /// Gets the state of the scanner module
    pub fn scanner_get_state(&self) -> State {
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
}


/// The main loop for accepting user input into the engine at the moment.
///
/// TODO this may need to be moved to its own thread in the future to allow the engine to
/// keep doing its thing whilst waiting on user input.
#[allow(dead_code)]
fn user_input_loop(
    driver_manager: &mut SanctumDriverManager,
) -> Option<()> {
    loop {
        println!("Make your selection below:");
        println!("------------------------------");
        println!("[1] Exit.");
        println!("[2] Install driver.");
        println!("[3] Uninstall driver.");
        println!("[4] Start driver.");
        println!("[5] Stop driver.");
        println!("[6] Ping driver and get string response.");
        println!("[7] Ping driver with a struct.");
        println!("[8] Scan file for malware.");
        println!("[9] Scan directory for malware.");

        let mut selection = String::new();
        if io::stdin().read_line(&mut selection).is_err() {
            eprintln!("[-] Error reading value from command line.");
            println!();
            continue;
        };

        let selection: i32 = if let Ok(s) = selection.trim().parse() {
            s
        } else {
            eprintln!("[-] Error parsing selection as int.");
            println!();
            continue;
        };

        match selection {
            1 => {
                // exit application
                return None;
            }
            2 => {
                // install driver
                driver_manager.install_driver();
            }
            3 => {
                // uninstall
                driver_manager.uninstall_driver();
            }
            4 => {
                // start driver
                driver_manager.start_driver();
            }
            5 => {
                // stop the driver
                driver_manager.stop_driver();
            }
            6 => {
                // ping the driver
                driver_manager.ioctl_ping_driver();
            },
            7 => {
                driver_manager.ioctl_ping_driver_w_struct();
            },

            8 => {
                // // scan a file against hashes
                // let res = match scanner.scan_file_against_hashes(PathBuf::from("MALWARE.ps1")) {
                //     Ok(v) => v,
                //     Err(e) => {
                //         eprintln!("[-] Scanner error: {e}");
                //         None
                //     },
                // };

                // if let Some(r) = res {
                //     println!("[+] Malware found, Hash: {}, file name: {}", r.0, r.1.display());
                // }
            }

            9 => {
                // let now = Instant::now();
                // // scan a folder for malware
                // let scan_results = scanner.scan_from_folder_all_children(PathBuf::from("C:\\"));

                // match scan_results {
                //     Ok(results) => {
                //         if !results.is_empty() {
                //             println!("[+] Malware found: {:?}", results);
                //         }
                //     },
                //     Err(e) => {
                //         eprintln!("[-] Folder scan returned error: {e}");
                //     }
                // }

                // let elapsed = now.elapsed().as_secs();
                // println!("[i] Took: {elapsed} secs. Mins: {}", elapsed * 60);
            }

            _ => {
                eprintln!("[-] Unhandled command.");
                println!();
                continue;
            }
        }

        println!();
    }
}