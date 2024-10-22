// ******************************************************************** //
// ************************** CORE CALLBACKS ************************** //
// ******************************************************************** //

use wdk::println;
use wdk_sys::{HANDLE, PEPROCESS, PS_CREATE_NOTIFY_INFO};

use crate::utils::unicode_to_str;

/// Callback function for a new process being created on the system.
pub unsafe extern "C" fn core_callback_notify_ps(process: PEPROCESS, pid: HANDLE, created: *mut PS_CREATE_NOTIFY_INFO){

    if !created.is_null() {
        // created contains information about the new process, if it is null, it is exiting.
        // todo maybe handle something for exiting processes? would that help from an edr pov?
        let image_name = unicode_to_str((*created).ImageFileName);
        if image_name.is_none() {
            return;
        }

        let image_name = image_name.unwrap();
        println!("[sanctum] [i] NULL Process started: {:#?}, command line: {:?}, ppid: {:?}, image: {}.", pid, (*created).CommandLine, (*created).ParentProcessId, image_name);
    } else {
        // todo
        println!("[sanctum] [-] Process terminated");
    }

}