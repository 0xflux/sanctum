#![feature(io_error_uncategorized)]

use ipc_handler::UmIpc;
use um_engine::UmEngine;
use std::sync::Arc;

mod driver_manager;
mod strings;
mod settings;
mod filescanner;
mod utils;
mod ipc_handler;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    // set up the engine
    let engine = Arc::new(UmEngine::new().await);

    // listen and deal with IPC requests
    UmIpc::listen(engine).await?;

    Ok(())

    // IPC input loop
    
}