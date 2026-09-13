//! Combined setup request planning for the `dx` CLI (M25 WP3 slice 1).
//!
//! Contract: `docs/cli/commands/environment-codegen-setup.md` (`dx setup`
//! scope: no argument prepares repository-wide codegen and environment
//! generations, one exact target label prepares that target, paths/
//! patterns/multiple labels/profiles/language selectors rejected) and
//! `docs/environments/managed-state.md` (one Bazel request with the union
//! of required roots, both aspects, both output groups, and one BEP
//! stream; codegen and env never invoke each other).
//!
//! This crate owns scope resolution, the combined request plan, pair
//! resolution (slice 2), and the commit layer (slice 3): re-reading
//! `.dx/setups/current` under the O36 commit lock and atomically
//! replacing the current pointer. Staging, validation, and generation
//! materialization land in later WP3 slices; the setup record links may
//! dangle until generations exist (ordinary host missing-target behavior,
//! never auto-repair).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Codegen collecting aspect applied in the combined request. Matches
/// `dx_codegen_plan_aspect` in `//generation:codegen.bzl` and
/// `CODEGEN_ASPECT` in `dx_codegen`.
pub const CODEGEN_ASPECT: &str = "//generation:codegen.bzl%dx_codegen_plan_aspect";

/// Environment collecting aspect applied in the combined request.
/// Matches `dx_env_plan_aspect` in `//env:plan.bzl` and `ENV_ASPECT` in
/// `dx_env_plan`.
pub const ENV_ASPECT: &str = "//env:plan.bzl%dx_env_plan_aspect";

/// Private output group carrying collected codegen shards plus every
/// referenced generated artifact. Matches
/// `DX_CODEGEN_PLAN_OUTPUT_GROUP` in `//generation:codegen.bzl` and
/// `OUTPUT_GROUP` in `dx_codegen`.
pub const CODEGEN_OUTPUT_GROUP: &str = "dx_codegen_plans";

/// Private output group carrying collected env shards plus every
/// referenced artifact. Matches `DX_ENV_PLAN_OUTPUT_GROUP` in
/// `//env:plan.bzl` and `OUTPUT_GROUP` in `dx_env_plan`.
pub const ENV_OUTPUT_GROUP: &str = "dx_env_plans";

/// Canonical repository-wide codegen selection built by bare `dx setup`.
/// Matches `REPOSITORY_TARGET` in `dx_codegen`. The effective Bazel roots
/// behind this label stay provisional pending the WP4 (O34) root
/// benchmark; this crate only owns the selection identity.
pub const CODEGEN_REPOSITORY_TARGET: &str = "//dx:codegen";

/// Canonical repository-wide env selection built by bare `dx setup`.
/// Matches `REPOSITORY_TARGET` in `dx_env_plan`.
pub const ENV_REPOSITORY_TARGET: &str = "//dx:env";

/// Selected setup scope: the whole repository or one exact target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupScope {
    Repository,
    Exact(String),
}

/// Exact-target scope failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeError {
    /// More than one positional target: setup selects at most one.
    MultipleTargets { count: usize },
    /// A target pattern (`...`, `*`, `?`): patterns never select setup.
    TargetPattern { value: String },
    /// Anything that is not an exact target label: paths, directories,
    /// profiles/flags, and language selectors.
    NotTargetLabel { value: String },
}

impl std::fmt::Display for ScopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ScopeError {}

/// Resolves the `dx setup` positional scope: empty selects the
/// repository (both canonical selections), one exact `//` or `@` label
/// selects its configured closure, and anything else fails before
/// execution. A compatible target whose closure contributes no shards
/// selects empty sides downstream (with carry-forward or managed empty
/// generations), never a failure here.
pub fn resolve_scope(targets: &[String]) -> Result<SetupScope, ScopeError> {
    match targets {
        [] => Ok(SetupScope::Repository),
        [single] => {
            if single.contains("...") || single.contains('*') || single.contains('?') {
                Err(ScopeError::TargetPattern {
                    value: single.clone(),
                })
            } else if single.starts_with("//") || single.starts_with('@') {
                Ok(SetupScope::Exact(single.clone()))
            } else {
                Err(ScopeError::NotTargetLabel {
                    value: single.clone(),
                })
            }
        }
        _ => Err(ScopeError::MultipleTargets {
            count: targets.len(),
        }),
    }
}

/// Bazel labels to build for `scope`: both canonical repository
/// selections, or the one exact label. The combined request builds this
/// root set once; Bazel deduplicates common configured closures and
/// actions across the two aspects.
pub fn scope_targets(scope: &SetupScope) -> Vec<String> {
    match scope {
        SetupScope::Repository => vec![
            CODEGEN_REPOSITORY_TARGET.to_owned(),
            ENV_REPOSITORY_TARGET.to_owned(),
        ],
        SetupScope::Exact(label) => vec![label.clone()],
    }
}

/// Collecting aspects applied together in the single setup request: the
/// codegen plan aspect plus the env plan aspect. Codegen and env never
/// invoke each other; each aspect stays bounded by provider
/// applicability to targets with a nonempty effective stage subset.
pub fn request_aspects() -> Vec<String> {
    vec![CODEGEN_ASPECT.to_owned(), ENV_ASPECT.to_owned()]
}

/// Bazel labels to build for a WP4 root plan behind the canonical
/// repository-wide selections: the baseline plan keeps both selection
/// identities (`//dx:codegen` plus `//dx:env`) while the benchmark runs;
/// every other candidate passes its single union root set through. The
/// query-pattern-file candidate carries no command-line patterns (Bazel
/// reads them from `--target_pattern_file`).
pub fn targets_for_root_plan(plan: &dx_roots::RepositoryRootPlan) -> Vec<String> {
    if plan.pattern_file.is_some() {
        return Vec::new();
    }
    if *plan == dx_roots::repository_plan() {
        return scope_targets(&SetupScope::Repository);
    }
    plan.roots.clone()
}

/// Plans the single combined Bazel request for a WP4 root plan: the
/// plan's union roots with both collecting aspects and both output
/// groups. Codegen and env never invoke each other; each aspect stays
/// bounded by provider applicability.
pub fn plan_request_for_root_plan(plan: &dx_roots::RepositoryRootPlan) -> SetupRequest {
    SetupRequest {
        roots: targets_for_root_plan(plan),
        aspects: request_aspects(),
        output_groups: request_output_groups(),
    }
}

