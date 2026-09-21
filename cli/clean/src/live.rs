//! Live process scan for `dx clean`.
//!
//! Split from `super` (`lib.rs`): owns [`LiveHexes`] (setup-record and
//! generation hexes observed live), [`scan_live_hexes`] (the `/proc`
//! sweep over working directories plus open file descriptors), and
//! [`collect_inventory_with_scan`] (inventory augmented with the live
//! `/proc` pin set). Re-exported through `super` so the public paths
//! stay `dx_clean::{LiveHexes, scan_live_hexes,
//! collect_inventory_with_scan}`. Distinct from the `flags` module
//! (frozen flag shapes), the `records` module (setup-record
//! validation), the `planning` module (pure prune selection), the
//! `inventory` module (filesystem collection), and the apply/bytes
//! modules.

use std::fs;
use std::path::{Path, PathBuf};

use dx_setup::{GenerationId, ENVIRONMENTS_DIR_NAME, GENERATED_DIR_NAME, SETUPS_DIR_NAME};

use super::inventory::{collect_inventory, CollectedInventory};
use super::planning::GenerationView;
use super::records::GenerationKind;
use super::CleanError;

/// Setup-record and generation hexes observed live by the process scan
/// ([`scan_live_hexes`]): never pruned while in use. Deterministic:
/// outputs sort ascending.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LiveHexes {
    /// Live setup-record hexes (paths under `.dx/setups/`).
    pub setup: Vec<String>,
    /// Live generations (paths under `.dx/environments/` or
    /// `.dx/generated/`).
    pub generations: Vec<GenerationView>,
}

/// Classifies one observed absolute path: strips the `dx_dir` prefix and
/// returns the addressed setup hex or generation when the first two
/// components are a managed root plus a digest-shaped name. Anything
/// else (foreign paths, unmanaged names, the `current` pointer itself)
/// contributes nothing.
fn classify_managed_path(
    dx_dir: &Path,
    observed: &Path,
) -> (Option<String>, Option<GenerationView>) {
    let Ok(relative) = observed.strip_prefix(dx_dir) else {
        return (None, None);
    };
    let mut components = relative.components();
    let (Some(root), Some(name)) = (components.next(), components.next()) else {
        return (None, None);
    };
    let (Some(root), Some(name)) = (root.as_os_str().to_str(), name.as_os_str().to_str()) else {
        return (None, None);
    };
    if GenerationId::new(name).is_err() {
        return (None, None);
    }
    if root == SETUPS_DIR_NAME {
        (Some(name.to_owned()), None)
    } else if root == ENVIRONMENTS_DIR_NAME {
        (
            None,
            Some(GenerationView {
                kind: GenerationKind::Environment,
                hex: name.to_owned(),
            }),
        )
    } else if root == GENERATED_DIR_NAME {
        (
            None,
            Some(GenerationView {
                kind: GenerationKind::Generated,
                hex: name.to_owned(),
            }),
        )
    } else {
        (None, None)
    }
}

/// Observes one process directory: its current working directory plus
/// every open file-descriptor target. Unreadable entries (exited
/// process, foreign owner, dangling link) contribute nothing; the scan
/// fails open per process, never aborting the whole sweep.
fn observe_process(dir: &Path, dx_dir: &Path, live: &mut LiveHexes) {
    let mut targets: Vec<PathBuf> = Vec::new();
    if let Ok(cwd) = fs::read_link(dir.join("cwd")) {
        targets.push(cwd);
    }
    if let Ok(fds) = fs::read_dir(dir.join("fd")) {
        for fd in fds.flatten() {
            if let Ok(target) = fs::read_link(fd.path()) {
                targets.push(target);
            }
        }
    }
    for target in &targets {
        let text = target.to_string_lossy();
        let trimmed: &str = text.trim_end_matches(" (deleted)");
        let (setup, generation) = classify_managed_path(dx_dir, Path::new(trimmed));
        if let Some(hex) = setup {
            live.setup.push(hex);
        }
        if let Some(generation) = generation {
            live.generations.push(generation);
        }
    }
}

