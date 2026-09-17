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
//! ([`walk_filtered`]), and the locked apply step ([`apply_plan`])
//! complete the surface; planning over injected views keeps selection
//! deterministic and unit-testable without a workspace.
//!
//! The commit-lock route is the shared workspace lock owned by
//! `acquire_lock` (dedicated lock file, contention-only retry until the
//! deadline): clean introduces no new lock file, mechanism, or deadline.
//! [`CLEAN_LOCK_TIMEOUT`] mirrors `dx_env::LOCK_TIMEOUT`, pinned equal by
//! test like `dx_setup::COMMIT_LOCK_TIMEOUT`.

// Issue #238: infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use dx_env::{acquire_lock, Error};
use dx_setup::{
    read_current_pair, setup_hex, GenerationId, SetupPair, CURRENT_LINK_NAME, CURRENT_STAGE_NAME,
    ENVIRONMENTS_DIR_NAME, ENVIRONMENT_LINK_NAME, GENERATED_DIR_NAME, GENERATED_LINK_NAME,
    SETUPS_DIR_NAME,
};

pub mod flags;

pub use flags::{bazel_forward_argv, BAZEL_FLAG, DRY_RUN_FLAG, RECOVERY_GUIDANCE};

/// Which managed generation tree a prunable directory belongs to.
/// Generations are link trees into Bazel outputs, never artifact
/// copies, so pruning one only removes metadata plus links.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GenerationKind {
    Environment,
    Generated,
}

impl GenerationKind {
    /// Directory name under `.dx` holding this generation kind.
    /// Matches `ENVIRONMENTS_DIR_NAME` / `GENERATED_DIR_NAME` in
    /// `dx_setup`.
    pub fn dir_name(&self) -> &'static str {
        match self {
            GenerationKind::Environment => ENVIRONMENTS_DIR_NAME,
            GenerationKind::Generated => GENERATED_DIR_NAME,
        }
    }
}

/// One validated setup record: a hash-addressed directory under
/// `.dx/setups` whose links resolve to exactly the pair its name
/// addresses. Validation happens in [`validate_record`]; only
/// validated records reach [`plan_prune`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupRecordView {
    /// Record directory name: lowercase hex of the setup digest.
    pub hex: String,
    /// Environment generation digest the record's `environment` link
    /// addresses.
    pub environment_hex: String,
    /// Generated-code generation digest the record's `generated` link
    /// addresses.
    pub generated_hex: String,
}

/// Setup-record validation failure. Invalid records are refused, never
/// adopted or repaired: the operator removes the offending path or
/// re-runs setup from a clean selection.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RecordProblem {
    /// Record, environment, or generated name is not a 64-character
    /// lowercase hexadecimal digest.
    #[error("malformed digest {value:?}: want 64-character lowercase hex")]
    MalformedDigest { value: String },
    /// Record links resolve to a pair whose digest differs from the
    /// record directory name (digest-spoofed path).
    #[error("spoofed record {record:?}: links resolve to {pair:?}")]
    Spoofed { record: String, pair: String },
}

/// Parses one digest-shaped name, refusing malformed values instead of
/// panicking at the call site.
fn validated_generation_id(hex: &str) -> Result<GenerationId, RecordProblem> {
    GenerationId::new(hex).map_err(|_| RecordProblem::MalformedDigest {
        value: hex.to_owned(),
    })
}

/// Validates one setup record against the [`dx_setup`] pair identity:
/// every name must be digest-shaped and the record name must equal the
/// digest of the linked pair. Returns the validated view or the reason
/// the record is refused as unmanaged/spoofed (never pruned by
/// [`plan_prune`]).
pub fn validate_record(
    hex: &str,
    environment_hex: &str,
    generated_hex: &str,
) -> Result<SetupRecordView, RecordProblem> {
    // Each name validates exactly once through one helper, so a malformed
    // digest fails here instead of panicking at pair construction.
    let environment = validated_generation_id(environment_hex)?;
    let generated = validated_generation_id(generated_hex)?;
    validated_generation_id(hex)?;
    let pair = SetupPair {
        environment,
        generated,
    };
    let want = setup_hex(&pair);
    if want != hex {
        return Err(RecordProblem::Spoofed {
            record: hex.to_owned(),
            pair: want,
        });
    }
    Ok(SetupRecordView {
        hex: hex.to_owned(),
        environment_hex: environment_hex.to_owned(),
        generated_hex: generated_hex.to_owned(),
    })
}

/// One present generation directory (digest-shaped name only;
/// anything else is unmanaged and refused, never inventoried here).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationView {
    pub kind: GenerationKind,
    pub hex: String,
}

