use std::{ffi::CStr, sync::Arc, thread::sleep, time::Duration};

use shared_no_std::{driver_ipc::ProcessStarted, ioctl::DriverMessages};
use windows::Win32::{Foundation::{CloseHandle, GetLastError}, System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPALL}};

use crate::{engine::UmEngine, utils::log::{Log, LogLevel}};

use super::process_monitor::ProcessMonitor;

pub struct Core {
    driver_poll_rate: u64,
}


impl Core {
    /// Starts the core of the usermode engine; kicking off the frequent polling of the 
    pub async fn start_core(engine: Arc<UmEngine>) -> ! {

        println!("Core starting");

        // create a local self contained instance of Core, as we don't need to instantiate 
        // the core outside of this entry function
        let core = Core {
            driver_poll_rate: 50,
        };

        let mut processes = ProcessMonitor::new();

        let logger = Log::new();

        //
        // To start with, we will snapshot all running processes and then add them to the active processes.
        // there is possible a short time window where processes are created / terminated, which may cause
        // a zone of 'invisibility' at this point in time, but this should be fixed in the future when
        // we receive handles / changes to processes, if they don't exist, they should be created then.
        // todo - marker for info re above.
        //
        let snapshot_processes = snapshot_all_processes();

        // extend the newly created local processes type from the results of the snapshot
        processes.extend_processes(snapshot_processes);
        

        //
        // Enter the polling & decision making loop, this here is the core / engine of the usermode engine.
        //
        loop {
            // contact the driver and get any messages from the kernel 
            let driver_response = {
                let mut mtx = engine.driver_manager.lock().unwrap();
                mtx.ioctl_get_driver_messages()
            };
            
            //
            // If we have new message(s) from the driver, process them in userland as appropriate 
            //
            if driver_response.is_some() {
                // first deal with process terminations to prevent trying to add to an old process id if there is a duplicate
                let driver_messages = driver_response.unwrap();
                let process_terminations = driver_messages.process_terminations;
                if !process_terminations.is_empty() {
                    for t in process_terminations {
                        processes.remove_process(t.pid);
                    }
                }

                // add a new process to the running process hashmap
                let process_creations = driver_messages.process_creations;
                if !process_creations.is_empty() {
                    for p in process_creations {
                        if processes.insert(&p).is_err() {
                            logger.log(LogLevel::Error, &format!("Failed to add new process to live processes. Process: {:?}", p));
                        }
                    }
                }

                // cache messages 
                // add process creations to a hashmap (ProcessMonitor struct)

                /*
                    todo long term: 
                        - thread creation 
                        - handle requests
                        - change of handle type (e.g. trying to evade detection)
                        - is the process doing bad things itself (allocating foreign mem)
                        
                    ^ to the abv hashmap
                */
            }

            sleep(Duration::from_millis(core.driver_poll_rate));
        }
    }

}

/// Enumerate all processes and add them to the active process monitoring hashmap.
fn snapshot_all_processes() -> ProcessMonitor {

    let logger = Log::new();
    let mut all_processes = ProcessMonitor::new();

    let snapshot = match unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPALL, 0)} {
        Ok(s) => {
            if s.is_invalid() {
                logger.panic(&format!("Unable to create snapshot of all processes. GLE: {}", unsafe { GetLastError().0 }));
            } else {
                s
            }
        },
        Err(_) => {
            // not really bothered about the error at this stage
            logger.panic(&format!("Unable to create snapshot of all processes. GLE: {}", unsafe { GetLastError().0 }));
        },
    };

    let mut process_entry = PROCESSENTRY32::default();
    process_entry.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;

    if unsafe { Process32First(snapshot,&mut process_entry)}.is_ok() {
        loop {
            // 
            // Get the process name
            //
            let current_process_name_ptr = process_entry.szExeFile.as_ptr() as *const _;
            let current_process_name = match unsafe { CStr::from_ptr(current_process_name_ptr) }.to_str() {
                Ok(process) => process.to_string(),
                Err(e) => {
                    logger.log(LogLevel::Error, &format!("Error converting process name. {e}"));
                    continue;
                }
            };

            logger.log(LogLevel::Success, &format!("Process name: {}, pid: {}, parent: {}", current_process_name, process_entry.th32ProcessID, process_entry.th32ParentProcessID));
            let process = ProcessStarted {
                image_name: current_process_name,
                command_line: "".to_string(),
                parent_pid: process_entry.th32ParentProcessID as u64,
                pid: process_entry.th32ProcessID as u64,
            };

            if let Err(e) = all_processes.insert(&process) {
                match e {
                    super::process_monitor::ProcessErrors::DuplicatePid => {
                        logger.log(LogLevel::Error, &format!("Duplicate PID found in process hashmap, did not insert. Pid in question: {}", process_entry.th32ProcessID));
                    },
                    _ => {
                        logger.log(LogLevel::Error, "An unknown error occurred whilst trying to insert into process hashmap.");
                    }
                }
            };

            // continue enumerating
            if !unsafe { Process32Next(snapshot, &mut process_entry) }.is_ok() {
                break;
            }
        }
    }

    unsafe { let _ = CloseHandle(snapshot); };

    all_processes
}