//! The main entrypoint for the usermode engine for the Sanctum EDR. This will run as a service
//! on the host machine and is responsible for all EDR related activity in usermode, including
//! communicating with the driver, GUI, DLL's; performing scanning; and decision making.

#![feature(io_error_uncategorized)]

use gui_communication::ipc::UmIpc;
use engine::UmEngine;
use tokio::time::sleep;
use std::{sync::Arc, time::Duration};

mod engine;
mod driver_manager;
mod strings;
mod settings;
mod filescanner;
mod utils;
mod gui_communication;


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
        // todo move this to a core module
        loop {
            let x = ec.driver_manager.lock().unwrap().ioctl_get_driver_messages();
            if x.is_some() {
                println!("x: {:?}", x);
            }

            sleep(Duration::from_millis(80)).await;
        }
    });

    //
    // Listen and deal with IPC requests, this should be IPC request specifically for
    // talking to the GUI.
    //
    UmIpc::listen(engine).await?;

    Ok(())
        
}