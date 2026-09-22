//! Managed `.dx/bin` bootstrap core (WP2).
//!
//! Refresh semantics for the runnable environment: adopt nothing, install
//! the staged `environment_tree` output set atomically, and prove
//! provenance with a binary marker so later runs can no-op or replace.
//!
//! Contract: `docs/environments/environment.md` (Tool Exposure, Ownership
//! and Refresh) and `docs/environments/managed-state.md` (Installation and
//! Ownership, Commit Lock and Concurrency, Windows Symlink Pre-Check,
//! Marker Encoding and Versioning).
//!
//! The binary shim (`main.rs`) owns process concerns only: flag parsing,
//! workspace discovery, staged-input location, and exit codes. Everything
//! here is unit-tested.

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use marker_proto::dx::env::v1::{EnvIdentity, EnvMarker, ToolEntry};
use prost::Message;
use serde::Deserialize;

/// Name of the provenance marker file inside the installed tree.
pub const MARKER_FILE_NAME: &str = ".rules_dx_managed";
/// Marker schema version written by this installer. Markers with any other
/// version are refused, never migrated.
pub const MARKER_SCHEMA_VERSION: u32 = 1;
/// Staged-metadata schema version this installer validates. It must equal
/// the Starlark `ENV_METADATA_SCHEMA_VERSION`; mismatches fail closed.
pub const STAGED_METADATA_SCHEMA_VERSION: u32 = 1;
/// Installed tool directory name under `.dx`.
pub const BIN_DIR_NAME: &str = "bin";
/// Commit-lock file name under `.dx`.
pub const LOCK_FILE_NAME: &str = ".commit.lock";
/// Staging directory name under `.dx`. Staging beside the target keeps
/// both renames on one filesystem, so each stays atomic.
pub const STAGE_DIR_NAME: &str = "bin.next";
/// Previous-tree directory name under `.dx` during the atomic swap.
pub const PREV_DIR_NAME: &str = "bin.prev";
/// Provenance identity digest length in bytes (BLAKE3-256).
pub const IDENTITY_LEN: usize = 32;
/// How long refresh contends for the commit lock before failing.
pub const LOCK_TIMEOUT: Duration = Duration::from_secs(10);
/// Host-name suffixes the installer refuses to materialize. Mirrors the
/// Starlark registry so hand-edited staged metadata cannot smuggle an
/// executable suffix past the boundary.
const EXECUTABLE_SUFFIXES: &[&str] = &[".exe", ".bat", ".cmd", ".com"];
/// Host-name stems reserved on Windows. Mirrors the Starlark registry.
const WINDOWS_RESERVED_STEMS: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// One validated tool awaiting installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolPlan {
    /// Primary host command name.
    pub bin_name: String,
    /// Configured producer target label (internal identity).
    pub owner: String,
    /// Host names materialized as links, in staged order.
    pub host_names: Vec<String>,
}

/// Staged tree management metadata as written by `environment_tree`.
#[derive(Deserialize)]
struct StagedMetadata {
    schema_version: u32,
    tools: Vec<StagedTool>,
}

/// One tool record inside the staged metadata.
#[derive(Deserialize)]
struct StagedTool {
    bin_name: String,
    owner: String,
    host_names: Vec<String>,
}

