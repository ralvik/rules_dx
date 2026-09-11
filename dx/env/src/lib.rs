//! Managed `.dx/bin` bootstrap core (M11 WP2).
//!
//! Refresh semantics for the runnable environment: adopt nothing, install
//! the staged `environment_tree` output set atomically, and prove
//! provenance with a binary marker so later runs can no-op or replace.
//!
//! Contracts: `docs/environments/environment.md` (Tool Exposure, Ownership
//! and Refresh) and `docs/environments/managed-state.md` (Installation and
//! Ownership, Commit Lock and Concurrency, Windows Symlink Pre-Check,
//! Marker Encoding and Versioning).
//!
//! The binary shim (`main.rs`) owns process concerns only: flag parsing,
//! workspace discovery, staged-input location, and exit codes. Everything
//! here is unit-tested.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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
/// Poll interval while contending for the commit lock.
const LOCK_POLL: Duration = Duration::from_millis(50);
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Workspace root is missing or not a directory.
    WorkspaceRoot { path: PathBuf },
    /// Staged inputs are missing, unreadable, or structurally invalid.
    Staged { reason: String },
    /// Staged metadata carries an unsupported schema version.
    UnsupportedStagedSchema { found: u32 },
    /// A staged tool record fails validation.
    InvalidTool { reason: String },
    /// `.dx/bin` exists but is not a tree this installer wrote. Never
    /// adopted, never modified: the operator removes or renames it.
    Unmanaged { path: PathBuf, detail: String },
    /// The installed marker carries an unsupported schema version,
    /// typically written by a newer installer. Upgrade, do not delete.
    UnsupportedMarkerSchema { found: u32 },
    /// The installed marker is present but undecodable.
    MarkerInvalid { reason: String },
    /// Another refresh holds the commit lock past the deadline.
    Busy { path: PathBuf },
    /// The commit lock cannot be opened or locked.
    LockFailed { path: PathBuf, reason: String },
    /// The host cannot create symlinks. Reported before any mutation.
    SymlinkUnsupported { detail: String },
    /// A workspace mutation failed. Staging is cleaned; the managed tree
    /// is either untouched (pre-commit failure) or fully swapped (the
    /// swap itself is two atomic renames).
    Install { reason: String },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::WorkspaceRoot { path } => {
                write!(f, "workspace root {} is not a directory", path.display())
            }
            Error::Staged { reason } => write!(f, "staged tree is unusable: {reason}"),
            Error::UnsupportedStagedSchema { found } => write!(
                f,
                "staged metadata schema {found} is unsupported (installer handles {STAGED_METADATA_SCHEMA_VERSION}); regenerate the tree"
            ),
            Error::InvalidTool { reason } => write!(f, "staged tool is invalid: {reason}"),
            Error::Unmanaged { path, detail } => write!(
                f,
                "refusing to touch unmanaged {}: {detail}",
                path.display()
            ),
            Error::UnsupportedMarkerSchema { found } => write!(
                f,
                "installed marker schema {found} is unsupported (installer handles {MARKER_SCHEMA_VERSION}); upgrade dx instead of deleting state"
            ),
            Error::MarkerInvalid { reason } => write!(f, "installed marker is invalid: {reason}"),
            Error::Busy { path } => write!(
                f,
                "another refresh holds {}; giving up after the commit-lock deadline",
                path.display()
            ),
            Error::LockFailed { path, reason } => {
                write!(f, "cannot lock {}: {reason}", path.display())
            }
            Error::SymlinkUnsupported { detail } => {
                write!(f, "symlinks are unusable on this host: {detail}")
            }
            Error::Install { reason } => write!(f, "installation failed: {reason}"),
        }
    }
}

impl std::error::Error for Error {}

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
/// with no algorithm negotiation. Routed through the shared result crate
/// so the digest algorithm has one owner.
fn identity_digest(canonical: &[u8]) -> [u8; 32] {
    quality_result::digest(canonical)
}

