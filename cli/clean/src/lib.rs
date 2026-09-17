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
//! prune selection over an injected inventory, dry-run rendering, and
//! the `bazel clean` forwarding shape with its recovery guidance.
//! Filesystem inventory collection ([`collect_inventory`]), process-scan
//! in-use detection ([`scan_live_hexes`]), reclaimable-bytes measurement
//! ([`measure_prune_bytes`]), ignore-aware workspace walks
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

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use dx_setup::SETUPS_DIR_NAME;
#[cfg(test)]
use dx_setup::{setup_hex, GenerationId, SetupPair};

pub mod apply;
pub mod flags;
pub mod inventory;
pub mod live;
pub mod planning;
pub mod records;

pub use apply::{apply_plan, apply_plan_with_timeout, CleanOutcome, CLEAN_LOCK_TIMEOUT};

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

/// Measured reclaimable bytes behind one [`CleanPlan`]: per-entry
/// on-disk sizes for exactly the prune sets. Generations are link trees
/// into Bazel outputs, never artifact copies, so sizes count metadata
/// plus links only: symlinks are measured, never followed, and Bazel
/// outputs behind the links contribute nothing. Deterministic: entries
/// sort ascending like the plan.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PruneBytes {
    /// `(setup-record hex, bytes)` for each pruned record, ascending.
    pub setup_record_bytes: Vec<(String, u64)>,
    /// `(generation, bytes)` for each pruned generation, ascending.
    pub generation_bytes: Vec<(GenerationView, u64)>,
}

impl PruneBytes {
    /// Total reclaimable bytes across the whole prune set.
    pub fn total(&self) -> u64 {
        let mut total = 0u64;
        for (_, bytes) in &self.setup_record_bytes {
            total = total.saturating_add(*bytes);
        }
        for (_, bytes) in &self.generation_bytes {
            total = total.saturating_add(*bytes);
        }
        total
    }

    /// Bytes attributable to entries [`apply_plan`] actually removed:
    /// measured sizes for the outcome's entries only, so entries skipped
    /// under the lock (reselected current, relinked generations) never
    /// inflate the reported reclaimed total.
    pub fn reclaimed(&self, outcome: &CleanOutcome) -> u64 {
        let mut total = 0u64;
        for removed in &outcome.removed_setup_records {
            for (hex, bytes) in &self.setup_record_bytes {
                if hex == removed {
                    total = total.saturating_add(*bytes);
                }
            }
        }
        for removed in &outcome.removed_generations {
            for (generation, bytes) in &self.generation_bytes {
                if generation == removed {
                    total = total.saturating_add(*bytes);
                }
            }
        }
        total
    }
}

/// Sums `symlink_metadata` sizes under `dir` without following symlinks.
/// A missing directory measures zero (already pruned: idempotent); any
/// other failure reports through [`CleanError`].
///
/// Implemented over [`walkdir::WalkDir`] (issue #223): recursive
/// traversal without following symlinks, matching the historical
/// manual stack (directories contribute nothing, files contribute
/// `symlink_metadata` length, saturating).
fn dir_bytes(dir: &Path) -> Result<u64, CleanError> {
    let fail = |reason: String| CleanError::Install { reason };
    if fs::read_dir(dir).is_err_and(|e| e.kind() == io::ErrorKind::NotFound) {
        return Ok(0);
    }
    let mut total = 0u64;
    for entry in walkdir::WalkDir::new(dir).follow_links(false) {
        let entry = entry.map_err(|e| fail(format!("cannot list {}: {e}", dir.display())))?;
        let path = entry.path();
        if path == dir {
            continue;
        }
        let meta = fs::symlink_metadata(path)
            .map_err(|e| fail(format!("cannot inspect {}: {e}", path.display())))?;
        if meta.is_dir() {
            continue;
        }
        total = total.saturating_add(meta.len());
    }
    Ok(total)
}

/// Measures reclaimable bytes for `plan` under `workspace_root`.
/// Entries that vanished since planning measure zero; the apply step
/// treats missing entries as idempotent successes, so measure and
/// apply agree without holding the lock.
pub fn measure_prune_bytes(
    workspace_root: &Path,
    plan: &CleanPlan,
) -> Result<PruneBytes, CleanError> {
    if !workspace_root.is_dir() {
        return Err(CleanError::WorkspaceRoot {
            path: workspace_root.to_path_buf(),
        });
    }
    let dx_dir = workspace_root.join(".dx");
    let setups_dir = dx_dir.join(SETUPS_DIR_NAME);
    let mut measured = PruneBytes::default();
    for hex in &plan.prune_setup_records {
        let bytes = dir_bytes(&setups_dir.join(hex))?;
        measured.setup_record_bytes.push((hex.clone(), bytes));
    }
    for generation in &plan.prune_generations {
        let bytes = dir_bytes(
            &dx_dir
                .join(generation.kind.dir_name())
                .join(&generation.hex),
        )?;
        measured.generation_bytes.push((generation.clone(), bytes));
    }
    Ok(measured)
}

