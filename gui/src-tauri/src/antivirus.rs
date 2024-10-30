//! Antivirus.rs contains all functions associated with the antivirus UI in Tauri.
//! This module will handle state, requests, async, and events.

use std::sync::Arc;
use tauri::{Emitter, State};
use std::path::PathBuf;
use um_engine::{UmEngine, ScanType, ScanResult};

// #[tauri::command]
// pub fn check_page_state(
//     engine: State<'_, Arc<UmEngine>>,
// 	app_handle: tauri::AppHandle,
// ) -> Result<(), ()> {

//     let engine = Arc::clone(&engine);

//     Ok(())
// }

#[tauri::command]
pub async fn start_individual_file_scan(
    file_path: String,
    engine: State<'_, Arc<UmEngine>>,
	app_handle: tauri::AppHandle,
) -> Result<String, ()> {

	let engine = Arc::clone(&engine);
    let path = PathBuf::from(file_path);

	tokio::spawn(async move {
        let result = engine.scanner_start_scan(path, ScanType::File);

        // The result is wrapped inside of an enum from the filescanner module, so we need to first match on that
        // as FileResult (since we are scanning a file). The result should never be anything else for this scan
        // so if it is something has gone wrong with the internal wiring.
		match result {
            ScanResult::FileResult(result) => {
                match result {
                    Err(e) => app_handle.emit("scan_error", format!("Error occurred whilst trying to scan file: {}", e)).unwrap(),
                    Ok(Some(v)) => app_handle.emit("scan_complete", format!("Found malware in file: {}, hash: {}", v.1.display(), v.0)).unwrap(),
                    Ok(None) => app_handle.emit("scan_complete",format!("File clean!")).unwrap(),
                }
            },
            ScanResult::ScanInProgress => app_handle.emit("scan_error", format!("A scan is already in progress.")).unwrap(),
            _ => {
                app_handle.emit("scan_error", format!("Internal error occurred")).unwrap();
            }
		}
	});
    
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
		let result = engine.scanner_start_scan(path, ScanType::Folder);

        // The result is wrapped inside of an enum from the filescanner module, so we need to first match on that
        // as DirectoryResult (since we are scanning a dir). The result should never be anything else for this scan
        // so if it is something has gone wrong with the internal wiring.
		match result {
            ScanResult::DirectoryResult(result) => {
                match result {
                    Ok(v) => {
                        if v.is_empty() {
                            app_handle.emit("folder_scan_no_results", "No malicious files found.").unwrap();
                        } else {
                            app_handle.emit("folder_scan_malware_found", &v).unwrap();
                        }
                    },
                    Err(e) => app_handle.emit("folder_scan_error", format!("Error occurred whilst trying to scan directory: {}", e)).unwrap(),
                }
            },
            ScanResult::ScanInProgress => app_handle.emit("folder_scan_error", format!("A scan is already in progress.")).unwrap(),
            _ => {
                app_handle.emit("folder_scan_error", format!("Internal error occurred")).unwrap();
            }
		}
	});

	// // todo some kind of feedback like 1/1 file scanned; but then same for the mass scanner, be good to show x files scanned, and time taken so far. Then completed time and 
	// // total files after.

	// todo this shouldn't show in every case..
	Ok(format!("Scan started..."))
}