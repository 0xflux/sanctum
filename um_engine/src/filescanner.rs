//! File scanner module 
//! 
//! This module provides functionality for scanning files and retrieving relevant
//! information about a file that the EDR may want to use in decision making. 

use std::{collections::{BTreeMap, BTreeSet, HashMap}, fs::{self, File}, io::{self, BufRead, BufReader, Read}, path::PathBuf, time::Instant};

use sha2::{Sha256, Digest};

/// Structure for containing results pertaining to an IOC match
#[derive(Debug)]
pub struct MatchedIOC {
    hash: String,
    file: PathBuf,
}

/// The FileScanner is the public interface into the module handling any static file scanning type capability.
pub struct FileScanner {
    // Using a BTreeSet for the IOCs as it has the best time complexity for searching - Rust's implementation in the stdlib
    // I don't think is the best optimised BTree out there, but it will do the job for now. Not adding any IOC metadata to this
    // list of hashes (aka turning this into a BTreeMap) as it's a waste of memory and that metadata can be looked up with automations
    // either locally on disk or in the cloud.
    iocs: BTreeSet<String>,
}

impl FileScanner {
    /// Construct a new instance of the FileScanner with no parameters.
    pub fn new() -> Result<Self, std::io::Error> {

        //
        // ingest latest IOC hash list
        //
        let mut bts: BTreeSet<String> = BTreeSet::new();
        let file = File::open("../../ioc_list.txt")?;
        let lines = BufReader::new(file).lines();

        for line in lines.flatten() {
            bts.insert(line);
        }

        Ok(
            FileScanner {
                iocs: bts,
            }
        )
    }


    /// Scan the file held by the FileScanner against a set of known bad hashes
    /// 
    /// # Returns
    /// 
    /// The function will return a tuple of Ok (String, PathBuf) if there were no IO errors, and the result of the Ok will be an Option of type
    /// (String, PathBuf). If the function returns None, then there was no hash match made for malware. 
    /// 
    /// If it returns the Some variant, the hash of the IOC will be returned for post-processing and decision making, as well as the file name / path as PathBuf.
    pub fn scan_file_against_hashes(&self, target: PathBuf) -> Result<Option<(String, PathBuf)>, std::io::Error>{
        
        //
        // In order to not read the whole file into memory (would be bad if the file size is > the amount of RAM available)
        // I've decided to loop over an array of 1024 bytes at at time until the end of the file, and use the hashing crate sha2
        // to update the hash values, this should produce the hash without requiring the whole file read into memory.
        //

        const BUF_SIZE: usize = 10240000; // read 10 MB at a time
        let file = File::open(&target)?;
        let mut reader = BufReader::new(file);

        let hash = {
            let mut hasher = Sha256::new();
            let mut buf = [0; BUF_SIZE];

            loop {
                let count = reader.read(&mut buf)?;
                if count == 0 {break;}
                hasher.update(&buf[..count]);
            }
            
            hasher.finalize()
        };
        let hash = format!("{:X}", hash); // format as string, uppercase

        // check the BTreeSet
        if self.iocs.contains(hash.as_str()) {
            // if we have a match on the malware..
            return Ok(Some((hash, target)));
        }

        // No malware found
        Ok(None)

    }


    // TODO do a folder scan next, maybe this could be a good pathway into a whole system scan
    // perhaps this should have 'power' settings depending on resources etc?
    /// Public API entry point, scans from a root folder including all children, this can be used on a small 
    /// scale for a folder scan, or used to initiate a system scan.
    pub fn scan_from_folder_all_children(&self, target: PathBuf) -> Result<Vec<MatchedIOC>, io::Error> {

        if !target.is_dir() {
            eprintln!("[-] Target {} is not a directory.", target.display());
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Target is not a directory."));
        }

        let mut matched_iocs: Vec<MatchedIOC> = Vec::new();
        let mut discovered_dirs: Vec<PathBuf> = vec![target];
        let mut time_map: BTreeMap<u128, PathBuf> = BTreeMap::new();

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
                        eprintln!("[-] Error with entry, e: {e}");
                        continue;
                    },
                };
                let path = entry.path();

                // todo some profiling here to see where the slowdowns are and if it can be improved
                // i suspect large file size ingests is causing the difference in speed as it reads it
                // into a buffer.
                println!("[i] Path: {}", path.display());

                // add the folder to the next iteration 
                if path.is_dir() {
                    discovered_dirs.push(path);
                    continue; // keep searching for a file
                }

                //
                // Check the file against the hashes, we are only interested in positive matches at this stage
                //
                let pclone = path.clone();
                let now = Instant::now();
                match self.scan_file_against_hashes(pclone) {
                    Ok(v) => {
                        if v.is_some() {
                            let v = v.unwrap();
                            matched_iocs.push(MatchedIOC {
                                hash: v.0,
                                file: v.1,
                            });
                        }
                    },
                    Err(e) => eprintln!("[-] Error scanning dir: {e}"),
                }

                let elapsed = now.elapsed().as_millis();

                time_map.insert(elapsed, path);
            }

            // println!("[+] Items remaining in queue: {}", discovered_dirs.len())
        }

        let min_val = time_map.iter().next().unwrap();
        let max_val = time_map.iter().next_back().unwrap();

        println!("[i] Min: {:?}, Max: {:?}", min_val, max_val);

        Ok(matched_iocs)

    }

    // TODO schedule daily scans



}