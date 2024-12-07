//! The inter-process communication module responsible for sending and receiving IPC requests from:
//! * Driver
//! * GUI
//! * DLLs
//! 
//! This does not handle IOCTL's, that can be found in the driver_manager module.
//! 
//! This IPC module is the main event loop for the application.

use std::{path::PathBuf, sync::Arc};

use serde_json::{from_slice, to_value, to_vec, Value};
use shared_no_std::{constants::PIPE_NAME, ipc::{CommandRequest, CommandResponse}};
use shared_std::settings::SanctumSettings;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::windows::named_pipe::{PipeMode, ServerOptions}};
use crate::{engine::UmEngine, settings::SanctumSettingsImpl, utils::log::{Log, LogLevel}}; 

/// An interface for the usermode IPC server
pub struct UmIpc{}

impl UmIpc {

    pub async fn listen(engine: Arc<UmEngine>) -> Result<(), Box<dyn std::error::Error>> {
        let logger = Log::init();
        logger.log(LogLevel::Info, &format!("Trying to start IPC server at {}...", PIPE_NAME));

        // set up IPC
        let mut server = ServerOptions::new()
            .first_pipe_instance(true)
            .pipe_mode(PipeMode::Message)
            .create(PIPE_NAME)?;

        logger.log(LogLevel::Success, &format!("Named pipe listening on {}", PIPE_NAME));

        loop {
            // create the next server instance before accepting the client connection, without this
            // there is a fraction of time where there will be no server listening
            let next_server = ServerOptions::new().create(PIPE_NAME)?;

            server.connect().await?;
            
            // move the current server instance to a client handler
            let mut client = server;
            server = next_server;
    
            let engine_clone = Arc::clone(&engine);
    
            tokio::spawn(async move {
                let mut buffer = vec![0; 1024];
                let logger = Log::init();
    
                // read the request
                match client.read(&mut buffer).await {
                    Ok(bytes_read) => {
                        if bytes_read == 0 {
                            logger.log(LogLevel::Info, "IPC client disconnected");
                            return;
                        }
    
                        // deserialise the request
                        match from_slice::<CommandRequest>(&buffer[..bytes_read]) {
                            Ok(request) => {
                                //
                                // Handle the incoming IPC request here
                                //
                                if let Some(response) = handle_ipc(request, engine_clone) {
                                    //
                                    // Serialise and send the response back to the client
                                    //
                                    match to_vec(&response) {
                                        Ok(response_bytes) => {
                                            if let Err(e) = client.write_all(&response_bytes).await {
                                                logger.log(LogLevel::Error, &format!("[-] Failed to send response to client via pipe: {}", e));
                                            }
                                        },
                                        // err serialising to vec
                                        Err(e) => logger.log(LogLevel::Error, &format!("[-] Failed to serialise response: {}", e)),
                                    };
                                };
                            },
                            // err serialising into CommandRequest
                            Err(e) => logger.log(LogLevel::Error, &format!("Failed to deserialise request: {:?}. Err: {}. Bytes read: {}", &buffer[..bytes_read], e, bytes_read)),
                        }
                    },
                    // err reading IPC
                    Err(e) => logger.log(LogLevel::Error, &format!("Failed to read from client: {}", e)),
                }
            });
        }
    }
}


/// IPC logic handler, this function accepts a request and an Arc of UmEngine which matches on a 
/// string based command to decide on what to do, this is considered the heart of the tasking of the 
/// engine where its come from the GUI, or even other sources which may feed in via IPC (such as injected
/// DLL's)
/// 
/// # Args
/// 
/// * 'request' - The CommandRequest type which will be matched on and logic will be executed accordingly.
/// * 'engine_clone' - An Arc of the UmEngine
/// 
/// # Returns
/// 
/// None if there is to be no response to the IPC - will usually be the case in respect of the driver sending a message. 
/// As the IPC channel is a 'one shot' from the driver implemented natively, the pipe will be closed on receipt in this function.
/// In the case of a Tokio IPC pipe, a response can be sent, in which case, it will be serialised to a Value and sent wrapped in a Some.
pub fn handle_ipc(request: CommandRequest, engine_clone: Arc<UmEngine>) -> Option<Value> {
    let response: Value = match request.command.as_str() {

        //
        // Scanner IPC requests
        //

        "scanner_check_page_state" => {
            to_value(engine_clone.scanner_get_state()).unwrap()
        },
        "scanner_get_scan_stats" => {
            to_value(engine_clone.scanner_get_scan_data()).unwrap()
        },
        "scanner_cancel_scan" => {
            engine_clone.scanner_cancel_scan();
            to_value("").unwrap()
        },
        "scanner_start_folder_scan" => {
            if let Some(args) = request.args {
                let target: Vec<PathBuf> = serde_json::from_value(args).unwrap();
                to_value(engine_clone.scanner_start_scan(target)).unwrap()
            } else {
                to_value(CommandResponse {
                    status: "error".to_string(),
                    message: "No path passed to scanner".to_string(),
                }).unwrap()
            }
        },
        "settings_get_common_scan_areas" => {
            to_value(engine_clone.settings_get_common_scan_areas()).unwrap()
        }


        //
        // Settings control page
        //
        "settings_load_page_state" => {
            let res = engine_clone.sanctum_settings.lock().unwrap().clone();
            to_value(res).unwrap()
        },
        "settings_update_settings" => {
            if let Some(args) = request.args {
                let settings: SanctumSettings = serde_json::from_value(args).unwrap();
                engine_clone.sanctum_settings.lock().unwrap().update_settings(settings);
                to_value("").unwrap()
            } else {
                to_value(CommandResponse {
                    status: "error".to_string(),
                    message: "No path passed to scanner".to_string(),
                }).unwrap()
            }
        },


        //
        // Driver control from GUI
        //
        "driver_install_driver" => {
            to_value(engine_clone.driver_install_driver()).unwrap()
        },
        "driver_uninstall_driver" => {
            to_value(engine_clone.driver_uninstall_driver()).unwrap()
        },
        "driver_start_driver" => {
            to_value(engine_clone.driver_start_driver()).unwrap()
        },
        "driver_stop_driver" => {
            to_value(engine_clone.driver_stop_driver()).unwrap()
        },
        "driver_get_state" => {
            to_value(engine_clone.driver_get_state()).unwrap()
        },
        


        //
        // IOCTL / IPC from driver
        //
        "ioctl_ping_driver" => {
            to_value(engine_clone.ioctl_ping_driver()).unwrap()
        },
        "driver_collect_knl_dbg_msg" => {
            to_value(engine_clone.driver_manager.lock().unwrap().ioctl_get_driver_messages()).unwrap()
        }


        //
        // Unhandled requests
        //
        _ => to_value(CommandResponse {
            status: "error".to_string(),
            message: "Unknown command".to_string(),
        }).unwrap(),
    };

    Some(response)

}