/// Full `bazel build` command line for a WP4 root plan: `build` plus the
/// union roots, both collecting aspects, and both output groups (plus
/// `--target_pattern_file` when the plan carries a pattern file).
pub fn build_argv_for_plan(plan: &dx_roots::RepositoryRootPlan) -> Vec<String> {
    let request = plan_request_for_root_plan(plan);
    let mut argv = vec!["build".to_owned()];
    argv.extend(request.roots);
    for aspect in &request.aspects {
        argv.push(format!("--aspects={aspect}"));
    }
    for group in &request.output_groups {
        argv.push(format!("--output_groups={group}"));
    }
    if let Some(pattern_arg) = plan.pattern_file_arg() {
        argv.push(pattern_arg);
    }
    argv
}

/// Output groups requested together in the single setup request: the
/// codegen plan group plus the env plan group, collected from one BEP
/// stream. No separate env/codegen Bazel commands or output bases are
/// used.
pub fn request_output_groups() -> Vec<String> {
    vec![CODEGEN_OUTPUT_GROUP.to_owned(), ENV_OUTPUT_GROUP.to_owned()]
}

/// The planned single Bazel request behind `dx setup`: the union of
/// required roots with both collecting aspects and both output groups.
/// Later slices execute this request, split the one BEP stream by output
/// group into the codegen and env collectors, and commit both sides (or
/// neither) through the atomic current-pointer replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupRequest {
    pub roots: Vec<String>,
    pub aspects: Vec<String>,
    pub output_groups: Vec<String>,
}

/// Plans the single combined Bazel request for `scope`. The repository
/// arm composes the WP4 (O34) [`dx_roots::repository_plan`] (still the
/// `//...` baseline) behind both canonical selections; exact scopes
/// bypass root selection.
pub fn plan_request(scope: &SetupScope) -> SetupRequest {
    match scope {
        SetupScope::Repository => plan_request_for_root_plan(&dx_roots::repository_plan()),
        SetupScope::Exact(label) => SetupRequest {
            roots: vec![label.clone()],
            aspects: request_aspects(),
            output_groups: request_output_groups(),
        },
    }
}

/// One immutable generation reference: the 64-character lowercase
/// hexadecimal BLAKE3-256 digest of its versioned identity, per
/// `docs/environments/managed-state.md`. The versioned Protobuf identity
/// encoding behind each digest freezes with its implementing milestone;
/// this layer only carries the digest shape, never the encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationId(String);

/// Generation reference failure: anything that is not a 64-character
/// lowercase hexadecimal digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationIdError(String);

impl std::fmt::Display for GenerationIdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid generation id {:?}: want 64 lowercase hex chars",
            self.0
        )
    }
}

impl std::error::Error for GenerationIdError {}

impl GenerationId {
    /// Carries one generation digest, validating its shape.
    pub fn new(id: &str) -> Result<Self, GenerationIdError> {
        let valid = id.len() == 64
            && id
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase());
        if valid {
            Ok(GenerationId(id.to_owned()))
        } else {
            Err(GenerationIdError(id.to_owned()))
        }
    }

    /// Renders the digest.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One complete setup selection: exactly one environment generation plus
/// exactly one generated-code generation, committed together through the
/// single atomic `.dx/setups/current` replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupPair {
    pub environment: GenerationId,
    pub generated: GenerationId,
}

/// Pair resolution failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    /// The selected scope prepared neither side: an exact target with no
    /// environment or codegen capability selects nothing, and committing
    /// an empty pair or re-committing the current pair would hide the
    /// usage error.
    NoCapability,
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ResolveError {}

/// Inputs to [`resolve_pair`]: the freshly prepared sides (each `None`
/// when the scope carries no such capability), the currently selected
/// pair (re-read under the commit lock; `None` on first selection), and
/// the managed empty generations pairing a first selection on one side
/// with a real immutable identity on the other.
pub struct PairInputs {
    pub prepared_environment: Option<GenerationId>,
    pub prepared_generated: Option<GenerationId>,
    pub current: Option<SetupPair>,
    pub empty_environment: GenerationId,
    pub empty_generated: GenerationId,
}

/// Resolves the complete pair to commit from prepared sides with
/// carry-forward, per `docs/environments/managed-state.md#selection-and-carry-forward`:
/// `dx env` pairs its new environment with the previously selected
/// generated generation, `dx codegen` pairs its new projection with the
/// previously selected environment, and `dx setup` pairs both prepared
/// sides (carrying the current side forward for each capability absent
/// from an exact target). A missing prior side uses its managed empty
/// generation. The commit layer commits the returned pair atomically
/// (both or neither); independent commits re-read the current pair under
/// the commit lock before calling this so a concurrently completed side
/// is never lost. Crash and interruption safety (prior pointer
/// preservation) belongs to the commit layer, not this pure function.
pub fn resolve_pair(inputs: PairInputs) -> Result<SetupPair, ResolveError> {
    let PairInputs {
        prepared_environment,
        prepared_generated,
        current,
        empty_environment,
        empty_generated,
    } = inputs;
    match (prepared_environment, prepared_generated) {
        (None, None) => Err(ResolveError::NoCapability),
        (environment, generated) => {
            let (current_environment, current_generated) = match current {
                Some(pair) => (Some(pair.environment), Some(pair.generated)),
                None => (None, None),
            };
            Ok(SetupPair {
                environment: environment
                    .or(current_environment)
                    .unwrap_or(empty_environment),
                generated: generated.or(current_generated).unwrap_or(empty_generated),
            })
        }
    }
}

/// Directory holding every immutable setup record under `.dx`.
pub const SETUPS_DIR_NAME: &str = "setups";

/// Name of the sole mutable selection pointer inside the setups directory.
/// It is a symlink to the selected setup record directory name.
pub const CURRENT_LINK_NAME: &str = "current";

/// Temporary current-pointer name staged beside the live pointer so the
/// publish rename stays on one directory (one filesystem) and atomic.
pub const CURRENT_STAGE_NAME: &str = "current.next";

