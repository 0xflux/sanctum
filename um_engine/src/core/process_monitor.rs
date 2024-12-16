use std::collections::HashMap;

use shared_no_std::driver_ipc::ProcessStarted;

use crate::utils::log::Log;

/// The ProcessMonitor is responsible for monitoring all processes running; this 
/// structure holds a hashmap of all processes by the pid as an integer, and 
/// the data within is a MonitoredProcess containing the details
/// 
/// The key of processes hashmap is the pid, which is duplicated inside the Process
/// struct.
#[derive(Debug)]
pub struct ProcessMonitor {
    processes: HashMap<u64, Process>
}

pub enum ProcessErrors {
    PidNotFound,
    DuplicatePid,
}

/// The Process is a structural representation of an individual process thats
/// running on the host machine, and keeping track of risk scores, and activity conducted
/// by processes. 
#[derive(Debug)]
pub struct Process {
    pid: u64,
    process_image: String,
    commandline_args: String,
    risk_score: u8,
    allow_listed: bool, // whether the application is allowed to exist without monitoring
    sanctum_protected_process: bool, // scc (sanctum protected process) defines processes which require additional protections from access / abuse, such as lsass.exe.
}

impl ProcessMonitor {
    pub fn new() -> Self {
        ProcessMonitor {
            processes: HashMap::new(),
        }
    }

    /// todo more fn comments
    pub fn insert(&mut self, proc: &ProcessStarted) -> Result<(), ProcessErrors> {
        //
        // First check we aren't inserting a duplicate PID, this may happen if we haven't received
        // a notification that a process has been terminated; or that we have a new process queued to
        // insert before a delete item which is queued.
        // todo this can be solved by first batch running deletes, before running updates.
        //

        let entry = self.processes.get(&proc.pid);
        if entry.is_some() {
            return Err(ProcessErrors::DuplicatePid);
        }

        self.processes.insert(proc.pid, Process {
            pid: proc.pid,
            process_image: proc.image_name.clone(),
            commandline_args: proc.command_line.clone(),
            risk_score: 0,
            allow_listed: false,
            sanctum_protected_process: false,
        });

        Ok(())
    }

    pub fn remove_process(&mut self, pid: u64) {
        self.processes.remove(&pid);
    }

    /// Extends the processes hashmap through the std extend function on the inner processes hashmap
    pub fn extend_processes(&mut self, foreign_hashmap: ProcessMonitor) {
        self.processes.extend(foreign_hashmap.processes);

        let logger = Log::new();
        logger.log(crate::utils::log::LogLevel::Info, &format!("Discovered {} running processes on startup.", self.processes.len()));
    }
}