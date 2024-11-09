//! Sanctum Tauri based GUI which will allow the user to interact with the kernel and usermode application.

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[allow(non_snake_case)]

mod antivirus;
mod settings;
mod driver_controller;

use std::sync::Arc;
use antivirus::{scanner_check_page_state, scanner_get_scan_stats, scanner_start_folder_scan, scanner_stop_scan, scanner_start_quick_scan};
use driver_controller::{driver_check_state, driver_install_driver, driver_start_driver, driver_stop_driver, driver_uninstall_driver};
use serde_json::to_vec;
use settings::{settings_load_page_state, settings_update_settings};
use shared_std::ipc::{CommandRequest, CommandResponse, PIPE_NAME};
use tokio::{io::{self, AsyncReadExt, AsyncWriteExt}, net::windows::named_pipe::{ClientOptions, NamedPipeClient}};
use um_engine::UmEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

	// configure IPC client
	let mut ipc_client = ClientOptions::new()
		.open(PIPE_NAME)?;
	let res = send_ipc(&mut ipc_client).await;
	match res {
		Ok(_) => println!("No error"),
		Err(e) => eprintln!("[-] Error from IPC: {e}"),
	}

	// the usermode engine will be used as a singleton
	let um_engine = Arc::new(UmEngine::new().await);
	
	Ok(
		tauri::Builder::default()
		.manage(um_engine)
			.invoke_handler(tauri::generate_handler![
				scanner_start_folder_scan, 
				scanner_check_page_state,
				scanner_stop_scan,
				scanner_get_scan_stats,
				scanner_start_quick_scan,
				settings_load_page_state,
				settings_update_settings,
				driver_install_driver,
				driver_uninstall_driver,
				driver_start_driver,
				driver_stop_driver,
				driver_check_state,
			])
			.run(tauri::generate_context!())
			.expect("error while running tauri application")
	)
}

async fn send_ipc(client: &mut NamedPipeClient) -> io::Result<()> {

	let message = CommandRequest {
        command: "install_driver".to_string(),
    };

	let message_data = to_vec(&message)?;
	client.write_all(&message_data).await?;

	// read the response
	let mut buffer = vec![0u8; 1024];
	let bytes_read = client.read(&mut buffer).await?;
	let received_data = &buffer[..bytes_read];

	// Deserialize the received JSON data into a Message struct
    let response_message: CommandResponse = serde_json::from_slice(received_data)?;
    println!("Received: {:?}", response_message);


	Ok(())

}