/// Refresh failure. Every variant is operational (exit 1 at the shim);
/// usage errors live in the shim and never surface as this type.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// Workspace root is missing or not a directory.
    #[error("workspace root {path} is not a directory", path = path.display())]
    WorkspaceRoot { path: PathBuf },
    /// Staged inputs are missing, unreadable, or structurally invalid.
    #[error("staged tree is unusable: {reason}")]
    Staged { reason: String },
    /// Staged metadata carries an unsupported schema version.
    #[error(
        "staged metadata schema {found} is unsupported (installer handles {STAGED_METADATA_SCHEMA_VERSION}); regenerate the tree"
    )]
    UnsupportedStagedSchema { found: u32 },
    /// A staged tool record fails validation.
    #[error("staged tool is invalid: {reason}")]
    InvalidTool { reason: String },
    /// `.dx/bin` exists but is not a tree this installer wrote. Never
    /// adopted, never modified: the operator removes or renames it.
    #[error("refusing to touch unmanaged {path}: {detail}", path = path.display())]
    Unmanaged { path: PathBuf, detail: String },
    /// The installed marker carries an unsupported schema version,
    /// typically written by a newer installer. Upgrade, do not delete.
    #[error(
        "installed marker schema {found} is unsupported (installer handles {MARKER_SCHEMA_VERSION}); upgrade dx instead of deleting state"
    )]
    UnsupportedMarkerSchema { found: u32 },
    /// The installed marker is present but undecodable.
    #[error("installed marker is invalid: {reason}")]
    MarkerInvalid { reason: String },
    /// Another refresh holds the commit lock past the deadline.
    #[error(
        "another refresh holds {path}; giving up after the commit-lock deadline",
        path = path.display()
    )]
    Busy { path: PathBuf },
    /// The commit lock cannot be opened or locked.
    #[error("cannot lock {path}: {reason}", path = path.display())]
    LockFailed { path: PathBuf, reason: String },
    /// The host cannot create symlinks. Reported before any mutation.
    #[error("symlinks are unusable on this host: {detail}")]
    SymlinkUnsupported { detail: String },
    /// A workspace mutation failed. Staging is cleaned; the managed tree
    /// is either untouched (pre-commit failure) or fully swapped (the
    /// swap itself is two atomic renames).
    #[error("installation failed: {reason}")]
    Install { reason: String },
}

/// Refresh inputs. `os` selects the symlink-failure guidance
/// (`std::env::consts::OS` at the call site) and `lock_timeout` bounds
/// commit-lock contention.
pub struct RefreshOptions {
    /// Workspace root owning the `.dx` directory.
    pub workspace_root: PathBuf,
    /// Staged tree `bin` directory (one symlink per host name).
    pub staged_bin: PathBuf,
    /// Staged tree metadata JSON file.
    pub staged_metadata: PathBuf,
    /// Host operating system for failure guidance.
    pub os: &'static str,
    /// How long to contend for the commit lock.
    pub lock_timeout: Duration,
}

/// Refresh outcome for operator messaging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshOutcome {
    /// Installed marker already matches the candidate set; nothing changed.
    AlreadyCurrent,
    /// No managed tree existed; the candidate set is now installed.
    InstalledFresh,
    /// A managed tree with a different set existed; it was replaced.
    InstalledReplacement,
}

/// Deterministic encoding of the installed set: tools sorted by
/// `(bin_name, owner)`, so configuration order never affects identity.
pub fn canonical_identity_bytes(tools: &[ToolPlan]) -> Vec<u8> {
    let mut sorted: Vec<&ToolPlan> = tools.iter().collect();
    sorted.sort_by(|a, b| {
        a.bin_name
            .cmp(&b.bin_name)
            .then_with(|| a.owner.cmp(&b.owner))
    });
    let identity = EnvIdentity {
        schema_version: MARKER_SCHEMA_VERSION,
        tools: sorted
            .iter()
            .map(|tool| ToolEntry {
                bin_name: tool.bin_name.clone(),
                owner: tool.owner.clone(),
                host_names: tool.host_names.clone(),
            })
            .collect(),
    };
    identity.encode_to_vec()
}

/// BLAKE3-256 over the canonical identity bytes: the snapshot identity,
/// with no algorithm negotiation. Routed through `dx_digest` so the digest
/// algorithm has one owner.
fn identity_digest(canonical: &[u8]) -> [u8; 32] {
    dx_digest::blake3(canonical)
}

/// Lowercase hex of the identity digest, for operator messaging.
pub fn identity_hex(tools: &[ToolPlan]) -> String {
    dx_digest::to_hex(&identity_digest(&canonical_identity_bytes(tools)))
}

/// Encodes a provenance marker for `identity`.
pub fn encode_marker(identity: &[u8; 32]) -> Vec<u8> {
    EnvMarker {
        schema_version: MARKER_SCHEMA_VERSION,
        identity: identity.to_vec(),
    }
    .encode_to_vec()
}

