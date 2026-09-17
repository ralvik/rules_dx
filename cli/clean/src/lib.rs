//! Explicit managed-state cleanup planning for `dx clean`.
//!
//! Contract: `docs/cli/commands/check-fix-clean.md` (`dx clean
//! [--dry-run] [--bazel]`: prune only validated unselected and unused
//! `.dx` generations and setup records; never delete
//! `.dx/setups/current`, its selected generations, tracked sources,
//! BUILD files, Bazel outputs, shell profiles, or global PATH entries;
//! refuse unmanaged or digest-spoofed paths; `--dry-run` deletes
//! nothing; no automatic pruning, age policy, or count limit) and
//! `docs/environments/managed-state.md#retention-and-recovery`.
//!
//! This crate owns the pure planning layer only: flag-shape constants,
//! setup-record validation against the [`dx_setup`] pair identity,
//! prune selection over an injected inventory, dry-run rendering (owned
//! by the `bytes` module), and
//! the `bazel clean` forwarding shape with its recovery guidance.
//! Filesystem inventory collection ([`collect_inventory`]), process-scan
//! in-use detection ([`scan_live_hexes`]), reclaimable-bytes measurement
//! ([`measure_prune_bytes`], owned by the `bytes` module), ignore-aware workspace walks
//! ([`walk_filtered`]), and the locked apply step ([`apply_plan`], owned
//! by the `apply` module) complete the surface; planning over injected
//! views keeps selection deterministic and unit-testable without a
//! workspace.
//!
//! The commit-lock route is the shared workspace lock owned by
//! `acquire_lock` (dedicated lock file, contention-only retry until the
//! deadline): clean introduces no new lock file, mechanism, or deadline.
//! [`CLEAN_LOCK_TIMEOUT`] (owned by the `apply` module) mirrors
//! `dx_env::LOCK_TIMEOUT`, pinned equal by test like
//! `dx_setup::COMMIT_LOCK_TIMEOUT`.

// Issue #238: infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

use std::path::PathBuf;

#[cfg(test)]
use dx_setup::{setup_hex, GenerationId, SetupPair};
#[cfg(test)]
use std::fs;
#[cfg(test)]
use std::path::Path;

pub mod apply;
pub mod bytes;
pub mod flags;
pub mod inventory;
pub mod live;
pub mod planning;
pub mod records;

pub use apply::{apply_plan, apply_plan_with_timeout, CleanOutcome, CLEAN_LOCK_TIMEOUT};

pub use bytes::{measure_prune_bytes, render_dry_run, PruneBytes};
pub use flags::{bazel_forward_argv, BAZEL_FLAG, DRY_RUN_FLAG, RECOVERY_GUIDANCE};
pub use inventory::{collect_inventory, walk_filtered, CollectedInventory};
pub use live::{collect_inventory_with_scan, scan_live_hexes, LiveHexes};
pub use planning::{plan_prune, CleanPlan, GenerationView, PruneInputs};
pub use records::{validate_record, GenerationKind, RecordProblem, SetupRecordView};

