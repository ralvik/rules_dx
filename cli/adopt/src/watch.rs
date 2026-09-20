//! Local watch loop for wrapped commands.
//!
//! Split from `super` (`lib.rs`): owns `WATCH_DEBOUNCE_MS`,
//! `WATCHABLE_COMMANDS`, `watch_iteration_accepts`, `plan_watch`,
//! `coalesce_watch_paths`, and `watch_for_change`. Re-exported through
//! `super` so the public path stays
//! `dx_adopt::{WATCH_DEBOUNCE_MS, WATCHABLE_COMMANDS,
//! watch_iteration_accepts, plan_watch, coalesce_watch_paths,
//! watch_for_change}`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use super::AdoptError;

/// Watch debounce milliseconds (frozen).
pub const WATCH_DEBOUNCE_MS: u64 = 200;

/// Commands watchable under ADR 0017/0018 (frozen).
pub const WATCHABLE_COMMANDS: &[&str] = &[
    "build",
    "test",
    "run",
    "lint",
    "typecheck",
    "format",
    "check",
    "fix",
];

/// Whether one watch iteration may run.
///
/// Watch is a thin local loop reusing the wrapped command verbatim (no
/// daemon, cache, graph, or remote): each iteration re-resolves its scope,
/// holds the `run` selection rule for that scope (single-runnable for
/// file/directory scopes, sequential multirun for explicit labels and
/// patterns), and refuses when running under
/// CI. Any violation blocks the iteration.
pub fn watch_iteration_accepts(
    scope_reresolved: bool,
    local_only: bool,
    single_runnable_held: bool,
) -> bool {
    scope_reresolved && local_only && single_runnable_held
}

/// Validate one watch invocation (frozen).
pub fn plan_watch(command: &str, ci: bool) -> Result<String, AdoptError> {
    if ci {
        return Err(AdoptError::WatchRefusesCi);
    }
    if !WATCHABLE_COMMANDS.contains(&command) {
        return Err(AdoptError::NotWatchable {
            command: command.to_owned(),
        });
    }
    Ok(format!("watch:{command}:debounce={WATCH_DEBOUNCE_MS}ms"))
}

/// Coalesces debounced watcher paths into a single deterministic
/// rebuild trigger: rapid create/modify/delete bursts
/// for one path collapse to one entry; outputs sort ascending with
/// duplicates removed so repeated runs render identically.
pub fn coalesce_watch_paths(mut paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths.sort();
    paths.dedup();
    paths
}

/// Blocks up to `timeout` for one debounced filesystem change under
/// `watch_root` returning the coalesced trigger paths.
///
/// Implemented over [`notify`] 8.x plus `notify-debouncer-mini`
/// (200 ms debounce per [`WATCH_DEBOUNCE_MS`]): create, modify, and
/// delete events all feed the same rebuild trigger. An empty vector
/// means the timeout elapsed with no changes (the caller re-arms);
/// only watcher setup and channel failures surface as [`AdoptError`].
/// Callers must validate via [`plan_watch`] first (local-only refusal
/// stays there, not here).
pub fn watch_for_change(watch_root: &Path, timeout: Duration) -> Result<Vec<PathBuf>, AdoptError> {
    use notify::RecursiveMode;
    let (tx, rx) = std::sync::mpsc::channel();
    let mut debouncer =
        notify_debouncer_mini::new_debouncer(Duration::from_millis(WATCH_DEBOUNCE_MS), tx)
            .map_err(|e| AdoptError::WatchSpawn {
                detail: e.to_string(),
            })?;
    debouncer
        .watcher()
        .watch(watch_root, RecursiveMode::Recursive)
        .map_err(|e| AdoptError::WatchSpawn {
            detail: e.to_string(),
        })?;
    match rx.recv_timeout(timeout) {
        Ok(Ok(events)) => {
            let paths: Vec<PathBuf> = events.into_iter().map(|event| event.path).collect();
            Ok(coalesce_watch_paths(paths))
        }
        Ok(Err(e)) => Err(AdoptError::WatchFailed {
            detail: e.to_string(),
        }),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Ok(Vec::new()),
        Err(e) => Err(AdoptError::WatchFailed {
            detail: e.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watch_iterations_stay_local_reresolved_and_single() {
        assert!(watch_iteration_accepts(true, true, true));
        assert!(!watch_iteration_accepts(false, true, true));
        assert!(!watch_iteration_accepts(true, false, true));
        assert!(!watch_iteration_accepts(true, true, false));
    }

    #[test]
    fn watch_freeze_holds() {
        assert!(plan_watch("test", false).is_ok());
        assert!(plan_watch("docs", false).is_err());
        assert!(plan_watch("test", true).is_err());
    }

    #[test]
    fn watch_execution_gaps_matrix_is_wont_fix() {
        // The 8 thin-loop commands stay watchable; the
        // remaining 21 registry commands stay fail-closed not watchable
        // and CI stays refused. Pinned with fixtures in
        // `cli/cli/tests/fixtures/cli_execution_gaps/`.
        for watchable in [
            "build",
            "test",
            "run",
            "lint",
            "typecheck",
            "format",
            "check",
            "fix",
        ] {
            assert!(
                plan_watch(watchable, false).is_ok(),
                "{watchable} must stay watchable"
            );
            assert!(
                plan_watch(watchable, true).is_err(),
                "{watchable} must still refuse CI"
            );
        }
        for not_watchable in [
            "audit",
            "bazel",
            "bump",
            "clean",
            "codegen",
            "completion",
            "coverage",
            "deps",
            "deploy",
            "env",
            "generate",
            "hooks",
            "init",
            "migrate",
            "owners",
            "setup",
            "status",
            "update",
            "version",
            "watch",
            "why",
        ] {
            let err = plan_watch(not_watchable, false).expect_err("not watchable");
            assert_eq!(
                err,
                AdoptError::NotWatchable {
                    command: not_watchable.to_owned(),
                },
                "{not_watchable} must stay not watchable"
            );
        }
        assert_eq!(WATCHABLE_COMMANDS.len(), 8);
        assert_eq!(WATCH_DEBOUNCE_MS, 200);
    }

    #[test]
    fn watch_coalesces_bursts_into_a_single_trigger() {
        // Rapid create/modify/delete bursts collapse to one
        // deterministic rebuild trigger per path.
        let first = PathBuf::from("/tmp/ws/src/main.rs");
        let second = PathBuf::from("/tmp/ws/src/lib.rs");
        let trigger = coalesce_watch_paths(vec![
            first.clone(),
            first.clone(),
            second.clone(),
            first.clone(),
        ]);
        assert_eq!(trigger, vec![second, first]);
    }

    #[test]
    fn watch_reports_created_files_and_times_out_when_idle() {
        // A real `notify` watcher emits a debounced trigger
        // for a created file, and reports empty when nothing changes.
        let scratch = dx_test_scratch::scratch("dx-adopt-watch-");
        let root = scratch.path().to_path_buf();
        let writer = root.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            let _ = std::fs::write(writer.join("trigger.txt"), "change");
        });
        let trigger = watch_for_change(&root, Duration::from_secs(5)).expect("watch create");
        assert!(
            trigger.iter().any(|path| path.ends_with("trigger.txt")),
            "created file must trigger a rebuild: {trigger:?}"
        );
        let idle = watch_for_change(&root, Duration::from_millis(300)).expect("watch idle");
        assert!(idle.is_empty(), "idle watch must time out empty: {idle:?}");
    }
}