/// Lowercase hex of the identity digest, for operator messaging.
pub fn identity_hex(tools: &[ToolPlan]) -> String {
    identity_digest(&canonical_identity_bytes(tools))
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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
    let stem = lower
        .split('.')
        .next()
        .expect("split yields at least one item");
    if WINDOWS_RESERVED_STEMS.contains(&stem) {
        return Err(bad(&format!("stem '{stem}' is reserved on Windows")));
    }
    Ok(())
}

/// Opens (creating) the commit-lock file and contends for an exclusive
/// flock until `timeout`. Only contention retries; any other flock failure
/// aborts immediately so platform errors are never misreported as busy.
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
    let start = Instant::now();
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(file),
            Err(std::fs::TryLockError::WouldBlock) => {
                if start.elapsed() >= timeout {
                    return Err(Error::Busy { path: path.clone() });
                }
                std::thread::sleep(LOCK_POLL);
            }
            // LCOV_EXCL_START - reason: non-contention flock failures are platform-specific and not triggerable on the seed host; open failures and contention are unit-covered.
            Err(e) => {
                return Err(Error::LockFailed {
                    path: path.clone(),
                    reason: format!("cannot lock commit lock: {e}"),
                });
                // LCOV_EXCL_STOP - reason: end of non-contention lock exclusion.
            }
        }
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
        // LCOV_EXCL_START - reason: non-NotFound metadata failures (I/O errors, permission denials on an accessible parent) are platform-specific and not triggerable on the seed host; absent, foreign, and marker states are unit-covered.
        Err(e) => {
            return Err(Error::Unmanaged {
                path: bin_dir.to_path_buf(),
                detail: format!("cannot inspect installed tree: {e}"),
            });
            // LCOV_EXCL_STOP - reason: end of metadata-failure exclusion.
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
        // LCOV_EXCL_START - reason: same-directory rename onto an absent target cannot fail without concurrent mutation, which the commit lock excludes; recovery success is unit-covered.
        reason: format!("cannot restore interrupted tree: {e}"),
    })?;
    // LCOV_EXCL_STOP - reason: end of recovery-rename exclusion.
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
        // LCOV_EXCL_START - reason: creating a fresh directory inside a writable parent the same actor just ensured cannot fail without fault injection; stale-staging cleanup is unit-covered.
        reason: format!("cannot create staging {}: {e}", stage_dir.display()),
    })?;
    // LCOV_EXCL_STOP - reason: end of staging-create exclusion.
    for (name, target) in staged {
        symlink_entry(target, &stage_dir.join(name)).map_err(|e| Error::Install {
            reason: format!("cannot stage host name '{name}': {e}"),
        })?;
    }
    fs::write(stage_dir.join(MARKER_FILE_NAME), encode_marker(identity)).map_err(|e| {
        // LCOV_EXCL_START - reason: writing into a directory the same actor just created and populated cannot fail without fault injection; link-staging failures are unit-covered.
        Error::Install {
            reason: format!("cannot stage provenance marker: {e}"),
        }
    })?;
    // LCOV_EXCL_STOP - reason: end of marker-write exclusion.
    Ok(())
}

/// Publishes staging over the managed tree as two same-directory renames;
/// each is atomic, so a crash lands in a recoverable state.
fn commit_swap(bin_dir: &Path, prev_dir: &Path, stage_dir: &Path) -> Result<(), Error> {
    if bin_dir.exists() {
        fs::rename(bin_dir, prev_dir).map_err(|e| Error::Install {
            // LCOV_EXCL_START - reason: same-directory retire rename onto the just-cleared absent prev cannot fail without concurrent mutation, which the commit lock excludes; publish success is unit-covered.
            reason: format!("cannot retire current tree: {e}"),
        })?;
        // LCOV_EXCL_STOP - reason: end of retire-rename exclusion.
    }
    fs::rename(stage_dir, bin_dir).map_err(|e| Error::Install {
        // LCOV_EXCL_START - reason: same-directory publish rename of the just-staged tree cannot fail without concurrent mutation, which the commit lock excludes; publish success is unit-covered.
        reason: format!("cannot publish staged tree: {e}"),
    })?;
    // LCOV_EXCL_STOP - reason: end of publish-rename exclusion.
    Ok(())
}

