//! Antivirus.rs contains all functions associated with the antivirus UI in Tauri.
//! This module will handle state, requests, async, and events.

use std::sync::Arc;
use serde_json::{to_value, Value};
use tauri::{Emitter, State};
use std::path::PathBuf;
use um_engine::{FileScannerState, ScanningLiveInfo, UmEngine};

use crate::ipc::IpcClient;

#[tauri::command]
pub async fn scanner_check_page_state(
    _engine: State<'_, Arc<UmEngine>>,
) -> Result<String, ()> {

    // let engine = Arc::clone(&engine);

    let mut ipc = IpcClient::new().expect("[-] Unable to start IPC client");
    match ipc.send_ipc::<FileScannerState, Option<Value>>("scanner_check_page_state", None).await {
        Ok(response) => {
            println!("[i] Page state response: {:?}", response);
            return Ok(format!("{:?}", response));
        },
        Err(e) => {
            eprintln!("[-] Error with IPC: {e}");
            return Ok("Inactive".to_string()); // todo proper error handling
        },
    };
    
}


/// Reports the scan statistics back to the UI 
#[tauri::command]
pub async fn scanner_get_scan_stats(
    _engine: State<'_, Arc<UmEngine>>,
) -> Result<String, ()> {

    let mut ipc = IpcClient::new().expect("[-] Unable to start IPC client");
    match ipc.send_ipc::<ScanningLiveInfo, Option<Value>>("scanner_get_scan_stats", None).await {
        Ok(response) => {
            println!("[i] Get scan stats response: {:?}", response);
            return Ok(format!("{:?}", response));
        },
        Err(e) => {
            eprintln!("[-] Error with IPC: {e}");
            return Ok("Inactive".to_string()); // todo proper error handling
        },
    };

    // let engine = Arc::clone(&engine);

    // let data = serde_json::to_string(&engine.scanner_get_scan_data()).unwrap_or(String::new());
    // Ok(data)
}



#[tauri::command]
pub async fn scanner_stop_scan(
    _engine: State<'_, Arc<UmEngine>>,
) -> Result<(), ()> {  

    let mut ipc = IpcClient::new().expect("[-] Unable to start IPC client");
    match ipc.send_ipc::<(), Option<Value>>("scanner_cancel_scan", None).await {
        Ok(response) => {
            println!("[i] stop scan response: {:?}", response);
        },
        Err(e) => {
            eprintln!("[-] Error with IPC for stop scan: {e}");
        },
    };

    // let engine = Arc::clone(&engine);
    // engine.scanner_cancel_scan();

    Ok(())
}


#[tauri::command]
pub async fn scanner_start_folder_scan(
    file_path: String,
    _engine: State<'_, Arc<UmEngine>>,
	app_handle: tauri::AppHandle,
) -> Result<String, ()> {

	// let engine = Arc::clone(&engine);
    let path = to_value(vec![PathBuf::from(file_path)]).unwrap();

    let mut ipc = IpcClient::new().expect("[-] Unable to start IPC client");

	tokio::spawn(async move {
        // The result is wrapped inside of an enum from the filescanner module, so we need to first match on that
        // as DirectoryResult (since we are scanning a dir). The result should never be anything else for this scan
        // so if it is something has gone wrong with the internal wiring.

        match ipc.send_ipc::<FileScannerState, _>("scanner_start_folder_scan", Some(path)).await {
            Ok(response) => {
                println!("[i] Folder scanner response: {:?}", response);
                match response {
                    um_engine::FileScannerState::Finished => {
        
                        let scan_result = ipc.send_ipc::<ScanningLiveInfo, Option<Value>>("scanner_get_scan_stats", None).await.unwrap();
        
                        if scan_result.scan_results.is_empty() {
                            app_handle.emit("folder_scan_no_results", "No malicious files found.").unwrap();
                        } else {
                            app_handle.emit("folder_scan_malware_found", &scan_result).unwrap();
                        }
                    },
                    um_engine::FileScannerState::FinishedWithError(v) => {
                        app_handle.emit("folder_scan_error", &v).unwrap();
                    },
                    um_engine::FileScannerState::Scanning => {
                        app_handle.emit("folder_scan_error", format!("A scan is already in progress.")).unwrap()
                    },
                    _ => (),
                }
            },
            Err(e) => {
                eprintln!("[-] Error with IPC: {e}");
            },
        };
	});

	// // todo some kind of feedback like 1/1 file scanned; but then same for the mass scanner, be good to show x files scanned, and time taken so far. Then completed time and 
	// // total files after.

	// todo this shouldn't show in every case..?
	Ok(format!("Scan in progress..."))
}


#[tauri::command]
pub async fn scanner_start_quick_scan(
    engine: State<'_, Arc<UmEngine>>,
	app_handle: tauri::AppHandle,
) -> Result<String, ()> {

	// let engine = Arc::clone(&engine);

    let paths = engine.settings_get_common_scan_areas();
    let mut ipc = IpcClient::new().expect("[-] Unable to start IPC client");

	tokio::spawn(async move {
        // The result is wrapped inside of an enum from the filescanner module, so we need to first match on that
        // as DirectoryResult (since we are scanning a dir). The result should never be anything else for this scan
        // so if it is something has gone wrong with the internal wiring.
		match ipc.send_ipc::<FileScannerState, _>("scanner_start_folder_scan", Some(paths)).await {
            Ok(response) => {
                println!("[i] Folder scanner response: {:?}", response);
                match response {
                    um_engine::FileScannerState::Finished => {
        
                        let scan_result = ipc.send_ipc::<ScanningLiveInfo, Option<Value>>("scanner_get_scan_stats", None).await.unwrap();
        
                        if scan_result.scan_results.is_empty() {
                            app_handle.emit("folder_scan_no_results", "No malicious files found.").unwrap();
                        } else {
                            app_handle.emit("folder_scan_malware_found", &scan_result).unwrap();
                        }
                    },
                    um_engine::FileScannerState::FinishedWithError(v) => {
                        app_handle.emit("folder_scan_error", &v).unwrap();
                    },
                    um_engine::FileScannerState::Scanning => {
                        app_handle.emit("folder_scan_error", format!("A scan is already in progress.")).unwrap()
                    },
                    _ => (),
                }
            },
            Err(e) => {
                eprintln!("[-] Error with IPC: {e}");
            },
        };
	});

	// // todo some kind of feedback like 1/1 file scanned; but then same for the mass scanner, be good to show x files scanned, and time taken so far. Then completed time and 
	// // total files after.

	// todo this shouldn't show in every case..
	Ok(format!("Scan in progress..."))
}