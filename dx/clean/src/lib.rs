//! Explicit managed-state cleanup planning for `dx clean` (M25 WP5, O60).
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
//! Filesystem inventory collection ([`collect_inventory`]) and the locked
//! apply step ([`apply_plan`]) complete the WP5 slice 2 surface; planning
//! over injected views keeps selection deterministic and unit-testable
//! without a workspace.
//!
//! The commit-lock route is the O36 lock owned by `dx_env::acquire_lock`
//! (dedicated lock file, contention-only retry until the deadline):
//! clean introduces no new lock file, mechanism, or deadline.
//! [`CLEAN_LOCK_TIMEOUT`] mirrors `dx_env::LOCK_TIMEOUT`, pinned equal by
//! test like `dx_setup::COMMIT_LOCK_TIMEOUT`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// `--dry-run` flag: list reclaimable generations and links without
/// deleting. Matches the `dx clean` contract; frozen here so CLI
/// parsing and help text cannot drift from the qualified shape (O60).
pub const DRY_RUN_FLAG: &str = "--dry-run";

/// `--bazel` flag: additionally forward `bazel clean` and print
/// [`RECOVERY_GUIDANCE`]. Explicit opt-in only; default `dx clean`
/// never touches Bazel outputs.
pub const BAZEL_FLAG: &str = "--bazel";

/// Recovery guidance printed after an explicit `dx clean --bazel`
/// forward, per the clean contract: `bazel clean` leaves managed links
/// dangling, and only explicit `dx env` / `dx codegen` / `dx setup`
/// repairs the projection. Links never self-repair.
pub const RECOVERY_GUIDANCE: &str = "bazel clean forwarded; managed links may now dangle: \
    re-run `dx setup` (or `dx env` / `dx codegen`) to repair the selection";

/// `bazel` subcommand forwarded by `dx clean --bazel`: exactly
/// `bazel clean`, never any other Bazel verb.
pub fn bazel_forward_argv() -> Vec<String> {
    vec!["clean".to_owned()]
}

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
            GenerationKind::Environment => dx_setup::ENVIRONMENTS_DIR_NAME,
            GenerationKind::Generated => dx_setup::GENERATED_DIR_NAME,
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordProblem {
    /// Record, environment, or generated name is not a 64-character
    /// lowercase hexadecimal digest.
    MalformedDigest { value: String },
    /// Record links resolve to a pair whose digest differs from the
    /// record directory name (digest-spoofed path).
    Spoofed { record: String, pair: String },
}

impl std::fmt::Display for RecordProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for RecordProblem {}

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
    for value in [hex, environment_hex, generated_hex] {
        if dx_setup::GenerationId::new(value).is_err() {
            return Err(RecordProblem::MalformedDigest {
                value: value.to_owned(),
            });
        }
    }
    let pair = dx_setup::SetupPair {
        environment: dx_setup::GenerationId::new(environment_hex)
            .expect("validated environment digest"),
        generated: dx_setup::GenerationId::new(generated_hex).expect("validated generated digest"),
    };
    let want = dx_setup::setup_hex(&pair);
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
/// failing with a busy diagnostic. Mirrors `dx_env::LOCK_TIMEOUT` (O36
/// ten-second deadline); pinned equal by test, never drifted silently.
pub const CLEAN_LOCK_TIMEOUT: Duration = Duration::from_secs(10);

/// Clean failure. Every variant is operational; `--dry-run` rendering
/// never surfaces these (it deletes nothing and holds no lock).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CleanError {
    /// Workspace root is missing or not a directory.
    WorkspaceRoot { path: PathBuf },
    /// Another command holds the commit lock past the deadline.
    Busy { path: PathBuf },
    /// The commit lock cannot be opened or locked.
    LockFailed { path: PathBuf, reason: String },
    /// `.dx/setups/current` is present but malformed. Never adopted,
    /// never repaired, and nothing is pruned: the operator removes the
    /// offending path or re-runs setup from a clean selection.
    CurrentInvalid { reason: String },
    /// A workspace mutation failed. Entries already deleted stay deleted
    /// (each removal is independent); the current pointer is never
    /// touched by clean, so selection is unchanged.
    Install { reason: String },
}

impl std::fmt::Display for CleanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for CleanError {}

