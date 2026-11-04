//! Loading/saving the state with a cross-process lock, a signed file and a
//! registry backup copy. Every change goes through `transact`.

use crate::sys;
use unscroll_core::state::{RuleError, State};
use unscroll_core::{choose, data_dir, encode, state_path, LoadOutcome};

pub struct Snapshot {
    pub state: State,
    pub outcome: LoadOutcome,
}

/// Loads, advances the lock clock, runs `f`, enforces and saves.
/// The clock tick and enforcement happen even if `f` is rejected.
pub fn transact<R>(f: impl FnOnce(&mut State) -> Result<R, RuleError>) -> Result<(R, Snapshot), String> {
    let _guard = sys::StateLock::acquire();
    let file = std::fs::read_to_string(state_path()).ok();
    let backup = sys::read_backup();
    let (mut state, outcome) = choose(file.as_deref(), backup.as_deref());

    let uptime = sys::uptime_ms();
    let now = sys::system_ms();
    let net = if state.crossed_boot(uptime) || outcome != LoadOutcome::Ok { sys::network_ms() } else { None };
    state.tick(uptime, now, net);

    let result = f(&mut state);

    let eff = state.effective();
    sys::apply(&eff, &mut state.owned_policies);
    save(&state);

    match result {
        Ok(r) => Ok((r, Snapshot { state, outcome })),
        Err(e) => Err(e.code().to_string()),
    }
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
