// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{path::PathBuf, sync::Mutex};

use tauri::State;
use um_engine::UmEngine;

fn main() {

	let um_engine = UmEngine::new();
	
	tauri::Builder::default()
	.manage(Mutex::new(um_engine))
		.invoke_handler(tauri::generate_handler![start_individual_file_scan])
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}

#[tauri::command]
fn start_individual_file_scan(filePath: String, engine: State<Mutex<um_engine::UmEngine>>) -> String {
	let engine_lock = engine.lock().unwrap();
	let res = engine_lock.scanner_scan_single_file(PathBuf::from(filePath));

	match res {
		Some(v) => {
			format!("Found malware in file: {}, hash: {}", v.1.display(), v.0)
		},
		None => format!("Device clean!"),
	}
}