/// Renders the `--dry-run` listing for `plan` with measured reclaimable
/// bytes (`bytes`): reclaimable setup records and generation links with
/// per-entry sizes, the preserved current selection, refused unmanaged
/// paths, and the reclaimable total. Deletes nothing; [`apply_plan`]
/// deletes exactly the listed prune sets.
pub fn render_dry_run(plan: &CleanPlan, bytes: &PruneBytes) -> String {
    let mut lines = vec!["dx clean --dry-run: reclaimable managed state".to_owned()];
    if plan.prune_setup_records.is_empty() && plan.prune_generations.is_empty() {
        lines.push("nothing to prune".to_owned());
    }
    for hex in &plan.prune_setup_records {
        let size = bytes
            .setup_record_bytes
            .iter()
            .find(|(entry, _)| entry == hex)
            .map(|(_, size)| *size)
            .unwrap_or(0);
        lines.push(format!(
            "prune setup record: .dx/setups/{hex} ({size} bytes)"
        ));
    }
    for generation in &plan.prune_generations {
        let size = bytes
            .generation_bytes
            .iter()
            .find(|(entry, _)| entry == generation)
            .map(|(_, size)| *size)
            .unwrap_or(0);
        lines.push(format!(
            "prune generation: .dx/{}/{} ({size} bytes)",
            generation.kind.dir_name(),
            generation.hex
        ));
    }
    match &plan.preserved_current {
        Some(current) => lines.push(format!("preserve current: .dx/setups/{current}")),
        None => lines.push("no current selection".to_owned()),
    }
    for unmanaged in &plan.refused_unmanaged {
        lines.push(format!("refuse unmanaged path: {unmanaged}"));
    }
    lines.push(format!("reclaimable total: {} bytes", bytes.total()));
    lines.join("\n")
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

    fn record(env: char, gen: char) -> SetupRecordView {
        let (hex, environment_hex, generated_hex) = pair_env_gen(env, gen);
        validate_record(&hex, &environment_hex, &generated_hex).expect("valid record")
    }

    fn generation(kind: GenerationKind, tag: char) -> GenerationView {
        GenerationView {
            kind,
            hex: digest(tag),
        }
    }

    fn inputs<'a>(
        records: &'a [SetupRecordView],
        generations: &'a [GenerationView],
        current_hex: Option<&'a str>,
    ) -> PruneInputs<'a> {
        PruneInputs {
            records,
            generations,
            current_hex,
            active_setup_hexes: &[],
            active_generation_hexes: &[],
            unmanaged_names: &[],
        }
    }

    #[test]
    fn dry_run_lists_prune_preserve_and_refuse() {
        let current = record('1', '2');
        let stale = record('3', '4');
        let records = vec![current.clone(), stale.clone()];
        let generations = vec![generation(GenerationKind::Environment, '3')];
        let unmanaged = vec!["latest".to_owned()];
        let plan = plan_prune(PruneInputs {
            records: &records,
            generations: &generations,
            current_hex: Some(&current.hex),
            active_setup_hexes: &[],
            active_generation_hexes: &[],
            unmanaged_names: &unmanaged,
        });
        let listing = render_dry_run(&plan, &PruneBytes::default());
        assert!(listing.contains(&format!(".dx/setups/{}", stale.hex)));
        assert!(listing.contains(&format!(".dx/environments/{}", digest('3'))));
        assert!(listing.contains(&format!(".dx/setups/{}", current.hex)));
        assert!(listing.contains("latest"));
        // Unmeasured entries render as zero bytes, never omitted.
        assert!(listing.contains("(0 bytes)"));
        assert!(listing.contains("reclaimable total: 0 bytes"));
    }

    #[test]
    fn dry_run_reports_empty_prune_set() {
        let current = record('1', '2');
        let records = vec![current.clone()];
        let plan = plan_prune(inputs(&records, &[], Some(&current.hex)));
        let listing = render_dry_run(&plan, &PruneBytes::default());
        assert!(listing.contains("nothing to prune"));
        assert!(listing.contains("reclaimable total: 0 bytes"));
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

    /// Stages a fake process directory under `proc_root/<pid>` with a
    /// `cwd` symlink and numbered `fd` symlinks to the given targets.
    /// Unix-only: the fake-`/proc` fixtures below need symlink atoms.
    #[cfg(unix)]
    #[test]
    fn measure_sums_prune_entries_and_skips_missing() {
        let scratch = clean_root("measure");
        let root = scratch.path().to_path_buf();
        let (workspace, stale_hex, _) = two_record_workspace(&root);
        let plan = collect_inventory(&workspace, &[], &[])
            .expect("collect")
            .plan();
        let bytes = measure_prune_bytes(&workspace, &plan).expect("measure");
        // The stale record holds the environment/generated pair links
        // (measured metadata); the empty generation dirs measure zero.
        let (measured_hex, record_bytes) = bytes
            .setup_record_bytes
            .iter()
            .find(|(hex, _)| hex == &stale_hex)
            .expect("stale record measured");
        assert_eq!(measured_hex, &stale_hex);
        assert!(
            *record_bytes > 0,
            "pair links measure nonzero: {record_bytes}"
        );
        assert_eq!(bytes.generation_bytes.len(), 2);
        assert!(bytes.generation_bytes.iter().all(|(_, size)| *size == 0));
        assert_eq!(
            bytes.total(),
            *record_bytes,
            "empty generation dirs add nothing"
        );
        // Entries that vanished since planning measure zero; apply treats
        // them as idempotent successes, so measure and apply agree.
        let ghost = CleanPlan {
            prune_setup_records: vec![digest('a')],
            prune_generations: vec![GenerationView {
                kind: GenerationKind::Environment,
                hex: digest('b'),
            }],
            refused_unmanaged: Vec::new(),
            preserved_current: None,
        };
        let ghost_bytes = measure_prune_bytes(&workspace, &ghost).expect("ghost measure");
        assert_eq!(ghost_bytes.total(), 0);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn measure_never_follows_symlinks() {
        let scratch = clean_root("measure-nofollow");
        let root = scratch.path().to_path_buf();
        let workspace = workspace_of(&root);
        dx_setup::commit_pair(&workspace, &setup_pair('1', '2')).expect("commit");
        let dx_dir = workspace.join(".dx");
        // A fat file behind a link inside a pruned generation: the link
        // measures, the 1 MiB target never does.
        let outside = root.join("fat.bin");
        fs::write(&outside, vec![7u8; 1 << 20]).expect("fat file");
        let stale_gen = dx_dir.join("generated").join(digest('9'));
        fs::create_dir_all(&stale_gen).expect("stale generation");
        std::os::unix::fs::symlink(&outside, stale_gen.join("artifact")).expect("link");
        let plan = CleanPlan {
            prune_setup_records: Vec::new(),
            prune_generations: vec![GenerationView {
                kind: GenerationKind::Generated,
                hex: digest('9'),
            }],
            refused_unmanaged: Vec::new(),
            preserved_current: None,
        };
        let bytes = measure_prune_bytes(&workspace, &plan).expect("measure");
        assert!(
            bytes.total() < 1 << 20,
            "link target must not count: {}",
            bytes.total()
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn measure_missing_workspace_fails() {
        let scratch = clean_root("measure-missing");
        let root = scratch.path().to_path_buf();
        let plan = CleanPlan {
            prune_setup_records: Vec::new(),
            prune_generations: Vec::new(),
            refused_unmanaged: Vec::new(),
            preserved_current: None,
        };
        assert!(matches!(
            measure_prune_bytes(&root.join("no-such-dir"), &plan),
            Err(CleanError::WorkspaceRoot { .. })
        ));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn reclaimed_counts_only_removed_entries() {
        let removed_hex = digest('3');
        let removed_generation = GenerationView {
            kind: GenerationKind::Environment,
            hex: digest('5'),
        };
        let bytes = PruneBytes {
            setup_record_bytes: vec![(removed_hex.clone(), 100), (digest('4'), 200)],
            generation_bytes: vec![
                (removed_generation.clone(), 300),
                (
                    GenerationView {
                        kind: GenerationKind::Generated,
                        hex: digest('6'),
                    },
                    400,
                ),
            ],
        };
        assert_eq!(bytes.total(), 1000);
        let outcome = CleanOutcome {
            removed_setup_records: vec![removed_hex],
            removed_generations: vec![removed_generation],
        };
        // Entries skipped under the lock (reselected current, relinked
        // generations) never inflate the reported reclaimed total.
        assert_eq!(bytes.reclaimed(&outcome), 400);
        assert_eq!(
            bytes.reclaimed(&CleanOutcome::default()),
            0,
            "empty outcome reclaims nothing"
        );
    }

    #[test]
    fn render_lists_per_entry_bytes_and_total() {
        let stale = record('3', '4');
        let current = record('1', '2');
        let records = vec![current.clone(), stale.clone()];
        let generations = vec![generation(GenerationKind::Environment, '3')];
        let plan = plan_prune(inputs(&records, &generations, Some(&current.hex)));
        let bytes = PruneBytes {
            setup_record_bytes: vec![(stale.hex.clone(), 120)],
            generation_bytes: vec![(generation(GenerationKind::Environment, '3'), 34)],
        };
        let listing = render_dry_run(&plan, &bytes);
        assert!(
            listing.contains(&format!(
                "prune setup record: .dx/setups/{} (120 bytes)",
                stale.hex
            )),
            "{listing}"
        );
        assert!(
            listing.contains(&format!(
                "prune generation: .dx/environments/{} (34 bytes)",
                digest('3')
            )),
            "{listing}"
        );
        assert!(
            listing.contains("reclaimable total: 154 bytes"),
            "{listing}"
        );
    }
}