/// Maps the shared commit-lock failure into the clean vocabulary.
/// Only contention reports busy; every other lock failure aborts
/// immediately so platform errors are never misreported.
fn map_lock_error(error: dx_env::Error) -> CleanError {
    match error {
        dx_env::Error::Busy { path } => CleanError::Busy { path },
        dx_env::Error::LockFailed { path, reason } => CleanError::LockFailed { path, reason },
        other => CleanError::LockFailed {
            path: PathBuf::from(".dx"),
            reason: other.to_string(),
        },
    }
}

/// Owned filesystem inventory behind [`PruneInputs`]: validated setup
/// records, digest-shaped generations, the current selection, and refused
/// unmanaged names. Active (in-use) sets are caller-provided: v1 has no
/// process-scan detector, so callers pass the hexes they know are live
/// (empty when none is known); unknown-live entries prune exactly as the
/// pure plan selects. Reclaimable-bytes reporting stays open under O60.
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
fn entry_names(dir: &Path) -> Result<Vec<String>, CleanError> {
    match fs::read_dir(dir) {
        Ok(entries) => {
            let mut names = Vec::new();
            for entry in entries {
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
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(CleanError::Install {
            reason: format!("cannot list {}: {e}", dir.display()),
        }),
    }
}

/// Extracts a generation digest from a record link target: the final path
/// component must be digest-shaped.
fn generation_hex_from_link_target(target: &Path) -> Option<String> {
    target
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|text| dx_setup::GenerationId::new(text).is_ok())
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
    let setups_dir = dx_dir.join(dx_setup::SETUPS_DIR_NAME);
    let mut inventory = CollectedInventory {
        active_setup_hexes: active_setup_hexes.to_vec(),
        active_generation_hexes: active_generation_hexes.to_vec(),
        ..CollectedInventory::default()
    };

    // Current selection: absent selects nothing; anything present but not
    // a digest-shaped symlink target fails closed.
    let current_link = setups_dir.join(dx_setup::CURRENT_LINK_NAME);
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
            if dx_setup::GenerationId::new(&hex).is_err() {
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
        if name == dx_setup::CURRENT_LINK_NAME || name == dx_setup::CURRENT_STAGE_NAME {
            continue;
        }
        if dx_setup::GenerationId::new(&name).is_err() {
            inventory.unmanaged_names.push(name);
            continue;
        }
        let record = setups_dir.join(&name);
        let environment = fs::read_link(record.join(dx_setup::ENVIRONMENT_LINK_NAME))
            .ok()
            .and_then(|target| generation_hex_from_link_target(&target));
        let generated = fs::read_link(record.join(dx_setup::GENERATED_LINK_NAME))
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
            if dx_setup::GenerationId::new(&name).is_ok() {
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
    let _lock = dx_env::acquire_lock(&dx_dir, timeout).map_err(map_lock_error)?;
    // Re-read the live selection under the lock; fail closed on foreign
    // state exactly like collection does.
    let live = collect_inventory(workspace_root, &[], &[])?;
    let live_current = live.current_hex;
    let live_pair = match dx_setup::read_current_pair(workspace_root) {
        Ok(pair) => pair,
        Err(e) => {
            return Err(CleanError::CurrentInvalid {
                reason: e.to_string(),
            });
        }
    };
    let mut outcome = CleanOutcome::default();
    let setups_dir = dx_dir.join(dx_setup::SETUPS_DIR_NAME);
    for hex in &plan.prune_setup_records {
        if dx_setup::GenerationId::new(hex).is_err() {
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
        if dx_setup::GenerationId::new(&generation.hex).is_err() {
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

/// Renders the `--dry-run` listing for `plan`: reclaimable setup
/// records and generation links, the preserved current selection, and
/// refused unmanaged paths. Deletes nothing; [`apply_plan`] deletes
/// exactly the listed prune sets.
pub fn render_dry_run(plan: &CleanPlan) -> String {
    let mut lines = vec!["dx clean --dry-run: reclaimable managed state".to_owned()];
    if plan.prune_setup_records.is_empty() && plan.prune_generations.is_empty() {
        lines.push("nothing to prune".to_owned());
    }
    for hex in &plan.prune_setup_records {
        lines.push(format!("prune setup record: .dx/setups/{hex}"));
    }
    for generation in &plan.prune_generations {
        lines.push(format!(
            "prune generation: .dx/{}/{}",
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
    lines.join("\n")
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
    fn flag_shape_is_frozen() {
        assert_eq!(DRY_RUN_FLAG, "--dry-run");
        assert_eq!(BAZEL_FLAG, "--bazel");
        assert_eq!(bazel_forward_argv(), vec!["clean".to_owned()]);
        assert!(RECOVERY_GUIDANCE.contains("dx setup"));
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
        let listing = render_dry_run(&plan);
        assert!(listing.contains(&format!(".dx/setups/{}", stale.hex)));
        assert!(listing.contains(&format!(".dx/environments/{}", digest('3'))));
        assert!(listing.contains(&format!(".dx/setups/{}", current.hex)));
        assert!(listing.contains("latest"));
    }

    #[test]
    fn dry_run_reports_empty_prune_set() {
        let current = record('1', '2');
        let records = vec![current.clone()];
        let plan = plan_prune(inputs(&records, &[], Some(&current.hex)));
        assert!(render_dry_run(&plan).contains("nothing to prune"));
    }

    #[cfg(windows)]
    fn symlink_dir(target: &Path, link: &Path) {
        std::os::windows::fs::symlink_dir(target, link).expect("stage test link");
    }

    #[cfg(not(windows))]
    fn symlink_dir(target: &Path, link: &Path) {
        std::os::unix::fs::symlink(target, link).expect("stage test link");
    }

    fn setup_pair(env: char, gen: char) -> dx_setup::SetupPair {
        dx_setup::SetupPair {
            environment: dx_setup::GenerationId::new(&digest(env)).expect("env digest"),
            generated: dx_setup::GenerationId::new(&digest(gen)).expect("gen digest"),
        }
    }

    fn clean_root(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dx-clean-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("ws")).expect("create workspace");
        dir
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
        let stale_hex = dx_setup::setup_hex(&stale);
        let current_hex = dx_setup::setup_hex(&current);
        (workspace, stale_hex, current_hex)
    }

    #[test]
    fn clean_lock_deadline_matches_env_owner() {
        assert_eq!(CLEAN_LOCK_TIMEOUT, dx_env::LOCK_TIMEOUT);
    }

    #[test]
    fn empty_workspace_collects_nothing() {
        let root = clean_root("empty");
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
        let root = clean_root("inventory");
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
        let root = clean_root("bad-current");
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
        let root = clean_root("apply");
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
            dx_setup::read_current_pair(&workspace)
                .expect("read")
                .map(|pair| dx_setup::setup_hex(&pair)),
            Some(current_hex)
        );
        // Second apply over the same plan is idempotent: nothing left.
        let again = apply_plan(&workspace, &plan).expect("re-apply");
        assert_eq!(again, CleanOutcome::default());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn apply_skips_entries_that_became_current() {
        let root = clean_root("race");
        let workspace = workspace_of(&root);
        let first = setup_pair('1', '2');
        let second = setup_pair('3', '4');
        dx_setup::commit_pair(&workspace, &first).expect("commit first");
        dx_setup::commit_pair(&workspace, &second).expect("commit second");
        // Plan built while `second` is current selects `first` for prune.
        let plan = collect_inventory(&workspace, &[], &[])
            .expect("collect")
            .plan();
        assert_eq!(plan.prune_setup_records, vec![dx_setup::setup_hex(&first)]);
        // A concurrent setup reselects `first` before clean applies.
        dx_setup::commit_pair(&workspace, &first).expect("reselect first");
        let outcome = apply_plan(&workspace, &plan).expect("apply");
        assert!(outcome.removed_setup_records.is_empty());
        assert_eq!(
            dx_setup::read_current_pair(&workspace).expect("read"),
            Some(first)
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn apply_skips_generations_referenced_by_live_current() {
        let root = clean_root("race-gen");
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
        let root = clean_root("busy");
        let workspace = workspace_of(&root);
        let dx_dir = workspace.join(".dx");
        fs::create_dir_all(&dx_dir).expect("dx dir");
        let _held =
            dx_env::acquire_lock(&dx_dir, Duration::from_secs(10)).expect("hold commit lock");
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
        let root = clean_root("ws-missing");
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
}
