use serde::{Deserialize, Serialize};

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
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommandResponse {
    pub status: String,
    pub message: String,
}