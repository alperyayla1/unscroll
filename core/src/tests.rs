use crate::plan::*;
use crate::state::*;
use crate::*;

const MIN: u64 = 60_000;
const HOUR: u64 = 3_600_000;

fn locked(ms: u64) -> State {
    let mut s = State::default();
    s.tick(1_000, 1_700_000_000_000, None);
    s.start_lock(ms).unwrap();
    s
}

#[test]
fn defaults_block_shorts_and_tiktok() {
    let s = State::default();
    let e = s.effective();
    assert!(e.active);
    assert_eq!(e.level("youtube"), Level::Shorts);
    assert_eq!(e.level("tiktok"), Level::Site);
    assert_eq!(e.level("x"), Level::Open);
}

#[test]
fn toggle_off_disables_everything_when_unlocked() {
    let mut s = State::default();
    s.set_shorts(false).unwrap();
    s.set_sites(false).unwrap();
    let e = s.effective();
    assert!(!e.active);
    assert_eq!(e.level("youtube"), Level::Open);
    assert!(hosts_entries(&e).is_empty());
    assert!(url_blocklist_common(&e).is_empty());
}

#[test]
fn lock_cannot_be_loosened() {
    let mut s = locked(HOUR);
    assert_eq!(s.set_shorts(false), Err(RuleError::Locked));
    assert_eq!(s.set_sites(false), Err(RuleError::Locked));
    assert_eq!(s.set_level("youtube", Level::Open), Err(RuleError::Locked));
    assert_eq!(s.set_hard(HardKey::Incognito, false), Err(RuleError::Locked));
    // Tightening is fine.
    s.set_level("x", Level::Site).unwrap();
    s.set_level("youtube", Level::Site).unwrap();
    // Going back down to the floor is fine, below it is not.
    s.set_level("youtube", Level::Shorts).unwrap();
    assert!(s.effective().locked);
}

#[test]
fn lock_forces_active_even_if_toggle_was_off() {
    let mut s = State::default();
    s.set_shorts(false).unwrap();
    s.tick(1, 1_700_000_000_000, None);
    s.start_lock(30 * MIN).unwrap();
    let e = s.effective();
    assert!(e.active && e.locked);
    assert_eq!(e.level("instagram"), Level::Shorts);
}

#[test]
fn lock_with_everything_open_uses_defaults() {
    let mut s = State::default();
    for p in crate::platforms::PLATFORMS {
        s.set_level(p.id, Level::Open).unwrap();
    }
    s.tick(1, 1_700_000_000_000, None);
    s.start_lock(HOUR).unwrap();
    assert_eq!(s.effective().level("youtube"), Level::Shorts);
}

#[test]
fn lock_duration_limits() {
    let mut s = State::default();
    assert_eq!(s.start_lock(1_000), Err(RuleError::BadDuration));
    assert_eq!(s.start_lock(MAX_LOCK_MS + 1), Err(RuleError::BadDuration));
    s.start_lock(MAX_LOCK_MS).unwrap();
}

#[test]
fn extending_adds_time() {
    let mut s = locked(HOUR);
    s.start_lock(HOUR).unwrap();
    assert_eq!(s.lock.as_ref().unwrap().remaining_ms, 2 * HOUR);
}

#[test]
fn clock_change_within_boot_does_not_end_lock() {
    let mut s = locked(HOUR);
    // 10 minutes of uptime pass, but the wall clock is set a week ahead.
    s.tick(1_000 + 10 * MIN, 1_700_000_000_000 + 7 * 24 * HOUR, None);
    assert_eq!(s.lock.as_ref().unwrap().remaining_ms, 50 * MIN);
}

#[test]
fn clock_set_back_does_not_extend_or_break() {
    let mut s = locked(HOUR);
    s.tick(1_000 + 5 * MIN, 1_600_000_000_000, None);
    assert_eq!(s.lock.as_ref().unwrap().remaining_ms, 55 * MIN);
}

#[test]
fn reboot_counts_real_time_and_never_negative() {
    let mut s = locked(HOUR);
    s.tick(1_000 + 10 * MIN, 1_700_000_000_000 + 10 * MIN, None); // 50 min left
    // Reboot (uptime reset), 20 minutes of wall time passed.
    s.tick(30_000, 1_700_000_000_000 + 30 * MIN, None);
    assert_eq!(s.lock.as_ref().unwrap().remaining_ms, 30 * MIN);
    // Reboot with clock set backwards: no time passes.
    s.tick(10_000, 1_600_000_000_000, None);
    assert_eq!(s.lock.as_ref().unwrap().remaining_ms, 30 * MIN);
}

#[test]
fn reboot_with_network_time_ignores_fake_clock() {
    let mut s = locked(HOUR);
    let real = 1_700_000_000_000u64;
    // Reboot, user set the clock a year ahead, but network says 5 min passed.
    s.tick(500, real + 365 * 24 * HOUR, Some(real + 5 * MIN));
    assert_eq!(s.lock.as_ref().unwrap().remaining_ms, 55 * MIN);
}