/// Decodes and validates a provenance marker.
pub fn decode_marker(bytes: &[u8]) -> Result<[u8; 32], Error> {
    let marker = EnvMarker::decode(bytes).map_err(|e| Error::MarkerInvalid {
        reason: format!("not a valid marker: {e}"),
    })?;
    if marker.schema_version != MARKER_SCHEMA_VERSION {
        return Err(Error::UnsupportedMarkerSchema {
            found: marker.schema_version,
        });
    }
    if marker.identity.len() != IDENTITY_LEN {
        return Err(Error::MarkerInvalid {
            reason: format!(
                "identity is {} bytes, want {IDENTITY_LEN}",
                marker.identity.len()
            ),
        });
    }
    let mut identity = [0u8; IDENTITY_LEN];
    identity.copy_from_slice(&marker.identity);
    Ok(identity)
}

/// Parses and validates staged tree metadata into installable plans.
pub fn parse_staged(text: &str) -> Result<Vec<ToolPlan>, Error> {
    let metadata: StagedMetadata = serde_json::from_str(text).map_err(|e| Error::Staged {
        reason: format!("metadata is not valid JSON: {e}"),
    })?;
    if metadata.schema_version != STAGED_METADATA_SCHEMA_VERSION {
        return Err(Error::UnsupportedStagedSchema {
            found: metadata.schema_version,
        });
    }
    if metadata.tools.is_empty() {
        return Err(Error::InvalidTool {
            reason: "metadata carries no tools".to_string(),
        });
    }
    metadata
        .tools
        .iter()
        .map(|tool| {
            if tool.bin_name.is_empty() {
                return Err(Error::InvalidTool {
                    reason: "tool with empty bin_name".to_string(),
                });
            }
            if tool.owner.is_empty() {
                return Err(Error::InvalidTool {
                    reason: format!("tool '{}' has an empty owner", tool.bin_name),
                });
            }
            if tool.host_names.is_empty() {
                return Err(Error::InvalidTool {
                    reason: format!("tool '{}' has no host names", tool.bin_name),
                });
            }
            for name in &tool.host_names {
                validate_host_name(&tool.bin_name, name)?;
            }
            Ok(ToolPlan {
                bin_name: tool.bin_name.clone(),
                owner: tool.owner.clone(),
                host_names: tool.host_names.clone(),
            })
        })
        .collect()
}

/// Validates one host name before it becomes a symlink. Mirrors the
/// Starlark registry so staged metadata that bypassed analysis still
/// cannot plant path traversal or platform-hostile names.
fn validate_host_name(bin_name: &str, name: &str) -> Result<(), Error> {
    let bad = |why: &str| Error::InvalidTool {
        reason: format!("tool '{bin_name}' has invalid host name '{name}': {why}"),
    };
    if name.is_empty() {
        return Err(bad("must be a non-empty single path component"));
    }
    if name == "." || name == ".." {
        return Err(bad("must not be '.' or '..'"));
    }
    if name.contains('/') || name.contains('\\') {
        return Err(bad("must not contain '/' or '\\'"));
    }
    let lower = name.to_lowercase();
    for suffix in EXECUTABLE_SUFFIXES.iter().copied() {
        if lower.ends_with(suffix) {
            return Err(bad(&format!(
                "must not carry an executable suffix (found '{suffix}')"
            )));
        }
    }
    // `split` always yields at least one item; `unwrap_or_default`
    // keeps this total without a panic path.
    let stem = lower.split('.').next().unwrap_or_default();
    if WINDOWS_RESERVED_STEMS.contains(&stem) {
        return Err(bad(&format!("stem '{stem}' is reserved on Windows")));
    }
    Ok(())
}

