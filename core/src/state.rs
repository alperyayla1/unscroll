//! Persistent state, the rules for changing it, and the lock clock.

use crate::platforms::{self, PLATFORMS};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MIN_LOCK_MS: u64 = 60_000; // 1 minute
pub const MAX_LOCK_MS: u64 = 365 * 24 * 3_600_000; // 1 year

/// How strictly a platform is blocked. Ordered: Open < Shorts < Site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Open,
    Shorts,
    Site,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hardening {
    /// Disable incognito/private windows while locked.
    pub no_incognito: bool,
    /// Disable guest mode and adding browser profiles while locked.
    pub no_guest: bool,
    /// Block chrome://extensions & co. while locked, so the extension can't be turned off.
    pub no_extension_page: bool,
}

impl Default for Hardening {
    fn default() -> Self {
        Self { no_incognito: true, no_guest: true, no_extension_page: true }
    }
}

impl Hardening {
    fn or(self, o: Hardening) -> Hardening {
        Hardening {
            no_incognito: self.no_incognito || o.no_incognito,
            no_guest: self.no_guest || o.no_guest,
            no_extension_page: self.no_extension_page || o.no_extension_page,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub levels: BTreeMap<String, Level>,
    #[serde(default)]
    pub hardening: Hardening,
}

impl Default for Settings {
    fn default() -> Self {
        let levels = PLATFORMS
            .iter()
            .map(|p| {
                let lvl = if p.all_shorts {
                    Level::Site
                } else if p.supports_shorts() {
                    Level::Shorts
                } else {
                    Level::Open
                };
                (p.id.to_string(), lvl)
            })
            .collect();
        Self { levels, hardening: Hardening::default() }
    }
}

impl Settings {
    pub fn level(&self, id: &str) -> Level {
        self.levels.get(id).copied().unwrap_or(Level::Open)
    }

    /// Stricter of the two, platform by platform.
    fn max(&self, o: &Settings) -> Settings {
        let levels = PLATFORMS
            .iter()
            .map(|p| (p.id.to_string(), self.level(p.id).max(o.level(p.id))))
            .collect();
        Settings { levels, hardening: self.hardening.or(o.hardening) }
    }

    /// Makes every level valid for its platform.
    fn normalize(&mut self) {
        for p in PLATFORMS {
            let l = self.level(p.id);
            let fixed = match l {
                Level::Shorts if p.all_shorts => Level::Site,
                Level::Shorts if !p.supports_shorts() => Level::Open,
                other => other,
            };
            self.levels.insert(p.id.to_string(), fixed);
        }
        self.levels.retain(|k, _| platforms::find(k).is_some());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lock {
    pub remaining_ms: u64,
    pub total_ms: u64,
    /// The settings at lock time (merged with any extensions). Nothing may go below this.
    pub floor: Settings,
}

/// Bookkeeping for the lock clock. Within one boot we count with the system
/// uptime counter, which ignores clock changes. Across reboots we use network
/// time when we can get it, and never let time run backwards.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Clock {
    pub last_uptime_ms: u64,
    pub last_trusted_ms: u64,
    /// network_time - system_time, from the last successful check.
    pub net_offset_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub v: u32,
    /// "Block Shorts" switch: Shorts levels and all-shorts sites (TikTok). Needs the extension.
    #[serde(default = "yes")]
    pub shorts_on: bool,
    /// "Block sites" switch: Site levels. Works without the extension.
    #[serde(default = "yes")]
    pub sites_on: bool,
    pub settings: Settings,
    pub lock: Option<Lock>,
    #[serde(default)]
    pub clock: Clock,
    /// Browser policy names (DWORD) this app set, so it can remove only its own.
    #[serde(default)]
    pub owned_policies: Vec<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            v: 2,
            shorts_on: true,
            sites_on: true,
            settings: Settings::default(),
            lock: None,
            clock: Clock::default(),
            owned_policies: Vec::new(),
        }
    }
}

fn yes() -> bool {
    true
}

/// Which switch a platform's current level belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    None,
    Shorts,
    Sites,
}

