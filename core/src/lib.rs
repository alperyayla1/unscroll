//! Shared logic for the Unshorts app, its background enforcer and the
//! native-messaging host. No platform code lives here.

pub mod plan;
pub mod platforms;
pub mod state;

use sha2::{Digest, Sha256};
use state::State;
use std::path::PathBuf;

pub const APP_NAME: &str = "Unshorts";
pub const NATIVE_HOST_NAME: &str = "com.darkskystudios.unshorts";
pub const TASK_NAME: &str = r"DarkSky Studios\Unshorts Enforcer";

/// ID of the unpacked extension (fixed by the "key" in extension/manifest.json).
pub const EXTENSION_ID_DEV: &str = "apjejhddoodocpnbmjkamfphefgfmmkc";
/// After publishing, put the Chrome Web Store ID here. It is then force-installed
/// in Chrome and Brave so it can't be removed or disabled.
pub const EXTENSION_IDS_CHROME: &[&str] = &[];
/// After publishing, put the Edge Add-ons ID here.
pub const EXTENSION_IDS_EDGE: &[&str] = &[];

/// Makes hand-editing state.json pointless: an edited file fails the check and
/// the backup copy (or a strict default) is used instead.
const SIGN_SALT: &[u8] = b"unshorts/darksky/v1/7f3c2e91b4a0";

pub fn data_dir() -> PathBuf {
    if let Ok(p) = std::env::var("UNSHORTS_DATA_DIR") {
        return PathBuf::from(p);
    }
    let base = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".into());
    PathBuf::from(base).join(APP_NAME)
}

pub fn state_path() -> PathBuf {
    data_dir().join("state.json")
}

fn sign(body: &str) -> String {
    let mut h = Sha256::new();
    h.update(SIGN_SALT);
    h.update(body.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Serializes and signs. The result is what gets stored (file and backup).
pub fn encode(state: &State) -> String {
    let body = serde_json::to_string(state).unwrap_or_default();
    serde_json::json!({ "sig": sign(&body), "body": body }).to_string()
}

/// None if missing, malformed or tampered with.
pub fn decode(text: &str) -> Option<State> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    let body = v.get("body")?.as_str()?;
    let sig = v.get("sig")?.as_str()?;
    if sign(body) != sig {
        return None;
    }
    let mut s: State = serde_json::from_str(body).ok()?;
    s.normalize();
    Some(s)
}

/// What the state file said when it was loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadOutcome {
    Ok,
    /// No file yet: first run.
    Fresh,
    /// File was invalid; the backup was used.
    Restored,
    /// File and backup invalid: strict state (everything on).
    Reset,
}

/// Picks the best state from the file and the backup copy.
/// When both are valid, the one with the longer lock wins, so deleting or
/// replacing one copy never shortens a lock.
pub fn choose(file: Option<&str>, backup: Option<&str>) -> (State, LoadOutcome) {
    let f = file.and_then(decode);
    let b = backup.and_then(decode);
    let rem = |s: &State| s.lock.as_ref().map_or(0, |l| l.remaining_ms);
    match (f, b) {
        (Some(f), Some(b)) => {
            if rem(&b) > rem(&f) {
                (b, LoadOutcome::Restored)
            } else {
                (f, LoadOutcome::Ok)
            }
        }
        (Some(f), None) => (f, LoadOutcome::Ok),
        (None, Some(b)) => (b, LoadOutcome::Restored),
        (None, None) if file.is_none() && backup.is_none() => (State::default(), LoadOutcome::Fresh),
        (None, None) => (State::strict(), LoadOutcome::Reset),
    }
}

#[cfg(test)]
mod tests;
