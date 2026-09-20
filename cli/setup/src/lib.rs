//! Combined setup request planning for the `dx` CLI.
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
//! `.dx/setups/current` under the commit lock and atomically
//! replacing the current pointer. Staging, validation, and generation
//! materialization land in later WP3 slices; the setup record links may
//! dangle until generations exist (ordinary host missing-target behavior,
//! never auto-repair).

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use dx_digest::blake3 as digest;
use dx_env::{acquire_lock, Error};
use dx_roots::{
    build_argv_union, invocation_targets_union, repository_plan, resolve_exact_target,
    ExactScopeError, RepositoryRootPlan,
};

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
/// behind this label stay provisional pending the WP4 root
/// fiat selection per ADR 0022; this crate only owns the selection identity.
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
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ScopeError {
    /// More than one positional target: setup selects at most one.
    #[error("expected at most one target, found {count}")]
    MultipleTargets { count: usize },
    /// A target pattern (`...`, `*`, `?`): patterns never select setup.
    #[error("invalid target {value:?}: patterns never select setup")]
    TargetPattern { value: String },
    /// Anything that is not an exact target label: paths, directories,
    /// profiles/flags, and language selectors.
    #[error("invalid target {value:?}: want an exact // or @ label")]
    NotTargetLabel { value: String },
}

