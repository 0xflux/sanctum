use std::{io, path::PathBuf, time::Instant};

use driver_manager::SanctumDriverManager;
use filescanner::FileScanner;

mod driver_manager;
mod strings;
mod filescanner;

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
    driver_manager: SanctumDriverManager,   // the interface for managing the driver
    file_scanner: FileScanner,              // interface for the file scanning module
}

impl UmEngine {

    /// Initialises the usermode engine, ensuring the driver file exists in the image directory.
    pub fn new() -> Self {

        println!("[i] Sanctum usermode engine staring..");

        //
        // Config setup
        //

        // driver ma nager
        let mut driver_manager: SanctumDriverManager = SanctumDriverManager::new();

        // scanner module
        let scanner = FileScanner::new();
        if let Err(e) = scanner {
            panic!("[-] Failed to initialise scanner: {e}.");
        }
        let file_scanner = scanner.unwrap();


        UmEngine{
            driver_manager,
            file_scanner,
        }
    }


    /// Scans a single file as per the input filepath.
    pub fn scanner_scan_single_file(&self, target: PathBuf) -> Option<(String, PathBuf)>{
        let res = match self.file_scanner.scan_file_against_hashes(target) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[-] Scanner error: {e}");
                None
            },
        };

        res
    }
}


/// The main loop for accepting user input into the engine at the moment.
///
/// TODO this may need to be moved to its own thread in the future to allow the engine to
/// keep doing its thing whilst waiting on user input.
fn user_input_loop(
    driver_manager: &mut SanctumDriverManager,
    scanner: &FileScanner
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
                // scan a file against hashes
                let res = match scanner.scan_file_against_hashes(PathBuf::from("MALWARE.ps1")) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("[-] Scanner error: {e}");
                        None
                    },
                };

                if let Some(r) = res {
                    println!("[+] Malware found, Hash: {}, file name: {}", r.0, r.1.display());
                }
            }

            9 => {
                let now = Instant::now();
                // scan a folder for malware
                let scan_results = scanner.scan_from_folder_all_children(PathBuf::from("C:\\"));

                match scan_results {
                    Ok(results) => {
                        if !results.is_empty() {
                            println!("[+] Malware found: {:?}", results);
                        }
                    },
                    Err(e) => {
                        eprintln!("[-] Folder scan returned error: {e}");
                    }
                }

                let elapsed = now.elapsed().as_secs();
                println!("[i] Took: {elapsed} secs. Mins: {}", elapsed * 60);
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