/// Opens (creating) the commit-lock file and contends for an exclusive
/// flock until `timeout`. Only contention retries; any other flock failure
/// aborts immediately so platform errors are never misreported as busy.
/// The contention loop is owned by `dx_atomic_fs::lock_exclusive`;
/// this function owns only the lock-file open.
pub fn acquire_lock(dx_dir: &Path, timeout: Duration) -> Result<File, Error> {
    let path = dx_dir.join(LOCK_FILE_NAME);
    // The lock file's bytes are never read; open-or-create must leave any
    // existing content undisturbed, so truncate behavior is explicitly off.
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|e| Error::LockFailed {
            path: path.clone(),
            reason: format!("cannot open commit lock: {e}"),
        })?;
    match dx_atomic_fs::lock_exclusive(&file, timeout) {
        Ok(()) => Ok(file),
        Err(std::fs::TryLockError::WouldBlock) => Err(Error::Busy { path: path.clone() }),
        // LCOV_EXCL_START - reason: defensive unreachable, issue: 1055, policy: docs/testing/strategy-details.md#coverage
        Err(e) => Err(Error::LockFailed {
            path: path.clone(),
            reason: format!("cannot lock commit lock: {e}"),
        }),
        // LCOV_EXCL_STOP - reason: end defensive unreachable, issue: 1055, policy: docs/testing/strategy-details.md#coverage
    }
}

/// Default symlink probe: proves link creation works in `dir` before the
/// installer mutates anything. The probe target is intentionally dangling:
/// link creation itself is what is under test.
pub fn probe_symlink(dir: &Path) -> io::Result<()> {
    let link = dir.join("symlink.probe");
    symlink_entry(&PathBuf::from("symlink.target"), &link)?;
    fs::remove_file(&link)
}

/// Platform symlink primitive for staged and probe links.
#[cfg(windows)]
fn symlink_entry(target: &Path, link: &Path) -> io::Result<()> {
    std::os::windows::fs::symlink_file(target, link)
}

/// Platform symlink primitive for staged and probe links.
#[cfg(not(windows))]
fn symlink_entry(target: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

/// Symlink-failure guidance for `os` (`std::env::consts::OS` values).
fn symlink_guidance(os: &str) -> &'static str {
    if os == "windows" {
        return "enable Developer Mode or grant SeBackupPrivilege so symlink creation works, then retry; no changes were made";
    }
    "symlink creation failed: check the filesystem and permissions, then retry; no changes were made"
}

/// Refreshes `.dx/bin` to the staged candidate set.
///
/// Order is the contract: validate staged inputs before inspecting the
/// workspace (corrupt inputs report `Staged` regardless of workspace
/// state), prove the workspace and symlinks before locking, and hold the
/// commit lock across every inspection and mutation so concurrent
/// refreshes serialize.
pub fn refresh(
    options: &RefreshOptions,
    probe: &dyn Fn(&Path) -> io::Result<()>,
) -> Result<RefreshOutcome, Error> {
    let text = fs::read_to_string(&options.staged_metadata).map_err(|e| Error::Staged {
        reason: format!(
            "cannot read staged metadata {}: {e}",
            options.staged_metadata.display()
        ),
    })?;
    let tools = parse_staged(&text)?;
    let staged = read_staged_links(&options.staged_bin, &tools)?;
    if !options.workspace_root.is_dir() {
        return Err(Error::WorkspaceRoot {
            path: options.workspace_root.clone(),
        });
    }
    let dx_dir = options.workspace_root.join(".dx");
    fs::create_dir_all(&dx_dir).map_err(|e| Error::Install {
        reason: format!("cannot create {}: {e}", dx_dir.display()),
    })?;
    probe(&dx_dir).map_err(|e| Error::SymlinkUnsupported {
        detail: format!("{}: {e}", symlink_guidance(options.os)),
    })?;
    let _lock = acquire_lock(&dx_dir, options.lock_timeout)?;
    let bin_dir = dx_dir.join(BIN_DIR_NAME);
    let prev_dir = dx_dir.join(PREV_DIR_NAME);
    let stage_dir = dx_dir.join(STAGE_DIR_NAME);
    recover_crashed_swap(&prev_dir, &bin_dir)?;
    if prev_dir.exists() {
        fs::remove_dir_all(&prev_dir).map_err(|e| Error::Install {
            reason: format!("cannot clear stale {}: {e}", prev_dir.display()),
        })?;
    }
    let current = read_current_identity(&bin_dir)?;
    let candidate = identity_digest(&canonical_identity_bytes(&tools));
    if current == Some(candidate) {
        clean_stage(&stage_dir);
        return Ok(RefreshOutcome::AlreadyCurrent);
    }
    let fresh = current.is_none();
    stage_tree(&stage_dir, &staged, &candidate)?;
    commit_swap(&bin_dir, &prev_dir, &stage_dir)?;
    let _ = fs::remove_dir_all(&prev_dir);
    if fresh {
        Ok(RefreshOutcome::InstalledFresh)
    } else {
        Ok(RefreshOutcome::InstalledReplacement)
    }
}

