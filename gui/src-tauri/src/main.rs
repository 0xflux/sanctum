// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::path::PathBuf;
#[allow(non_snake_case)]

use std::sync::Arc;
use tauri::{Manager, State};
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


#[tauri::command]
async fn start_individual_file_scan(
    file_path: String,
    engine: State<'_, Arc<UmEngine>>,
	app_handle: tauri::AppHandle,
) -> Result<String, ()> {

	let engine = Arc::clone(&engine);
    let path = PathBuf::from(file_path);

	tokio::spawn(async move {
        let result = engine.scanner_scan_single_file(path);

		match result {
			Err(e) => app_handle.emit_all("scan_error", format!("Error occurred whilst trying to scan file: {}", e)).unwrap(),
			Ok(Some(v)) => app_handle.emit_all("scan_complete", format!("Found malware in file: {}, hash: {}", v.1.display(), v.0)).unwrap(),
			Ok(None) => app_handle.emit_all("scan_complete",format!("File clean!")).unwrap(),
		}
	});

	// todo this shouldn't show in every case..
	Ok(format!("Scan started..."))
}


#[tauri::command]
async fn start_folder_scan(
    file_path: String,
    engine: State<'_, Arc<UmEngine>>,
	app_handle: tauri::AppHandle,
) -> Result<String, ()> {

	let engine = Arc::clone(&engine);
    let path = PathBuf::from(file_path);

	tokio::spawn(async move {
		let result = engine.scanner_scan_directory(path);

		match result {
			Ok(v) => {
				if v.is_empty() {
					app_handle.emit_all("folder_scan_no_results", "No malicious files found.").unwrap();
				} else {
					app_handle.emit_all("folder_scan_malware_found", &v).unwrap();
				}
			},
			Err(e) => app_handle.emit_all("folder_scan_error", format!("Error occurred whilst trying to scan directory: {}", e)).unwrap(),
		}

	});

	// // todo some kind of feedback like 1/1 file scanned; but then same for the mass scanner, be good to show x files scanned, and time taken so far. Then completed time and 
	// // total files after.

	// todo this shouldn't show in every case..
	Ok(format!("Scan started..."))
}


// folder_scan_result"></p>
// <p id="folder_scan_err