/// Scans `proc_root` (the live `/proc` on Linux) for processes whose
/// working directory or open files sit under `dx_dir`, and reports the
/// addressed setup and generation hexes as live (in-use).
///
/// Only numeric process directories are inspected; anything else under
/// `proc_root` is ignored, so a missing `/proc` (non-Linux hosts)
/// scans empty rather than failing (fail open to over-retention, never
/// abort the sweep). Over-retention is the only failure
/// direction: a hex observed anywhere under the managed roots is
/// preserved, whether or not it is still referenced. Deterministic:
/// outputs sort ascending with duplicates removed.
///
/// Keep the manual `/proc` sweep over `procfs` (spike):
/// `all_processes_with_root`/`new_with_root` keeps the fake-`proc_root`
/// fixtures testable, but `cwd` still appends `" (deleted)"` and `fd()`
/// still fails per process, so both hand-owned edges stay; PID-reuse
/// safety costs a held fd per `Process` (exhaustion risk the sweep
/// avoids by holding none); default features pull `chrono`/`flate2`
/// plus `procfs-core`/`rustix` for PID/cwd/fd reads only; and the
/// Linux-only crate needs cfg-gated deps plus `select()` to keep the
/// portable build running everywhere — new dep, lockfile churn, and
/// supply-chain review for zero prune-decision change (over-retention
/// is already the safe direction; typed diagnostics stay unsurfaced).
pub fn scan_live_hexes(proc_root: &Path, dx_dir: &Path) -> LiveHexes {
    let mut live = LiveHexes::default();
    let entries = match fs::read_dir(proc_root) {
        Ok(entries) => entries,
        Err(_) => return live,
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let pid = name.to_string_lossy();
        if pid.bytes().all(|byte| byte.is_ascii_digit()) {
            observe_process(&entry.path(), dx_dir, &mut live);
        }
    }
    live.setup.sort();
    live.setup.dedup();
    live.generations.sort_by(|left, right| {
        left.kind
            .dir_name()
            .cmp(right.kind.dir_name())
            .then_with(|| left.hex.cmp(&right.hex))
    });
    live.generations.dedup();
    live
}