/// Resolves every promised host name to its staged content before the
/// workspace is touched, so input failures stay mutation-free. Targets
/// canonicalize to absolute Bazel output paths: installed links survive
/// working-directory changes. They still reference Bazel outputs, so a
/// `bazel clean` wants a fresh refresh.
fn read_staged_links(
    staged_bin: &Path,
    tools: &[ToolPlan],
) -> Result<Vec<(String, PathBuf)>, Error> {
    let mut staged = Vec::new();
    for tool in tools {
        for name in &tool.host_names {
            let link = staged_bin.join(name);
            let target = fs::read_link(&link).map_err(|e| Error::Staged {
                reason: format!(
                    "staged link {} is unreadable or not a symlink: {e}",
                    link.display()
                ),
            })?;
            let absolute = link.parent().unwrap_or(staged_bin).join(&target);
            let canonical = fs::canonicalize(&absolute).map_err(|e| Error::Staged {
                reason: format!(
                    "staged link {} has an unresolvable target: {e}",
                    link.display()
                ),
            })?;
            staged.push((name.clone(), canonical));
        }
    }
    Ok(staged)
}

/// Inspects the installed tree: absent means fresh, a valid marker yields
/// its identity, and anything else is foreign state we refuse to adopt.
fn read_current_identity(bin_dir: &Path) -> Result<Option<[u8; 32]>, Error> {
    let meta = match fs::symlink_metadata(bin_dir) {
        Ok(meta) => meta,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        // LCOV_EXCL_START - reason: defensive unreachable, issue: 1055, policy: docs/testing/strategy-details.md#coverage
        Err(e) => {
            return Err(Error::Unmanaged {
                path: bin_dir.to_path_buf(),
                detail: format!("cannot inspect installed tree: {e}"),
            });
            // LCOV_EXCL_STOP - reason: end defensive unreachable, issue: 1055, policy: docs/testing/strategy-details.md#coverage
        }
    };
    if meta.file_type().is_symlink() {
        return Err(unmanaged(
            bin_dir,
            "is a symlink; refusing to adopt foreign state",
        ));
    }
    if !meta.is_dir() {
        return Err(unmanaged(
            bin_dir,
            "is not a directory; refusing to adopt foreign state",
        ));
    }
    let marker_path = bin_dir.join(MARKER_FILE_NAME);
    if !marker_path.exists() {
        return Err(unmanaged(bin_dir, "exists without a provenance marker"));
    }
    let bytes = fs::read(&marker_path).map_err(|e| Error::Unmanaged {
        path: bin_dir.to_path_buf(),
        detail: format!("provenance marker is unreadable: {e}"),
    })?;
    match decode_marker(&bytes) {
        Ok(identity) => Ok(Some(identity)),
        Err(Error::UnsupportedMarkerSchema { found }) => {
            Err(Error::UnsupportedMarkerSchema { found })
        }
        Err(other) => Err(Error::Unmanaged {
            path: bin_dir.to_path_buf(),
            detail: format!("provenance marker is not ours: {other}"),
        }),
    }
}

/// Refusal constructor for foreign `.dx/bin` states.
fn unmanaged(bin_dir: &Path, detail: &str) -> Error {
    Error::Unmanaged {
        path: bin_dir.to_path_buf(),
        detail: format!(
            "{detail}; remove or rename it so the installer can start from a clean state"
        ),
    }
}

