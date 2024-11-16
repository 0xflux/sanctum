//! Definitions for IPC structures shared between the user mode modules and the driver
//! for serialisation through IPC.
extern crate alloc;
use alloc::string::String;

pub struct ProcessStarted {
    pub image_name: String,
    pub command_line: String,
    pub parent_pid: String,
}