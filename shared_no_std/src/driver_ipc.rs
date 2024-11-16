//! Definitions for IPC structures shared between the user mode modules and the driver
//! for serialisation through IPC.
extern crate alloc;
use alloc::string::String;

pub struct ProcessStarted {
    image_name: String,
    command_line: String,
    parent_pid: String,
}