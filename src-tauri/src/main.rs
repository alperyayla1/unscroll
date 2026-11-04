#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod store;

#[cfg(windows)]
#[path = "sys_windows.rs"]
mod sys;
#[cfg(not(windows))]
#[path = "sys_stub.rs"]
mod sys;

use unscroll_core::state::RuleError;

fn main() {
    let arg = std::env::args().nth(1).unwrap_or_default();
    match arg.as_str() {
        // Background enforcer, run every minute by the scheduled task.
        "--enforce" => {
            let _ = store::transact(|_| Ok(()));
        }
        // Installer: register everything and apply the current state.
        "--setup" => {
            sys::setup();
            let _ = store::transact(|_| Ok(()));
        }
        // Uninstaller: exit code 1 while a lock is running.
        "--uninstall-check" => {
            let locked = store::transact(|s| Ok::<bool, RuleError>(s.is_locked())).map(|(l, _)| l).unwrap_or(true);
            std::process::exit(if locked { 1 } else { 0 });
        }
        // Uninstaller: remove every trace. Refuses while locked.
        "--cleanup" => {
            let r = store::transact(|s| {
                if s.is_locked() {
                    return Err(RuleError::Locked);
                }
                s.set_shorts(false)?;
                s.set_sites(false)?;
                Ok(std::mem::take(&mut s.owned_policies))
            });
            match r {
                Ok((mut owned, _)) => sys::cleanup(&mut owned),
                Err(_) => std::process::exit(1),
            }
        }
        _ => run_app(),
    }
}

fn run_app() {
    // Cheap and idempotent: keeps the task and the extension bridge pointing at this exe.
    sys::setup();
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_view,
            commands::set_shorts,
            commands::set_sites,
            commands::set_level,
            commands::set_hard,
            commands::start_lock,
            commands::open_extension_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Unscroll");
}
