//! Non-Windows stand-in so the UI can be developed elsewhere. Enforces nothing.

use std::path::PathBuf;
use unscroll_core::state::Effective;

pub struct StateLock;
impl StateLock {
    pub fn acquire() -> StateLock {
        StateLock
    }
}

pub fn uptime_ms() -> u64 {
    std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| s.split_whitespace().next().and_then(|v| v.parse::<f64>().ok()))
        .map(|s| (s * 1000.0) as u64)
        .unwrap_or(0)
}

pub fn system_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn network_ms() -> Option<u64> {
    None
}
pub fn read_backup() -> Option<String> {
    None
}
pub fn write_backup(_text: &str) {}
pub fn apply(_eff: &Effective, _owned: &mut Vec<String>) {}
pub fn setup() {}
pub fn cleanup(_owned: &mut Vec<String>) {}
pub fn extension_status() -> (bool, u64, String) {
    (false, 0, String::new())
}
pub fn extension_folder() -> PathBuf {
    PathBuf::from("../extension")
}
pub fn open_path(_path: &std::path::Path) {}
