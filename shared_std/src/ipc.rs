use serde::{Deserialize, Serialize};
use serde_json::Value;

//
// Consts
//

pub const PIPE_NAME: &'static str = r"\\.\pipe\sanctum_um_engine_pipe";

//
// Structs
//

/// The CommandRequest is the inbound (GUI to engine) request for the engine to perform some form of work.
/// This contains the command in question as a String, and 'args' which is a generic JSON serialised "Value"
/// from Serde which allows the struct to contain any number of arguments, serialised to / from a struct that 
/// is appropriate for the calling / receiving functions.
/// 
/// # Sending function
/// 
/// The sending function must encode data like so:
/// 
/// ## No data to send:
/// 
/// ```
/// // where IPC is of type IpcClient as implemented in the GUI.
/// ipc.send_ipc::<(), Option<Value>>("scanner_cancel_scan", None).await
/// ```
/// 
/// ## Data of type A to send:
/// 
/// ```
/// let path = to_value(vec![PathBuf::from(file_path)]).unwrap();
/// ipc.send_ipc::<FileScannerState, _>("scanner_start_folder_scan", Some(path)).await
/// ```
#[derive(Serialize, Deserialize, Debug)]
pub struct CommandRequest {
    pub command: String,
    pub args: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommandResponse {
    pub status: String,
    pub message: String,
}