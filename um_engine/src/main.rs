//! The main entrypoint for the usermode engine for the Sanctum EDR. This will run as a service
//! on the host machine and is responsible for all EDR related activity in usermode, including
//! communicating with the driver, GUI, DLL's; performing scanning; and decision making.

#![feature(io_error_uncategorized)]

use gui_communication::ipc::UmIpc;
use engine::UmEngine;
use core::core::Core;
use std::sync::Arc;
mod engine;
mod driver_manager;
mod strings;
mod settings;
mod filescanner;
mod utils;
mod gui_communication;
mod core;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    // set up the engine
    let engine = Arc::new(UmEngine::new().await);
    let ec = engine.clone();

    //
    // Spawn the core of the engine which will constantly talk to the driver and process any IO
    // from / to the driver and other working parts of the EDR, except for the GUI which will
    // be handled below
    //
    tokio::spawn(async move {
        Core::start_core(ec).await;
    });

    //
    // Listen and deal with IPC requests, this should be IPC request specifically for
    // talking to the GUI.
    //
    UmIpc::listen(engine).await?;

    Ok(())
        
}