use std::mem::take;

use serde_json::Value;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DriverState {
    Uninstalled(String),
    Installed(String),
    Started(String),
    Stopped(String),
}

/// A structure to hold data from kernel debug messaging for use in usermode applications.
/// Data can be enqueued and dequeued from a vector as required.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct KernelDbgMsgQueue {
    data: Vec<Value>,
}

impl KernelDbgMsgQueue {
    pub fn new() -> Self {
        KernelDbgMsgQueue {
            data: Vec::new(),
        }
    }

    /// Get the data held in the struct.
    /// 
    /// # Performance
    /// 
    /// This will make a deep clone of the underlying data.
    pub fn get(&self) -> Vec<Value> {
        self.data.clone()
    }

    /// Clear the content of the structure
    pub fn clear(&mut  self) {
        self.data.clear();
    }

    // Push an item to the queue
    pub fn push(&mut self, item: &Value) {
        self.data.push(item.clone());
    }

    // Pop an item from the queue
    pub fn pop(&mut self) -> Option<Value> {
        self.data.pop()
    }

    /// Gets and removes all data from the queue, transferring ownership to the caller without cloning.
    ///
    /// This method efficiently moves the internal vector `self.data` out of the queue and into the caller,
    /// avoiding any deep copies or cloning of the data. After calling this method, `self.data` will be empty.
    ///
    /// # Returns
    ///
    /// - `Some(Vec<Value>)` containing all the data if the queue is not empty.
    /// - `None` if the queue is empty.
    pub fn get_and_empty(&mut self) -> Option<Vec<Value>> {
        if self.data.is_empty() {
            return None
        }

        Some(take(&mut self.data))
    }
    
}