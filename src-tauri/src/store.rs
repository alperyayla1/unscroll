//! Loading/saving the state with a cross-process lock, a signed file and a
//! registry backup copy. Every change goes through `transact`.
//!
//! The window never waits for enforcement: UI changes are saved right away and
//! the hosts file / browser policies are updated on a background thread.

use crate::sys;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use unshorts_core::state::{RuleError, State};
use unshorts_core::{choose, data_dir, encode, state_path, LoadOutcome};

pub struct Snapshot {
    pub state: State,
    pub outcome: LoadOutcome,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// From the window: fast. No network, enforcement happens in the background.
    Ui,
    /// Background apply: enforce inline, no network.
    Apply,
    /// Scheduled task / installer: may check network time, enforce inline.
    Enforce,
}

/// Loads, advances the lock clock, runs `f`, saves (and enforces, depending on `mode`).
/// The clock tick is saved even if `f` is rejected.
pub fn transact<R>(mode: Mode, f: impl FnOnce(&mut State) -> Result<R, RuleError>) -> Result<(R, Snapshot), String> {
    let started = Instant::now();
    let result;
    let snapshot;
    {
        let _guard = sys::StateLock::acquire();
        let file = std::fs::read_to_string(state_path()).ok();
        let backup = sys::read_backup();
        let (mut state, outcome) = choose(file.as_deref(), backup.as_deref());

        let uptime = sys::uptime_ms();
        let now = sys::system_ms();
        let want_net = mode == Mode::Enforce && (state.crossed_boot(uptime) || outcome != LoadOutcome::Ok);
        let net = if want_net { sys::network_ms() } else { None };
        state.tick(uptime, now, net);

        result = f(&mut state);

        if mode != Mode::Ui {
            let eff = state.effective();
            sys::apply(&eff, &mut state.owned_policies);
        }
        save(&state);
        snapshot = Snapshot { state, outcome };
    }
    if mode == Mode::Ui {
        schedule_apply();
    }
    sys::log(&format!("transact {} {} ms", mode_name(mode), started.elapsed().as_millis()));

    match result {
        Ok(r) => Ok((r, snapshot)),
        Err(e) => Err(e.code().to_string()),
    }
}

fn mode_name(m: Mode) -> &'static str {
    match m {
        Mode::Ui => "ui",
        Mode::Apply => "apply",
        Mode::Enforce => "enforce",
    }
}

static APPLY_RUNNING: AtomicBool = AtomicBool::new(false);
static APPLY_DIRTY: AtomicBool = AtomicBool::new(false);

/// Runs enforcement on a worker thread. Rapid clicks collapse into one run.
pub fn schedule_apply() {
    APPLY_DIRTY.store(true, Ordering::SeqCst);
    if APPLY_RUNNING.swap(true, Ordering::SeqCst) {
        return; // the running worker will pick it up
    }
    std::thread::spawn(|| {
        while APPLY_DIRTY.swap(false, Ordering::SeqCst) {
            let _ = transact(Mode::Apply, |_| Ok(()));
        }
        APPLY_RUNNING.store(false, Ordering::SeqCst);
        if APPLY_DIRTY.load(Ordering::SeqCst) {
            schedule_apply();
        }
    });
}

fn save(state: &State) {
    let text = encode(state);
    let dir = data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let tmp = dir.join("state.json.tmp");
    if std::fs::write(&tmp, &text).is_ok() {
        let _ = std::fs::rename(&tmp, state_path());
    }
    sys::write_backup(&text);
}
