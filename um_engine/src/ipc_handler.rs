use std::{path::PathBuf, sync::Arc};

use serde_json::{from_slice, to_value, to_vec, Value};
use shared_std::ipc::{CommandRequest, CommandResponse, PIPE_NAME};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::windows::named_pipe::{PipeMode, ServerOptions}};
use um_engine::UmEngine;

/// An interface for the usermode IPC server
pub struct UmIpc{}

impl UmIpc {

    pub async fn listen(engine: Arc<UmEngine>) -> Result<(), Box<dyn std::error::Error>> {
        println!("[i] Trying to start IPC server at {}...", PIPE_NAME);

        // set up IPC
        let mut server = ServerOptions::new()
            .first_pipe_instance(true)
            .pipe_mode(PipeMode::Message)
            .create(PIPE_NAME)?;

        println!("[+] Named pipe listening on {}", PIPE_NAME);

        loop {
            server.connect().await?;
    
            let mut client = server;
            server = ServerOptions::new().create(PIPE_NAME)?;
    
            let engine_clone = Arc::clone(&engine);
    
            tokio::spawn(async move {
                let mut buffer = vec![0; 1024];
    
                // read the request
                match client.read(&mut buffer).await {
                    Ok(bytes_read) => {
                        if bytes_read == 0 {
                            println!("[i] Client disconnected.");
                            return;
                        }
    
                        // deserialise the request
                        match from_slice::<CommandRequest>(&buffer[..bytes_read]) {
                            Ok(request) => {
                                //
                                // Handle the incoming IPC request here
                                //
                                let response = handle_ipc(request, engine_clone);
    
                                //
                                // Serialise and send the response back to the client
                                //
                                match to_vec(&response) {
                                    Ok(response_bytes) => {
                                        if let Err(e) = client.write_all(&response_bytes).await {
                                            eprintln!("[-] Failed to send response to client via pipe: {}", e);
                                        }
                                    },
                                    // err serialising to vec
                                    Err(e) => eprintln!("[-] Failed to serialise response: {}", e),
                                };
                            },
                            // err serialising into CommandRequest
                            Err(e) => eprintln!("Failed to deserialise request: {}", e),
                        }
                    },
                    // err reading IPC
                    Err(e) => eprintln!("Failed to read from client: {}", e),
                }
            });
        }
    }
    
}


/// IPC logic handler, this function accepts a request and an Arc of UmEngine which matches on a 
/// string based command to decide on what to do, this is considered the heart of the tasking of the 
/// engine where its come from the GUI, or even other sources which may feed in via IPC (such as injected
/// DLL's)
pub fn handle_ipc(request: CommandRequest, engine_clone: Arc<UmEngine>) -> Value {
    let response: Value = match request.command.as_str() {
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
        _ => to_value(CommandResponse {
            status: "error".to_string(),
            message: "Unknown command".to_string(),
        }).unwrap(),
    };

    response

}