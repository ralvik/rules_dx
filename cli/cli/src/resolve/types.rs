//! Shared types for scope resolution (issue #236).
//!
//! Split from `super` (`resolve.rs`): owns the query result/runner
//! seam ([`QueryResult`], [`QueryRunner`], [`ProcessQueryRunner`]),
//! the resolution output ([`ResolvedScope`]), and the failure type
//! ([`ResolveError`]). Every domain submodule (`classify`, `query`,
//! `entry`, `run_deploy`, `test_map`) builds on these; `super`
//! re-exports them so `crate::resolve::{...}` paths are unchanged.

use std::io;
use std::path::Path;

use dx_process::Scope;

/// Captured result of one `bazel query` invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryResult {
    /// Process exit code (`None` when killed by signal).
    pub code: Option<i32>,
    /// Raw stdout bytes: one canonical label per line on success.
    pub stdout: Vec<u8>,
    /// Raw stderr bytes: Bazel diagnostics on failure.
    pub stderr: Vec<u8>,
}

/// Subprocess seam for ownership queries. Production calls spawn Bazel;
/// tests substitute scripted outputs and record argv.
pub trait QueryRunner {
    /// Runs `argv` with `cwd` as working directory and captures stdio.
    fn run_query(&self, argv: &[String], cwd: &Path) -> io::Result<QueryResult>;
}

/// Production query runner: spawns the launcher with piped stdio and
/// waits for completion.
// LCOV_EXCL_START - reason: thin process-spawn seam; resolution logic is unit-covered through scripted runners and the spawner itself is verified by M08 dogfood evidence.
pub struct ProcessQueryRunner;

impl QueryRunner for ProcessQueryRunner {
    fn run_query(&self, argv: &[String], cwd: &Path) -> io::Result<QueryResult> {
        let (binary, args) = argv
            .split_first()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "query needs a binary"))?;
        let output = std::process::Command::new(binary)
            .args(args)
            .current_dir(cwd)
            .output()?;
        Ok(QueryResult {
            code: output.status.code(),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}
// LCOV_EXCL_STOP - reason: end of process-spawn seam exclusion.

/// Resolved scope: exact Bazel targets plus the summary scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedScope {
    /// Summary scope: `Repository` for empty input, `Labels` for
    /// label-only input (order preserved), `ResolvedOwners` once any
    /// file or directory resolves (sorted and deduplicated).
    pub scope: Scope,
    /// Exact targets for command construction.
    pub targets: Vec<String>,
}

/// Scope resolution failure. Every variant is a pre-execution failure
/// (exit 2) except [`ResolveError::NoRunnable`] and
/// [`ResolveError::AmbiguousRunnable`], which are operational `dx run`
/// failures (exit 1) per O52: the scope resolved, but no single
/// executable owner exists.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResolveError {
    /// Empty positional scope.
    #[error("empty scope: pass a label, pattern, file, or directory")]
    EmptyScope,
    /// Package-relative label such as `:target`, which would resolve
    /// against the current directory instead of the workspace.
    #[error(
        "unsupported scope {scope:?}: package-relative labels resolve against the current directory; spell the workspace label starting with //"
    )]
    RelativeLabel { scope: String },
    /// External-repository label or pattern for a workflow scope.
    #[error(
        "unsupported scope {scope:?}: workflow commands accept main-workspace labels, patterns, files, and directories only"
    )]
    ExternalScope { scope: String },
    /// Absolute path or a path escaping the workspace through `..`.
    #[error("unsupported scope {scope:?}: pass a workspace-relative path without .. escapes")]
    OutsideWorkspace { scope: String },
    /// Path with no workspace entry.
    #[error("unknown path {scope:?}: no such file or directory under the workspace")]
    PathNotFound { scope: String },
    /// Workspace entry that is neither a file nor a directory.
    #[error("unsupported path {scope:?}: scope paths must be regular files or directories")]
    NotFileOrDir { scope: String },
    /// File whose directory chain holds no Bazel package: no
    /// `BUILD.bazel`/`BUILD` marker exists from the parent directory up
    /// to the workspace root.
    #[error(
        "not a package {scope:?}: no enclosing Bazel package holds the file; add a BUILD file for its directory or pass an explicit target label"
    )]
    NotAPackage { scope: String },
    /// Filename with control characters that cannot round-trip
    /// through query syntax and line-oriented output.
    #[error(
        "unsupported path {scope:?}: filenames with control characters cannot resolve through Bazel query"
    )]
    UnsupportedName { scope: String },
    /// File with no direct source owner in the query graph.
    #[error(
        "no Bazel target owns {file:?} (queried as {label}): add the file to a target srcs list or pass an explicit target label"
    )]
    NoOwner { file: String, label: String },
    /// Direct owners with no reverse-dependent test in the query graph.
    /// Distinct from [`ResolveError::NoOwner`]: the file is owned, but no
    /// test reaches those owners, so there is nothing to run.
    #[error(
        "no test depends on {owners}: pass an explicit test label or pattern such as //pkg/...",
        owners = owners.join(" ")
    )]
    NoTests { owners: Vec<String> },
    /// `dx run` file/directory scope with no executable owner: no
    /// depth-1 owner has a rule kind ending in `_binary`.
    #[error(
        "no executable target owns {scopes}: add a *_binary rule owning the file or pass an explicit runnable label",
        scopes = scopes.join(" ")
    )]
    NoRunnable { scopes: Vec<String> },
    /// `dx run` file/directory scope with multiple executable owners.
    /// Candidates are bytewise sorted.
    #[error(
        "multiple executable targets own the scope ({candidates}): pass one explicit runnable label",
        candidates = candidates.join(" ")
    )]
    AmbiguousRunnable { candidates: Vec<String> },
    /// `dx deploy` scope count: exactly one label is required.
    #[error(
        "dx deploy needs exactly one label, got {count}: pass a deploy label such as //deploy:production"
    )]
    DeployCount { count: usize },
    /// `dx deploy` single scope that is not a main-workspace label:
    /// patterns, files, directories, external or relative labels.
    #[error(
        "unsupported deploy scope {scope:?}: pass exactly one deploy label such as //deploy:production (patterns like //..., files, and directories are not deployable)"
    )]
    DeployScope { scope: String },
    /// `dx deploy` label that is neither a `dx_deployment` (no
    /// `DxDeployInfo`) nor executable. Bazel owns executability;
    /// aliases included.
    #[error(
        "not_deployable {label}: target provides no DxDeployInfo and is not executable; pass a dx_deployment target or an executable (see docs/deploy/authoring.md)"
    )]
    NotDeployable { label: String },
    /// Ownership query failed or returned unusable output.
    #[error("ownership query for {label} failed: {detail}")]
    QueryFailed { label: String, detail: String },
}
