use std::mem::take;

use serde_json::Value;
use shared_no_std::driver_ipc::ProcessStarted;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DriverState {
    Uninstalled(String),
    Installed(String),
    Started(String),
    Stopped(String),
}

/// A structure to hold data from kernel debug messaging for use in usermode applications.
/// Data can be enqueued and dequeued from a vector as required.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KernelDbgMsgQueue {
    messages: Vec<String>,
    process_creations: Vec<ProcessStarted>,
}

impl Default for KernelDbgMsgQueue {
    fn default() -> Self {
        KernelDbgMsgQueue {
            messages: Vec::new(),
            process_creations: Vec::new(),
        }
    }
}

impl KernelDbgMsgQueue {

    /// Get the data held in the struct.
    /// 
    /// # Performance
    /// 
    /// This will make a deep clone of the underlying data.
    pub fn get(&self) -> KernelDbgMsgQueue {
        self.clone()
    }

    // /// Clear the content of the structure
    // pub fn clear(&mut self) {
    //     self.process_creations.clear();
    // }

    /// Push a process_creation item to the queue
    pub fn push_process_creations(&mut self, item: &ProcessStarted) {
        self.process_creations.push(item.clone());
    }

    /// Push a message item to the queue
    pub fn push_message(&mut self, item: &String) {
        self.messages.push(item.clone());
    }

    /// Gets and removes all data from the queue, transferring ownership to the caller without cloning.
    ///
    /// This method efficiently moves the internal vector `self.data` out of the queue and into the caller,
    /// avoiding any deep copies or cloning of the data. After calling this method, `self.data` will be empty.
    pub fn get_and_empty(&mut self) -> KernelDbgMsgQueue {
        take(self)
    }
    
}