//! Turns the effective config into concrete changes: hosts file text, browser
//! policies, the scheduled-task XML and the native-messaging manifest.
//! Pure functions, so they are testable on any OS.

use crate::platforms::PLATFORMS;
use crate::state::{Effective, Level};
use crate::{EXTENSION_IDS_CHROME, EXTENSION_IDS_EDGE, NATIVE_HOST_NAME};

pub const HOSTS_BEGIN: &str = "# >>> Unscroll (DarkSky Studios) - do not edit >>>";
pub const HOSTS_END: &str = "# <<< Unscroll <<<";

/// Value names we use inside list policies (URLBlocklist etc.). A high, fixed
/// range so we never touch entries the user or an admin created.
pub const LIST_INDEX_START: u32 = 9001;
pub const LIST_INDEX_END: u32 = 9999;

pub struct Vendor {
    /// Key under HKLM, e.g. SOFTWARE\Policies\Google\Chrome
    pub policy_key: &'static str,
    pub incognito_policy: &'static str,
    pub extension_pages: &'static [&'static str],
    pub store_update_url: &'static str,
    pub store_ids: &'static [&'static str],
}

pub const VENDORS: &[Vendor] = &[
    Vendor {
        policy_key: r"SOFTWARE\Policies\Google\Chrome",
        incognito_policy: "IncognitoModeAvailability",
        extension_pages: &["chrome://extensions", "chrome://flags"],
        store_update_url: "https://clients2.google.com/service/update2/crx",
        store_ids: EXTENSION_IDS_CHROME,
    },
    Vendor {
        policy_key: r"SOFTWARE\Policies\Microsoft\Edge",
        incognito_policy: "InPrivateModeAvailability",
        extension_pages: &["edge://extensions", "chrome://extensions", "edge://flags"],
        store_update_url: "https://edge.microsoft.com/extensionwebstorebase/v1/crx",
        store_ids: EXTENSION_IDS_EDGE,
    },
    Vendor {
        policy_key: r"SOFTWARE\Policies\BraveSoftware\Brave",
        incognito_policy: "IncognitoModeAvailability",
        extension_pages: &["brave://extensions", "chrome://extensions", "brave://flags"],
        store_update_url: "https://clients2.google.com/service/update2/crx",
        store_ids: EXTENSION_IDS_CHROME,
    },
];

/// Registry keys where browsers look up native messaging hosts.
pub const NATIVE_HOST_KEYS: &[&str] = &[
    r"SOFTWARE\Google\Chrome\NativeMessagingHosts",
    r"SOFTWARE\Microsoft\Edge\NativeMessagingHosts",
    r"SOFTWARE\BraveSoftware\Brave-Browser\NativeMessagingHosts",
    r"SOFTWARE\Chromium\NativeMessagingHosts",
];

/// One policy change for one browser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorPolicies {
    pub policy_key: &'static str,
    /// Entries for the URLBlocklist list policy.
    pub url_blocklist: Vec<String>,
    /// Entries for the ExtensionInstallForcelist list policy.
    pub force_install: Vec<String>,
    /// DWORD policies to set. Anything we set before but isn't here gets removed.
    pub dwords: Vec<(&'static str, u32)>,
}

pub fn hosts_entries(eff: &Effective) -> Vec<&'static str> {
    let mut out = Vec::new();
    for p in PLATFORMS {
        let lvl = eff.level(p.id);
        if lvl == Level::Site || (lvl == Level::Shorts && p.all_shorts) {
            out.extend_from_slice(p.hosts);
        }
    }
    out
}

/// Rewrites the hosts file: removes our old section, appends the new one.
pub fn rewrite_hosts(original: &str, hosts: &[&str]) -> String {
    let nl = if original.contains("\r\n") { "\r\n" } else { "\n" };
    let mut kept: Vec<&str> = Vec::new();
    let mut inside = false;
    for line in original.lines() {
        let t = line.trim_end_matches('\r');
        if t.trim() == HOSTS_BEGIN {
            inside = true;
            continue;
        }
        if t.trim() == HOSTS_END {
            inside = false;
            continue;
        }
        if !inside {
            kept.push(t);
        }
    }
    while kept.last().map_or(false, |l| l.trim().is_empty()) {
        kept.pop();
    }
    let mut s = kept.join(nl);
    if !s.is_empty() {
        s.push_str(nl);
    }
    if !hosts.is_empty() {
        s.push_str(nl);
        s.push_str(HOSTS_BEGIN);
        s.push_str(nl);
        for h in hosts {
            s.push_str(&format!("0.0.0.0 {h}{nl}"));
            s.push_str(&format!(":: {h}{nl}"));
        }
        s.push_str(HOSTS_END);
        s.push_str(nl);
    }
    s
}