pub fn category(id: &str, level: Level) -> Category {
    let all_shorts = platforms::find(id).map_or(false, |p| p.all_shorts);
    match level {
        Level::Open => Category::None,
        Level::Shorts => Category::Shorts,
        Level::Site if all_shorts => Category::Shorts,
        Level::Site => Category::Sites,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleError {
    /// Would loosen something the lock protects.
    Locked,
    BadDuration,
    UnknownPlatform,
    Unsupported,
}

impl RuleError {
    pub fn code(&self) -> &'static str {
        match self {
            RuleError::Locked => "locked",
            RuleError::BadDuration => "bad_duration",
            RuleError::UnknownPlatform => "unknown_platform",
            RuleError::Unsupported => "unsupported",
        }
    }
}

/// What actually gets enforced right now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Effective {
    pub active: bool,
    pub locked: bool,
    pub lock_remaining_ms: u64,
    pub levels: BTreeMap<String, Level>,
    /// Only meaningful while locked.
    pub hardening: Hardening,
}

impl Effective {
    pub fn level(&self, id: &str) -> Level {
        if !self.active {
            return Level::Open;
        }
        self.levels.get(id).copied().unwrap_or(Level::Open)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardKey {
    Incognito,
    Guest,
    ExtensionPage,
}

impl HardKey {
    pub fn parse(s: &str) -> Option<HardKey> {
        match s {
            "no_incognito" => Some(HardKey::Incognito),
            "no_guest" => Some(HardKey::Guest),
            "no_extension_page" => Some(HardKey::ExtensionPage),
            _ => None,
        }
    }
    fn get(self, h: &Hardening) -> bool {
        match self {
            HardKey::Incognito => h.no_incognito,
            HardKey::Guest => h.no_guest,
            HardKey::ExtensionPage => h.no_extension_page,
        }
    }
    fn set(self, h: &mut Hardening, v: bool) {
        match self {
            HardKey::Incognito => h.no_incognito = v,
            HardKey::Guest => h.no_guest = v,
            HardKey::ExtensionPage => h.no_extension_page = v,
        }
    }
}

impl State {
    pub fn normalize(&mut self) {
        self.settings.normalize();
        if let Some(l) = &mut self.lock {
            l.floor.normalize();
            if l.remaining_ms == 0 {
                self.lock = None;
            }
        }
    }

    pub fn is_locked(&self) -> bool {
        self.lock.as_ref().map_or(false, |l| l.remaining_ms > 0)
    }

    pub fn effective(&self) -> Effective {
        match &self.lock {
            Some(l) if l.remaining_ms > 0 => {
                let s = self.settings.max(&l.floor);
                Effective {
                    active: true,
                    locked: true,
                    lock_remaining_ms: l.remaining_ms,
                    levels: s.levels,
                    hardening: s.hardening,
                }
            }
            _ => {
                let levels: BTreeMap<String, Level> = self
                    .settings
                    .levels
                    .iter()
                    .map(|(id, &lvl)| {
                        let on = match category(id, lvl) {
                            Category::None => false,
                            Category::Shorts => self.shorts_on,
                            Category::Sites => self.sites_on,
                        };
                        (id.clone(), if on { lvl } else { Level::Open })
                    })
                    .collect();
                Effective {
                    active: levels.values().any(|l| *l != Level::Open),
                    locked: false,
                    lock_remaining_ms: 0,
                    levels,
                    hardening: self.settings.hardening,
                }
            }
        }
    }

    /// Floor level for a platform (Open when not locked).
    pub fn floor_level(&self, id: &str) -> Level {
        match &self.lock {
            Some(l) if self.is_locked() => l.floor.level(id),
            _ => Level::Open,
        }
    }

    pub fn floor_hard(&self) -> Hardening {
        match &self.lock {
            Some(l) if self.is_locked() => l.floor.hardening,
            _ => Hardening { no_incognito: false, no_guest: false, no_extension_page: false },
        }
    }

    pub fn set_shorts(&mut self, on: bool) -> Result<(), RuleError> {
        if !on && self.is_locked() {
            return Err(RuleError::Locked);
        }
        self.shorts_on = on;
        Ok(())
    }

    pub fn set_sites(&mut self, on: bool) -> Result<(), RuleError> {
        if !on && self.is_locked() {
            return Err(RuleError::Locked);
        }
        self.sites_on = on;
        Ok(())
    }

    pub fn set_level(&mut self, id: &str, level: Level) -> Result<(), RuleError> {
        let p = platforms::find(id).ok_or(RuleError::UnknownPlatform)?;
        if level == Level::Shorts && !p.supports_shorts() {
            return Err(RuleError::Unsupported);
        }
        let level = if level == Level::Shorts && p.all_shorts { Level::Site } else { level };
        if level < self.floor_level(id) {
            return Err(RuleError::Locked);
        }
        self.settings.levels.insert(id.to_string(), level);
        Ok(())
    }

    pub fn set_hard(&mut self, key: HardKey, on: bool) -> Result<(), RuleError> {
        if !on && key.get(&self.floor_hard()) {
            return Err(RuleError::Locked);
        }
        key.set(&mut self.settings.hardening, on);
        Ok(())
    }

    /// Starts a lock, or extends the running one. A lock can only grow.
    pub fn start_lock(&mut self, duration_ms: u64) -> Result<(), RuleError> {
        if !(MIN_LOCK_MS..=MAX_LOCK_MS).contains(&duration_ms) {
            return Err(RuleError::BadDuration);
        }
        // Locking always means blocking: if every platform is Open, use defaults.
        let mut snapshot = self.settings.clone();
        if snapshot.levels.values().all(|l| *l == Level::Open) {
            snapshot = snapshot.max(&Settings::default());
            self.settings = snapshot.clone();
        }
        match &mut self.lock {
            Some(l) if l.remaining_ms > 0 => {
                l.remaining_ms = (l.remaining_ms + duration_ms).min(MAX_LOCK_MS);
                l.total_ms = l.total_ms.saturating_add(duration_ms);
                l.floor = l.floor.max(&snapshot);
            }
            _ => {
                self.lock = Some(Lock { remaining_ms: duration_ms, total_ms: duration_ms, floor: snapshot });
            }
        }
        Ok(())
    }

    /// Advances the lock clock.
    ///
    /// * `uptime_ms` – milliseconds since boot (monotonic, includes sleep).
    /// * `system_ms` – wall clock (Unix ms), can be changed by the user.
    /// * `network_ms` – trusted time if it was fetched for this tick.
    ///
    /// Returns true when the lock ended during this tick.
    pub fn tick(&mut self, uptime_ms: u64, system_ms: u64, network_ms: Option<u64>) -> bool {
        if let Some(n) = network_ms {
            self.clock.net_offset_ms = Some(n as i64 - system_ms as i64);
        }
        let trusted_now = match (network_ms, self.clock.net_offset_ms) {
            (Some(n), _) => n,
            (None, Some(off)) => (system_ms as i64 + off).max(0) as u64,
            (None, None) => system_ms,
        };

        let first = self.clock.last_trusted_ms == 0;
        let same_boot = !first && uptime_ms >= self.clock.last_uptime_ms;
        let elapsed = if first {
            0
        } else if same_boot {
            // The uptime counter can't be faked by changing the clock.
            uptime_ms - self.clock.last_uptime_ms
        } else {
            // Rebooted: best available time, never backwards.
            trusted_now.saturating_sub(self.clock.last_trusted_ms)
        };
        self.clock.last_trusted_ms = if first {
            trusted_now
        } else if same_boot {
            self.clock.last_trusted_ms.saturating_add(elapsed)
        } else {
            trusted_now.max(self.clock.last_trusted_ms)
        };
        self.clock.last_uptime_ms = uptime_ms;

        if let Some(l) = &mut self.lock {
            if l.remaining_ms > 0 {
                l.remaining_ms = l.remaining_ms.saturating_sub(elapsed);
                if l.remaining_ms == 0 {
                    self.lock = None;
                    return true;
                }
            } else {
                self.lock = None;
            }
        }
        false
    }

    /// True if the last tick was in an earlier boot (so a network check is worth it).
    pub fn crossed_boot(&self, uptime_ms: u64) -> bool {
        self.clock.last_trusted_ms != 0 && uptime_ms < self.clock.last_uptime_ms
    }
}
