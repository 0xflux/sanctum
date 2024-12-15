//! Definitions for IPC structures shared between the user mode modules and the driver
//! for serialisation through IPC.
extern crate alloc;
use alloc::string::String;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessStarted {
    pub image_name: String,
    pub command_line: String,
    pub parent_pid: u64,
    pub pid: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessTerminated {
    pub pid: u64,
}