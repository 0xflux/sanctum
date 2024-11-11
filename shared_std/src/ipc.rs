use serde::{Deserialize, Serialize};
use serde_json::Value;

//
// Consts
//

pub const PIPE_NAME: &'static str = r"\\.\pipe\sanctum_um_engine_pipe";

//
// Structs
//

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