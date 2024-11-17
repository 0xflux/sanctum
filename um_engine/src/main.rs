//! The main entrypoint for the usermode engine for the Sanctum EDR. This will run as a service
//! on the host machine and is responsible for all EDR related activity in usermode, including
//! communicating with the driver, GUI, DLL's; performing scanning; and decision making.

#![feature(io_error_uncategorized)]

use communication::ipc::UmIpc;
use engine::UmEngine;
use std::sync::Arc;

mod engine;
mod driver_manager;
mod strings;
mod settings;
mod filescanner;
mod utils;
mod communication;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    // set up the engine
    let engine = Arc::new(UmEngine::new().await);

    // listen and deal with IPC requests
    UmIpc::listen(engine).await?;

    Ok(())
        
}