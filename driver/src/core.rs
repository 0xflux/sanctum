// ******************************************************************** //
// ************************** CORE CALLBACKS ************************** //
// ******************************************************************** //

use core::sync::atomic::Ordering;

use alloc::format;
use shared_no_std::driver_ipc::ProcessStarted;
use wdk::println;
use wdk_sys::{HANDLE, PEPROCESS, PS_CREATE_NOTIFY_INFO};

use crate::{utils::unicode_to_string, DRIVER_MESSAGES};

/// Callback function for a new process being created on the system.
pub unsafe extern "C" fn core_callback_notify_ps(process: PEPROCESS, pid: HANDLE, created: *mut PS_CREATE_NOTIFY_INFO) {

    if !created.is_null() {
        // created contains information about the new process, if it is null, it is exiting.
        // todo maybe handle something for exiting processes? would that help from an edr pov?
        let image_name = unicode_to_string((*created).ImageFileName);
        let command_line = unicode_to_string((*created).CommandLine);
        let ppid = format!("{:?}", (*created).ParentProcessId);

        if image_name.is_err() || command_line.is_err() {
            return;
        }

        let process_started = ProcessStarted {
            image_name: image_name.unwrap().replace("\\??\\", ""),
            command_line: command_line.unwrap().replace("\\??\\", ""),
            parent_pid: ppid,
        };

        // println!("[sanctum] [i] Process started: {:?}.", process_started);
        
        
        // Attempt to dereference the DRIVER_MESSAGES global; if the dereference is successful,
        // add the relevant data to the queue
        if !DRIVER_MESSAGES.load(Ordering::SeqCst).is_null() {
            let obj = unsafe { &mut *DRIVER_MESSAGES.load(Ordering::SeqCst) };
            obj.add_process_creation_to_queue(process_started);
        } else {
            println!("[sanctum] [-] Driver messages is null");
        };
        
    } else {
        // todo
        // println!("[sanctum] [-] Process terminated");
    }
}