/// Recovers the single crash window: a run that died between retiring the
/// old tree and publishing the staged one leaves `prev` without `bin`.
/// The retired tree is the last good state, so it comes back first.
fn recover_crashed_swap(prev_dir: &Path, bin_dir: &Path) -> Result<(), Error> {
    if !prev_dir.exists() {
        return Ok(());
    }
    if bin_dir.exists() {
        return Ok(());
    }
    fs::rename(prev_dir, bin_dir).map_err(|e| Error::Install {
        // LCOV_EXCL_START - reason: defensive unreachable, issue: 1055, policy: docs/testing/strategy-details.md#coverage
        reason: format!("cannot restore interrupted tree: {e}"),
    })?;
    // LCOV_EXCL_STOP - reason: end defensive unreachable, issue: 1055, policy: docs/testing/strategy-details.md#coverage
    Ok(())
}

/// Stages links plus the provenance marker into a clean staging directory.
/// The marker travels with the tree so the swap publishes both atomically.
fn stage_tree(
    stage_dir: &Path,
    staged: &[(String, PathBuf)],
    identity: &[u8; 32],
) -> Result<(), Error> {
    if stage_dir.exists() {
        fs::remove_dir_all(stage_dir).map_err(|e| Error::Install {
            reason: format!("cannot clear stale staging {}: {e}", stage_dir.display()),
        })?;
    }
    fs::create_dir_all(stage_dir).map_err(|e| Error::Install {
        // LCOV_EXCL_START - reason: defensive unreachable, issue: 1055, policy: docs/testing/strategy-details.md#coverage
        reason: format!("cannot create staging {}: {e}", stage_dir.display()),
    })?;
    // LCOV_EXCL_STOP - reason: end defensive unreachable, issue: 1055, policy: docs/testing/strategy-details.md#coverage
    for (name, target) in staged {
        symlink_entry(target, &stage_dir.join(name)).map_err(|e| Error::Install {
            reason: format!("cannot stage host name '{name}': {e}"),
        })?;
    }
    fs::write(stage_dir.join(MARKER_FILE_NAME), encode_marker(identity)).map_err(|e| {
        // LCOV_EXCL_START - reason: defensive unreachable, issue: 1055, policy: docs/testing/strategy-details.md#coverage
        Error::Install {
            reason: format!("cannot stage provenance marker: {e}"),
        }
    })?;
    // LCOV_EXCL_STOP - reason: end defensive unreachable, issue: 1055, policy: docs/testing/strategy-details.md#coverage
    Ok(())
}

/// Publishes staging over the managed tree as two same-directory renames;
/// each is atomic, so a crash lands in a recoverable state.
fn commit_swap(bin_dir: &Path, prev_dir: &Path, stage_dir: &Path) -> Result<(), Error> {
    if bin_dir.exists() {
        fs::rename(bin_dir, prev_dir).map_err(|e| Error::Install {
            // LCOV_EXCL_START - reason: defensive unreachable, issue: 1055, policy: docs/testing/strategy-details.md#coverage
            reason: format!("cannot retire current tree: {e}"),
        })?;
        // LCOV_EXCL_STOP - reason: end defensive unreachable, issue: 1055, policy: docs/testing/strategy-details.md#coverage
    }
    fs::rename(stage_dir, bin_dir).map_err(|e| Error::Install {
        // LCOV_EXCL_START - reason: defensive unreachable, issue: 1055, policy: docs/testing/strategy-details.md#coverage
        reason: format!("cannot publish staged tree: {e}"),
    })?;
    // LCOV_EXCL_STOP - reason: end defensive unreachable, issue: 1055, policy: docs/testing/strategy-details.md#coverage
    Ok(())
}

/// Best-effort staging cleanup: leftovers are reclaimed on the next refresh.
fn clean_stage(stage_dir: &Path) {
    if stage_dir.exists() {
        let _ = fs::remove_dir_all(stage_dir);
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod lib_tests;