/// Sibling directory holding immutable environment generations. The setup
/// record links dangle until generations materialize; dangling has the
/// host's ordinary missing-target behavior, never auto-repair.
pub const ENVIRONMENTS_DIR_NAME: &str = "environments";

/// Sibling directory holding immutable generated-code generations.
pub const GENERATED_DIR_NAME: &str = "generated";

/// Link name inside a setup record pointing at its environment generation.
pub const ENVIRONMENT_LINK_NAME: &str = "environment";

/// Link name inside a setup record pointing at its generated generation.
pub const GENERATED_LINK_NAME: &str = "generated";

/// How long a setup commit contends for the workspace commit lock before
/// failing with a busy diagnostic. Mirrors `dx_env::LOCK_TIMEOUT` (O36
/// ten-second deadline); pinned equal by test, never drifted silently.
pub const COMMIT_LOCK_TIMEOUT: Duration = Duration::from_secs(10);

/// Commit outcome for operator messaging. Mirrors the `dx_env` refresh
/// outcome vocabulary over the setup pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitOutcome {
    /// The current pointer already selects this pair; nothing changed.
    AlreadyCurrent,
    /// No setup was selected before; the pair is now selected.
    InstalledFresh,
    /// A different setup was selected before; it is now replaced.
    InstalledReplacement,
}

/// Setup commit failure. Every variant is operational; usage errors
/// (scope selection) live in [`ScopeError`] and never surface here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitError {
    /// Workspace root is missing or not a directory.
    WorkspaceRoot { path: PathBuf },
    /// Another command holds the commit lock past the deadline.
    Busy { path: PathBuf },
    /// The commit lock cannot be opened or locked.
    LockFailed { path: PathBuf, reason: String },
    /// `.dx/setups/current` or its record links are present but malformed.
    /// Never adopted, never repaired: the operator removes the offending
    /// path or re-runs setup from a clean selection.
    CurrentInvalid { reason: String },
    /// A record already exists at the expected setup hash but its links do
    /// not match the pair byte-for-byte. The commit fails without touching
    /// the current pointer (digest-spoof refusal).
    RecordMismatch { reason: String },
    /// A workspace mutation failed. The current pointer is either untouched
    /// (pre-swap failure) or fully swapped (the swap itself is one atomic
    /// rename).
    Install { reason: String },
    /// The selected scope prepared neither side; committing an empty or
    /// recycled pair would hide the usage error. Mirrors [`ResolveError`].
    NoCapability,
}

impl std::fmt::Display for CommitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for CommitError {}

/// Freshly prepared sides for an atomic
/// read-resolve-commit under one lock hold.
pub struct PreparedSides {
    pub prepared_environment: Option<GenerationId>,
    pub prepared_generated: Option<GenerationId>,
    pub empty_environment: GenerationId,
    pub empty_generated: GenerationId,
}

/// Provisional versioned setup fingerprint over the pair. The frozen
/// versioned binary Protobuf setup identity (containing both generation
/// identities) lands with its implementing slice; until then this
/// `dx-setup/v0` prefix keeps the pre-image versioned and deterministic so
/// the hash-addressed record layout, idempotency checks, and atomic swap
/// all exercise their real paths. No ad-hoc unversioned digest input.
pub fn setup_fingerprint(pair: &SetupPair) -> String {
    format!(
        "dx-setup/v0\n{}\n{}\n",
        pair.environment.as_str(),
        pair.generated.as_str()
    )
}

/// BLAKE3-256 over the provisional fingerprint: the setup record identity.
/// Routed through the shared result crate so the digest algorithm has one
/// owner.
pub fn setup_digest(pair: &SetupPair) -> [u8; 32] {
    quality_result::digest(setup_fingerprint(pair).as_bytes())
}

/// Lowercase hex of the setup digest: the setup record directory name.
pub fn setup_hex(pair: &SetupPair) -> String {
    setup_digest(pair)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Expected relative link text from a setup record to its environment
/// generation. Relative (never absolute) so the workspace stays
/// relocatable; the layout's trailing slash denotes a directory, while the
/// frozen link text carries no trailing slash.
fn expected_environment_target(pair: &SetupPair) -> String {
    format!(
        "../../{}/{}/",
        ENVIRONMENTS_DIR_NAME,
        pair.environment.as_str()
    )
}

/// Expected relative link text from a setup record to its generated
/// generation.
fn expected_generated_target(pair: &SetupPair) -> String {
    format!("../../{}/{}/", GENERATED_DIR_NAME, pair.generated.as_str())
}

/// Platform directory-symlink primitive for record and pointer links.
#[cfg(windows)]
fn symlink_dir(target: &Path, link: &Path) -> io::Result<()> {
    std::os::windows::fs::symlink_dir(target, link)
}

/// Platform directory-symlink primitive for record and pointer links.
#[cfg(not(windows))]
fn symlink_dir(target: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

/// Maps the shared commit-lock failure into the setup commit vocabulary.
/// Only contention reports busy; every other lock failure aborts
/// immediately so platform errors are never misreported.
fn map_lock_error(error: dx_env::Error) -> CommitError {
    match error {
        dx_env::Error::Busy { path } => CommitError::Busy { path },
        dx_env::Error::LockFailed { path, reason } => CommitError::LockFailed { path, reason },
        other => CommitError::LockFailed {
            path: PathBuf::from(".dx"),
            reason: other.to_string(),
        },
    }
}

/// Acquires the shared workspace commit lock. This is the O36 route owned
/// by `dx_env::acquire_lock` (dedicated lock file, `File::try_lock`,
/// contention-only retry until the deadline): setup introduces no new lock
/// file, mechanism, or deadline.
fn acquire_commit_lock(dx_dir: &Path, timeout: Duration) -> Result<std::fs::File, CommitError> {
    dx_env::acquire_lock(dx_dir, timeout).map_err(map_lock_error)
}

/// Extracts a generation digest from a record link target: the final path
/// component must be a valid [`GenerationId`].
fn generation_from_link_target(target: &Path) -> Option<GenerationId> {
    target
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|text| GenerationId::new(text).ok())
}

/// Reads the currently selected pair without locking (diagnostics and
/// tests). The authoritative read inside a commit happens under the commit
/// lock via the same helper. Absent pointer selects nothing (`Ok(None)`);
/// any present-but-malformed state fails closed.
pub fn read_current_pair(workspace_root: &Path) -> Result<Option<SetupPair>, CommitError> {
    if !workspace_root.is_dir() {
        return Err(CommitError::WorkspaceRoot {
            path: workspace_root.to_path_buf(),
        });
    }
    let current = workspace_root
        .join(".dx")
        .join(SETUPS_DIR_NAME)
        .join(CURRENT_LINK_NAME);
    let meta = match fs::symlink_metadata(&current) {
        Ok(meta) => meta,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(CommitError::CurrentInvalid {
                reason: format!("cannot inspect {}: {e}", current.display()),
            });
        }
    };
    if !meta.file_type().is_symlink() {
        return Err(CommitError::CurrentInvalid {
            reason: format!(
                "{} is not a symlink; refusing to adopt foreign state",
                current.display()
            ),
        });
    }
    let pointer_target = fs::read_link(&current).map_err(|e| CommitError::CurrentInvalid {
        reason: format!("cannot read {}: {e}", current.display()),
    })?;
    let environment = fs::read_link(current.join(ENVIRONMENT_LINK_NAME))
        .ok()
        .and_then(|target| generation_from_link_target(&target));
    let generated = fs::read_link(current.join(GENERATED_LINK_NAME))
        .ok()
        .and_then(|target| generation_from_link_target(&target));
    let (environment, generated) = match (environment, generated) {
        (Some(environment), Some(generated)) => (environment, generated),
        _ => {
            return Err(CommitError::CurrentInvalid {
                reason: format!(
                    "{} does not resolve to a complete setup record",
                    current.display()
                ),
            });
        }
    };
    let pair = SetupPair {
        environment,
        generated,
    };
    let want = setup_hex(&pair);
    let pointed = pointer_target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if pointed != want {
        return Err(CommitError::CurrentInvalid {
            reason: format!(
                "{} points at {pointed:?}, want setup {want:?}; refusing digest-spoofed state",
                current.display()
            ),
        });
    }
    Ok(Some(pair))
}