/// Inventory [`plan_prune`] selects from. Filesystem collection lands
/// in a later WP5 slice; planning over injected views keeps selection
/// pure and pinned by test.
pub struct PruneInputs<'a> {
    /// Validated setup records present under `.dx/setups` (validated
    /// by [`validate_record` upstream of this call).
    pub records: &'a [SetupRecordView],
    /// Generations present under `.dx/environments` and
    /// `.dx/generated` (digest-shaped names only).
    pub generations: &'a [GenerationView],
    /// Currently selected setup record hex (`.dx/setups/current`
    /// target), if any.
    pub current_hex: Option<&'a str>,
    /// Setup-record hexes still referenced by an active process
    /// (open shells, editors): never pruned while in use.
    pub active_setup_hexes: &'a [String],
    /// Generation hexes still referenced by an active process: never
    /// pruned while in use.
    pub active_generation_hexes: &'a [String],
    /// Directory names under the managed roots that are not
    /// digest-shaped (or otherwise unmanaged): refused, never deleted.
    pub unmanaged_names: &'a [String],
}

/// Prune selection: validated unselected and unused generations and
/// setup records only. The current record and its selected
/// generations, every active (in-use) record and generation, every
/// generation referenced by a retained record, and every unmanaged
/// path are preserved or refused, never deleted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanPlan {
    /// Validated setup-record hexes safe to remove: neither current,
    /// nor active, and (by construction) unselected.
    pub prune_setup_records: Vec<String>,
    /// Present generations safe to remove: unreferenced by every
    /// retained record and not active.
    pub prune_generations: Vec<GenerationView>,
    /// Unmanaged names refused (never deleted).
    pub refused_unmanaged: Vec<String>,
    /// Currently selected setup hex, preserved by this plan.
    pub preserved_current: Option<String>,
}

/// Selects the prune set from `inputs`, per
/// `docs/environments/managed-state.md#retention-and-recovery`: a
/// setup record may be removed only when it is neither
/// `.dx/setups/current` nor used by an active process; a generation
/// may be removed only when no retained record references it and no
/// active process uses it. Deterministic: outputs sort ascending.
pub fn plan_prune(inputs: PruneInputs<'_>) -> CleanPlan {
    let mut retained: Vec<&str> = Vec::new();
    if let Some(current) = inputs.current_hex {
        retained.push(current);
    }
    retained.extend(
        inputs
            .active_setup_hexes
            .iter()
            .map(std::string::String::as_str),
    );
    retained.sort_unstable();
    retained.dedup();

    let mut prune_setup_records: Vec<String> = inputs
        .records
        .iter()
        .filter(|record| !retained.contains(&record.hex.as_str()))
        .map(|record| record.hex.clone())
        .collect();
    prune_setup_records.sort();
    prune_setup_records.dedup();

    let mut referenced: Vec<(GenerationKind, &str)> = Vec::new();
    for record in inputs.records {
        if retained.contains(&record.hex.as_str()) {
            referenced.push((GenerationKind::Environment, record.environment_hex.as_str()));
            referenced.push((GenerationKind::Generated, record.generated_hex.as_str()));
        }
    }
    let mut prune_generations: Vec<GenerationView> = inputs
        .generations
        .iter()
        .filter(|generation| {
            !inputs
                .active_generation_hexes
                .iter()
                .any(|active| active == &generation.hex)
                && !referenced
                    .iter()
                    .any(|(kind, hex)| *kind == generation.kind && *hex == generation.hex.as_str())
        })
        .cloned()
        .collect();
    prune_generations.sort_by(|left, right| {
        left.kind
            .dir_name()
            .cmp(right.kind.dir_name())
            .then_with(|| left.hex.cmp(&right.hex))
    });

    let mut refused_unmanaged: Vec<String> =
        inputs.unmanaged_names.iter().map(Clone::clone).collect();
    refused_unmanaged.sort();
    refused_unmanaged.dedup();

    CleanPlan {
        prune_setup_records,
        prune_generations,
        refused_unmanaged,
        preserved_current: inputs.current_hex.map(str::to_owned),
    }
}

/// How long a clean apply contends for the workspace commit lock before
/// failing with a busy diagnostic. Mirrors `dx_env::LOCK_TIMEOUT`
/// (ten-second deadline); pinned equal by test, never drifted silently.
pub const CLEAN_LOCK_TIMEOUT: Duration = Duration::from_secs(10);

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

/// Maps the shared commit-lock failure into the clean vocabulary.
/// Only contention reports busy; every other lock failure aborts
/// immediately so platform errors are never misreported.
fn map_lock_error(error: Error) -> CleanError {
    match error {
        Error::Busy { path } => CleanError::Busy { path },
        Error::LockFailed { path, reason } => CleanError::LockFailed { path, reason },
        other => CleanError::LockFailed {
            path: PathBuf::from(".dx"),
            reason: other.to_string(),
        },
    }
}

/// Owned filesystem inventory behind [`PruneInputs`]: validated setup
/// records, digest-shaped generations, the current selection, and refused
/// unmanaged names. Active (in-use) sets combine caller-provided hexes
/// with the [`scan_live_hexes`] process scan; unknown-live entries prune
/// exactly as the pure plan selects.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CollectedInventory {
    /// Validated setup records (see [`validate_record`]).
    pub records: Vec<SetupRecordView>,
    /// Digest-shaped generations present under the managed roots.
    pub generations: Vec<GenerationView>,
    /// Currently selected setup record hex, if any.
    pub current_hex: Option<String>,
    /// Setup-record hexes still referenced by an active process.
    pub active_setup_hexes: Vec<String>,
    /// Generation hexes still referenced by an active process.
    pub active_generation_hexes: Vec<String>,
    /// Directory names under the managed roots that are not
    /// digest-shaped (or fail record validation): refused, never deleted.
    pub unmanaged_names: Vec<String>,
}

