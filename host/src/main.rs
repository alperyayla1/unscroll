//! Native messaging host. The browser starts it, sends one JSON message, reads
//! one reply and closes it. Runs as the normal user, read-only.
#![windows_subsystem = "windows"]

use std::io::{Read, Write};
use unshorts_core::{decode, state::State, state_path};

fn read_message() -> Option<serde_json::Value> {
    let mut len = [0u8; 4];
    std::io::stdin().read_exact(&mut len).ok()?;
    let len = u32::from_le_bytes(len) as usize;
    if len > 64 * 1024 {
        return None;
    }
    let mut buf = vec![0u8; len];
    std::io::stdin().read_exact(&mut buf).ok()?;
    serde_json::from_slice(&buf).ok()
}

fn write_message(v: &serde_json::Value) {
    let bytes = serde_json::to_vec(v).unwrap_or_default();
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(&(bytes.len() as u32).to_le_bytes());
    let _ = out.write_all(&bytes);
    let _ = out.flush();
}

/// Lets the app show "extension connected".
fn heartbeat(origin: &str, browser: &str) {
    let Ok(base) = std::env::var("LOCALAPPDATA") else { return };
    let dir = std::path::Path::new(&base).join("Unshorts");
    let _ = std::fs::create_dir_all(&dir);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let v = serde_json::json!({ "ts": now, "origin": origin, "browser": browser });
    let _ = std::fs::write(dir.join("extension.json"), v.to_string());
}

fn main() {
    // Chrome passes the caller's origin as the first argument.
    let origin = std::env::args().nth(1).unwrap_or_default();
    let msg = read_message().unwrap_or(serde_json::Value::Null);
    let browser = msg.get("browser").and_then(|b| b.as_str()).unwrap_or("").to_string();

    // A missing file means the app was never set up. A tampered one means
    // someone edited it: answer with the strict default rather than "off".
    let state = match std::fs::read_to_string(state_path()) {
        Ok(text) => decode(&text).unwrap_or_else(State::strict),
        Err(_) => State::default(),
    };
    let eff = state.effective();
    let levels: serde_json::Map<String, serde_json::Value> = unshorts_core::platforms::PLATFORMS
        .iter()
        .map(|p| (p.id.to_string(), serde_json::to_value(eff.level(p.id)).unwrap_or_default()))
        .collect();

    write_message(&serde_json::json!({
        "ok": true,
        "v": 1,
        "active": eff.active,
        "locked": eff.locked,
        "remainingMs": eff.lock_remaining_ms,
        "levels": levels,
    }));
    heartbeat(&origin, &browser);
}