/// Clean failure. Every variant is operational; `--dry-run` rendering
/// never surfaces these (it deletes nothing and holds no lock).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CleanError {
    /// Workspace root is missing or not a directory.
    #[error("invalid workspace root {path:?}: missing or not a directory", path = path.display())]
    WorkspaceRoot { path: PathBuf },
    /// Another command holds the commit lock past the deadline.
    #[error("workspace busy at {path:?}: another command holds the commit lock", path = path.display())]
    Busy { path: PathBuf },
    /// The commit lock cannot be opened or locked.
    #[error("cannot lock {path:?}: {reason}", path = path.display())]
    LockFailed { path: PathBuf, reason: String },
    /// `.dx/setups/current` is present but malformed. Never adopted,
    /// never repaired, and nothing is pruned: the operator removes the
    /// offending path or re-runs setup from a clean selection.
    #[error("invalid current selection: {reason}")]
    CurrentInvalid { reason: String },
    /// A workspace mutation failed. Entries already deleted stay deleted
    /// (each removal is independent); the current pointer is never
    /// touched by clean, so selection is unchanged.
    #[error("install failed: {reason}")]
    Install { reason: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(tag: char) -> String {
        tag.to_string().repeat(64)
    }

    fn pair_env_gen(env: char, gen: char) -> (String, String, String) {
        let environment = GenerationId::new(&digest(env)).expect("env digest");
        let generated = GenerationId::new(&digest(gen)).expect("gen digest");
        let pair = SetupPair {
            environment,
            generated,
        };
        (setup_hex(&pair), digest(env), digest(gen))
    }

    #[cfg(windows)]
    fn symlink_dir(target: &Path, link: &Path) {
        std::os::windows::fs::symlink_dir(target, link).expect("stage test link");
    }

    #[cfg(not(windows))]
    fn symlink_dir(target: &Path, link: &Path) {
        std::os::unix::fs::symlink(target, link).expect("stage test link");
    }

    fn setup_pair(env: char, gen: char) -> SetupPair {
        SetupPair {
            environment: GenerationId::new(&digest(env)).expect("env digest"),
            generated: GenerationId::new(&digest(gen)).expect("gen digest"),
        }
    }

    fn clean_root(name: &str) -> dx_test_scratch::TempDir {
        let scratch = dx_test_scratch::scratch(&format!("dx-clean-test-{name}-"));
        fs::create_dir_all(scratch.path().join("ws")).expect("create workspace");
        scratch
    }

    fn workspace_of(root: &Path) -> PathBuf {
        root.join("ws")
    }

    /// Commits two setup pairs (stale `('3','4')`, then current
    /// `('1','2')`) and materializes all four generation directories.
    /// Returns the workspace path plus the (stale, current) setup hexes.
    fn two_record_workspace(root: &Path) -> (PathBuf, String, String) {
        let workspace = workspace_of(root);
        let stale = setup_pair('3', '4');
        let current = setup_pair('1', '2');
        dx_setup::commit_pair(&workspace, &stale).expect("commit stale");
        dx_setup::commit_pair(&workspace, &current).expect("commit current");
        let dx_dir = workspace.join(".dx");
        for (kind, tag) in [
            (GenerationKind::Environment, '1'),
            (GenerationKind::Generated, '2'),
            (GenerationKind::Environment, '3'),
            (GenerationKind::Generated, '4'),
        ] {
            fs::create_dir_all(dx_dir.join(kind.dir_name()).join(digest(tag)))
                .expect("create generation dir");
        }
        let stale_hex = setup_hex(&stale);
        let current_hex = setup_hex(&current);
        (workspace, stale_hex, current_hex)
    }

    #[test]
    fn inventory_validates_records_and_flags_unmanaged() {
        let scratch = clean_root("inventory");
        let root = scratch.path().to_path_buf();
        let (workspace, stale_hex, current_hex) = two_record_workspace(&root);
        let dx_dir = workspace.join(".dx");
        // Unmanaged names under each managed root are refused, never
        // inventoried as generations.
        fs::create_dir_all(dx_dir.join("environments").join("latest")).expect("unmanaged gen");
        fs::create_dir_all(dx_dir.join("generated").join("not-hex")).expect("unmanaged gen");
        fs::create_dir_all(dx_dir.join("setups").join("scratch")).expect("unmanaged record");
        // A digest-spoofed record (valid name shape, wrong pair links) is
        // refused like unmanaged state, never validated.
        let (spoofed_hex, _, _) = pair_env_gen('5', '6');
        let spoofed = dx_dir.join("setups").join(&spoofed_hex);
        fs::create_dir_all(&spoofed).expect("spoof record");
        symlink_dir(
            Path::new(&format!("../../environments/{}/", digest('1'))),
            &spoofed.join("environment"),
        );
        symlink_dir(
            Path::new(&format!("../../generated/{}/", digest('2'))),
            &spoofed.join("generated"),
        );

        let inventory = collect_inventory(&workspace, &[], &[]).expect("collect");
        assert_eq!(inventory.current_hex, Some(current_hex.clone()));
        let hexes: Vec<&str> = inventory
            .records
            .iter()
            .map(|record| record.hex.as_str())
            .collect();
        assert!(hexes.contains(&stale_hex.as_str()));
        assert!(hexes.contains(&current_hex.as_str()));
        assert!(!hexes.contains(&spoofed_hex.as_str()));
        assert_eq!(inventory.generations.len(), 4);
        for refused in ["latest", "not-hex", "scratch", spoofed_hex.as_str()] {
            assert!(
                inventory.unmanaged_names.contains(&refused.to_owned()),
                "refused names must contain {refused:?}: {:?}",
                inventory.unmanaged_names
            );
        }
        // The refused entries never become prune candidates.
        let plan = inventory.plan();
        assert!(!plan.prune_setup_records.contains(&spoofed_hex));
        assert!(!plan
            .prune_generations
            .iter()
            .any(|generation| generation.hex == "latest" || generation.hex == "not-hex"));
        let _ = fs::remove_dir_all(&root);
    }
}
