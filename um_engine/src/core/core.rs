use std::{sync::Arc, thread::sleep, time::Duration};

use shared_no_std::ioctl::DriverMessages;

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
            driver_poll_rate: 500,
        };

        let mut processes = ProcessMonitor::new();

        let logger = Log::new();

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

                println!("{:?}", processes);

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