impl CollectedInventory {
    /// Borrows this inventory as the pure planner input.
    pub fn prune_inputs(&self) -> PruneInputs<'_> {
        PruneInputs {
            records: &self.records,
            generations: &self.generations,
            current_hex: self.current_hex.as_deref(),
            active_setup_hexes: &self.active_setup_hexes,
            active_generation_hexes: &self.active_generation_hexes,
            unmanaged_names: &self.unmanaged_names,
        }
    }

    /// Selects the prune set from this inventory.
    pub fn plan(&self) -> CleanPlan {
        plan_prune(self.prune_inputs())
    }
}

/// Reads directory entry names under `dir`, sorted ascending. A missing
/// directory contributes nothing (first selection has no generations
/// yet); any other listing failure reports through [`CleanError`].
///
/// Implemented over [`walkdir::WalkDir`] at depth 1 (issue #223): the
/// managed `.dx` roots stay a direct-children listing with identical
/// semantics to the historical `read_dir` loop (sorted names,
/// non-UTF8 placeholder), while recursive and ignore-aware traversal
/// lives in [`walk_filtered`].
fn entry_names(dir: &Path) -> Result<Vec<String>, CleanError> {
    if !dir.exists() {
        match fs::read_dir(dir) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => {
                return Err(CleanError::Install {
                    reason: format!("cannot list {}: {e}", dir.display()),
                });
            }
            Ok(_) => {}
        }
    }
    let mut names = Vec::new();
    for entry in walkdir::WalkDir::new(dir).max_depth(1).min_depth(1) {
        let entry = entry.map_err(|e| CleanError::Install {
            reason: format!("cannot list {}: {e}", dir.display()),
        })?;
        if let Some(name) = entry.file_name().to_str() {
            names.push(name.to_owned());
        } else {
            // Non-UTF8 names are unmanaged by construction: they
            // can never be digest-shaped. Record a placeholder so
            // the plan refuses something rather than ignoring it.
            names.push("<non-utf8-name>".to_owned());
        }
    }
    names.sort();
    Ok(names)
}

/// Recursively walks `root` honoring `.gitignore` and related ignore
/// files (issue #223), skipping hidden entries and git-ignored paths
/// via the [`ignore`] crate (ripgrep family), with additional
/// caller-supplied glob exclusions via [`globset`].
///
/// `exclude_globs` are gitignore-style globs matched against paths
/// relative to `root` (for example `["*.log", "target/**"]`); invalid
/// globs fail through [`CleanError::Install`]. The returned paths are
/// sorted ascending for deterministic plans. A missing root walks
/// empty; any other traversal failure reports through [`CleanError`].
pub fn walk_filtered(root: &Path, exclude_globs: &[String]) -> Result<Vec<PathBuf>, CleanError> {
    let mut builder = globset::GlobSetBuilder::new();
    for pattern in exclude_globs {
        let glob = globset::Glob::new(pattern).map_err(|e| CleanError::Install {
            reason: format!("invalid exclude glob {pattern:?}: {e}"),
        })?;
        builder.add(glob);
    }
    let excludes = builder.build().map_err(|e| CleanError::Install {
        reason: format!("invalid exclude globs: {e}"),
    })?;
    if fs::read_dir(root).is_err_and(|e| e.kind() == io::ErrorKind::NotFound) {
        return Ok(Vec::new());
    }
    let mut paths = Vec::new();
    let walker = ignore::WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .require_git(false)
        .parents(true)
        .build();
    for entry in walker {
        let entry = entry.map_err(|e| CleanError::Install {
            reason: format!("cannot walk {}: {e}", root.display()),
        })?;
        let path = entry.path().to_path_buf();
        if path == root {
            continue;
        }
        let relative = path.strip_prefix(root).unwrap_or(&path);
        if excludes.is_match(relative) {
            continue;
        }
        paths.push(path);
    }
    paths.sort();
    Ok(paths)
}

/// Extracts a generation digest from a record link target: the final path
/// component must be digest-shaped.
fn generation_hex_from_link_target(target: &Path) -> Option<String> {
    target
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|text| GenerationId::new(text).is_ok())
        .map(str::to_owned)
}

