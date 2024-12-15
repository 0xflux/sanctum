// ******************************************************************** //
// ************************** CORE CALLBACKS ************************** //
// ******************************************************************** //

use core::sync::atomic::Ordering;

use shared_no_std::driver_ipc::{ProcessStarted, ProcessTerminated};
use wdk::println;
use wdk_sys::{HANDLE, PEPROCESS, PS_CREATE_NOTIFY_INFO};

use crate::{utils::unicode_to_string, DRIVER_MESSAGES};

/// Callback function for a new process being created on the system.
pub unsafe extern "C" fn core_callback_notify_ps(process: PEPROCESS, pid: HANDLE, created: *mut PS_CREATE_NOTIFY_INFO) {

    //
    // If `created` is not a null pointer, this means a new process was started, and you can query the 
    // args for information about the newly spawned process.
    //
    // In the event that `create` is null, it means a process was terminated.
    //

    if !created.is_null() {
        // process started

        let image_name = unicode_to_string((*created).ImageFileName);
        let command_line = unicode_to_string((*created).CommandLine);
        let parent_pid = (*created).ParentProcessId as u64;
        let pid = pid as u64;

        if image_name.is_err() || command_line.is_err() {
            return;
        }

        let process_started = ProcessStarted {
            image_name: image_name.unwrap().replace("\\??\\", ""),
            command_line: command_line.unwrap().replace("\\??\\", ""),
            parent_pid,
            pid,
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
        // process terminated

        let pid = pid as u64;
        let process_terminated = ProcessTerminated {
            pid,
        };

        println!("[sanctum] [-] Process terminated, {:?}", process_terminated);

        // Attempt to dereference the DRIVER_MESSAGES global; if the dereference is successful,
        // add the relevant data to the queue
        if !DRIVER_MESSAGES.load(Ordering::SeqCst).is_null() {
            let obj = unsafe { &mut *DRIVER_MESSAGES.load(Ordering::SeqCst) };
            obj.add_process_termination_to_queue(process_terminated);
        } else {
            println!("[sanctum] [-] Driver messages is null");
        };
    }
}