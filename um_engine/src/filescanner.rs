//! File scanner module 
//! 
//! This module provides functionality for scanning files and retrieving relevant
//! information about a file that the EDR may want to use in decision making. 

use std::{collections::BTreeSet, fs::{self, File}, io::{self, BufRead, BufReader, Read, Write}, os::windows::fs::MetadataExt, path::PathBuf, sync::{Arc, Mutex}, thread, time::{Duration, Instant}};
use md5::{Digest, Md5};
use shared_no_std::constants::{IOC_LIST_LOCATION, IOC_URL};
use shared_std::file_scanner::{FileScannerState, MatchedIOC, ScanningLiveInfo};

use crate::utils::log::{Log, LogLevel};


/// The FileScanner is the public interface into the module handling any static file scanning type capability.
/// This struct is public for visibility from lib.rs the core of um_engine, but it not intended to be accessed from the 
/// Tauri application - for handling state (which tauri will need to interact with), see FileScannerState
pub struct FileScanner {
    // iocs:
    // Using a BTreeSet for the IOCs as it has the best time complexity for searching - Rust's implementation in the stdlib
    // I don't think is the best optimised BTree out there, but it will do the job for now. Not adding any IOC metadata to this
    // list of hashes (aka turning this into a BTreeMap) as it's a waste of memory and that metadata can be looked up with automations
    // either locally on disk or in the cloud.
    iocs: BTreeSet<String>,
    // state - The state of the scanner so we can lock it whilst scanning
    pub state: Arc<Mutex<FileScannerState>>,
    pub scanning_info: Arc<Mutex<ScanningLiveInfo>>,
    log: Log,
}

trait SLI {
    fn new() -> Self;
    fn reset(&mut self);
}

impl SLI for ScanningLiveInfo {
    fn new() -> Self {
        ScanningLiveInfo {
            num_files_scanned: 0,
            time_taken: Duration::new(0, 0),
            scan_results: Vec::<MatchedIOC>::new(),
        }
    }

    fn reset(&mut self) {
        self.num_files_scanned = 0;
        self.scan_results = Vec::new();
        self.time_taken = Duration::new(0, 0);
    }
}


impl FileScanner {
    /// Construct a new instance of the FileScanner with no parameters.
    pub async fn new() -> Result<Self, std::io::Error> {

        let log = Log::new();

        //
        // ingest latest IOC hash list
        //
        let mut bts: BTreeSet<String> = BTreeSet::new();
        let mut ioc_location: String = std::env::var("APPDATA")
            .expect("[-] Could not find App Data folder in environment variables.]");
        ioc_location.push_str(format!("\\{}", IOC_LIST_LOCATION).as_str());

        let file = match File::open(&ioc_location) {
            Ok(f) => f,
            Err(e) => {
                log.log(LogLevel::Warning, format!("[-] IOC list not found, downloading to {}.", ioc_location).as_str());
                if e.kind() == io::ErrorKind::NotFound {
                    let file_data = reqwest::get(IOC_URL)
                        .await.unwrap()
                        .text().await.unwrap();
                    let mut f = File::create_new(&ioc_location).expect(format!("[-] Could not create new file for IOCs. Loc: {}", ioc_location).as_str());
                    f.write_all(file_data.as_bytes()).expect("[-] Could not write data for IOCs");
                    
                    f
                } else {
                    panic!("[-] Unknown error occurred when trying to ingest IOC files. {e}");
                }
            },
        };
        let lines = BufReader::new(file).lines();

        for line in lines.map_while(Result::ok) {
            bts.insert(line);
        }

        Ok(
            FileScanner {
                iocs: bts,
                state: Arc::new(Mutex::new(FileScannerState::Inactive)),
                scanning_info: Arc::new(Mutex::new(ScanningLiveInfo::new())),
                log,
            }
        )
    }


    /// Cancels the current scan
    pub fn cancel_scan(&self) -> Option<ScanningLiveInfo>{
        let mut lock = self.state.lock().unwrap();

        // check we are scanning, if not return
        if *lock == FileScannerState::Scanning {
            *lock = FileScannerState::Cancelled; // update state
            let sli = self.scanning_info.lock().unwrap();

            return Some(sli.clone());
        } 

        None
    }


    pub fn scan_started(&self) {
        let mut lock = self.state.lock().unwrap();
        *lock = FileScannerState::Scanning;
        // reset the stats
        self.scanning_info.lock().unwrap().reset();
    }