/// Collects the clean inventory for `workspace_root`, augmenting
/// caller-visible state with the [`scan_live_hexes`] process scan over
/// the live `/proc`: shells, editors, or build actions whose working
/// directory or open files sit under the workspace `.dx` roots pin
/// their setup and generation hexes as active (never pruned).
pub fn collect_inventory_with_scan(
    workspace_root: &Path,
) -> Result<CollectedInventory, CleanError> {
    let dx_dir = workspace_root.join(".dx");
    let live = scan_live_hexes(Path::new("/proc"), &dx_dir);
    let setup: Vec<String> = live.setup;
    let generations: Vec<String> = live
        .generations
        .iter()
        .map(|view| view.hex.clone())
        .collect();
    collect_inventory(workspace_root, &setup, &generations)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(tag: char) -> String {
        tag.to_string().repeat(64)
    }

    fn pair_env_gen(env: char, gen: char) -> (String, String, String) {
        let environment = dx_setup::GenerationId::new(&digest(env)).expect("env digest");
        let generated = dx_setup::GenerationId::new(&digest(gen)).expect("gen digest");
        let pair = dx_setup::SetupPair {
            environment,
            generated,
        };
        (dx_setup::setup_hex(&pair), digest(env), digest(gen))
    }

    fn setup_pair(env: char, gen: char) -> dx_setup::SetupPair {
        dx_setup::SetupPair {
            environment: dx_setup::GenerationId::new(&digest(env)).expect("env digest"),
            generated: dx_setup::GenerationId::new(&digest(gen)).expect("gen digest"),
        }
    }

    fn workspace_of(root: &Path) -> PathBuf {
        root.join("ws")
    }

    /// Commits two setup pairs (stale `('3','4')`, then current
    /// `('1','2')`) and materializes all four generation directories.
    /// Returns the workspace path plus the (stale, current) setup hexes.
    fn two_record_workspace(root: &Path) -> (PathBuf, String, String) {
        use crate::records::GenerationKind as Kind;
        let workspace = workspace_of(root);
        let stale = setup_pair('3', '4');
        let current = setup_pair('1', '2');
        dx_setup::commit_pair(&workspace, &stale).expect("commit stale");
        dx_setup::commit_pair(&workspace, &current).expect("commit current");
        let dx_dir = workspace.join(".dx");
        for (kind, tag) in [
            (Kind::Environment, '1'),
            (Kind::Generated, '2'),
            (Kind::Environment, '3'),
            (Kind::Generated, '4'),
        ] {
            fs::create_dir_all(dx_dir.join(kind.dir_name()).join(digest(tag)))
                .expect("create generation dir");
        }
        let stale_hex = dx_setup::setup_hex(&stale);
        let current_hex = dx_setup::setup_hex(&current);
        (workspace, stale_hex, current_hex)
    }

    /// Portable symlink planter for the fake `/proc` tree
    /// portable route): the scan itself is portable (missing `/proc`
    /// fails open to empty), so its fixtures must run everywhere instead
    /// of unix-gating. Windows planting fails fast with the OS privilege
    /// error rather than silently skipping cover.
    #[cfg(windows)]
    fn stage_symlink(target: &Path, link: &Path) {
        // Fake proc entries address files/dirs that may not exist (e.g.
        // the ` (deleted)` suffix case builds the link from display
        // text); `symlink_file` creates the link without touching the
        // target, matching the unix behavior below.
        std::os::windows::fs::symlink_file(target, link).expect("stage test link");
    }

    /// Portable symlink planter for the fake `/proc` tree
    /// portable route): see the windows variant above.
    #[cfg(not(windows))]
    fn stage_symlink(target: &Path, link: &Path) {
        std::os::unix::fs::symlink(target, link).expect("stage test link");
    }

    fn stage_process(proc_root: &Path, pid: &str, cwd: Option<&Path>, fds: &[&Path]) {
        let dir = proc_root.join(pid);
        fs::create_dir_all(dir.join("fd")).expect("stage fd dir");
        if let Some(target) = cwd {
            stage_symlink(target, &dir.join("cwd"));
        }
        for (index, target) in fds.iter().enumerate() {
            stage_symlink(target, &dir.join("fd").join(index.to_string()));
        }
    }

    #[test]
    fn scan_missing_proc_root_scans_empty() {
        let scratch = {
            let __scratch = dx_test_scratch::scratch("dx-clean-test-scan-missing-");
            std::fs::create_dir_all(__scratch.path().join("ws")).expect("create workspace");
            __scratch
        };
        let root = scratch.path().to_path_buf();
        let dx_dir = workspace_of(&root).join(".dx");
        assert_eq!(
            scan_live_hexes(&root.join("no-such-proc"), &dx_dir),
            LiveHexes::default()
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn scan_ignores_non_numeric_entries() {
        let scratch = {
            let __scratch = dx_test_scratch::scratch("dx-clean-test-scan-nonnumeric-");
            std::fs::create_dir_all(__scratch.path().join("ws")).expect("create workspace");
            __scratch
        };
        let root = scratch.path().to_path_buf();
        let workspace = workspace_of(&root);
        let dx_dir = workspace.join(".dx");
        let proc_root = root.join("proc");
        fs::create_dir_all(&proc_root).expect("proc root");
        let stale = digest('3');
        stage_process(
            &proc_root,
            "self",
            Some(&dx_dir.join("setups").join(&stale)),
            &[],
        );
        assert_eq!(scan_live_hexes(&proc_root, &dx_dir), LiveHexes::default());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn scan_reports_cwd_and_fd_targets_under_managed_roots() {
        let scratch = {
            let __scratch = dx_test_scratch::scratch("dx-clean-test-scan-live-");
            std::fs::create_dir_all(__scratch.path().join("ws")).expect("create workspace");
            __scratch
        };
        let root = scratch.path().to_path_buf();
        let workspace = workspace_of(&root);
        let dx_dir = workspace.join(".dx");
        let proc_root = root.join("proc");
        fs::create_dir_all(&proc_root).expect("proc root");
        let (setup_hex_value, _, _) = pair_env_gen('3', '4');
        let env_hex = digest('1');
        let gen_hex = digest('2');
        let foreign = root.join("elsewhere");
        stage_process(
            &proc_root,
            "4242",
            Some(&dx_dir.join("setups").join(&setup_hex_value)),
            &[
                &dx_dir.join("environments").join(&env_hex),
                &dx_dir.join("generated").join(&gen_hex),
                &foreign,
            ],
        );
        let live = scan_live_hexes(&proc_root, &dx_dir);
        assert_eq!(live.setup, vec![setup_hex_value]);
        assert_eq!(
            live.generations,
            vec![
                GenerationView {
                    kind: GenerationKind::Environment,
                    hex: env_hex,
                },
                GenerationView {
                    kind: GenerationKind::Generated,
                    hex: gen_hex,
                },
            ]
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn scan_trims_deleted_suffix_and_ignores_unmanaged_paths() {
        let scratch = {
            let __scratch = dx_test_scratch::scratch("dx-clean-test-scan-edge-");
            std::fs::create_dir_all(__scratch.path().join("ws")).expect("create workspace");
            __scratch
        };
        let root = scratch.path().to_path_buf();
        let workspace = workspace_of(&root);
        let dx_dir = workspace.join(".dx");
        let proc_root = root.join("proc");
        fs::create_dir_all(&proc_root).expect("proc root");
        let env_hex = digest('5');
        // A (deleted) suffix marks an unlinked-but-open directory: still
        // an observed address, so the staged link carries the suffix the
        // kernel appends to `readlink` results.
        let deleted = dx_dir.join("environments").join(&env_hex);
        let deleted_text = format!("{} (deleted)", deleted.display());
        let dir = proc_root.join("7");
        fs::create_dir_all(dir.join("fd")).expect("fd dir");
        stage_symlink(Path::new(&deleted_text), &dir.join("cwd"));
        // Unmanaged names, non-digest names, and foreign roots
        // contribute nothing even when observed.
        stage_symlink(
            &dx_dir.join("setups").join("latest"),
            &dir.join("fd").join("0"),
        );
        stage_symlink(
            &dx_dir.join("notes").join(digest('9')),
            &dir.join("fd").join("1"),
        );
        stage_symlink(
            &root.join("elsewhere").join(digest('8')),
            &dir.join("fd").join("2"),
        );
        let live = scan_live_hexes(&proc_root, &dx_dir);
        assert!(live.setup.is_empty());
        assert_eq!(
            live.generations,
            vec![GenerationView {
                kind: GenerationKind::Environment,
                hex: env_hex,
            }]
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn scan_dedupes_and_sorts_across_processes() {
        let scratch = {
            let __scratch = dx_test_scratch::scratch("dx-clean-test-scan-dedupe-");
            std::fs::create_dir_all(__scratch.path().join("ws")).expect("create workspace");
            __scratch
        };
        let root = scratch.path().to_path_buf();
        let workspace = workspace_of(&root);
        let dx_dir = workspace.join(".dx");
        let proc_root = root.join("proc");
        fs::create_dir_all(&proc_root).expect("proc root");
        let low = digest('1');
        let high = digest('9');
        stage_process(
            &proc_root,
            "100",
            Some(&dx_dir.join("generated").join(&high)),
            &[&dx_dir.join("generated").join(&low)],
        );
        stage_process(
            &proc_root,
            "200",
            Some(&dx_dir.join("generated").join(&low)),
            &[&dx_dir.join("generated").join(&high)],
        );
        let live = scan_live_hexes(&proc_root, &dx_dir);
        assert!(live.setup.is_empty());
        assert_eq!(
            live.generations,
            vec![
                GenerationView {
                    kind: GenerationKind::Generated,
                    hex: low,
                },
                GenerationView {
                    kind: GenerationKind::Generated,
                    hex: high,
                },
            ]
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn with_scan_matches_plain_inventory_without_live_processes() {
        let scratch = {
            let __scratch = dx_test_scratch::scratch("dx-clean-test-with-scan-");
            std::fs::create_dir_all(__scratch.path().join("ws")).expect("create workspace");
            __scratch
        };
        let root = scratch.path().to_path_buf();
        let (workspace, stale_hex, _) = two_record_workspace(&root);
        let scanned = collect_inventory_with_scan(&workspace).expect("scan collect");
        let plain = collect_inventory(&workspace, &[], &[]).expect("plain collect");
        // No test-runner process holds the fixture workspace open, so the
        // live scan pins nothing and both routes agree.
        assert_eq!(scanned.plan(), plain.plan());
        assert_eq!(scanned.plan().prune_setup_records, vec![stale_hex]);
        let _ = fs::remove_dir_all(&root);
    }
}