pub fn url_blocklist_common(eff: &Effective) -> Vec<String> {
    let mut out = Vec::new();
    for p in PLATFORMS {
        match eff.level(p.id) {
            Level::Open => {}
            Level::Site => out.extend(p.domains.iter().map(|d| d.to_string())),
            Level::Shorts if p.all_shorts => out.extend(p.domains.iter().map(|d| d.to_string())),
            Level::Shorts => out.extend(p.shorts_paths.iter().map(|d| d.to_string())),
        }
    }
    out
}

pub fn vendor_policies(eff: &Effective) -> Vec<VendorPolicies> {
    let common = url_blocklist_common(eff);
    VENDORS
        .iter()
        .map(|v| {
            let mut url_blocklist = common.clone();
            let mut dwords = Vec::new();
            if eff.locked {
                if eff.hardening.no_extension_page {
                    url_blocklist.extend(v.extension_pages.iter().map(|s| s.to_string()));
                }
                if eff.hardening.no_incognito {
                    dwords.push((v.incognito_policy, 1));
                }
                if eff.hardening.no_guest {
                    dwords.push(("BrowserGuestModeEnabled", 0));
                    dwords.push(("BrowserAddPersonEnabled", 0));
                }
            }
            let force_install = v
                .store_ids
                .iter()
                .map(|id| format!("{id};{}", v.store_update_url))
                .collect();
            VendorPolicies { policy_key: v.policy_key, url_blocklist, force_install, dwords }
        })
        .collect()
}

/// Every extension ID allowed to talk to the native host.
pub fn allowed_origins() -> Vec<String> {
    let mut ids: Vec<&str> = vec![crate::EXTENSION_ID_DEV];
    ids.extend_from_slice(EXTENSION_IDS_CHROME);
    ids.extend_from_slice(EXTENSION_IDS_EDGE);
    ids.sort();
    ids.dedup();
    ids.into_iter().map(|id| format!("chrome-extension://{id}/")).collect()
}

pub fn native_host_manifest(host_exe: &str) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "name": NATIVE_HOST_NAME,
        "description": "Unscroll bridge between the desktop app and the browser extension",
        "path": host_exe,
        "type": "stdio",
        "allowed_origins": allowed_origins(),
    }))
    .unwrap_or_default()
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Scheduled task: runs `exe --enforce` as SYSTEM at boot, at every logon and
/// every minute. Keeps the block in place while the app is closed.
pub fn task_xml(exe: &str) -> String {
    let exe = xml_escape(exe);
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Author>DarkSky Studios</Author>
    <Description>Keeps Unscroll's blocks and focus lock in place.</Description>
  </RegistrationInfo>
  <Triggers>
    <BootTrigger><Enabled>true</Enabled></BootTrigger>
    <LogonTrigger><Enabled>true</Enabled></LogonTrigger>
    <TimeTrigger>
      <Repetition><Interval>PT1M</Interval><StopAtDurationEnd>false</StopAtDurationEnd></Repetition>
      <StartBoundary>2020-01-01T00:00:00</StartBoundary>
      <Enabled>true</Enabled>
    </TimeTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>S-1-5-18</UserId>
      <RunLevel>HighestAvailable</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <IdleSettings><StopOnIdleEnd>false</StopOnIdleEnd><RestartOnIdle>false</RestartOnIdle></IdleSettings>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>true</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle>
    <WakeToRun>false</WakeToRun>
    <ExecutionTimeLimit>PT2M</ExecutionTimeLimit>
    <Priority>7</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>"{exe}"</Command>
      <Arguments>--enforce</Arguments>
    </Exec>
  </Actions>
</Task>
"#
    )
}