/// Best-effort staging cleanup: leftovers are reclaimed on the next refresh.
fn clean_stage(stage_dir: &Path) {
    if stage_dir.exists() {
        let _ = fs::remove_dir_all(stage_dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unique scratch root per test; callers clean up best-effort at the end.
    fn test_root(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dx-env-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create test root");
        dir
    }

    /// One tool plan with a single host name.
    fn plan(bin_name: &str, owner: &str) -> ToolPlan {
        ToolPlan {
            bin_name: bin_name.to_string(),
            owner: owner.to_string(),
            host_names: vec![bin_name.to_string()],
        }
    }

    /// Builds a staged tree on disk: real executable files under
    /// `targets/`, one symlink per host name under `staged/`, and metadata
    /// JSON. Returns `(staged_bin, metadata_path, metadata_text)`.
    fn write_staged(root: &Path, tools: &[(&str, &str, &[&str])]) -> (PathBuf, PathBuf, String) {
        let targets = root.join("targets");
        let staged_bin = root.join("staged");
        fs::create_dir_all(&targets).expect("create targets");
        fs::create_dir_all(&staged_bin).expect("create staged bin");
        let mut entries = Vec::new();
        for (bin_name, owner, hosts) in tools {
            let real = targets.join(bin_name);
            fs::write(&real, format!("#!/bin/sh\necho {bin_name}\n")).expect("write target");
            for host in *hosts {
                let _ = fs::remove_file(staged_bin.join(host));
                std::os::unix::fs::symlink(&real, staged_bin.join(host)).expect("stage link");
            }
            let host_list = hosts
                .iter()
                .map(|host| format!("\"{host}\""))
                .collect::<Vec<_>>()
                .join(",");
            entries.push(format!(
                "{{\"bin_name\":\"{bin_name}\",\"owner\":\"{owner}\",\"host_names\":[{host_list}]}}"
            ));
        }
        let text = format!(
            "{{\"schema_version\":{STAGED_METADATA_SCHEMA_VERSION},\"tools\":[{}]}}",
            entries.join(",")
        );
        let metadata_path = root.join("staged.metadata.json");
        fs::write(&metadata_path, &text).expect("write metadata");
        (staged_bin, metadata_path, text)
    }

    /// Refresh options over a fresh workspace with a 10s lock timeout.
    /// The workspace directory itself always exists (the shim runs inside
    /// a real workspace); only `.dx` starts absent.
    fn options(root: &Path, staged_bin: PathBuf, metadata: PathBuf) -> RefreshOptions {
        fs::create_dir_all(root.join("ws")).expect("create workspace");
        RefreshOptions {
            workspace_root: root.join("ws"),
            staged_bin,
            staged_metadata: metadata,
            os: "linux",
            lock_timeout: Duration::from_secs(10),
        }
    }

    /// Refreshes with the real symlink probe and unwraps success.
    fn refresh_ok(options: &RefreshOptions) -> RefreshOutcome {
        refresh(options, &probe_symlink).expect("refresh succeeds")
    }

    #[test]
    fn canonical_identity_is_order_independent() {
        let forward = vec![
            plan("alpha", "//env:tool_alpha"),
            plan("beta", "//env:tool_beta"),
        ];
        let reverse = vec![
            plan("beta", "//env:tool_beta"),
            plan("alpha", "//env:tool_alpha"),
        ];
        assert_eq!(
            canonical_identity_bytes(&forward),
            canonical_identity_bytes(&reverse)
        );
        let different = vec![plan("alpha", "//env:tool_alpha")];
        assert_ne!(
            canonical_identity_bytes(&forward),
            canonical_identity_bytes(&different)
        );
    }

    #[test]
    fn identity_hex_is_lowercase_64() {
        let hex = identity_hex(&[plan("alpha", "//env:tool_alpha")]);
        assert_eq!(hex.len(), 64);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(hex, hex.to_lowercase());
    }

    #[test]
    fn marker_roundtrip() {
        let tools = vec![plan("alpha", "//env:tool_alpha")];
        let identity = identity_digest(&canonical_identity_bytes(&tools));
        assert_eq!(
            decode_marker(&encode_marker(&identity)).expect("decode"),
            identity
        );
    }

    #[test]
    fn marker_rejects_garbage() {
        assert!(matches!(
            decode_marker(b"definitely not a marker"),
            Err(Error::MarkerInvalid { .. })
        ));
    }

    #[test]
    fn marker_rejects_schema() {
        let bytes = EnvMarker {
            schema_version: 999,
            identity: vec![0u8; IDENTITY_LEN],
        }
        .encode_to_vec();
        assert_eq!(
            decode_marker(&bytes),
            Err(Error::UnsupportedMarkerSchema { found: 999 })
        );
    }

    #[test]
    fn marker_rejects_short_identity() {
        let bytes = EnvMarker {
            schema_version: MARKER_SCHEMA_VERSION,
            identity: vec![7u8; 4],
        }
        .encode_to_vec();
        assert!(matches!(
            decode_marker(&bytes),
            Err(Error::MarkerInvalid { .. })
        ));
    }

    #[test]
    fn parse_staged_ok() {
        let root = test_root("parse-ok");
        let (_, _, text) = write_staged(
            &root,
            &[
                ("alpha", "//env:tool_alpha", &["alpha", "a"]),
                ("beta", "//env:tool_beta", &["beta"]),
            ],
        );
        let plans = parse_staged(&text).expect("parse");
        assert_eq!(plans.len(), 2);
        assert_eq!(
            plans[0].host_names,
            vec!["alpha".to_string(), "a".to_string()]
        );
        assert_eq!(plans[1].owner, "//env:tool_beta".to_string());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn parse_staged_rejects_syntax_and_shape() {
        assert!(matches!(parse_staged("{oops"), Err(Error::Staged { .. })));
        assert!(matches!(
            parse_staged(r#"{"schema_version":1}"#),
            Err(Error::Staged { .. })
        ));
    }

    #[test]
    fn parse_staged_rejects_schema() {
        assert_eq!(
            parse_staged(r#"{"schema_version":2,"tools":[]}"#),
            Err(Error::UnsupportedStagedSchema { found: 2 })
        );
    }

    #[test]
    fn parse_staged_rejects_empty_set() {
        assert!(matches!(
            parse_staged(r#"{"schema_version":1,"tools":[]}"#),
            Err(Error::InvalidTool { .. })
        ));
    }

    #[test]
    fn parse_staged_rejects_bad_records() {
        let empty_bin =
            r#"{"schema_version":1,"tools":[{"bin_name":"","owner":"o","host_names":["a"]}]}"#;
        assert!(matches!(
            parse_staged(empty_bin),
            Err(Error::InvalidTool { .. })
        ));
        let empty_owner =
            r#"{"schema_version":1,"tools":[{"bin_name":"a","owner":"","host_names":["a"]}]}"#;
        assert!(matches!(
            parse_staged(empty_owner),
            Err(Error::InvalidTool { .. })
        ));
        let no_hosts =
            r#"{"schema_version":1,"tools":[{"bin_name":"a","owner":"o","host_names":[]}]}"#;
        assert!(matches!(
            parse_staged(no_hosts),
            Err(Error::InvalidTool { .. })
        ));
    }

    #[test]
    fn host_name_table() {
        for valid in ["a", "doctor-2", "x.y", "my_tool"] {
            let text = format!(
                "{{\"schema_version\":1,\"tools\":[{{\"bin_name\":\"t\",\"owner\":\"o\",\"host_names\":[\"{valid}\"]}}]}}"
            );
            assert!(
                parse_staged(&text).is_ok(),
                "valid host name rejected: {valid}"
            );
        }
        for invalid in [
            "", ".", "..", "a/b", "a\\b", "run.exe", "RUN.BAT", "x.cmd", "y.com", "con", "aux.txt",
            "COM1", "lpt9", "NUL",
        ] {
            // The metadata travels as JSON text, so a literal backslash in
            // the name must be JSON-escaped here to reach the validator as
            // one; otherwise `\b` would arrive as a backspace control.
            let json_name = invalid.replace('\\', "\\\\");
            let text = format!(
                "{{\"schema_version\":1,\"tools\":[{{\"bin_name\":\"t\",\"owner\":\"o\",\"host_names\":[\"{json_name}\"]}}]}}"
            );
            assert!(
                matches!(parse_staged(&text), Err(Error::InvalidTool { .. })),
                "invalid host name accepted: {invalid:?}"
            );
        }
    }

    #[test]
    fn error_display_names_every_variant() {
        let dir = PathBuf::from("/ws/.dx/bin");
        let errors = [
            Error::WorkspaceRoot {
                path: PathBuf::from("/ws"),
            },
            Error::Staged {
                reason: "r".to_string(),
            },
            Error::UnsupportedStagedSchema { found: 2 },
            Error::InvalidTool {
                reason: "r".to_string(),
            },
            Error::Unmanaged {
                path: dir.clone(),
                detail: "d".to_string(),
            },
            Error::UnsupportedMarkerSchema { found: 3 },
            Error::MarkerInvalid {
                reason: "r".to_string(),
            },
            Error::Busy { path: dir.clone() },
            Error::LockFailed {
                path: dir.clone(),
                reason: "r".to_string(),
            },
            Error::SymlinkUnsupported {
                detail: "d".to_string(),
            },
            Error::Install {
                reason: "r".to_string(),
            },
        ];
        for error in &errors {
            assert!(!format!("{error}").is_empty());
        }
        assert!(matches!(
            &errors[4],
            Error::Unmanaged { detail, .. } if detail == "d"
        ));
        assert!(matches!(
            &errors[5],
            Error::UnsupportedMarkerSchema { found } if *found == 3
        ));
        assert!(matches!(&errors[7], Error::Busy { path } if path == &dir));
    }

    #[test]
    fn workspace_missing() {
        let root = test_root("ws-missing");
        let (staged_bin, metadata, _) = write_staged(&root, &[("a", "//o:a", &["a"])]);
        let mut opts = options(&root, staged_bin, metadata);
        opts.workspace_root = root.join("no-such-dir");
        assert!(matches!(
            refresh(&opts, &probe_symlink),
            Err(Error::WorkspaceRoot { .. })
        ));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn staged_inputs_fail_before_workspace_mutation() {
        let root = test_root("staged-fail");
        let (staged_bin, metadata, _) = write_staged(&root, &[("a", "//o:a", &["a"])]);
        let missing_metadata = options(&root, staged_bin.clone(), root.join("nope.json"));
        assert!(matches!(
            refresh(&missing_metadata, &probe_symlink),
            Err(Error::Staged { .. })
        ));
        let missing_bin = options(&root, root.join("nope"), metadata);
        assert!(matches!(
            refresh(&missing_bin, &probe_symlink),
            Err(Error::Staged { .. })
        ));
        assert!(!root.join("ws").join(".dx").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn staged_rejects_non_link_and_dangling() {
        let root = test_root("staged-links");
        let (staged_bin, metadata, text) = write_staged(&root, &[("a", "//o:a", &["a"])]);
        fs::remove_file(staged_bin.join("a")).expect("remove link");
        fs::write(staged_bin.join("a"), "regular file, not a link").expect("clobber link");
        let opts = options(&root, staged_bin.clone(), metadata.clone());
        assert!(matches!(
            refresh(&opts, &probe_symlink),
            Err(Error::Staged { .. })
        ));
        fs::remove_file(staged_bin.join("a")).expect("remove clobber");
        std::os::unix::fs::symlink(root.join("dangling-target"), staged_bin.join("a"))
            .expect("dangling link");
        assert!(matches!(
            refresh(&opts, &probe_symlink),
            Err(Error::Staged { .. })
        ));
        assert!(parse_staged(&text).is_ok());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn fresh_install_noop_and_replacement() {
        let root = test_root("lifecycle");
        let (staged_bin, metadata, _) =
            write_staged(&root, &[("alpha", "//o:alpha", &["alpha", "a"])]);
        let opts = options(&root, staged_bin, metadata);
        assert_eq!(refresh_ok(&opts), RefreshOutcome::InstalledFresh);
        let bin = root.join("ws").join(".dx").join("bin");
        assert_eq!(
            fs::read_link(bin.join("a")).expect("alias link"),
            fs::canonicalize(root.join("targets").join("alpha")).expect("canonical target")
        );
        assert_eq!(refresh_ok(&opts), RefreshOutcome::AlreadyCurrent);
        let (staged_bin, metadata, _) = write_staged(
            &root,
            &[
                ("alpha", "//o:alpha", &["alpha"]),
                ("beta", "//o:beta", &["beta"]),
            ],
        );
        let opts = options(&root, staged_bin, metadata);
        assert_eq!(refresh_ok(&opts), RefreshOutcome::InstalledReplacement);
        assert!(!bin.join("a").exists());
        assert!(bin.join("beta").exists());
        assert!(!root.join("ws").join(".dx").join("bin.prev").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn workspace_path_with_spaces_installs() {
        // M11 evidence: the installer never shells out, so workspace roots
        // containing spaces install and resolve exactly like plain paths.
        let root = test_root("with space");
        let (staged_bin, metadata, _) = write_staged(&root, &[("a", "//o:a", &["a"])]);
        let opts = options(&root, staged_bin, metadata);
        assert_eq!(refresh_ok(&opts), RefreshOutcome::InstalledFresh);
        let link = root.join("ws").join(".dx").join("bin").join("a");
        let target = fs::canonicalize(root.join("targets").join("a")).expect("canonical target");
        assert_eq!(fs::read_link(&link).expect("tool link"), target);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&target, fs::Permissions::from_mode(0o755)).expect("chmod target");
        }
        let output = std::process::Command::new(&link)
            .output()
            .expect("run installed tool");
        assert!(output.status.success());
        assert_eq!(refresh_ok(&opts), RefreshOutcome::AlreadyCurrent);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn already_current_cleans_stale_staging() {
        let root = test_root("noop-clean");
        let (staged_bin, metadata, _) = write_staged(&root, &[("a", "//o:a", &["a"])]);
        let opts = options(&root, staged_bin, metadata);
        assert_eq!(refresh_ok(&opts), RefreshOutcome::InstalledFresh);
        let stage_dir = root.join("ws").join(".dx").join("bin.next");
        fs::create_dir_all(&stage_dir).expect("leftover staging");
        fs::write(stage_dir.join("junk"), "junk").expect("junk");
        assert_eq!(refresh_ok(&opts), RefreshOutcome::AlreadyCurrent);
        assert!(!stage_dir.exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn unmanaged_states_refuse_without_mutation() {
        let root = test_root("unmanaged");
        let (staged_bin, metadata, _) = write_staged(&root, &[("a", "//o:a", &["a"])]);
        let opts = options(&root, staged_bin, metadata);
        let dx = root.join("ws").join(".dx");
        fs::create_dir_all(&dx).expect("dx dir");

        fs::write(dx.join("bin"), "not a directory").expect("file bin");
        assert!(matches!(
            refresh(&opts, &probe_symlink),
            Err(Error::Unmanaged { .. })
        ));
        assert_eq!(
            fs::read(dx.join("bin")).expect("untouched"),
            b"not a directory".to_vec()
        );
        fs::remove_file(dx.join("bin")).expect("remove file bin");

        std::os::unix::fs::symlink(root.join("elsewhere"), dx.join("bin")).expect("symlink bin");
        assert!(matches!(
            refresh(&opts, &probe_symlink),
            Err(Error::Unmanaged { .. })
        ));
        fs::remove_file(dx.join("bin")).expect("remove symlink bin");

        fs::create_dir_all(dx.join("bin")).expect("bare bin dir");
        assert!(matches!(
            refresh(&opts, &probe_symlink),
            Err(Error::Unmanaged { .. })
        ));

        fs::write(dx.join("bin").join(MARKER_FILE_NAME), b"garbage").expect("garbage marker");
        let error = refresh(&opts, &probe_symlink).unwrap_err();
        assert!(matches!(error, Error::Unmanaged { .. }));
        assert!(matches!(
            error,
            Error::Unmanaged { detail, .. } if detail.contains("not ours")
        ));

        fs::remove_file(dx.join("bin").join(MARKER_FILE_NAME)).expect("remove garbage");
        fs::create_dir_all(dx.join("bin").join(MARKER_FILE_NAME)).expect("marker dir");
        assert!(matches!(
            refresh(&opts, &probe_symlink),
            Err(Error::Unmanaged { .. })
        ));
        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn stale_prev_undeletable_reports_install() {
        use std::os::unix::fs::PermissionsExt;
        let root = test_root("stale-prev-perms");
        let (staged_bin, metadata, _) = write_staged(&root, &[("a", "//o:a", &["a"])]);
        let opts = options(&root, staged_bin, metadata);
        assert!(matches!(
            refresh(&opts, &probe_symlink),
            Ok(RefreshOutcome::InstalledFresh)
        ));
        // A previous run crashed after publishing but before clearing
        // `prev`; a read-only `.dx` makes the stale-prev cleanup fail for
        // real. The probe is injected because it also needs a writable
        // `.dx`, and the lock file already exists so opening it needs no
        // directory write permission.
        let dx = root.join("ws").join(".dx");
        fs::create_dir_all(dx.join(PREV_DIR_NAME)).expect("stale prev");
        fs::set_permissions(&dx, fs::Permissions::from_mode(0o555)).expect("read-only dx");
        let error = refresh(&opts, &|_: &Path| Ok(())).unwrap_err();
        assert!(matches!(
            error,
            Error::Install { reason } if reason.contains("cannot clear stale")
        ));
        fs::set_permissions(&dx, fs::Permissions::from_mode(0o755)).expect("writable dx");
        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn stale_stage_undeletable_reports_install() {
        use std::os::unix::fs::PermissionsExt;
        let root = test_root("stale-stage-perms");
        let (staged_bin, metadata, _) = write_staged(&root, &[("a", "//o:a", &["a"])]);
        let opts = options(&root, staged_bin, metadata);
        // Fresh workspace with a pre-seeded `.dx`: the lock file already
        // exists so the read-only parent only breaks stale-staging cleanup.
        let dx = root.join("ws").join(".dx");
        fs::create_dir_all(&dx).expect("dx dir");
        fs::write(dx.join(LOCK_FILE_NAME), b"").expect("lock file");
        fs::create_dir_all(dx.join(STAGE_DIR_NAME)).expect("stale stage");
        fs::set_permissions(&dx, fs::Permissions::from_mode(0o555)).expect("read-only dx");
        let error = refresh(&opts, &|_: &Path| Ok(())).unwrap_err();
        assert!(matches!(
            error,
            Error::Install { reason } if reason.contains("cannot clear stale staging")
        ));
        fs::set_permissions(&dx, fs::Permissions::from_mode(0o755)).expect("writable dx");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn duplicate_host_name_reports_install() {
        let root = test_root("dup-host");
        let target_a = root.join("tool-a");
        let target_b = root.join("tool-b");
        fs::write(&target_a, b"a").expect("target a");
        fs::write(&target_b, b"b").expect("target b");
        let error = stage_tree(
            &root.join("stage"),
            &[("dup".to_string(), target_a), ("dup".to_string(), target_b)],
            &[7u8; 32],
        )
        .unwrap_err();
        assert!(matches!(
            error,
            Error::Install { reason } if reason.contains("cannot stage host name 'dup'")
        ));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn newer_marker_refuses_with_upgrade_guidance() {
        let root = test_root("newer-marker");
        let (staged_bin, metadata, _) = write_staged(&root, &[("a", "//o:a", &["a"])]);
        let opts = options(&root, staged_bin, metadata);
        let bin = root.join("ws").join(".dx").join("bin");
        fs::create_dir_all(&bin).expect("bin dir");
        let bytes = EnvMarker {
            schema_version: 999,
            identity: vec![0u8; IDENTITY_LEN],
        }
        .encode_to_vec();
        fs::write(bin.join(MARKER_FILE_NAME), bytes).expect("newer marker");
        assert_eq!(
            refresh(&opts, &probe_symlink),
            Err(Error::UnsupportedMarkerSchema { found: 999 })
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn crash_between_renames_restores_then_replaces() {
        let root = test_root("crash");
        let (staged_bin, metadata, _) = write_staged(&root, &[("old", "//o:old", &["old"])]);
        let opts = options(&root, staged_bin, metadata);
        assert_eq!(refresh_ok(&opts), RefreshOutcome::InstalledFresh);
        let dx = root.join("ws").join(".dx");
        fs::rename(dx.join("bin"), dx.join("bin.prev")).expect("simulate crash");
        let (staged_bin, metadata, _) = write_staged(&root, &[("new", "//o:new", &["new"])]);
        let opts = options(&root, staged_bin, metadata);
        assert_eq!(refresh_ok(&opts), RefreshOutcome::InstalledReplacement);
        assert!(dx.join("bin").join("new").exists());
        assert!(!dx.join("bin").join("old").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn stale_prev_is_cleared() {
        let root = test_root("stale-prev");
        let (staged_bin, metadata, _) = write_staged(&root, &[("a", "//o:a", &["a"])]);
        let opts = options(&root, staged_bin, metadata);
        assert_eq!(refresh_ok(&opts), RefreshOutcome::InstalledFresh);
        let prev = root.join("ws").join(".dx").join("bin.prev");
        fs::create_dir_all(&prev).expect("stale prev");
        assert_eq!(refresh_ok(&opts), RefreshOutcome::AlreadyCurrent);
        assert!(!prev.exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn busy_lock_fails_after_deadline() {
        let root = test_root("busy");
        let dx = root.join("ws").join(".dx");
        fs::create_dir_all(&dx).expect("dx dir");
        let _held = acquire_lock(&dx, Duration::from_secs(10)).expect("setup lock");
        let (staged_bin, metadata, _) = write_staged(&root, &[("a", "//o:a", &["a"])]);
        let mut opts = options(&root, staged_bin, metadata);
        opts.lock_timeout = Duration::from_millis(1);
        let error = refresh(&opts, &probe_symlink).unwrap_err();
        assert!(matches!(error, Error::Busy { .. }));
        drop(_held);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn lock_open_failure_aborts() {
        let root = test_root("lock-open");
        let (staged_bin, metadata, _) = write_staged(&root, &[("a", "//o:a", &["a"])]);
        let opts = options(&root, staged_bin, metadata);
        let dx = root.join("ws").join(".dx");
        fs::create_dir_all(&dx).expect("dx dir");
        fs::create_dir_all(dx.join(LOCK_FILE_NAME)).expect("lock is a directory");
        assert!(matches!(
            refresh(&opts, &probe_symlink),
            Err(Error::LockFailed { .. })
        ));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn dx_creation_failure_aborts() {
        let root = test_root("dx-create");
        let (staged_bin, metadata, _) = write_staged(&root, &[("a", "//o:a", &["a"])]);
        let opts = options(&root, staged_bin, metadata);
        fs::create_dir_all(root.join("ws")).expect("ws dir");
        fs::write(root.join("ws").join(".dx"), "file blocks dir").expect("file dx");
        assert!(matches!(
            refresh(&opts, &probe_symlink),
            Err(Error::Install { .. })
        ));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn probe_failure_reports_before_mutation() {
        let root = test_root("probe-fail");
        let (staged_bin, metadata, _) = write_staged(&root, &[("a", "//o:a", &["a"])]);
        let failing = |_: &Path| -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "denied"))
        };
        let mut opts = options(&root, staged_bin.clone(), metadata.clone());
        opts.os = "windows";
        let error = refresh(&opts, &failing).unwrap_err();
        assert!(matches!(error, Error::SymlinkUnsupported { .. }));
        assert!(matches!(
            error,
            Error::SymlinkUnsupported { detail } if detail.contains("Developer Mode")
        ));
        opts.os = "other";
        let error = refresh(&opts, &failing).unwrap_err();
        assert!(matches!(
            error,
            Error::SymlinkUnsupported { detail } if detail.contains("no changes were made")
        ));
        assert!(!root.join("ws").join(".dx").join("bin").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn default_probe_accepts_writable_dir() {
        let root = test_root("probe-ok");
        probe_symlink(&root).expect("probe succeeds");
        assert!(!root.join("symlink.probe").exists());
        let _ = fs::remove_dir_all(&root);
    }
}