/// Collects the clean inventory for `workspace_root`, validating every
/// setup record against the [`dx_setup`] pair identity. Present-but-
/// malformed current state fails closed ([`CleanError::CurrentInvalid`],
/// nothing prunable); digest-spoofed or otherwise invalid records join
/// the refused unmanaged set (never pruned, never adopted).
pub fn collect_inventory(
    workspace_root: &Path,
    active_setup_hexes: &[String],
    active_generation_hexes: &[String],
) -> Result<CollectedInventory, CleanError> {
    if !workspace_root.is_dir() {
        return Err(CleanError::WorkspaceRoot {
            path: workspace_root.to_path_buf(),
        });
    }
    let dx_dir = workspace_root.join(".dx");
    let setups_dir = dx_dir.join(SETUPS_DIR_NAME);
    let mut inventory = CollectedInventory {
        active_setup_hexes: active_setup_hexes.to_vec(),
        active_generation_hexes: active_generation_hexes.to_vec(),
        ..CollectedInventory::default()
    };

    // Current selection: absent selects nothing; anything present but not
    // a digest-shaped symlink target fails closed.
    let current_link = setups_dir.join(CURRENT_LINK_NAME);
    match fs::symlink_metadata(&current_link) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => {
            return Err(CleanError::CurrentInvalid {
                reason: format!("cannot inspect {}: {e}", current_link.display()),
            });
        }
        Ok(meta) => {
            if !meta.file_type().is_symlink() {
                return Err(CleanError::CurrentInvalid {
                    reason: format!(
                        "{} is not a symlink; refusing to adopt foreign state",
                        current_link.display()
                    ),
                });
            }
            let target = fs::read_link(&current_link).map_err(|e| CleanError::CurrentInvalid {
                reason: format!("cannot read {}: {e}", current_link.display()),
            })?;
            let hex = target
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_owned();
            if GenerationId::new(&hex).is_err() {
                return Err(CleanError::CurrentInvalid {
                    reason: format!(
                        "{} points at {hex:?}, want a setup digest; refusing digest-spoofed state",
                        current_link.display()
                    ),
                });
            }
            inventory.current_hex = Some(hex);
        }
    }

    // Setup records: digest-shaped names validate against their link
    // targets; anything else (including the staged `current.next` pointer
    // and digest-spoofed records) is refused, never pruned.
    //
    // The current pointer itself resolves through the record directory
    // (setup reads links via `current/<link>`), so record validation must
    // read links relative to each record directory, not the pointer.
    for name in entry_names(&setups_dir)? {
        if name == CURRENT_LINK_NAME || name == CURRENT_STAGE_NAME {
            continue;
        }
        if GenerationId::new(&name).is_err() {
            inventory.unmanaged_names.push(name);
            continue;
        }
        let record = setups_dir.join(&name);
        let environment = fs::read_link(record.join(ENVIRONMENT_LINK_NAME))
            .ok()
            .and_then(|target| generation_hex_from_link_target(&target));
        let generated = fs::read_link(record.join(GENERATED_LINK_NAME))
            .ok()
            .and_then(|target| generation_hex_from_link_target(&target));
        match (environment, generated) {
            (Some(environment_hex), Some(generated_hex)) => {
                match validate_record(&name, &environment_hex, &generated_hex) {
                    Ok(view) => inventory.records.push(view),
                    Err(_) => inventory.unmanaged_names.push(name),
                }
            }
            _ => inventory.unmanaged_names.push(name),
        }
    }

    // Generations: digest-shaped names only; anything else is unmanaged.
    for kind in [GenerationKind::Environment, GenerationKind::Generated] {
        let dir = dx_dir.join(kind.dir_name());
        for name in entry_names(&dir)? {
            if GenerationId::new(&name).is_ok() {
                inventory
                    .generations
                    .push(GenerationView { kind, hex: name });
            } else {
                inventory.unmanaged_names.push(name);
            }
        }
    }

    inventory
        .records
        .sort_by(|left, right| left.hex.cmp(&right.hex));
    inventory.generations.sort_by(|left, right| {
        left.kind
            .dir_name()
            .cmp(right.kind.dir_name())
            .then_with(|| left.hex.cmp(&right.hex))
    });
    inventory.unmanaged_names.sort();
    inventory.unmanaged_names.dedup();
    Ok(inventory)
}

/// Outcome of [`apply_plan`]: exactly which prune entries were removed.
/// The current pointer is never touched, so selection is unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CleanOutcome {
    /// Setup-record hexes removed from `.dx/setups`.
    pub removed_setup_records: Vec<String>,
    /// Generations removed from `.dx/environments` / `.dx/generated`.
    pub removed_generations: Vec<GenerationView>,
}

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
/// scans empty rather than failing. Over-retention is the only failure
/// direction: a hex observed anywhere under the managed roots is
/// preserved, whether or not it is still referenced. Deterministic:
/// outputs sort ascending with duplicates removed.
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

/// Applies `plan` to `workspace_root`: deletes exactly its prune sets
/// under the shared workspace commit lock, then releases the lock.
/// `--dry-run` plans never reach this function (rendering deletes
/// nothing and holds no lock).
///
/// Race safety: the current selection is re-read under the lock and any
/// prune entry that is now current (or referenced by the now-current
/// record) is skipped rather than deleted, so a concurrently completed
/// `dx env` / `dx codegen` / `dx setup` is never uninstalled. Every prune
/// name is re-validated as digest-shaped before deletion; missing entries
/// (already pruned) are idempotent successes.
pub fn apply_plan(workspace_root: &Path, plan: &CleanPlan) -> Result<CleanOutcome, CleanError> {
    apply_plan_with_timeout(workspace_root, plan, CLEAN_LOCK_TIMEOUT)
}

