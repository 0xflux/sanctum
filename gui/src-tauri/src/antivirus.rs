//! Antivirus.rs contains all functions associated with the antivirus UI in Tauri.
//! This module will handle state, requests, async, and events.

use core::time;
use std::sync::Arc;
use tauri::{Manager, State};
use std::path::PathBuf;
use um_engine::UmEngine;

#[tauri::command]
pub async fn start_individual_file_scan(
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
pub async fn start_folder_scan(
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