    /// Checks whether a scan is in progress
    pub fn is_scanning(&self) -> bool {
        let lock = self.state.lock().unwrap();
        match *lock {
            FileScannerState::Scanning => true,
            FileScannerState::Finished => false,
            FileScannerState::FinishedWithError(_) => false,
            FileScannerState::Inactive => false,
            FileScannerState::Cancelled => false,
        }
    }


    /// Updates the internal is_scanning state to false
    pub fn end_scan(&self) {
        let mut lock = self.state.lock().unwrap();
        *lock = FileScannerState::Inactive;
    }


    /// Scan the file held by the FileScanner against a set of known bad hashes
    /// 
    /// # Returns
    /// 
    /// The function will return a tuple of Ok (String, PathBuf) if there were no IO errors, and the result of the Ok will be an Option of type
    /// (String, PathBuf). If the function returns None, then there was no hash match made for malware. 
    /// 
    /// If it returns the Some variant, the hash of the IOC will be returned for post-processing and decision making, as well as the file name / path as PathBuf.
    fn scan_file_against_hashes(&self, target: &PathBuf, files_scanned: &Arc<Mutex<u32>>) -> Result<Option<(String, PathBuf)>, std::io::Error>{
        //
        // In order to not read the whole file into memory (would be bad if the file size is > the amount of RAM available)
        // I've decided to loop over an array of 1024 bytes at at time until the end of the file, and use the hashing crate sha2
        // to update the hash values, this should produce the hash without requiring the whole file read into memory.
        //

        let file = File::open(target)?;
        let mut reader = BufReader::new(&file);

        let hash = {
            let mut hasher = Md5::new();

            //
            // We are going to put the file data as bytes onto the heap to prevent a stack buffer overrun, and in doing so
            // we don't want to consume all the available memory. Therefore, we will limit the maximum heap allocation to
            // 50 mb per file. If the file is of a size less than this, we will only heap allocate the amount of size needed
            // otherwise, we will heap allocate 50 mb.
            //

            const MAX_HEAP_SIZE: usize = 50000000; // 50 mb

            let alloc_size: usize = if let Ok(f) = file.metadata() {
                let file_size = f.file_size() as usize;

                if file_size < MAX_HEAP_SIZE {
                    // less than 50 mb
                    file_size
                } else {
                    MAX_HEAP_SIZE
                }                    
            } else {
                // if there was an error getting the metadata, default to the max size
                MAX_HEAP_SIZE
            };


            let mut buf = vec![0u8; alloc_size];

            // let mut buf = vec![0u8; alloc_size];
            
            //
            // ingest the file and update hash value per chunk(if chunking)
            //
            loop {
                //
                // This is a sensible place to check whether the user has cancelled the scan, anything before this is likely
                // too short a time period to have the user stop the scan.
                //
                if self.get_state() == FileScannerState::Cancelled {
                    return Ok(None);
                }

                let count = reader.read(&mut buf)?;
                if count == 0 {break;}
                hasher.update(&buf[..count]);
            }
            
            hasher.finalize()
        };
        let hash: String = hash.iter().map(|byte| format!("{:02X}", byte)).collect();

        // increment the number of files scanned
        {
            let mut files_scanned = files_scanned.lock().unwrap();
            *files_scanned += 1;
        }
        
        // check the BTreeSet
        if self.iocs.contains(hash.as_str()) {
            // if we have a match on the malware..
            return Ok(Some((hash, target.clone())));
        }

        // No malware found
        Ok(None)

    }