/// Ensures the hash-addressed setup record exists and matches the pair
/// byte-for-byte. Existing records validate structurally; mismatches fail
/// without replacement. Returns the record path.
fn ensure_setup_record(setups_dir: &Path, pair: &SetupPair) -> Result<PathBuf, CommitError> {
    let record = setups_dir.join(setup_hex(pair));
    if record.exists() {
        let environment = fs::read_link(record.join(ENVIRONMENT_LINK_NAME)).map_err(|e| {
            CommitError::RecordMismatch {
                reason: format!("existing record {} is malformed: {e}", record.display()),
            }
        })?;
        let generated = fs::read_link(record.join(GENERATED_LINK_NAME)).map_err(|e| {
            CommitError::RecordMismatch {
                reason: format!("existing record {} is malformed: {e}", record.display()),
            }
        })?;
        if environment.as_os_str().to_string_lossy().as_ref() != expected_environment_target(pair)
            || generated.as_os_str().to_string_lossy().as_ref() != expected_generated_target(pair)
        {
            return Err(CommitError::RecordMismatch {
                reason: format!(
                    "existing record {} does not match the committing pair; refusing to replace it",
                    record.display()
                ),
            });
        }
        return Ok(record);
    }
    fs::create_dir_all(&record).map_err(|e| CommitError::Install {
        reason: format!("cannot create {}: {e}", record.display()),
    })?;
    let created = |link: &str, target: &str| {
        symlink_dir(Path::new(target), &record.join(link)).map_err(|e| CommitError::Install {
            reason: format!("cannot stage setup link '{link}': {e}"),
        })
    };
    if let Err(error) = created(ENVIRONMENT_LINK_NAME, &expected_environment_target(pair))
        .and_then(|()| created(GENERATED_LINK_NAME, &expected_generated_target(pair)))
    {
        let _ = fs::remove_dir_all(&record);
        return Err(error);
    }
    Ok(record)
}

/// Removes a stale staged pointer left by an interrupted swap.
/// Best-effort only when the path is a symlink or file; a surviving
/// directory reports through the install error so it is never silently
/// adopted.
fn clear_staged_pointer(stage: &Path) -> Result<(), CommitError> {
    match fs::symlink_metadata(stage) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(CommitError::Install {
            reason: format!("cannot inspect stale {}: {e}", stage.display()),
        }),
        Ok(meta) => {
            if meta.file_type().is_dir() && !meta.file_type().is_symlink() {
                Err(CommitError::Install {
                    reason: format!(
                        "stale {} is a directory; refusing to adopt foreign state",
                        stage.display()
                    ),
                })
            } else {
                fs::remove_file(stage).map_err(|e| CommitError::Install {
                    reason: format!("cannot clear stale {}: {e}", stage.display()),
                })
            }
        }
    }
}

/// Installs the record and atomically swaps the current pointer. The
/// caller holds the commit lock; the swap itself is one same-directory
/// rename, so a crash lands on the prior pointer plus a reclaimable staged
/// link. Records install idempotently; mismatches fail with the prior
/// pointer preserved.
fn install_and_swap(
    setups_dir: &Path,
    prior: Option<SetupPair>,
    pair: &SetupPair,
) -> Result<CommitOutcome, CommitError> {
    fs::create_dir_all(setups_dir).map_err(|e| CommitError::Install {
        reason: format!("cannot create {}: {e}", setups_dir.display()),
    })?;
    let current = setups_dir.join(CURRENT_LINK_NAME);
    let stage = setups_dir.join(CURRENT_STAGE_NAME);
    clear_staged_pointer(&stage)?;
    let record_name = setup_hex(pair);
    ensure_setup_record(setups_dir, pair)?;
    if prior.as_ref() == Some(pair) {
        return Ok(CommitOutcome::AlreadyCurrent);
    }
    let fresh = prior.is_none();
    symlink_dir(Path::new(&record_name), &stage).map_err(|e| CommitError::Install {
        reason: format!("cannot stage {}: {e}", stage.display()),
    })?;
    fs::rename(&stage, &current).map_err(|e| CommitError::Install {
        reason: format!(
            "cannot publish {}: {e}; the prior pointer is preserved",
            current.display()
        ),
    })?;
    if fresh {
        Ok(CommitOutcome::InstalledFresh)
    } else {
        Ok(CommitOutcome::InstalledReplacement)
    }
}