/// [`apply_plan`] with an injectable lock deadline (tests only).
pub fn apply_plan_with_timeout(
    workspace_root: &Path,
    plan: &CleanPlan,
    timeout: Duration,
) -> Result<CleanOutcome, CleanError> {
    if !workspace_root.is_dir() {
        return Err(CleanError::WorkspaceRoot {
            path: workspace_root.to_path_buf(),
        });
    }
    let dx_dir = workspace_root.join(".dx");
    fs::create_dir_all(&dx_dir).map_err(|e| CleanError::Install {
        reason: format!("cannot create {}: {e}", dx_dir.display()),
    })?;
    let _lock = acquire_lock(&dx_dir, timeout).map_err(map_lock_error)?;
    // Re-read the live selection under the lock; fail closed on foreign
    // state exactly like collection does.
    let live = collect_inventory(workspace_root, &[], &[])?;
    let live_current = live.current_hex;
    let live_pair = match read_current_pair(workspace_root) {
        Ok(pair) => pair,
        Err(e) => {
            return Err(CleanError::CurrentInvalid {
                reason: e.to_string(),
            });
        }
    };
    let mut outcome = CleanOutcome::default();
    let setups_dir = dx_dir.join(SETUPS_DIR_NAME);
    for hex in &plan.prune_setup_records {
        if GenerationId::new(hex).is_err() {
            continue;
        }
        if live_current.as_deref() == Some(hex.as_str()) {
            continue;
        }
        let record = setups_dir.join(hex);
        match fs::symlink_metadata(&record) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(e) => {
                return Err(CleanError::Install {
                    reason: format!("cannot inspect {}: {e}", record.display()),
                });
            }
            Ok(_) => {}
        }
        fs::remove_dir_all(&record).map_err(|e| CleanError::Install {
            reason: format!("cannot prune {}: {e}", record.display()),
        })?;
        outcome.removed_setup_records.push(hex.clone());
    }
    // Generations referenced by the live current record are preserved
    // even when the (older) plan selected them.
    let mut live_referenced: Vec<(GenerationKind, String)> = Vec::new();
    if let Some(pair) = live_pair {
        live_referenced.push((
            GenerationKind::Environment,
            pair.environment.as_str().to_owned(),
        ));
        live_referenced.push((
            GenerationKind::Generated,
            pair.generated.as_str().to_owned(),
        ));
    }
    for generation in &plan.prune_generations {
        if GenerationId::new(&generation.hex).is_err() {
            continue;
        }
        if live_referenced
            .iter()
            .any(|(kind, hex)| *kind == generation.kind && *hex == generation.hex)
        {
            continue;
        }
        let dir = dx_dir
            .join(generation.kind.dir_name())
            .join(&generation.hex);
        match fs::symlink_metadata(&dir) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(e) => {
                return Err(CleanError::Install {
                    reason: format!("cannot inspect {}: {e}", dir.display()),
                });
            }
            Ok(_) => {}
        }
        fs::remove_dir_all(&dir).map_err(|e| CleanError::Install {
            reason: format!("cannot prune {}: {e}", dir.display()),
        })?;
        outcome.removed_generations.push(generation.clone());
    }
    outcome.removed_setup_records.sort();
    outcome.removed_generations.sort_by(|left, right| {
        left.kind
            .dir_name()
            .cmp(right.kind.dir_name())
            .then_with(|| left.hex.cmp(&right.hex))
    });
    Ok(outcome)
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
    fn record_validation_pins_pair_identity() {
        let (hex, env_hex, gen_hex) = pair_env_gen('1', '2');
        let view = validate_record(&hex, &env_hex, &gen_hex).expect("valid");
        assert_eq!(view.hex, hex);
        assert_eq!(view.environment_hex, env_hex);
        assert_eq!(view.generated_hex, gen_hex);
    }

    #[test]
    fn malformed_digests_are_refused() {
        assert!(matches!(
            validate_record("not-a-digest", &digest('1'), &digest('2')),
            Err(RecordProblem::MalformedDigest { .. })
        ));
        assert!(matches!(
            validate_record(&digest('a'), &digest('1'), "SPOOF"),
            Err(RecordProblem::MalformedDigest { .. })
        ));
    }

    #[test]
    fn spoofed_record_names_are_refused() {
        let (_, env_hex, gen_hex) = pair_env_gen('1', '2');
        let (other_hex, _, _) = pair_env_gen('3', '4');
        let error = validate_record(&other_hex, &env_hex, &gen_hex).unwrap_err();
        assert!(matches!(error, RecordProblem::Spoofed { .. }));
        assert!(!format!("{error}").is_empty());
    }

    #[test]
    fn current_record_and_its_generations_are_preserved() {
        let current = record('1', '2');
        let stale = record('3', '4');
        let records = vec![current.clone(), stale.clone()];
        let generations = vec![
            generation(GenerationKind::Environment, '1'),
            generation(GenerationKind::Generated, '2'),
            generation(GenerationKind::Environment, '3'),
            generation(GenerationKind::Generated, '4'),
        ];
        let plan = plan_prune(inputs(&records, &generations, Some(&current.hex)));
        assert_eq!(plan.prune_setup_records, vec![stale.hex.clone()]);
        assert_eq!(
            plan.prune_generations,
            vec![
                generation(GenerationKind::Environment, '3'),
                generation(GenerationKind::Generated, '4'),
            ]
        );
        assert_eq!(plan.preserved_current, Some(current.hex.clone()));
        assert!(plan.refused_unmanaged.is_empty());
    }

    #[test]
    fn generations_shared_with_retained_records_are_preserved() {
        let current = record('1', '2');
        let records = vec![current.clone()];
        let generations = vec![
            generation(GenerationKind::Environment, '1'),
            generation(GenerationKind::Generated, '2'),
        ];
        let plan = plan_prune(inputs(&records, &generations, Some(&current.hex)));
        assert!(plan.prune_setup_records.is_empty());
        assert!(plan.prune_generations.is_empty());
    }

    #[test]
    fn active_records_and_generations_are_preserved() {
        let current = record('1', '2');
        let active_record = record('3', '4');
        let records = vec![current.clone(), active_record.clone()];
        let generations = vec![
            generation(GenerationKind::Environment, '3'),
            generation(GenerationKind::Generated, '4'),
        ];
        let active_setup = vec![active_record.hex.clone()];
        let active_generations = vec![digest('3'), digest('4')];
        let plan = plan_prune(PruneInputs {
            records: &records,
            generations: &generations,
            current_hex: Some(&current.hex),
            active_setup_hexes: &active_setup,
            active_generation_hexes: &active_generations,
            unmanaged_names: &[],
        });
        assert!(plan.prune_setup_records.is_empty());
        assert!(plan.prune_generations.is_empty());
    }

    #[test]
    fn unmanaged_names_are_refused_never_pruned() {
        let records = vec![record('1', '2')];
        let unmanaged = vec!["latest".to_owned(), "not-hex".to_owned()];
        let plan = plan_prune(PruneInputs {
            records: &records,
            generations: &[],
            current_hex: None,
            active_setup_hexes: &[],
            active_generation_hexes: &[],
            unmanaged_names: &unmanaged,
        });
        assert_eq!(plan.refused_unmanaged, unmanaged);
        assert!(!plan
            .prune_setup_records
            .iter()
            .any(|hex| unmanaged.contains(hex)));
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

    #[test]
    fn walk_filtered_skips_gitignored_and_glob_excluded_files() {
        // Issue #223: recursive walks honor `.gitignore` (via the
        // `ignore` crate) plus caller-supplied `globset` exclusions.
        let scratch = dx_test_scratch::scratch("dx-clean-walk-");
        let root = scratch.path();
        fs::write(root.join(".gitignore"), "ignored.txt\n").expect("gitignore");
        fs::write(root.join("ignored.txt"), "skip me").expect("ignored file");
        fs::write(root.join("kept.txt"), "keep me").expect("kept file");
        fs::write(root.join("skip.log"), "glob me").expect("glob file");
        fs::create_dir_all(root.join("sub")).expect("subdir");
        fs::write(root.join("sub").join("nested.txt"), "nested").expect("nested");

        let walked = walk_filtered(root, &[]).expect("walk");
        let names: Vec<String> = walked
            .iter()
            .filter_map(|path| {
                path.strip_prefix(root)
                    .ok()
                    .and_then(|relative| relative.to_str().map(str::to_owned))
            })
            .collect();
        assert!(
            names.iter().any(|name| name == "kept.txt"),
            "kept file must walk: {names:?}"
        );
        assert!(
            names
                .iter()
                .any(|name| name == "sub/nested.txt" || name == "sub\\nested.txt"),
            "nested file must walk: {names:?}"
        );
        assert!(
            !names.iter().any(|name| name == "ignored.txt"),
            "gitignored file must be skipped: {names:?}"
        );

        let filtered = walk_filtered(root, &["*.log".to_owned()]).expect("filtered walk");
        let filtered_names: Vec<String> = filtered
            .iter()
            .filter_map(|path| {
                path.strip_prefix(root)
                    .ok()
                    .and_then(|relative| relative.to_str().map(str::to_owned))
            })
            .collect();
        assert!(
            !filtered_names.iter().any(|name| name == "skip.log"),
            "glob-excluded file must be skipped: {filtered_names:?}"
        );
        assert!(
            filtered_names.iter().any(|name| name == "kept.txt"),
            "kept file must survive glob filtering: {filtered_names:?}"
        );
        // Deterministic order for plans.
        let mut sorted = filtered.clone();
        sorted.sort();
        assert_eq!(filtered, sorted, "walks must be sorted");
    }

    #[test]
    fn walk_filtered_rejects_invalid_globs_and_walks_missing_empty() {
        let missing = std::env::temp_dir().join("dx-clean-missing-walk-root");
        let _ = fs::remove_dir_all(&missing);
        let empty = walk_filtered(&missing, &[]).expect("missing walks empty");
        assert!(empty.is_empty());
        let scratch = dx_test_scratch::scratch("dx-clean-glob-");
        assert!(matches!(
            walk_filtered(scratch.path(), &["[invalid".to_owned()]),
            Err(CleanError::Install { .. })
        ));
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
    fn clean_lock_deadline_matches_env_owner() {
        assert_eq!(CLEAN_LOCK_TIMEOUT, dx_env::LOCK_TIMEOUT);
    }

    #[test]
    fn empty_workspace_collects_nothing() {
        let scratch = clean_root("empty");
        let root = scratch.path().to_path_buf();
        let workspace = workspace_of(&root);
        let inventory = collect_inventory(&workspace, &[], &[]).expect("collect");
        assert!(inventory.records.is_empty());
        assert!(inventory.generations.is_empty());
        assert_eq!(inventory.current_hex, None);
        assert!(inventory.unmanaged_names.is_empty());
        let plan = inventory.plan();
        assert!(plan.prune_setup_records.is_empty());
        assert!(plan.prune_generations.is_empty());
        let outcome = apply_plan(&workspace, &plan).expect("apply empty");
        assert_eq!(outcome, CleanOutcome::default());
        let _ = fs::remove_dir_all(&root);
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

    #[test]
    fn malformed_current_fails_closed() {
        let scratch = clean_root("bad-current");
        let root = scratch.path().to_path_buf();
        let workspace = workspace_of(&root);
        dx_setup::commit_pair(&workspace, &setup_pair('1', '2')).expect("commit");
        let current = workspace.join(".dx").join("setups").join("current");
        fs::remove_file(&current).expect("remove pointer");
        fs::write(&current, "not a symlink").expect("file pointer");
        assert!(matches!(
            collect_inventory(&workspace, &[], &[]),
            Err(CleanError::CurrentInvalid { .. })
        ));
        let plan = CleanPlan {
            prune_setup_records: Vec::new(),
            prune_generations: Vec::new(),
            refused_unmanaged: Vec::new(),
            preserved_current: None,
        };
        assert!(matches!(
            apply_plan(&workspace, &plan),
            Err(CleanError::CurrentInvalid { .. })
        ));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn apply_removes_prune_set_and_preserves_current() {
        let scratch = clean_root("apply");
        let root = scratch.path().to_path_buf();
        let (workspace, stale_hex, current_hex) = two_record_workspace(&root);
        let inventory = collect_inventory(&workspace, &[], &[]).expect("collect");
        let plan = inventory.plan();
        assert_eq!(plan.prune_setup_records, vec![stale_hex.clone()]);
        let mut pruned_hexes: Vec<String> = plan
            .prune_generations
            .iter()
            .map(|generation| generation.hex.clone())
            .collect();
        pruned_hexes.sort();
        assert_eq!(pruned_hexes, vec![digest('3'), digest('4')]);
        let outcome = apply_plan(&workspace, &plan).expect("apply");
        assert_eq!(outcome.removed_setup_records, vec![stale_hex.clone()]);
        assert_eq!(outcome.removed_generations.len(), 2);
        // Current record, its generations, and the pointer survive.
        let setups = workspace.join(".dx").join("setups");
        assert!(setups.join(&current_hex).join("environment").is_symlink());
        assert!(workspace
            .join(".dx")
            .join("environments")
            .join(digest('1'))
            .is_dir());
        assert!(workspace
            .join(".dx")
            .join("generated")
            .join(digest('2'))
            .is_dir());
        assert!(!setups.join(&stale_hex).exists());
        assert_eq!(
            read_current_pair(&workspace)
                .expect("read")
                .map(|pair| setup_hex(&pair)),
            Some(current_hex)
        );
        // Second apply over the same plan is idempotent: nothing left.
        let again = apply_plan(&workspace, &plan).expect("re-apply");
        assert_eq!(again, CleanOutcome::default());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn apply_skips_entries_that_became_current() {
        let scratch = clean_root("race");
        let root = scratch.path().to_path_buf();
        let workspace = workspace_of(&root);
        let first = setup_pair('1', '2');
        let second = setup_pair('3', '4');
        dx_setup::commit_pair(&workspace, &first).expect("commit first");
        dx_setup::commit_pair(&workspace, &second).expect("commit second");
        // Plan built while `second` is current selects `first` for prune.
        let plan = collect_inventory(&workspace, &[], &[])
            .expect("collect")
            .plan();
        assert_eq!(plan.prune_setup_records, vec![setup_hex(&first)]);
        // A concurrent setup reselects `first` before clean applies.
        dx_setup::commit_pair(&workspace, &first).expect("reselect first");
        let outcome = apply_plan(&workspace, &plan).expect("apply");
        assert!(outcome.removed_setup_records.is_empty());
        assert_eq!(read_current_pair(&workspace).expect("read"), Some(first));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn apply_skips_generations_referenced_by_live_current() {
        let scratch = clean_root("race-gen");
        let root = scratch.path().to_path_buf();
        let (workspace, stale_hex, _) = two_record_workspace(&root);
        let stale_record = record('3', '4');
        // Stale plan: no current known, so the stale generations prune.
        let plan = plan_prune(PruneInputs {
            records: &[stale_record],
            generations: &[
                generation(GenerationKind::Environment, '1'),
                generation(GenerationKind::Generated, '2'),
            ],
            current_hex: None,
            active_setup_hexes: &[],
            active_generation_hexes: &[],
            unmanaged_names: &[],
        });
        assert_eq!(plan.prune_setup_records, vec![stale_hex.clone()]);
        assert_eq!(plan.prune_generations.len(), 2);
        // Live current still references both generations: apply skips them
        // while removing the unselected record.
        let outcome = apply_plan(&workspace, &plan).expect("apply");
        assert_eq!(outcome.removed_setup_records, vec![stale_hex]);
        assert!(outcome.removed_generations.is_empty());
        assert!(workspace
            .join(".dx")
            .join("environments")
            .join(digest('1'))
            .is_dir());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn busy_lock_fails_after_deadline() {
        let scratch = clean_root("busy");
        let root = scratch.path().to_path_buf();
        let workspace = workspace_of(&root);
        let dx_dir = workspace.join(".dx");
        fs::create_dir_all(&dx_dir).expect("dx dir");
        let _held = acquire_lock(&dx_dir, Duration::from_secs(10)).expect("hold commit lock");
        let plan = CleanPlan {
            prune_setup_records: Vec::new(),
            prune_generations: Vec::new(),
            refused_unmanaged: Vec::new(),
            preserved_current: None,
        };
        let error =
            apply_plan_with_timeout(&workspace, &plan, Duration::from_millis(1)).unwrap_err();
        assert!(matches!(error, CleanError::Busy { .. }));
        drop(_held);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn workspace_missing_fails() {
        let scratch = clean_root("ws-missing");
        let root = scratch.path().to_path_buf();
        let missing = root.join("no-such-dir");
        assert!(matches!(
            collect_inventory(&missing, &[], &[]),
            Err(CleanError::WorkspaceRoot { .. })
        ));
        let plan = CleanPlan {
            prune_setup_records: Vec::new(),
            prune_generations: Vec::new(),
            refused_unmanaged: Vec::new(),
            preserved_current: None,
        };
        assert!(matches!(
            apply_plan(&missing, &plan),
            Err(CleanError::WorkspaceRoot { .. })
        ));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn clean_errors_display() {
        let errors = [
            CleanError::WorkspaceRoot {
                path: PathBuf::from("/ws"),
            },
            CleanError::Busy {
                path: PathBuf::from("/ws/.dx/.commit.lock"),
            },
            CleanError::LockFailed {
                path: PathBuf::from("/ws/.dx/.commit.lock"),
                reason: "r".to_string(),
            },
            CleanError::CurrentInvalid {
                reason: "r".to_string(),
            },
            CleanError::Install {
                reason: "r".to_string(),
            },
        ];
        for error in &errors {
            assert!(!format!("{error}").is_empty());
        }
    }

    /// Stages a fake process directory under `proc_root/<pid>` with a
    /// `cwd` symlink and numbered `fd` symlinks to the given targets.
    /// Unix-only: the fake-`/proc` fixtures below need symlink atoms.
    #[cfg(unix)]
    fn stage_process(proc_root: &Path, pid: &str, cwd: Option<&Path>, fds: &[&Path]) {
        let dir = proc_root.join(pid);
        fs::create_dir_all(dir.join("fd")).expect("stage fd dir");
        if let Some(target) = cwd {
            std::os::unix::fs::symlink(target, dir.join("cwd")).expect("stage cwd");
        }
        for (index, target) in fds.iter().enumerate() {
            std::os::unix::fs::symlink(target, dir.join("fd").join(index.to_string()))
                .expect("stage fd");
        }
    }

    #[test]
    fn scan_missing_proc_root_scans_empty() {
        let scratch = clean_root("scan-missing");
        let root = scratch.path().to_path_buf();
        let dx_dir = workspace_of(&root).join(".dx");
        assert_eq!(
            scan_live_hexes(&root.join("no-such-proc"), &dx_dir),
            LiveHexes::default()
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    #[cfg(unix)]
    fn scan_ignores_non_numeric_entries() {
        let scratch = clean_root("scan-nonnumeric");
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
    #[cfg(unix)]
    fn scan_reports_cwd_and_fd_targets_under_managed_roots() {
        let scratch = clean_root("scan-live");
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
    #[cfg(unix)]
    fn scan_trims_deleted_suffix_and_ignores_unmanaged_paths() {
        let scratch = clean_root("scan-edge");
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
        std::os::unix::fs::symlink(&deleted_text, dir.join("cwd")).expect("cwd");
        // Unmanaged names, non-digest names, and foreign roots
        // contribute nothing even when observed.
        std::os::unix::fs::symlink(
            dx_dir.join("setups").join("latest"),
            dir.join("fd").join("0"),
        )
        .expect("fd");
        std::os::unix::fs::symlink(
            dx_dir.join("notes").join(digest('9')),
            dir.join("fd").join("1"),
        )
        .expect("fd");
        std::os::unix::fs::symlink(
            root.join("elsewhere").join(digest('8')),
            dir.join("fd").join("2"),
        )
        .expect("fd");
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
    #[cfg(unix)]
    fn scan_dedupes_and_sorts_across_processes() {
        let scratch = clean_root("scan-dedupe");
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
        let scratch = clean_root("with-scan");
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
