// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use tauri::State;
use tokio::sync::Mutex;
use um_engine::UmEngine;

#[tokio::main]
async fn main() {

	let um_engine = UmEngine::new();
	
	tauri::Builder::default()
	.manage(Mutex::new(um_engine))
		.invoke_handler(tauri::generate_handler![start_individual_file_scan])
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}

#[tauri::command]
async fn start_individual_file_scan(filePath: String, engine: State<'_, Mutex<um_engine::UmEngine>>) -> Result<String, ()> {
	let engine_lock = engine.lock().await;
	let res = engine_lock.scanner_scan_single_file(PathBuf::from(filePath)).await;

	match res {
		Err(e) => Ok(format!("Error occurred whilst trying to scan file: {}", e)),
		Ok(Some(v)) => Ok(format!("Found malware in file: {}, hash: {}", v.1.display(), v.0)),
		Ok(None) => Ok(format!("File clean!")),
	}
}