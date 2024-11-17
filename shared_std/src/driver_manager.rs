#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DriverState {
    Uninstalled(String),
    Installed(String),
    Started(String),
    Stopped(String),
}