/// Resolves the `dx setup` positional scope: empty selects the
/// repository (both canonical selections), one exact `//` or `@` label
/// selects its configured closure, and anything else fails before
/// execution. A compatible target whose closure contributes no shards
/// selects empty sides downstream (with carry-forward or managed empty
/// generations), never a failure here.
///
/// Single-source scope validation for `#651`: the `...`/`*`/`?` and
/// `//`/`@` checks live in [`dx_roots::resolve_exact_target`]; this keeps
/// the noun-specific `SetupScope`/`ScopeError` while sharing the logic with
/// `dx_codegen` and `dx_env_plan` (intentional divergence: distinct scope
/// types and the two-target union below).
pub fn resolve_scope(targets: &[String]) -> Result<SetupScope, ScopeError> {
    match resolve_exact_target(targets) {
        Ok(None) => Ok(SetupScope::Repository),
        Ok(Some(label)) => Ok(SetupScope::Exact(label)),
        Err(ExactScopeError::MultipleTargets { count }) => {
            Err(ScopeError::MultipleTargets { count })
        }
        Err(ExactScopeError::TargetPattern { value }) => Err(ScopeError::TargetPattern { value }),
        Err(ExactScopeError::NotTargetLabel { value }) => Err(ScopeError::NotTargetLabel { value }),
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
/// identities (`//dx:codegen` plus `//dx:env`) while the fiat selection stands;
/// every other candidate passes its single union root set through. The
/// query-pattern-file candidate carries no command-line patterns (Bazel
/// reads them from `--target_pattern_file`).
///
/// Single-source union helper for `#651`: shares the baseline/pattern-file
/// policy with [`dx_roots::invocation_targets`] via
/// [`invocation_targets_union`]; the two-target union is the intentional
/// divergence from the single-target codegen/env helpers.
pub fn targets_for_root_plan(plan: &RepositoryRootPlan) -> Vec<String> {
    invocation_targets_union(plan, &[CODEGEN_REPOSITORY_TARGET, ENV_REPOSITORY_TARGET])
}

/// Plans the single combined Bazel request for a WP4 root plan: the
/// plan's union roots with both collecting aspects and both output
/// groups. Codegen and env never invoke each other; each aspect stays
/// bounded by provider applicability.
pub fn plan_request_for_root_plan(plan: &RepositoryRootPlan) -> SetupRequest {
    SetupRequest {
        roots: targets_for_root_plan(plan),
        aspects: request_aspects(),
        output_groups: request_output_groups(),
    }
}

/// Full `bazel build` command line for a WP4 root plan: `build` plus the
/// union roots, both collecting aspects, and both output groups (plus
/// `--target_pattern_file` when the plan carries a pattern file).
///
/// Single-source union argv for `#651`: shares assembly with
/// [`dx_roots::build_argv`] via [`build_argv_union`].
pub fn build_argv_for_plan(plan: &RepositoryRootPlan) -> Vec<String> {
    build_argv_union(
        plan,
        &[CODEGEN_REPOSITORY_TARGET, ENV_REPOSITORY_TARGET],
        &request_aspects(),
        &request_output_groups(),
    )
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
/// arm composes the WP4 [`dx_roots::repository_plan`] (still the
/// `//...` baseline) behind both canonical selections; exact scopes
/// bypass root selection.
pub fn plan_request(scope: &SetupScope) -> SetupRequest {
    match scope {
        SetupScope::Repository => plan_request_for_root_plan(&repository_plan()),
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
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid generation id {0:?}: want 64 lowercase hex chars")]
pub struct GenerationIdError(String);

impl GenerationId {
    /// Carries one generation digest, validating its shape via the single
    /// digest owner (`dx_digest::is_hex`: 64 lowercase hex chars).
    pub fn new(id: &str) -> Result<Self, GenerationIdError> {
        if dx_digest::is_hex(id) {
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
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResolveError {
    /// The selected scope prepared neither side: an exact target with no
    /// environment or codegen capability selects nothing, and committing
    /// an empty pair or re-committing the current pair would hide the
    /// usage error.
    #[error("no capability: selected scope prepared neither environment nor codegen")]
    NoCapability,
}

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
/// failing with a busy diagnostic. Mirrors `dx_env::LOCK_TIMEOUT` (
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
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CommitError {
    /// Workspace root is missing or not a directory.
    #[error("invalid workspace root {path:?}: missing or not a directory", path = path.display())]
    WorkspaceRoot { path: PathBuf },
    /// Another command holds the commit lock past the deadline.
    #[error("workspace busy at {path:?}: another command holds the commit lock", path = path.display())]
    Busy { path: PathBuf },
    /// The commit lock cannot be opened or locked.
    #[error("cannot lock {path:?}: {reason}", path = path.display())]
    LockFailed { path: PathBuf, reason: String },
    /// `.dx/setups/current` or its record links are present but malformed.
    /// Never adopted, never repaired: the operator removes the offending
    /// path or re-runs setup from a clean selection.
    #[error("invalid current selection: {reason}")]
    CurrentInvalid { reason: String },
    /// A record already exists at the expected setup hash but its links do
    /// not match the pair byte-for-byte. The commit fails without touching
    /// the current pointer (digest-spoof refusal).
    #[error("record mismatch: {reason}")]
    RecordMismatch { reason: String },
    /// A workspace mutation failed. The current pointer is either untouched
    /// (pre-swap failure) or fully swapped (the swap itself is one atomic
    /// rename).
    #[error("install failed: {reason}")]
    Install { reason: String },
    /// The selected scope prepared neither side; committing an empty or
    /// recycled pair would hide the usage error. Mirrors [`ResolveError`].
    #[error("no capability: selected scope prepared neither environment nor codegen")]
    NoCapability,
}

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
/// Routed through `dx_digest` so the digest algorithm has one owner
///.
pub fn setup_digest(pair: &SetupPair) -> [u8; 32] {
    digest(setup_fingerprint(pair).as_bytes())
}

/// Lowercase hex of the setup digest: the setup record directory name.
pub fn setup_hex(pair: &SetupPair) -> String {
    dx_digest::to_hex(&setup_digest(pair))
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
fn map_lock_error(error: Error) -> CommitError {
    match error {
        Error::Busy { path } => CommitError::Busy { path },
        Error::LockFailed { path, reason } => CommitError::LockFailed { path, reason },
        other => CommitError::LockFailed {
            path: PathBuf::from(".dx"),
            reason: other.to_string(),
        },
    }
}

/// Acquires the shared workspace commit lock. This is the route owned
/// by `dx_env::acquire_lock` over `dx_atomic_fs::lock_exclusive`
/// dedicated lock file, `File::try_lock`, contention-only retry until the
/// deadline): setup introduces no new lock file, mechanism, or deadline.
fn acquire_commit_lock(dx_dir: &Path, timeout: Duration) -> Result<std::fs::File, CommitError> {
    acquire_lock(dx_dir, timeout).map_err(map_lock_error)
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
#[path = "lib_tests.rs"]
mod lib_tests;