/// Commits an already-resolved pair through the lock and atomic pointer
/// replacement, re-reading the current selection under the lock so the
/// no-op check never races a concurrent commit.
pub fn commit_pair(workspace_root: &Path, pair: &SetupPair) -> Result<CommitOutcome, CommitError> {
    commit_pair_with_timeout(workspace_root, pair, COMMIT_LOCK_TIMEOUT)
}

/// [`commit_pair`] with an injectable lock deadline (tests only).
pub fn commit_pair_with_timeout(
    workspace_root: &Path,
    pair: &SetupPair,
    timeout: Duration,
) -> Result<CommitOutcome, CommitError> {
    if !workspace_root.is_dir() {
        return Err(CommitError::WorkspaceRoot {
            path: workspace_root.to_path_buf(),
        });
    }
    let dx_dir = workspace_root.join(".dx");
    fs::create_dir_all(&dx_dir).map_err(|e| CommitError::Install {
        reason: format!("cannot create {}: {e}", dx_dir.display()),
    })?;
    let _lock = acquire_commit_lock(&dx_dir, timeout)?;
    let setups_dir = dx_dir.join(SETUPS_DIR_NAME);
    let prior = read_current_pair(workspace_root)?;
    install_and_swap(&setups_dir, prior, pair)
}

/// Atomically resolves prepared sides against the newest current pair and
/// commits the result: one lock hold across the re-read, [`resolve_pair`],
/// idempotent record installation, and atomic pointer swap. Independent
/// env/codegen commits use this so a concurrently completed opposite side
/// is carried forward instead of lost. Both-or-neither: failures before
/// the swap leave the prior pointer unchanged.
pub fn commit_prepared(
    workspace_root: &Path,
    sides: PreparedSides,
) -> Result<(SetupPair, CommitOutcome), CommitError> {
    commit_prepared_with_timeout(workspace_root, sides, COMMIT_LOCK_TIMEOUT)
}