#[test]
fn lock_ends() {
    let mut s = locked(30 * MIN);
    assert!(s.tick(1_000 + 31 * MIN, 0, None));
    assert!(s.lock.is_none());
    assert!(!s.effective().locked);
    s.set_shorts(false).unwrap();
}

#[test]
fn tamper_detection_and_restore() {
    let s = locked(HOUR);
    let good = encode(&s);
    assert_eq!(decode(&good), Some(s.clone()));
    let tampered = good.replace("3600000", "0000060");
    assert!(decode(&tampered).is_none());
    let (r, o) = choose(Some(&tampered), Some(&good));
    assert_eq!(o, LoadOutcome::Restored);
    assert!(r.is_locked());
    // Deleting the file: backup still wins.
    let (r, _) = choose(None, Some(&good));
    assert!(r.is_locked());
    // Replacing the file with an older, unlocked but validly signed state: the locked backup wins.
    let (r, _) = choose(Some(&encode(&State::default())), Some(&good));
    assert!(r.is_locked());
    // Both garbage: strict default.
    let (r, o) = choose(Some("x"), Some("y"));
    assert_eq!(o, LoadOutcome::Reset);
    assert!(r.effective().active);
    let (_, o) = choose(None, None);
    assert_eq!(o, LoadOutcome::Fresh);
}

#[test]
fn hosts_rewrite_is_idempotent_and_reversible() {
    let orig = "127.0.0.1 localhost\r\n# my stuff\r\n";
    let e = State::default().effective();
    let entries = hosts_entries(&e);
    assert!(entries.contains(&"www.tiktok.com"));
    let once = rewrite_hosts(orig, &entries);
    let twice = rewrite_hosts(&once, &entries);
    assert_eq!(once, twice);
    assert!(once.contains("0.0.0.0 www.tiktok.com\r\n"));
    let back = rewrite_hosts(&once, &[]);
    assert_eq!(back, orig);
}

#[test]
fn policies_follow_lock_hardening() {
    let s = State::default();
    let p = vendor_policies(&s.effective());
    assert!(p.iter().all(|v| v.dwords.is_empty()));
    assert!(p[0].url_blocklist.contains(&"youtube.com/shorts".to_string()));
    assert!(!p[0].url_blocklist.iter().any(|u| u.starts_with("chrome://")));

    let s = locked(HOUR);
    let p = vendor_policies(&s.effective());
    let chrome = &p[0];
    assert!(chrome.url_blocklist.contains(&"chrome://extensions".to_string()));
    assert!(chrome.dwords.contains(&("IncognitoModeAvailability", 1)));
    assert!(p[1].dwords.contains(&("InPrivateModeAvailability", 1)));
}

#[test]
fn site_level_blocks_whole_domain() {
    let mut s = State::default();
    s.set_level("x", Level::Site).unwrap();
    let e = s.effective();
    assert!(url_blocklist_common(&e).contains(&"x.com".to_string()));
    assert!(hosts_entries(&e).contains(&"twitter.com"));
    assert_eq!(s.set_level("x", Level::Shorts), Err(RuleError::Unsupported));
}

#[test]
fn manifest_and_task_are_well_formed() {
    let m = native_host_manifest(r"C:\Program Files\Unshorts\unshorts-host.exe");
    let v: serde_json::Value = serde_json::from_str(&m).unwrap();
    assert_eq!(v["name"], NATIVE_HOST_NAME);
    assert!(v["allowed_origins"][0].as_str().unwrap().starts_with("chrome-extension://"));
    let x = task_xml(r"C:\Program Files\Unshorts\unshorts.exe");
    assert!(x.contains("--enforce") && x.contains("S-1-5-18"));
}

#[test]
fn switches_are_independent() {
    let mut s = State::default();
    s.set_level("x", Level::Site).unwrap();
    // Shorts off: YouTube Shorts and TikTok open, X still blocked.
    s.set_shorts(false).unwrap();
    let e = s.effective();
    assert_eq!(e.level("youtube"), Level::Open);
    assert_eq!(e.level("tiktok"), Level::Open);
    assert_eq!(e.level("x"), Level::Site);
    assert!(e.active);
    // Sites off, shorts on.
    s.set_shorts(true).unwrap();
    s.set_sites(false).unwrap();
    let e = s.effective();
    assert_eq!(e.level("youtube"), Level::Shorts);
    assert_eq!(e.level("tiktok"), Level::Site);
    assert_eq!(e.level("x"), Level::Open);
    // Lock turns both back on.
    s.tick(1, 1_700_000_000_000, None);
    s.start_lock(HOUR).unwrap();
    assert_eq!(s.effective().level("x"), Level::Site);
}

#[test]
fn old_state_files_still_load() {
    let mut v: serde_json::Value = serde_json::to_value(State::default()).unwrap();
    let o = v.as_object_mut().unwrap();
    o.remove("shorts_on");
    o.remove("sites_on");
    o.insert("enabled".into(), true.into());
    let s: State = serde_json::from_value(v).unwrap();
    assert!(s.shorts_on && s.sites_on);
}