    /// Public API entry point, scans from a root folder including all children, this can be used on a small 
    /// scale for a folder scan, or used to initiate a system scan.
    pub fn begin_scan(&self, input_dirs: Vec<PathBuf>) -> Result<FileScannerState, io::Error> {
        
        let mut discovered_dirs: Vec<PathBuf> = Vec::new();

        // If the target is a directory, then add it back to the discovered dirs as that will be iterated
        // separate to the target - target is just used for scanning a single file.
        // This could be refactored at a later date so this check is done more inline below whilst still adhering
        // to how the functionality works.
        let mut target = PathBuf::new();
        if input_dirs.len() == 1 {
            target = input_dirs.clone().pop().unwrap();
            discovered_dirs.push(target.clone());
        } else {
            for t in input_dirs {
                if t.exists() && t.is_dir() {
                    discovered_dirs.push(t.clone());
                    target = t;
                }
            }
        }        

        let stop_clock = Arc::new(Mutex::new(false));
        let clock_clone = Arc::clone(&stop_clock);
        let self_scanning_info_clone = Arc::clone(&self.scanning_info);

        let files_scanned: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
        let files_scanned_clone = Arc::clone(&files_scanned);
        let files_scanned_for_scanner = Arc::clone(&files_scanned);

        // timer in its own green thread
        thread::spawn(move || {
            let timer = Instant::now();

            loop {
                // first check if the task is cancelled
                if *clock_clone.lock().unwrap() == true {
                    break;
                }

                // not cancelled, so get the elapsed time
                let elapsed = timer.elapsed();
                let delta_files_scanned = {
                    let mut files_scanned_lock = files_scanned_clone.lock().unwrap();
                    let r = *files_scanned_lock; // get the result value
                    *files_scanned_lock = 0; // reset to 0
                    r
                };
                {
                    let mut lock = self_scanning_info_clone.lock().unwrap();
                    lock.time_taken = elapsed;
                    lock.num_files_scanned = lock.num_files_scanned + delta_files_scanned as u128;
                }

                std::thread::sleep(Duration::from_millis(10));
            }
        });
        
        // if the target is a FILE, then scan only the 1 file
        if !target.is_dir() {
            let res = self.scan_file_against_hashes(&target, &files_scanned_for_scanner);
            match res {
                Ok(res) => {
                    if let Some(v) = res {
                        let mut lock = self.scanning_info.lock().unwrap();
                        lock.scan_results.push(
                            MatchedIOC {
                                hash: v.0,
                                file: v.1,
                            }
                        );
                        
                        // result will contain the matched IOC
                        *stop_clock.lock().unwrap() = true;
                        return Ok(FileScannerState::Finished);
                    }

                    return Ok(FileScannerState::Finished);
                },
                Err(e) => {
                    *stop_clock.lock().unwrap() = true;

                    if e.kind() == io::ErrorKind::Uncategorized {
                        // results will be empty here
                        return Ok(FileScannerState::Cancelled);
                    }

                    return Err(e);
                },
            }
        }

        // otherwise, we are a directory so start this off
        while !discovered_dirs.is_empty() {

            // pop a directory
            let target = discovered_dirs.pop();
            if target.is_none() { continue; }

            // attempt to read the directory, if we don't have permission, continue to next item.
            let read_dir = fs::read_dir(target.unwrap());
            if read_dir.is_err() { continue; }

            for entry in read_dir.unwrap() {
                let entry = match entry {
                    Ok(b) => b,
                    Err(e) => {
                        self.log.log(LogLevel::Warning, &format!("[-] Error with entry, e: {e}"));
                        continue;
                    },
                };

                // check whether the scan is cancelled
                {
                    let lock = self.state.lock().unwrap();
                    if *lock == FileScannerState::Cancelled {
                        // todo update the error type of this fn to something more flexible
                        *stop_clock.lock().unwrap() = true;
                        return Err(io::Error::new(io::ErrorKind::Uncategorized, "User cancelled scan."));
                    }
                }

                let path = entry.path();

                // todo some profiling here to see where the slowdowns are and if it can be improved
                // i suspect large file size ingests is causing the difference in speed as it reads it
                // into a buffer.

                // add the folder to the next iteration 
                if path.is_dir() {
                    discovered_dirs.push(path);
                    continue; // keep searching for a file
                }

                //
                // Check the file against the hashes, we are only interested in positive matches at this stage
                //
                match self.scan_file_against_hashes(&path, &files_scanned_for_scanner) {
                    Ok(v) => {
                        if v.is_some() {
                            let v = v.unwrap();
                            let mut lock = self.scanning_info.lock().unwrap();
                            lock.scan_results.push(MatchedIOC {
                                hash: v.0,
                                file: v.1,
                            });
                        }
                    },
                    Err(e) => self.log.log(LogLevel::Warning, &format!("[-] Error scanning: {e}")),
                }
            }
        }

        *stop_clock.lock().unwrap() = true;

        Ok(FileScannerState::Finished)

    }


    pub fn get_state(&self) -> FileScannerState {
        let lock = self.state.lock().unwrap();
        lock.clone()
    }

}