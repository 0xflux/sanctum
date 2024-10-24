//! File scanner module 
//! 
//! This module provides functionality for scanning files and retrieving relevant
//! information about a file that the EDR may want to use in decision making. 

use std::{collections::BTreeSet, fs::File, io::{self, BufRead, BufReader, Read}, path::PathBuf};

use sha2::{Sha256, Digest};

/// The FileScanner is the public interface into the module handling any static file scanning type capability.
pub struct FileScanner {
    target_file: Option<PathBuf>,
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
        let file = File::open("./ioc_list.txt")?;
        let lines = BufReader::new(file).lines();

        for line in lines.flatten() {
            bts.insert(line);
        }

        Ok(
            FileScanner {
                target_file: None,
                iocs: bts,
            }
        )
    }


    /// Construct a new instance of the FileScanner from a starting path.
    pub fn from(path: PathBuf) -> Result<Self, io::Error> {

        // first make a call to new to use that as a single point of initialisation
        let mut scanner = FileScanner::new()?;
        scanner.target_file = Some(path);

        Ok(scanner)
    }


    /// Scan the file held by the FileScanner against a set of known bad hashes
    /// 
    /// # Returns
    /// 
    /// The function will return a tuple of Ok (String, PathBuf) if there were no IO errors, and the result of the Ok will be an Option of type
    /// (String, PathBuf). If the function returns None, then there was no hash match made for malware. 
    /// 
    /// If it returns the Some variant, the hash of the IOC will be returned for post-processing and decision making, as well as the file name / path as PathBuf.
    pub fn scan_against_hashes(&self) -> Result<Option<(String, PathBuf)>, std::io::Error>{
        
        //
        // In order to not read the whole file into memory (would be bad if the file size is > the amount of RAM available)
        // I've decided to loop over an array of 1024 bytes at at time until the end of the file, and use the hashing crate sha2
        // to update the hash values, this should produce the hash without requiring the whole file read into memory.
        //

        let file = File::open(self.target_file.clone().unwrap())?;
        let mut reader = BufReader::new(file);

        let hash = {
            let mut hasher = Sha256::new();
            let mut buf = [0; 1024];

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
            return Ok(Some((hash, self.target_file.clone().unwrap())));
        }

        // No malware found
        Ok(None)

    }   


}