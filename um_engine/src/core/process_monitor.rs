/// The ProcessMonitor is responsible for monitoring all processes running
/// on the system, and keeping track of risk scores, and activity conducted
/// by processes.
pub struct ProcessMonitor {
    pid: u32,
    process_name: String,
    risk_score: u8,
    allow_listed: bool, // whether the application is allowed to exist without monitoring
}