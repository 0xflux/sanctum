// ******************************************************************** //
// ************************** CORE CALLBACKS ************************** //
// ******************************************************************** //

use alloc::format;
use wdk::println;
use wdk_sys::{HANDLE, PEPROCESS, PS_CREATE_NOTIFY_INFO};

use crate::utils::unicode_to_string;

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

        println!("[sanctum] [i] Process started: {:#?}, command line: {}, ppid: {}, image: {}.", pid, command_line.unwrap(), ppid, image_name.unwrap());
        
    } else {
        // todo
        println!("[sanctum] [-] Process terminated");
    }
}