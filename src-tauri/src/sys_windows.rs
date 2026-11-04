//! Windows side: hosts file, browser policies, native-messaging registration,
//! the scheduled task, the registry backup and the cross-process lock.

use std::io::Write;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use unscroll_core::plan::{self, LIST_INDEX_END, LIST_INDEX_START};
use unscroll_core::state::Effective;
use unscroll_core::{data_dir, NATIVE_HOST_NAME, TASK_NAME};
use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_ALL_ACCESS, KEY_READ};
use winreg::RegKey;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const BACKUP_KEY: &str = r"SOFTWARE\DarkSky Studios\Unscroll";

fn hklm() -> RegKey {
    RegKey::predef(HKEY_LOCAL_MACHINE)
}

fn run_hidden(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// ---------------------------------------------------------------- lock

/// Exclusive lock file shared by the app window and the background enforcer.
pub struct StateLock(#[allow(dead_code)] Option<std::fs::File>);

impl StateLock {
    pub fn acquire() -> StateLock {
        let dir = data_dir();
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("state.lock");
        for _ in 0..100 {
            let f = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .share_mode(0)
                .open(&path);
            if let Ok(f) = f {
                return StateLock(Some(f));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        StateLock(None) // give up after ~5 s rather than hang
    }
}

// ---------------------------------------------------------------- time

pub fn uptime_ms() -> u64 {
    // Includes time spent asleep; unaffected by changes to the system clock.
    unsafe { windows_sys::Win32::System::SystemInformation::GetTickCount64() }
}

pub fn system_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Current time from the `Date` header of a well-known HTTPS server.
pub fn network_ms() -> Option<u64> {
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(4)).build();
    for url in ["https://www.google.com/generate_204", "https://www.cloudflare.com/"] {
        if let Ok(resp) = agent.head(url).call() {
            if let Some(date) = resp.header("date") {
                if let Ok(t) = httpdate::parse_http_date(date) {
                    if let Ok(d) = t.duration_since(std::time::UNIX_EPOCH) {
                        return Some(d.as_millis() as u64);
                    }
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------- backup

pub fn read_backup() -> Option<String> {
    hklm()
        .open_subkey_with_flags(BACKUP_KEY, KEY_READ)
        .ok()?
        .get_value::<String, _>("state")
        .ok()
}

pub fn write_backup(text: &str) {
    if let Ok((k, _)) = hklm().create_subkey(BACKUP_KEY) {
        let _ = k.set_value("state", &text.to_string());
    }
}

// ---------------------------------------------------------------- enforcement

fn hosts_path() -> PathBuf {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
    PathBuf::from(root).join(r"System32\drivers\etc\hosts")
}

fn sync_hosts(entries: &[&str]) {
    let path = hosts_path();
    let original = std::fs::read_to_string(&path).unwrap_or_default();
    let updated = plan::rewrite_hosts(&original, entries);
    if updated == original {
        return;
    }
    if let Ok(meta) = std::fs::metadata(&path) {
        let mut perms = meta.permissions();
        if perms.readonly() {
            #[allow(clippy::permissions_set_readonly_false)]
            perms.set_readonly(false);
            let _ = std::fs::set_permissions(&path, perms);
        }
    }
    if let Ok(mut f) = std::fs::File::create(&path) {
        if f.write_all(updated.as_bytes()).is_ok() {
            run_hidden("ipconfig", &["/flushdns"]);
        }
    }
}

/// Makes a list policy (URLBlocklist, ExtensionInstallForcelist) contain exactly
/// `wanted` in our index range. Entries outside the range are left alone.
/// Skips writing when nothing changed, so browsers don't reload policies needlessly.
fn sync_list(key_path: &str, wanted: &[String]) {
    let existing: Vec<(String, String)> = hklm()
        .open_subkey_with_flags(key_path, KEY_READ)
        .map(|k| {
            let names: Vec<String> = k
                .enum_values()
                .filter_map(|v| v.ok())
                .map(|(name, _)| name)
                .filter(|name| {
                    name.parse::<u32>().map_or(false, |n| (LIST_INDEX_START..=LIST_INDEX_END).contains(&n))
                })
                .collect();
            names
                .into_iter()
                .map(|name| {
                    let val = k.get_value::<String, _>(&name).unwrap_or_default();
                    (name, val)
                })
                .collect()
        })
        .unwrap_or_default();

    let desired: Vec<(String, String)> = wanted
        .iter()
        .take((LIST_INDEX_END - LIST_INDEX_START) as usize)
        .enumerate()
        .map(|(i, v)| ((LIST_INDEX_START + i as u32).to_string(), v.clone()))
        .collect();

    let mut a = existing.clone();
    let mut b = desired.clone();
    a.sort();
    b.sort();
    if a == b {
        return;
    }
    if desired.is_empty() && existing.is_empty() {
        return;
    }
    let Ok((k, _)) = hklm().create_subkey(key_path) else { return };
    for (name, _) in &existing {
        let _ = k.delete_value(name);
    }
    for (name, val) in &desired {
        let _ = k.set_value(name, val);
    }
}

fn sync_policies(eff: &Effective, owned: &mut Vec<String>) {
    let mut now_owned = Vec::new();
    for vp in plan::vendor_policies(eff) {
        sync_list(&format!(r"{}\URLBlocklist", vp.policy_key), &vp.url_blocklist);
        sync_list(&format!(r"{}\ExtensionInstallForcelist", vp.policy_key), &vp.force_install);
        if !vp.dwords.is_empty() {
            if let Ok((k, _)) = hklm().create_subkey(vp.policy_key) {
                for (name, val) in &vp.dwords {
                    if k.get_value::<u32, _>(name).ok() != Some(*val) {
                        let _ = k.set_value(name, val);
                    }
                    now_owned.push(format!("{}|{}", vp.policy_key, name));
                }
            }
        }
    }
    // Remove DWORD policies we set earlier that are no longer wanted.
    for entry in owned.iter() {
        if now_owned.contains(entry) {
            continue;
        }
        if let Some((key, name)) = entry.split_once('|') {
            if let Ok(k) = hklm().open_subkey_with_flags(key, KEY_ALL_ACCESS) {
                let _ = k.delete_value(name);
            }
        }
    }
    *owned = now_owned;
}

pub fn apply(eff: &Effective, owned_policies: &mut Vec<String>) {
    sync_hosts(&plan::hosts_entries(eff));
    sync_policies(eff, owned_policies);
}

// ---------------------------------------------------------------- setup / cleanup

fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default()
}

/// Only SYSTEM and Administrators may change the data folder; users can read it
/// (the extension bridge runs as the user and needs to read the state).
fn harden_data_dir() {
    let dir = data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let d = dir.to_string_lossy().to_string();
    run_hidden(
        "icacls",
        &[
            d.as_str(),
            "/inheritance:r",
            "/grant:r",
            "*S-1-5-18:(OI)(CI)F",
            "*S-1-5-32-544:(OI)(CI)F",
            "*S-1-5-32-545:(OI)(CI)RX",
            "/T",
            "/Q",
        ],
    );
}

fn register_native_host() {
    let host = exe_dir().join("unscroll-host.exe");
    let manifest = plan::native_host_manifest(&host.to_string_lossy());
    let manifest_path = data_dir().join("native-host.json");
    if std::fs::write(&manifest_path, manifest).is_err() {
        return;
    }
    let value = manifest_path.to_string_lossy().to_string();
    for base in plan::NATIVE_HOST_KEYS {
        if let Ok((k, _)) = hklm().create_subkey(format!(r"{base}\{NATIVE_HOST_NAME}")) {
            let _ = k.set_value("", &value);
        }
    }
}

fn register_task() {
    let exe = exe_dir().join("unscroll.exe");
    let xml = plan::task_xml(&exe.to_string_lossy());
    // The XML declares UTF-16, so write it as UTF-16LE with a BOM.
    let mut bytes = vec![0xFF, 0xFE];
    for u in xml.encode_utf16() {
        bytes.extend_from_slice(&u.to_le_bytes());
    }
    let path = data_dir().join("task.xml");
    if std::fs::write(&path, bytes).is_ok() {
        let xml_path = path.to_string_lossy().to_string();
        run_hidden("schtasks", &["/Create", "/TN", TASK_NAME, "/XML", xml_path.as_str(), "/F"]);
        let _ = std::fs::remove_file(&path);
    }
}

pub fn setup() {
    harden_data_dir();
    register_native_host();
    register_task();
}

pub fn cleanup(owned_policies: &mut Vec<String>) {
    run_hidden("schtasks", &["/Delete", "/TN", TASK_NAME, "/F"]);
    let off = Effective {
        active: false,
        locked: false,
        lock_remaining_ms: 0,
        levels: Default::default(),
        hardening: Default::default(),
    };
    apply(&off, owned_policies);
    for base in plan::NATIVE_HOST_KEYS {
        let _ = hklm().delete_subkey_all(format!(r"{base}\{NATIVE_HOST_NAME}"));
    }
    let _ = hklm().delete_subkey_all(BACKUP_KEY);
    let _ = std::fs::remove_dir_all(data_dir());
}

// ---------------------------------------------------------------- misc

/// (connected, last seen ms, browser) from the bridge's heartbeat file.
pub fn extension_status() -> (bool, u64, String) {
    let Ok(base) = std::env::var("LOCALAPPDATA") else { return (false, 0, String::new()) };
    let path = std::path::Path::new(&base).join(r"Unscroll\extension.json");
    let Ok(text) = std::fs::read_to_string(path) else { return (false, 0, String::new()) };
    let v: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
    let ts = v["ts"].as_u64().unwrap_or(0);
    let browser = v["browser"].as_str().unwrap_or("").to_string();
    let fresh = system_ms().saturating_sub(ts) < 3 * 60_000;
    (fresh, ts, browser)
}

pub fn extension_folder() -> PathBuf {
    exe_dir().join("extension")
}

pub fn open_path(path: &std::path::Path) {
    let _ = Command::new("explorer").arg(path).spawn();
}