/// [`commit_prepared`] with an injectable lock deadline (tests only).
pub fn commit_prepared_with_timeout(
    workspace_root: &Path,
    sides: PreparedSides,
    timeout: Duration,
) -> Result<(SetupPair, CommitOutcome), CommitError> {
    if !workspace_root.is_dir() {
        return Err(CommitError::WorkspaceRoot {
            path: workspace_root.to_path_buf(),
        });
    }
    let dx_dir = workspace_root.join(".dx");
    fs::create_dir_all(&dx_dir).map_err(|e| CommitError::Install {
        reason: format!("cannot create {}: {e}", dx_dir.display()),
    })?;
    let _lock = acquire_commit_lock(&dx_dir, timeout)?;
    let current = read_current_pair(workspace_root)?;
    let PreparedSides {
        prepared_environment,
        prepared_generated,
        empty_environment,
        empty_generated,
    } = sides;
    let pair = resolve_pair(PairInputs {
        prepared_environment,
        prepared_generated,
        current,
        empty_environment,
        empty_generated,
    })
    .map_err(|_| CommitError::NoCapability)?;
    let setups_dir = dx_dir.join(SETUPS_DIR_NAME);
    let prior = read_current_pair(workspace_root)?;
    let outcome = install_and_swap(&setups_dir, prior, &pair)?;
    Ok((pair, outcome))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_identities_match_codegen_and_env() {
        assert_eq!(
            CODEGEN_ASPECT,
            "//generation:codegen.bzl%dx_codegen_plan_aspect"
        );
        assert_eq!(ENV_ASPECT, "//env:plan.bzl%dx_env_plan_aspect");
        assert_eq!(CODEGEN_OUTPUT_GROUP, "dx_codegen_plans");
        assert_eq!(ENV_OUTPUT_GROUP, "dx_env_plans");
        assert_eq!(CODEGEN_REPOSITORY_TARGET, "//dx:codegen");
        assert_eq!(ENV_REPOSITORY_TARGET, "//dx:env");
    }

    #[test]
    fn empty_scope_selects_both_repository_targets() {
        assert_eq!(resolve_scope(&[]), Ok(SetupScope::Repository));
        assert_eq!(
            scope_targets(&SetupScope::Repository),
            vec!["//dx:codegen", "//dx:env"]
        );
    }

    #[test]
    fn root_plan_composes_baseline_combined_request() {
        let plan = dx_roots::repository_plan();
        assert_eq!(
            targets_for_root_plan(&plan),
            vec!["//dx:codegen", "//dx:env"]
        );
        let request = plan_request_for_root_plan(&plan);
        assert_eq!(request, plan_request(&SetupScope::Repository));
        assert_eq!(
            build_argv_for_plan(&plan),
            vec![
                "build".to_owned(),
                "//dx:codegen".to_owned(),
                "//dx:env".to_owned(),
                format!("--aspects={CODEGEN_ASPECT}"),
                format!("--aspects={ENV_ASPECT}"),
                format!("--output_groups={CODEGEN_OUTPUT_GROUP}"),
                format!("--output_groups={ENV_OUTPUT_GROUP}"),
            ]
        );
    }

    #[test]
    fn root_plan_passes_aggregate_roots_through() {
        let plan = dx_roots::RepositoryRootPlan::monolithic_aggregate("//dx:setup_roots");
        assert_eq!(
            targets_for_root_plan(&plan),
            vec!["//dx:setup_roots".to_owned()]
        );
        assert_eq!(
            plan_request_for_root_plan(&plan).roots,
            vec!["//dx:setup_roots".to_owned()]
        );
    }

    #[test]
    fn exact_labels_pass_through() {
        for label in ["//app:server", "@rules_dx//generation:result_proto_rs"] {
            let scope = resolve_scope(&[label.to_owned()]).expect("exact label");
            assert_eq!(scope, SetupScope::Exact(label.to_owned()));
            assert_eq!(scope_targets(&scope), vec![label.to_owned()]);
        }
    }

    #[test]
    fn scope_rejects_non_exact_inputs() {
        assert_eq!(
            resolve_scope(&["//a:x".to_owned(), "//b:y".to_owned()]),
            Err(ScopeError::MultipleTargets { count: 2 })
        );
        for pattern in ["//...", "//app/...", "//app:*", "@repo//pkg:all?"] {
            assert_eq!(
                resolve_scope(&[pattern.to_owned()]),
                Err(ScopeError::TargetPattern {
                    value: pattern.to_owned(),
                }),
                "pattern {pattern:?} must fail"
            );
        }
        for other in ["app/server", "gen", "rust", "--profile=fast", ":relative"] {
            assert_eq!(
                resolve_scope(&[other.to_owned()]),
                Err(ScopeError::NotTargetLabel {
                    value: other.to_owned(),
                }),
                "non-label {other:?} must fail"
            );
        }
    }

    #[test]
    fn scope_errors_display() {
        assert!(ScopeError::MultipleTargets { count: 2 }
            .to_string()
            .contains("MultipleTargets"));
    }

    #[test]
    fn repository_request_unions_both_sides() {
        let request = plan_request(&SetupScope::Repository);
        assert_eq!(request.roots, vec!["//dx:codegen", "//dx:env"]);
        assert_eq!(
            request.aspects,
            vec![
                "//generation:codegen.bzl%dx_codegen_plan_aspect",
                "//env:plan.bzl%dx_env_plan_aspect",
            ]
        );
        assert_eq!(
            request.output_groups,
            vec!["dx_codegen_plans", "dx_env_plans"]
        );
    }

    #[test]
    fn exact_request_applies_both_sides_to_one_root() {
        let request = plan_request(&SetupScope::Exact("//app:server".to_owned()));
        assert_eq!(request.roots, vec!["//app:server"]);
        assert_eq!(request.aspects.len(), 2);
        assert_eq!(request.output_groups.len(), 2);
    }

    #[test]
    fn request_sides_are_deterministic() {
        let first = plan_request(&SetupScope::Repository);
        let second = plan_request(&SetupScope::Repository);
        assert_eq!(first, second);
    }

    fn generation(tag: char) -> GenerationId {
        GenerationId::new(&tag.to_string().repeat(64)).expect("fixture digest")
    }

    fn pair_inputs(
        prepared_environment: Option<GenerationId>,
        prepared_generated: Option<GenerationId>,
        current: Option<SetupPair>,
    ) -> PairInputs {
        PairInputs {
            prepared_environment,
            prepared_generated,
            current,
            empty_environment: generation('e'),
            empty_generated: generation('0'),
        }
    }

    #[test]
    fn generation_ids_validate_digest_shape() {
        assert_eq!(generation('a').as_str(), &"a".repeat(64));
        assert!(GenerationId::new(&"0".repeat(64)).is_ok());
        for bad in [
            String::new(),
            "a".repeat(63),
            "a".repeat(65),
            "A".repeat(64),
            "g".repeat(64),
            format!("{}!", "a".repeat(63)),
        ] {
            assert!(GenerationId::new(&bad).is_err(), "digest {bad:?} must fail");
        }
        assert!(GenerationIdError("x".to_owned())
            .to_string()
            .contains("generation id"));
    }

    #[test]
    fn setup_with_both_sides_ignores_current() {
        let pair = resolve_pair(pair_inputs(
            Some(generation('1')),
            Some(generation('2')),
            Some(SetupPair {
                environment: generation('3'),
                generated: generation('4'),
            }),
        ))
        .expect("pair");
        assert_eq!(pair.environment, generation('1'));
        assert_eq!(pair.generated, generation('2'));
    }

    #[test]
    fn independent_sides_carry_the_current_opposite_forward() {
        let current = || SetupPair {
            environment: generation('3'),
            generated: generation('4'),
        };
        let env_pair = resolve_pair(pair_inputs(Some(generation('1')), None, Some(current())))
            .expect("env pair");
        assert_eq!(env_pair.environment, generation('1'));
        assert_eq!(env_pair.generated, generation('4'));
        let codegen_pair = resolve_pair(pair_inputs(None, Some(generation('2')), Some(current())))
            .expect("codegen pair");
        assert_eq!(codegen_pair.environment, generation('3'));
        assert_eq!(codegen_pair.generated, generation('2'));
    }

    #[test]
    fn first_selection_pairs_with_managed_empty_generations() {
        let env_pair =
            resolve_pair(pair_inputs(Some(generation('1')), None, None)).expect("env pair");
        assert_eq!(env_pair.environment, generation('1'));
        assert_eq!(env_pair.generated, generation('0'));
        let codegen_pair =
            resolve_pair(pair_inputs(None, Some(generation('2')), None)).expect("codegen pair");
        assert_eq!(codegen_pair.environment, generation('e'));
        assert_eq!(codegen_pair.generated, generation('2'));
        let setup_pair = resolve_pair(pair_inputs(
            Some(generation('1')),
            Some(generation('2')),
            None,
        ))
        .expect("setup pair");
        assert_eq!(setup_pair.environment, generation('1'));
        assert_eq!(setup_pair.generated, generation('2'));
    }

    #[test]
    fn scope_without_either_capability_fails() {
        assert_eq!(
            resolve_pair(pair_inputs(None, None, None)),
            Err(ResolveError::NoCapability)
        );
        assert_eq!(
            resolve_pair(pair_inputs(
                None,
                None,
                Some(SetupPair {
                    environment: generation('3'),
                    generated: generation('4'),
                }),
            )),
            Err(ResolveError::NoCapability)
        );
        assert!(ResolveError::NoCapability
            .to_string()
            .contains("NoCapability"));
    }

    fn pair(env: char, gen: char) -> SetupPair {
        SetupPair {
            environment: generation(env),
            generated: generation(gen),
        }
    }

    fn commit_root(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dx-setup-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("ws")).expect("create workspace");
        dir
    }

    fn workspace_of(root: &Path) -> PathBuf {
        root.join("ws")
    }

    fn commit_ok(workspace: &Path, pair: &SetupPair) -> CommitOutcome {
        commit_pair(workspace, pair).expect("commit succeeds")
    }

    #[test]
    fn commit_lock_deadline_matches_env_owner() {
        assert_eq!(COMMIT_LOCK_TIMEOUT, dx_env::LOCK_TIMEOUT);
    }

    #[test]
    fn setup_hex_is_deterministic_lowercase_64() {
        let first = setup_hex(&pair('1', '2'));
        let second = setup_hex(&pair('1', '2'));
        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
        assert!(first.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(first, first.to_lowercase());
        assert_ne!(first, setup_hex(&pair('1', '3')));
        assert_ne!(first, setup_hex(&pair('3', '2')));
        assert!(setup_fingerprint(&pair('1', '2')).starts_with("dx-setup/v0\n"));
    }

    #[test]
    fn fresh_install_noop_and_replacement() {
        let root = commit_root("lifecycle");
        let workspace = workspace_of(&root);
        assert_eq!(read_current_pair(&workspace).expect("read"), None);
        assert_eq!(
            commit_ok(&workspace, &pair('1', '2')),
            CommitOutcome::InstalledFresh
        );
        let selected = read_current_pair(&workspace)
            .expect("read")
            .expect("selected");
        assert_eq!(selected, pair('1', '2'));
        assert_eq!(
            commit_ok(&workspace, &pair('1', '2')),
            CommitOutcome::AlreadyCurrent
        );
        assert_eq!(
            commit_ok(&workspace, &pair('3', '4')),
            CommitOutcome::InstalledReplacement
        );
        assert_eq!(
            read_current_pair(&workspace).expect("read"),
            Some(pair('3', '4'))
        );
        // The record links are relative, and the pointer names the setup hash.
        let setups = workspace.join(".dx").join("setups");
        let record = setups.join(setup_hex(&pair('3', '4')));
        assert!(record.join("environment").is_symlink());
        assert!(record.join("generated").is_symlink());
        assert_eq!(
            fs::read_link(setups.join("current")).expect("pointer"),
            PathBuf::from(setup_hex(&pair('3', '4')))
        );
        assert!(!setups.join("current.next").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn workspace_path_with_spaces_commits() {
        let root = commit_root("with space");
        let workspace = workspace_of(&root);
        assert_eq!(
            commit_ok(&workspace, &pair('a', 'b')),
            CommitOutcome::InstalledFresh
        );
        assert_eq!(
            read_current_pair(&workspace).expect("read"),
            Some(pair('a', 'b'))
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn prepared_commits_carry_forward_under_one_lock() {
        let root = commit_root("carry-commit");
        let workspace = workspace_of(&root);
        let sides = |env: Option<char>, gen: Option<char>| PreparedSides {
            prepared_environment: env.map(generation),
            prepared_generated: gen.map(generation),
            empty_environment: generation('e'),
            empty_generated: generation('0'),
        };
        let (first, outcome) = commit_prepared(&workspace, sides(Some('1'), None)).expect("env");
        assert_eq!(outcome, CommitOutcome::InstalledFresh);
        assert_eq!(first, pair('1', '0'));
        let (second, outcome) = commit_prepared(&workspace, sides(None, Some('2'))).expect("gen");
        assert_eq!(outcome, CommitOutcome::InstalledReplacement);
        assert_eq!(second, pair('1', '2'));
        assert_eq!(
            read_current_pair(&workspace).expect("read"),
            Some(pair('1', '2'))
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn prepared_without_capability_fails_before_mutation() {
        let root = commit_root("no-capability");
        let workspace = workspace_of(&root);
        let error = commit_prepared(
            &workspace,
            PreparedSides {
                prepared_environment: None,
                prepared_generated: None,
                empty_environment: generation('e'),
                empty_generated: generation('0'),
            },
        )
        .unwrap_err();
        assert_eq!(error, CommitError::NoCapability);
        assert_eq!(read_current_pair(&workspace).expect("read"), None);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn record_mismatch_preserves_current() {
        let root = commit_root("mismatch");
        let workspace = workspace_of(&root);
        assert_eq!(
            commit_ok(&workspace, &pair('1', '2')),
            CommitOutcome::InstalledFresh
        );
        // Corrupt the record for a different pair, then try to commit it:
        // the commit must fail without moving the pointer.
        let spoofed = pair('3', '4');
        let setups = workspace.join(".dx").join("setups");
        let record = setups.join(setup_hex(&spoofed));
        fs::create_dir_all(&record).expect("spoof record");
        symlink_dir(
            Path::new("../../environments/wrong"),
            &record.join("environment"),
        )
        .expect("wrong link");
        symlink_dir(
            Path::new(&expected_generated_target(&spoofed)),
            &record.join("generated"),
        )
        .expect("right link");
        let error = commit_pair(&workspace, &spoofed).unwrap_err();
        assert!(matches!(error, CommitError::RecordMismatch { .. }));
        assert_eq!(
            read_current_pair(&workspace).expect("read"),
            Some(pair('1', '2'))
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn unmanaged_current_states_fail_closed() {
        let root = commit_root("unmanaged");
        let workspace = workspace_of(&root);
        assert_eq!(
            commit_ok(&workspace, &pair('1', '2')),
            CommitOutcome::InstalledFresh
        );
        let setups = workspace.join(".dx").join("setups");
        let current = setups.join("current");
        let pointed = fs::read_link(&current).expect("pointer");

        fs::remove_file(&current).expect("remove pointer");
        fs::write(&current, "not a symlink").expect("file pointer");
        assert!(matches!(
            read_current_pair(&workspace),
            Err(CommitError::CurrentInvalid { .. })
        ));
        assert!(matches!(
            commit_pair(&workspace, &pair('3', '4')),
            Err(CommitError::CurrentInvalid { .. })
        ));
        fs::remove_file(&current).expect("remove file pointer");

        fs::create_dir(&current).expect("dir pointer");
        assert!(matches!(
            read_current_pair(&workspace),
            Err(CommitError::CurrentInvalid { .. })
        ));
        fs::remove_dir(&current).expect("remove dir pointer");

        symlink_dir(Path::new("missing-setup"), &current).expect("dangling pointer");
        assert!(matches!(
            read_current_pair(&workspace),
            Err(CommitError::CurrentInvalid { .. })
        ));
        fs::remove_file(&current).expect("remove dangling");
        symlink_dir(Path::new(&pointed), &current).expect("restore pointer");
        assert_eq!(
            read_current_pair(&workspace).expect("read"),
            Some(pair('1', '2'))
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn digest_spoofed_pointer_fails_closed() {
        let root = commit_root("spoof");
        let workspace = workspace_of(&root);
        assert_eq!(
            commit_ok(&workspace, &pair('1', '2')),
            CommitOutcome::InstalledFresh
        );
        let setups = workspace.join(".dx").join("setups");
        let current = setups.join("current");
        // Point at a valid record directory name that does not match the
        // pair the record links resolve to.
        let other = setup_hex(&pair('3', '4'));
        fs::create_dir_all(setups.join(&other)).expect("other record");
        symlink_dir(
            Path::new(&expected_environment_target(&pair('1', '2'))),
            &setups.join(&other).join("environment"),
        )
        .expect("env link");
        symlink_dir(
            Path::new(&expected_generated_target(&pair('1', '2'))),
            &setups.join(&other).join("generated"),
        )
        .expect("gen link");
        fs::remove_file(&current).expect("remove pointer");
        symlink_dir(Path::new(&other), &current).expect("spoofed pointer");
        assert!(matches!(
            read_current_pair(&workspace),
            Err(CommitError::CurrentInvalid { .. })
        ));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn stale_staged_pointer_is_reclaimed() {
        let root = commit_root("stale-next");
        let workspace = workspace_of(&root);
        let setups = workspace.join(".dx").join("setups");
        fs::create_dir_all(&setups).expect("setups");
        symlink_dir(Path::new("stale-target"), &setups.join("current.next")).expect("stale");
        assert_eq!(
            commit_ok(&workspace, &pair('1', '2')),
            CommitOutcome::InstalledFresh
        );
        assert!(!setups.join("current.next").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn workspace_missing_fails() {
        let root = commit_root("ws-missing");
        let missing = root.join("no-such-dir");
        assert!(matches!(
            read_current_pair(&missing),
            Err(CommitError::WorkspaceRoot { .. })
        ));
        assert!(matches!(
            commit_pair(&missing, &pair('1', '2')),
            Err(CommitError::WorkspaceRoot { .. })
        ));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn busy_lock_fails_after_deadline() {
        let root = commit_root("busy");
        let workspace = workspace_of(&root);
        let dx_dir = workspace.join(".dx");
        fs::create_dir_all(&dx_dir).expect("dx dir");
        let _held =
            dx_env::acquire_lock(&dx_dir, Duration::from_secs(10)).expect("hold commit lock");
        let error = commit_pair_with_timeout(&workspace, &pair('1', '2'), Duration::from_millis(1))
            .unwrap_err();
        assert!(matches!(error, CommitError::Busy { .. }));
        drop(_held);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn lock_open_failure_aborts() {
        let root = commit_root("lock-open");
        let workspace = workspace_of(&root);
        let dx_dir = workspace.join(".dx");
        fs::create_dir_all(&dx_dir).expect("dx dir");
        fs::create_dir_all(dx_dir.join(dx_env::LOCK_FILE_NAME)).expect("lock is a directory");
        assert!(matches!(
            commit_pair_with_timeout(&workspace, &pair('1', '2'), Duration::from_secs(1)),
            Err(CommitError::LockFailed { .. })
        ));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn concurrent_commits_serialize_with_idempotent_reuse() {
        let root = commit_root("concurrent");
        let workspace = workspace_of(&root);
        // Eight racing commits over four distinct pairs: the O36 commit
        // lock must serialize them so every commit succeeds, every record
        // installs, and duplicate pairs reuse the installed record
        // (`AlreadyCurrent` or a same-pair replacement, never a failure
        // or a lost opposite side). The final pointer names one of the
        // four pairs.
        let wanted: Vec<SetupPair> = (0..4)
            .map(|i| pair((b'1' + i) as char, (b'a' + i) as char))
            .collect();
        let outcomes = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..8)
                .map(|i| {
                    let workspace_ref = &workspace;
                    let candidate = wanted[i % wanted.len()].clone();
                    scope.spawn(move || commit_pair(workspace_ref, &candidate))
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().expect("worker panics fail the test"))
                .collect::<Vec<_>>()
        });
        for outcome in &outcomes {
            assert!(outcome.is_ok(), "racing commit must succeed: {outcome:?}");
        }
        let setups = workspace.join(".dx").join("setups");
        for candidate in &wanted {
            let record = setups.join(setup_hex(candidate));
            assert!(record.join("environment").is_symlink());
            assert!(record.join("generated").is_symlink());
        }
        let selected = read_current_pair(&workspace)
            .expect("read")
            .expect("selected");
        assert!(wanted.contains(&selected));
        assert!(!setups.join("current.next").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn staged_directory_preserves_current() {
        let root = commit_root("staged-dir");
        let workspace = workspace_of(&root);
        assert_eq!(
            commit_ok(&workspace, &pair('1', '2')),
            CommitOutcome::InstalledFresh
        );
        // An interrupted swap never leaves a directory at the staged
        // pointer through this code, but a foreign directory there must
        // refuse the commit with the prior pointer preserved, never be
        // adopted or silently replaced.
        let setups = workspace.join(".dx").join("setups");
        fs::create_dir_all(setups.join("current.next")).expect("foreign staged dir");
        assert!(matches!(
            commit_pair(&workspace, &pair('3', '4')),
            Err(CommitError::Install { .. })
        ));
        assert_eq!(
            read_current_pair(&workspace).expect("read"),
            Some(pair('1', '2'))
        );
        fs::remove_dir(setups.join("current.next")).expect("remove foreign staged dir");
        assert_eq!(
            commit_ok(&workspace, &pair('3', '4')),
            CommitOutcome::InstalledReplacement
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn commit_errors_display() {
        let errors = [
            CommitError::WorkspaceRoot {
                path: PathBuf::from("/ws"),
            },
            CommitError::Busy {
                path: PathBuf::from("/ws/.dx/.commit.lock"),
            },
            CommitError::LockFailed {
                path: PathBuf::from("/ws/.dx/.commit.lock"),
                reason: "r".to_string(),
            },
            CommitError::CurrentInvalid {
                reason: "r".to_string(),
            },
            CommitError::RecordMismatch {
                reason: "r".to_string(),
            },
            CommitError::Install {
                reason: "r".to_string(),
            },
            CommitError::NoCapability,
        ];
        for error in &errors {
            assert!(!format!("{error}").is_empty());
        }
    }
}
