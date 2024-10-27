//! Sanctum Tauri based GUI which will allow the user to interact with the kernel and usermode application.

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[allow(non_snake_case)]

mod antivirus;

use std::sync::Arc;
use antivirus::{start_folder_scan, start_individual_file_scan};
use um_engine::UmEngine;

#[tokio::main]
async fn main() {

	// the usermode engine will be used as a singleton
	let um_engine = Arc::new(UmEngine::new());
	
	tauri::Builder::default()
	.manage(um_engine)
		.invoke_handler(tauri::generate_handler![start_individual_file_scan, start_folder_scan])
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}