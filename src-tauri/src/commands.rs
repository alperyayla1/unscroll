//! Commands the UI calls. Each returns the full view so the UI just re-renders.

use crate::store::{transact, Snapshot};
use crate::sys;
use serde::Serialize;
use unshorts_core::platforms::PLATFORMS;
use unshorts_core::state::{HardKey, Hardening, Level};
use unshorts_core::LoadOutcome;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformView {
    id: &'static str,
    name: &'static str,
    level: Level,
    /// Lowest level allowed right now (the lock's floor).
    floor: Level,
    supports_shorts: bool,
    all_shorts: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionView {
    connected: bool,
    last_seen_ms: u64,
    browser: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    shorts_on: bool,
    sites_on: bool,
    active: bool,
    locked: bool,
    remaining_ms: u64,
    total_ms: u64,
    platforms: Vec<PlatformView>,
    hardening: Hardening,
    hard_floor: Hardening,
    extension: ExtensionView,
    /// "ok" | "fresh" | "restored" | "reset"
    load: &'static str,
}

fn view(s: &Snapshot) -> View {
    let st = &s.state;
    let eff = st.effective();
    let (connected, last_seen_ms, browser) = sys::extension_status();
    View {
        shorts_on: st.shorts_on,
        sites_on: st.sites_on,
        active: eff.active,
        locked: eff.locked,
        remaining_ms: eff.lock_remaining_ms,
        total_ms: st.lock.as_ref().map_or(0, |l| l.total_ms),
        platforms: PLATFORMS
            .iter()
            .map(|p| PlatformView {
                id: p.id,
                name: p.name,
                // Show what is actually enforced (never below the floor).
                level: st.settings.level(p.id).max(st.floor_level(p.id)),
                floor: st.floor_level(p.id),
                supports_shorts: p.supports_shorts(),
                all_shorts: p.all_shorts,
            })
            .collect(),
        hardening: eff.hardening,
        hard_floor: st.floor_hard(),
        extension: ExtensionView { connected, last_seen_ms, browser },
        load: match s.outcome {
            LoadOutcome::Ok => "ok",
            LoadOutcome::Fresh => "fresh",
            LoadOutcome::Restored => "restored",
            LoadOutcome::Reset => "reset",
        },
    }
}

fn run(f: impl FnOnce(&mut unshorts_core::state::State) -> Result<(), unshorts_core::state::RuleError>) -> Result<View, String> {
    transact(f).map(|((), snap)| view(&snap))
}

#[tauri::command]
pub fn get_view() -> Result<View, String> {
    run(|_| Ok(()))
}

#[tauri::command]
pub fn set_shorts(on: bool) -> Result<View, String> {
    run(|s| s.set_shorts(on))
}

#[tauri::command]
pub fn set_sites(on: bool) -> Result<View, String> {
    run(|s| s.set_sites(on))
}

#[tauri::command]
pub fn set_level(id: String, level: Level) -> Result<View, String> {
    run(|s| s.set_level(&id, level))
}

#[tauri::command]
pub fn set_hard(key: String, on: bool) -> Result<View, String> {
    let key = HardKey::parse(&key).ok_or_else(|| "unknown_setting".to_string())?;
    run(|s| s.set_hard(key, on))
}

#[tauri::command]
pub fn start_lock(minutes: u64) -> Result<View, String> {
    let ms = minutes.saturating_mul(60_000);
    run(|s| s.start_lock(ms))
}

#[tauri::command]
pub fn open_extension_folder() {
    sys::open_path(&sys::extension_folder());
}
