use std::{io, path::PathBuf};

use driver_manager::SanctumDriverManager;
use filescanner::FileScanner;

mod driver_manager;
mod strings;
mod filescanner;

fn main() {
    println!("[i] Sanctum usermode engine staring..");

    // init the driver manager
    let mut driver_manager: SanctumDriverManager = SanctumDriverManager::new();

    //
    // Loop through the menu until the user has selected exit
    // if exit is selected, then return out of main.
    //
    if user_input_loop(&mut driver_manager).is_none() {
        return;
    };

    // TO PORT FROM C :)
}

/// The main loop for accepting user input into the engine at the moment.
///
/// TODO this may need to be moved to its own thread in the future to allow the engine to
/// keep doing its thing whilst waiting on user input.
fn user_input_loop(driver_manager: &mut SanctumDriverManager) -> Option<()> {
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
        println!("[8] Get file hash of hardcoded file.");

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
                let scanner = FileScanner::from(PathBuf::from("MALWARE.ps1")).unwrap();
                let res = match scanner.scan_against_hashes() {
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

            _ => {
                eprintln!("[-] Unhandled command.");
                println!();
                continue;
            }
        }

        println!();
    }
}
