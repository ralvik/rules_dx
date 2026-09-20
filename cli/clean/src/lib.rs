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

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

use std::path::PathBuf;

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
