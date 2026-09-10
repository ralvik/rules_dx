//! Scope resolution: labels, patterns, files, directories, and
//! test/coverage mapping (M08 WP1+WP2).
//!
//! Contract: `docs/cli/target-resolution.md#input-classification` and
//! `#file-ownership`. Main-workspace labels and target patterns pass
//! through after workspace validation; existing files resolve to every
//! direct source owner through one unconfigured `bazel query` invocation
//! per file, addressed by the nearest enclosing package; directories
//! become recursive Bazel patterns without
//! filesystem enumeration. This module never reads BUILD files and never
//! lists directories: the only filesystem calls are existence/kind probes
//! (`symlink_metadata`), and ownership facts come solely from Bazel
//! query stdout.

use std::io;
use std::path::{Component, Path};

use dx_process::{launcher_argv0, Scope, WORKFLOW_STARTUP_OPTS};

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    /// Empty positional scope.
    EmptyScope,
    /// Package-relative label such as `:target`, which would resolve
    /// against the current directory instead of the workspace.
    RelativeLabel { scope: String },
    /// External-repository label or pattern for a workflow scope.
    ExternalScope { scope: String },
    /// Absolute path or a path escaping the workspace through `..`.
    OutsideWorkspace { scope: String },
    /// Path with no workspace entry.
    PathNotFound { scope: String },
    /// Workspace entry that is neither a file nor a directory.
    NotFileOrDir { scope: String },
    /// File whose directory chain holds no Bazel package: no
    /// `BUILD.bazel`/`BUILD` marker exists from the parent directory up
    /// to the workspace root.
    NotAPackage { scope: String },
    /// Filename with control characters that cannot round-trip
    /// through query syntax and line-oriented output.
    UnsupportedName { scope: String },
    /// File with no direct source owner in the query graph.
    NoOwner { file: String, label: String },
    /// Direct owners with no reverse-dependent test in the query graph.
    /// Distinct from [`ResolveError::NoOwner`]: the file is owned, but no
    /// test reaches those owners, so there is nothing to run.
    NoTests { owners: Vec<String> },
    /// `dx run` file/directory scope with no executable owner: no
    /// depth-1 owner has a rule kind ending in `_binary`.
    NoRunnable { scopes: Vec<String> },
    /// `dx run` file/directory scope with multiple executable owners.
    /// Candidates are bytewise sorted.
    AmbiguousRunnable { candidates: Vec<String> },
    /// Ownership query failed or returned unusable output.
    QueryFailed { label: String, detail: String },
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolveError::EmptyScope => write!(f, "empty scope: pass a label, pattern, file, or directory"),
            ResolveError::RelativeLabel { scope } => write!(
                f,
                "unsupported scope {scope:?}: package-relative labels resolve against the current directory; spell the workspace label starting with //"
            ),
            ResolveError::ExternalScope { scope } => write!(
                f,
                "unsupported scope {scope:?}: workflow commands accept main-workspace labels, patterns, files, and directories only"
            ),
            ResolveError::OutsideWorkspace { scope } => write!(
                f,
                "unsupported scope {scope:?}: pass a workspace-relative path without .. escapes"
            ),
            ResolveError::PathNotFound { scope } => write!(
                f,
                "unknown path {scope:?}: no such file or directory under the workspace"
            ),
            ResolveError::NotFileOrDir { scope } => write!(
                f,
                "unsupported path {scope:?}: scope paths must be regular files or directories"
            ),
            ResolveError::NotAPackage { scope } => write!(
                f,
                "not a package {scope:?}: no enclosing Bazel package holds the file; add a BUILD file for its directory or pass an explicit target label"
            ),
            ResolveError::UnsupportedName { scope } => write!(
                f,
                "unsupported path {scope:?}: filenames with control characters cannot resolve through Bazel query"
            ),
            ResolveError::NoOwner { file, label } => write!(
                f,
                "no Bazel target owns {file:?} (queried as {label}): add the file to a target srcs list or pass an explicit target label"
            ),
            ResolveError::NoTests { owners } => write!(
                f,
                "no test depends on {}: pass an explicit test label or pattern such as //pkg/...",
                owners.join(" ")
            ),
            ResolveError::NoRunnable { scopes } => write!(
                f,
                "no executable target owns {}: add a *_binary rule owning the file or pass an explicit runnable label",
                scopes.join(" ")
            ),
            ResolveError::AmbiguousRunnable { candidates } => write!(
                f,
                "multiple executable targets own the scope ({}): pass one explicit runnable label",
                candidates.join(" ")
            ),
            ResolveError::QueryFailed { label, detail } => {
                write!(f, "ownership query for {label} failed: {detail}")
            }
        }
    }
}

impl std::error::Error for ResolveError {}

/// One `bazel query` invocation per input file (O44): depth-1 reverse
/// dependencies constrained to rules over the main-workspace universe.
fn ownership_expression(label: &str) -> String {
    format!("kind('rule', rdeps(//..., {}, 1))", quote_label(label))
}

/// Quotes a label as a double-quoted query string literal, escaping
/// backslashes and quotes. Rejects control characters, which cannot
/// round-trip through line-oriented query output.
fn quote_label(label: &str) -> String {
    let mut quoted = String::with_capacity(label.len() + 2);
    quoted.push('"');
    for ch in label.chars() {
        match ch {
            '\\' => quoted.push_str("\\\\"),
            '"' => quoted.push_str("\\\""),
            _ => quoted.push(ch),
        }
    }
    quoted.push('"');
    quoted
}

/// Exact query argv: launcher plus workflow startup options (the
/// workspace `.bazelrc` stays in effect) and the ownership expression.
/// No user Bazel options leak into resolution.
fn ownership_argv(label: &str) -> Vec<String> {
    let mut argv = Vec::with_capacity(WORKFLOW_STARTUP_OPTS.len() + 4);
    argv.push(launcher_argv0().to_owned());
    argv.extend(WORKFLOW_STARTUP_OPTS.iter().map(ToString::to_string));
    argv.push("query".to_owned());
    argv.push("--".to_owned());
    argv.push(ownership_expression(label));
    argv
}

/// Exact query argv for an arbitrary unconfigured query expression.
/// Used by runnable and test-mapping queries; no user Bazel options
/// leak into resolution.
fn query_argv(expression: &str) -> Vec<String> {
    let mut argv = Vec::with_capacity(WORKFLOW_STARTUP_OPTS.len() + 4);
    argv.push(launcher_argv0().to_owned());
    argv.extend(WORKFLOW_STARTUP_OPTS.iter().map(ToString::to_string));
    argv.push("query".to_owned());
    argv.push("--".to_owned());
    argv.push(expression.to_owned());
    argv
}

/// Runs one unconfigured `bazel query` for `expression` and parses
/// stdout into sorted deduplicated labels. Query failures and non-UTF-8
/// output become [`ResolveError::QueryFailed`].
fn run_label_query(
    expression: &str,
    workspace: &Path,
    runner: &dyn QueryRunner,
) -> Result<Vec<String>, ResolveError> {
    let argv = query_argv(expression);
    let result = runner
        .run_query(&argv, workspace)
        .map_err(|error| ResolveError::QueryFailed {
            label: expression.to_owned(),
            detail: error.to_string(),
        })?;
    if result.code != Some(0) {
        return Err(ResolveError::QueryFailed {
            label: expression.to_owned(),
            detail: first_line(&result.stderr),
        });
    }
    parse_owners(&result.stdout, expression)
}

/// Runnable file-owner expression (O52): depth-1 reverse dependencies
/// constrained to rules whose kind ends in `_binary`. Aliases are not
/// followed for file scopes: only direct `_binary` owners qualify.
fn runnable_expression(label: &str) -> String {
    format!(
        "kind('.*_binary rule', rdeps(//..., {}, 1))",
        quote_label(label)
    )
}

/// Runnable directory expression (O52): every `_binary` rule under the
/// recursive pattern. Recursion is performed by Bazel, never by
/// filesystem traversal.
fn dir_runnable_expression(pattern: &str) -> String {
    format!("kind('.*_binary rule', {pattern})")
}

/// Normalizes a workspace-relative scope path lexically (no filesystem
/// access): drops `.` and empty segments, rejects absolute paths and
/// `..` escapes. An empty result addresses the workspace root.
fn normalize_rel(raw: &str) -> Result<String, ResolveError> {
    if raw.is_empty() {
        return Err(ResolveError::EmptyScope);
    }
    let path = Path::new(raw);
    if path.is_absolute() {
        return Err(ResolveError::OutsideWorkspace {
            scope: raw.to_owned(),
        });
    }
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(ResolveError::OutsideWorkspace {
                    scope: raw.to_owned(),
                });
            }
        }
    }
    Ok(parts.join("/"))
}

/// Package markers: a directory is a Bazel package when it holds one.
/// Only existence is probed; contents are never read.
const PACKAGE_FILES: [&str; 2] = ["BUILD.bazel", "BUILD"];

/// Finds the nearest enclosing Bazel package for a normalized relative
/// file path by walking from the parent directory up to the workspace
/// root. Returns `(package, path_in_package)`, where the root package
/// is `""`. Returns `None` when no directory in the chain holds a
/// package marker.
fn enclosing_package(workspace: &Path, rel: &str) -> Option<(String, String)> {
    let mut dir = match rel.rfind('/') {
        Some(index) => &rel[..index],
        None => "",
    };
    loop {
        let base = if dir.is_empty() {
            workspace.to_path_buf()
        } else {
            workspace.join(dir)
        };
        if PACKAGE_FILES
            .iter()
            .any(|marker| std::fs::symlink_metadata(base.join(marker)).is_ok())
        {
            let in_package = if dir.is_empty() {
                rel.to_owned()
            } else {
                rel[dir.len() + 1..].to_owned()
            };
            return Some((dir.to_owned(), in_package));
        }
        if dir.is_empty() {
            return None;
        }
        dir = match dir.rfind('/') {
            Some(index) => &dir[..index],
            None => "",
        };
    }
}

/// Maps a normalized relative file path to its source label through
/// the nearest enclosing package (`pkg/src/deep/a.py` to
/// `//pkg:src/deep/a.py`, root files to `//:file`). Files with no
/// enclosing package are [`ResolveError::NotAPackage`], never an
/// invalid label: Bazel file labels require a package.
fn file_label(workspace: &Path, rel: &str, scope: &str) -> Result<String, ResolveError> {
    match enclosing_package(workspace, rel) {
        Some((package, in_package)) => {
            if package.is_empty() {
                Ok(format!("//:{in_package}"))
            } else {
                Ok(format!("//{package}:{in_package}"))
            }
        }
        None => Err(ResolveError::NotAPackage {
            scope: scope.to_owned(),
        }),
    }
}

/// Maps a normalized relative directory path to its recursive pattern:
/// the workspace root becomes `//...`, anything else `//path/...`.
/// Recursion is performed by Bazel, never by filesystem traversal.
fn dir_pattern(rel: &str) -> String {
    if rel.is_empty() {
        "//...".to_owned()
    } else {
        format!("//{rel}/...")
    }
}

/// First non-empty stderr line, bounded for diagnostics.
fn first_line(bytes: &[u8]) -> String {
    const LIMIT: usize = 300;
    let text = String::from_utf8_lossy(bytes);
    let line = text.lines().map(str::trim).find(|line| !line.is_empty());
    let line = line.unwrap_or("no Bazel diagnostic");
    if line.len() > LIMIT {
        format!("{}...", &line[..LIMIT])
    } else {
        line.to_owned()
    }
}

/// Parses query stdout into canonical owner labels: trims lines, drops
/// empties, sorts bytewise, and deduplicates.
fn parse_owners(stdout: &[u8], label: &str) -> Result<Vec<String>, ResolveError> {
    let text = std::str::from_utf8(stdout).map_err(|_| ResolveError::QueryFailed {
        label: label.to_owned(),
        detail: "query output is not UTF-8".to_owned(),
    })?;
    let mut owners: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect();
    owners.sort();
    owners.dedup();
    Ok(owners)
}

/// Resolves one file scope to every direct source owner through Bazel
/// query. The file content and BUILD text are never read: ownership is
/// a graph fact reported on query stdout.
fn resolve_file(
    rel: &str,
    scope: &str,
    workspace: &Path,
    runner: &dyn QueryRunner,
) -> Result<Vec<String>, ResolveError> {
    if rel.chars().any(char::is_control) {
        return Err(ResolveError::UnsupportedName {
            scope: scope.to_owned(),
        });
    }
    let label = file_label(workspace, rel, scope)?;
    let argv = ownership_argv(&label);
    let result = runner
        .run_query(&argv, workspace)
        .map_err(|error| ResolveError::QueryFailed {
            label: label.clone(),
            detail: error.to_string(),
        })?;
    if result.code != Some(0) {
        return Err(ResolveError::QueryFailed {
            label: label.clone(),
            detail: first_line(&result.stderr),
        });
    }
    let owners = parse_owners(&result.stdout, &label)?;
    if owners.is_empty() {
        return Err(ResolveError::NoOwner {
            file: scope.to_owned(),
            label,
        });
    }
    Ok(owners)
}

/// Classifies one scope positional: main-workspace labels and patterns
/// pass through, external and package-relative labels fail, and
/// anything else is a workspace-relative file or directory path.
fn resolve_one(
    raw: &str,
    workspace: &Path,
    runner: &dyn QueryRunner,
    labels: &mut Vec<String>,
    resolved: &mut Vec<String>,
) -> Result<(), ResolveError> {
    if raw.starts_with("//") {
        labels.push(raw.to_owned());
        return Ok(());
    }
    if raw.starts_with('@') {
        return Err(ResolveError::ExternalScope {
            scope: raw.to_owned(),
        });
    }
    if raw.starts_with(':') {
        return Err(ResolveError::RelativeLabel {
            scope: raw.to_owned(),
        });
    }
    let rel = normalize_rel(raw)?;
    let entry = workspace.join(&rel);
    let metadata = std::fs::symlink_metadata(&entry).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            ResolveError::PathNotFound {
                scope: raw.to_owned(),
            }
        } else {
            ResolveError::QueryFailed {
                label: file_label(workspace, &rel, raw).unwrap_or_else(|_| raw.to_owned()),
                detail: error.to_string(),
            }
        }
    })?;
    if metadata.is_dir() {
        resolved.push(dir_pattern(&rel));
    } else if metadata.is_file() {
        resolved.extend(resolve_file(&rel, raw, workspace, runner)?);
    } else {
        return Err(ResolveError::NotFileOrDir {
            scope: raw.to_owned(),
        });
    }
    Ok(())
}

/// Resolves explicit scope positionals into exact Bazel targets.
///
/// Empty input selects the repository scope (`//...`). Label-only input
/// passes through in order. Once any file or directory resolves, every
/// target is canonicalized, deduplicated, and bytewise sorted under
/// [`Scope::ResolvedOwners`].
pub fn resolve(
    scopes: &[String],
    workspace: &Path,
    runner: &dyn QueryRunner,
) -> Result<ResolvedScope, ResolveError> {
    if scopes.is_empty() {
        return Ok(ResolvedScope {
            scope: Scope::Repository,
            targets: vec!["//...".to_owned()],
        });
    }
    let mut labels = Vec::new();
    let mut resolved = Vec::new();
    for raw in scopes {
        resolve_one(raw, workspace, runner, &mut labels, &mut resolved)?;
    }
    if resolved.is_empty() {
        return Ok(ResolvedScope {
            scope: Scope::Labels(labels.clone()),
            targets: labels,
        });
    }
    let mut targets = labels;
    targets.extend(resolved);
    targets.sort();
    targets.dedup();
    Ok(ResolvedScope {
        scope: Scope::ResolvedOwners(targets.clone()),
        targets,
    })
}

/// Resolves test/coverage scope: labels, patterns, and directories pass
/// through, while file scopes map through every direct owner to all
/// transitive reverse-dependent tests.
///
/// Returns the exact Bazel targets for `bazel test` / `bazel coverage`.
/// File owners with no reverse-dependent test are
/// [`ResolveError::NoTests`], never silent success. Directory scopes
/// stay recursive patterns for Bazel to expand; label scopes pass
/// through in order. With no file scope, this matches [`resolve`].
pub fn resolve_for_test(
    scopes: &[String],
    workspace: &Path,
    runner: &dyn QueryRunner,
) -> Result<ResolvedScope, ResolveError> {
    if scopes.is_empty() {
        return Ok(ResolvedScope {
            scope: Scope::Repository,
            targets: vec!["//...".to_owned()],
        });
    }
    let mut labels = Vec::new();
    let mut patterns = Vec::new();
    let mut file_owners = Vec::new();
    let mut saw_resolved = false;
    for raw in scopes {
        if raw.starts_with("//") {
            labels.push(raw.clone());
            continue;
        }
        if raw.starts_with('@') {
            return Err(ResolveError::ExternalScope { scope: raw.clone() });
        }
        if raw.starts_with(':') {
            return Err(ResolveError::RelativeLabel { scope: raw.clone() });
        }
        let rel = normalize_rel(raw)?;
        let entry = workspace.join(&rel);
        let metadata = std::fs::symlink_metadata(&entry).map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                ResolveError::PathNotFound { scope: raw.clone() }
            } else {
                ResolveError::QueryFailed {
                    label: file_label(workspace, &rel, raw).unwrap_or_else(|_| raw.clone()),
                    detail: error.to_string(),
                }
            }
        })?;
        saw_resolved = true;
        if metadata.is_dir() {
            patterns.push(dir_pattern(&rel));
        } else if metadata.is_file() {
            file_owners.extend(resolve_file(&rel, raw, workspace, runner)?);
        } else {
            return Err(ResolveError::NotFileOrDir { scope: raw.clone() });
        }
    }
    if file_owners.is_empty() {
        if !saw_resolved {
            return Ok(ResolvedScope {
                scope: Scope::Labels(labels.clone()),
                targets: labels,
            });
        }
        let mut targets = labels;
        targets.extend(patterns);
        targets.sort();
        targets.dedup();
        return Ok(ResolvedScope {
            scope: Scope::ResolvedOwners(targets.clone()),
            targets,
        });
    }
    file_owners.sort();
    file_owners.dedup();
    let tests = map_owners_to_tests(&file_owners, workspace, runner)?;
    let mut targets = labels;
    targets.extend(patterns);
    targets.extend(tests);
    targets.sort();
    targets.dedup();
    Ok(ResolvedScope {
        scope: Scope::ResolvedOwners(targets.clone()),
        targets,
    })
}

/// Resolves `dx run` scope to exact Bazel targets (O52).
///
/// Labels and target patterns pass through unchanged in order (Bazel
/// owns alias and executability) without any query. Once any file or
/// directory scope is present, every file maps to its depth-1 `_binary`
/// owners and every directory to its `_binary` rules under the
/// recursive pattern; explicit labels join the candidate set. Exactly
/// one candidate must remain: zero is [`ResolveError::NoRunnable`]
/// and multiple is [`ResolveError::AmbiguousRunnable`], both
/// operational failures (exit 1). An empty scope is
/// [`ResolveError::EmptyScope`] (pre-exec, exit 2): `bazel run`
/// needs an explicit target.
pub fn resolve_run(
    scopes: &[String],
    workspace: &Path,
    runner: &dyn QueryRunner,
) -> Result<Vec<String>, ResolveError> {
    if scopes.is_empty() {
        return Err(ResolveError::EmptyScope);
    }
    let mut labels = Vec::new();
    let mut has_path = false;
    for raw in scopes {
        if raw.starts_with("//") {
            labels.push(raw.clone());
        } else if raw.starts_with('@') {
            return Err(ResolveError::ExternalScope { scope: raw.clone() });
        } else if raw.starts_with(':') {
            return Err(ResolveError::RelativeLabel { scope: raw.clone() });
        } else {
            has_path = true;
        }
    }
    if !has_path {
        return Ok(labels);
    }
    let mut candidates: Vec<String> = labels;
    let mut scope_names = Vec::new();
    for raw in scopes {
        if raw.starts_with("//") || raw.starts_with('@') || raw.starts_with(':') {
            continue;
        }
        let rel = normalize_rel(raw)?;
        let entry = workspace.join(&rel);
        let metadata = std::fs::symlink_metadata(&entry).map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                ResolveError::PathNotFound { scope: raw.clone() }
            } else {
                ResolveError::QueryFailed {
                    label: raw.clone(),
                    detail: error.to_string(),
                }
            }
        })?;
        if metadata.is_dir() {
            if rel.chars().any(char::is_control) {
                return Err(ResolveError::UnsupportedName { scope: raw.clone() });
            }
            let pattern = dir_pattern(&rel);
            let found = run_label_query(&dir_runnable_expression(&pattern), workspace, runner)?;
            candidates.extend(found);
            scope_names.push(raw.clone());
        } else if metadata.is_file() {
            if rel.chars().any(char::is_control) {
                return Err(ResolveError::UnsupportedName { scope: raw.clone() });
            }
            let label = file_label(workspace, &rel, raw)?;
            let found = run_label_query(&runnable_expression(&label), workspace, runner)?;
            if found.is_empty() {
                // Distinguish "file owned but not executable" from "file
                // unowned": check plain ownership for guidance.
                let owned = run_label_query(&ownership_expression(&label), workspace, runner)?;
                if owned.is_empty() {
                    return Err(ResolveError::NoOwner {
                        file: raw.clone(),
                        label,
                    });
                }
            }
            candidates.extend(found);
            scope_names.push(raw.clone());
        } else {
            return Err(ResolveError::NotFileOrDir { scope: raw.clone() });
        }
    }
    candidates.sort();
    candidates.dedup();
    if candidates.is_empty() {
        scope_names.sort();
        scope_names.dedup();
        return Err(ResolveError::NoRunnable {
            scopes: scope_names,
        });
    }
    if candidates.len() > 1 {
        return Err(ResolveError::AmbiguousRunnable { candidates });
    }
    Ok(candidates)
}

/// One `bazel query` invocation per owner set (O44): every rule whose
/// kind ends in `_test` reaching the owners over the main-workspace
/// universe, at any depth. Owners are bytewise sorted so the expression
/// is deterministic.
fn tests_expression(owners: &[String]) -> String {
    let mut sorted: Vec<&String> = owners.iter().collect();
    sorted.sort();
    let set = sorted
        .iter()
        .map(|owner| quote_label(owner))
        .collect::<Vec<_>>()
        .join(" ");
    format!("kind('.*_test rule', rdeps(//..., set({set})))")
}

/// Maps direct source owners to every transitive reverse-dependent test
/// target through one unconfigured `bazel query` invocation.
///
/// Returned tests are canonicalized, deduplicated, and bytewise sorted;
/// no distance limit or package-location heuristic is applied, so
/// `select()`-gated branches stay conservatively included. An empty
/// mapping is [`ResolveError::NoTests`], never silent success: callers
/// must not pass a source-owning library to `bazel test` or
/// `bazel coverage` merely because it owns the file. An empty owner
/// set maps to no tests without touching Bazel.
pub fn map_owners_to_tests(
    owners: &[String],
    workspace: &Path,
    runner: &dyn QueryRunner,
) -> Result<Vec<String>, ResolveError> {
    if owners.is_empty() {
        return Ok(Vec::new());
    }
    let expression = tests_expression(owners);
    let mut argv = Vec::with_capacity(WORKFLOW_STARTUP_OPTS.len() + 4);
    argv.push(launcher_argv0().to_owned());
    argv.extend(WORKFLOW_STARTUP_OPTS.iter().map(ToString::to_string));
    argv.push("query".to_owned());
    argv.push("--".to_owned());
    argv.push(expression.clone());
    let result = runner
        .run_query(&argv, workspace)
        .map_err(|error| ResolveError::QueryFailed {
            label: expression.clone(),
            detail: error.to_string(),
        })?;
    if result.code != Some(0) {
        return Err(ResolveError::QueryFailed {
            label: expression,
            detail: first_line(&result.stderr),
        });
    }
    let tests = parse_owners(&result.stdout, &expression)?;
    if tests.is_empty() {
        let mut sorted = owners.to_vec();
        sorted.sort();
        return Err(ResolveError::NoTests { owners: sorted });
    }
    Ok(tests)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::path::PathBuf;

    /// Scripted query runner: records argv/cwd and replays canned
    /// outputs per expression in call order.
    struct FakeQuery {
        calls: RefCell<Vec<(Vec<String>, PathBuf)>>,
        outputs: RefCell<Vec<QueryResult>>,
    }

    impl FakeQuery {
        fn new(outputs: Vec<QueryResult>) -> Self {
            FakeQuery {
                calls: RefCell::new(Vec::new()),
                outputs: RefCell::new(outputs),
            }
        }

        fn ok(lines: &str) -> QueryResult {
            QueryResult {
                code: Some(0),
                stdout: lines.as_bytes().to_vec(),
                stderr: Vec::new(),
            }
        }

        fn failed(stderr: &str) -> QueryResult {
            QueryResult {
                code: Some(2),
                stdout: Vec::new(),
                stderr: stderr.as_bytes().to_vec(),
            }
        }

        fn calls(&self) -> Vec<(Vec<String>, PathBuf)> {
            self.calls.borrow().clone()
        }
    }

    impl QueryRunner for FakeQuery {
        fn run_query(&self, argv: &[String], cwd: &Path) -> io::Result<QueryResult> {
            self.calls
                .borrow_mut()
                .push((argv.to_vec(), cwd.to_path_buf()));
            Ok(self.outputs.borrow_mut().remove(0))
        }
    }

    /// Query runner that fails the test on any call: directory scopes
    /// must resolve without touching Bazel.
    struct NeverQuery;

    // LCOV_EXCL_START - reason: test-only guard; NeverQuery is never called.
    impl QueryRunner for NeverQuery {
        fn run_query(&self, _argv: &[String], _cwd: &Path) -> io::Result<QueryResult> {
            panic!("directory and label scopes must not run queries");
        }
    }
    // LCOV_EXCL_STOP - reason: end of NeverQuery exclusion.

    fn scopes(words: &[&str]) -> Vec<String> {
        words.iter().map(ToString::to_string).collect()
    }

    fn temp_workspace(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("dx-resolve-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp workspace");
        dir
    }

    fn write(workspace: &Path, rel: &str, text: &str) {
        let full = workspace.join(rel);
        std::fs::create_dir_all(full.parent().expect("parent")).expect("parent dir");
        std::fs::write(full, text).expect("write file");
    }

    fn cleanup(workspace: &Path) {
        let _ = std::fs::remove_dir_all(workspace);
    }

    #[test]
    fn empty_scope_selects_repository() {
        let query = NeverQuery;
        let workspace = temp_workspace("empty");
        let got = resolve(&[], &workspace, &query).expect("resolve");
        assert_eq!(got.scope, Scope::Repository);
        assert_eq!(got.targets, scopes(&["//..."]));
        cleanup(&workspace);
    }

    #[test]
    fn label_only_scopes_pass_through_in_order() {
        let query = NeverQuery;
        let workspace = temp_workspace("labels");
        let input = scopes(&["//b/...", "//a:one", "@repo//c/..."]);
        let err = resolve(&input, &workspace, &query).expect_err("external must fail");
        assert_eq!(
            err,
            ResolveError::ExternalScope {
                scope: "@repo//c/...".to_owned(),
            }
        );
        let input = scopes(&["//b/...", "//a:one"]);
        let got = resolve(&input, &workspace, &query).expect("resolve");
        assert_eq!(got.scope, Scope::Labels(scopes(&["//b/...", "//a:one"])));
        assert_eq!(got.targets, scopes(&["//b/...", "//a:one"]));
        cleanup(&workspace);
    }

    #[test]
    fn relative_labels_fail_with_guidance() {
        let query = NeverQuery;
        let workspace = temp_workspace("relative");
        let err = resolve(&scopes(&[":corpus"]), &workspace, &query).expect_err("relative");
        assert_eq!(
            err,
            ResolveError::RelativeLabel {
                scope: ":corpus".to_owned(),
            }
        );
        assert!(err.to_string().contains("//"));
        cleanup(&workspace);
    }

    #[test]
    fn file_resolves_through_single_rule_constrained_query() {
        let workspace = temp_workspace("file");
        write(&workspace, "pkg/BUILD.bazel", "");
        write(&workspace, "pkg/a.py", "x = 1\n");
        let query = FakeQuery::new(vec![FakeQuery::ok("//pkg:lib\n//pkg:lib\n//pkg:extra\n")]);
        let got = resolve(&scopes(&["pkg/a.py"]), &workspace, &query).expect("resolve");
        assert_eq!(
            got.targets,
            scopes(&["//pkg:extra", "//pkg:lib"]),
            "owners are sorted and deduplicated"
        );
        assert_eq!(
            got.scope,
            Scope::ResolvedOwners(scopes(&["//pkg:extra", "//pkg:lib"]))
        );
        let calls = query.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1, workspace, "query runs in the workspace");
        assert_eq!(
            calls[0].0,
            scopes(&[
                "bazel",
                "--nohome_rc",
                "--nosystem_rc",
                "query",
                "--",
                "kind('rule', rdeps(//..., \"//pkg:a.py\", 1))",
            ])
        );
        cleanup(&workspace);
    }

    #[test]
    fn file_argv_quotes_spaces_and_special_characters() {
        let workspace = temp_workspace("quoting");
        write(&workspace, "pkg/BUILD.bazel", "");
        write(&workspace, "pkg/my file.py", "x = 1\n");
        let query = FakeQuery::new(vec![FakeQuery::ok("//pkg:lib\n")]);
        resolve(&scopes(&["pkg/my file.py"]), &workspace, &query).expect("resolve");
        let calls = query.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].0.last().expect("expression"),
            "kind('rule', rdeps(//..., \"//pkg:my file.py\", 1))"
        );
        assert_eq!(quote_label("//pkg:a\"b\\c"), "\"//pkg:a\\\"b\\\\c\"");
        cleanup(&workspace);
    }

    #[test]
    fn root_file_maps_to_root_package_label() {
        let workspace = temp_workspace("root-file");
        write(&workspace, "BUILD.bazel", "");
        write(&workspace, "top.py", "x = 1\n");
        let query = FakeQuery::new(vec![FakeQuery::ok("//:lib\n")]);
        let got = resolve(&scopes(&["top.py"]), &workspace, &query).expect("resolve");
        assert_eq!(got.targets, scopes(&["//:lib"]));
        let calls = query.calls();
        assert!(calls[0]
            .0
            .last()
            .expect("expression")
            .contains("\"//:top.py\""));
        cleanup(&workspace);
    }

    #[test]
    fn build_file_text_is_never_consulted() {
        // The BUILD file names no target textually: `srcs` hide behind a
        // glob and a comment points at a decoy owner. Resolution still
        // succeeds because ownership comes only from query stdout.
        let workspace = temp_workspace("build-text");
        write(
            &workspace,
            "pkg/BUILD.bazel",
            "# owner: //decoy:not_real\nmy_files = glob([\"*.py\"])\n",
        );
        write(&workspace, "pkg/a.py", "x = 1\n");
        let query = FakeQuery::new(vec![FakeQuery::ok("//pkg:real\n")]);
        let got = resolve(&scopes(&["pkg/a.py"]), &workspace, &query).expect("resolve");
        assert_eq!(got.targets, scopes(&["//pkg:real"]));
        cleanup(&workspace);
    }

    #[test]
    fn directory_becomes_recursive_pattern_without_query_or_listing() {
        let workspace = temp_workspace("dir");
        write(&workspace, "src/nested/deep.py", "x = 1\n");
        write(&workspace, "src/top.py", "x = 1\n");
        let query = NeverQuery;
        let got = resolve(&scopes(&["src"]), &workspace, &query).expect("resolve");
        assert_eq!(got.targets, scopes(&["//src/..."]));
        assert_eq!(got.scope, Scope::ResolvedOwners(scopes(&["//src/..."])));
        cleanup(&workspace);
    }

    #[test]
    fn workspace_root_directory_maps_to_repository_pattern() {
        let workspace = temp_workspace("root-dir");
        let query = NeverQuery;
        for root in [".", "./"] {
            let got = resolve(&scopes(&[root]), &workspace, &query).expect("resolve");
            assert_eq!(got.targets, scopes(&["//..."]), "root {root}");
        }
        cleanup(&workspace);
    }

    #[test]
    fn mixed_labels_and_paths_merge_sorted_and_deduplicated() {
        let workspace = temp_workspace("mixed");
        write(&workspace, "pkg/BUILD.bazel", "");
        write(&workspace, "pkg/a.py", "x = 1\n");
        let query = FakeQuery::new(vec![FakeQuery::ok("//pkg:lib\n//z:z\n")]);
        let got = resolve(&scopes(&["//z:z", "pkg/a.py"]), &workspace, &query).expect("resolve");
        assert_eq!(got.targets, scopes(&["//pkg:lib", "//z:z"]));
        assert_eq!(
            got.scope,
            Scope::ResolvedOwners(scopes(&["//pkg:lib", "//z:z"]))
        );
        cleanup(&workspace);
    }

    #[test]
    fn missing_and_escaping_paths_fail() {
        let query = NeverQuery;
        let workspace = temp_workspace("missing");
        assert_eq!(
            resolve(&scopes(&["nope.py"]), &workspace, &query).expect_err("missing"),
            ResolveError::PathNotFound {
                scope: "nope.py".to_owned(),
            }
        );
        for escaping in ["/abs/path.py", "../escape.py", "pkg/../../escape.py"] {
            assert_eq!(
                resolve(&scopes(&[escaping]), &workspace, &query).expect_err("escape"),
                ResolveError::OutsideWorkspace {
                    scope: escaping.to_owned(),
                }
            );
        }
        assert_eq!(
            resolve(&scopes(&[""]), &workspace, &query).expect_err("empty"),
            ResolveError::EmptyScope
        );
        cleanup(&workspace);
    }

    #[test]
    fn non_file_entries_fail() {
        let workspace = temp_workspace("special");
        #[cfg(unix)]
        {
            use std::os::unix::net::UnixListener;
            let path = workspace.join("sock");
            let _listener = UnixListener::bind(&path).expect("bind socket");
            let query = NeverQuery;
            assert_eq!(
                resolve(&scopes(&["sock"]), &workspace, &query).expect_err("socket"),
                ResolveError::NotFileOrDir {
                    scope: "sock".to_owned(),
                }
            );
        }
        #[cfg(not(unix))]
        {
            let _ = workspace;
        }
        cleanup(&workspace);
    }

    #[test]
    fn control_characters_in_names_fail_before_query() {
        let workspace = temp_workspace("control");
        write(&workspace, "a\nb.py", "x = 1\n");
        let query = NeverQuery;
        assert_eq!(
            resolve(&scopes(&["a\nb.py"]), &workspace, &query).expect_err("control"),
            ResolveError::UnsupportedName {
                scope: "a\nb.py".to_owned(),
            }
        );
        cleanup(&workspace);
    }

    #[test]
    fn ownerless_files_suggest_explicit_labels() {
        let workspace = temp_workspace("no-owner");
        write(&workspace, "pkg/BUILD.bazel", "");
        write(&workspace, "pkg/orphan.py", "x = 1\n");
        let query = FakeQuery::new(vec![FakeQuery::ok("\n")]);
        let err = resolve(&scopes(&["pkg/orphan.py"]), &workspace, &query).expect_err("orphan");
        assert_eq!(
            err,
            ResolveError::NoOwner {
                file: "pkg/orphan.py".to_owned(),
                label: "//pkg:orphan.py".to_owned(),
            }
        );
        assert!(err.to_string().contains("explicit target label"));
        cleanup(&workspace);
    }

    #[test]
    fn query_failures_report_the_first_bazel_line() {
        let workspace = temp_workspace("query-fail");
        write(&workspace, "pkg/BUILD.bazel", "");
        write(&workspace, "pkg/a.py", "x = 1\n");
        let query = FakeQuery::new(vec![FakeQuery::failed(
            "\n  no such package 'pkg': BUILD file not found  \nmore context\n",
        )]);
        let err = resolve(&scopes(&["pkg/a.py"]), &workspace, &query).expect_err("failed");
        assert_eq!(
            err,
            ResolveError::QueryFailed {
                label: "//pkg:a.py".to_owned(),
                detail: "no such package 'pkg': BUILD file not found".to_owned(),
            }
        );
        let query = FakeQuery::new(vec![QueryResult {
            code: Some(0),
            stdout: vec![0xff, 0xfe],
            stderr: Vec::new(),
        }]);
        let err = resolve(&scopes(&["pkg/a.py"]), &workspace, &query).expect_err("non-utf8");
        assert_eq!(
            err,
            ResolveError::QueryFailed {
                label: "//pkg:a.py".to_owned(),
                detail: "query output is not UTF-8".to_owned(),
            }
        );
        cleanup(&workspace);
    }

    #[test]
    fn empty_stderr_reports_a_default_diagnostic() {
        assert_eq!(first_line(b""), "no Bazel diagnostic");
        assert_eq!(first_line(b"\n  \n"), "no Bazel diagnostic");
        let long = format!("x: {}", "y".repeat(400));
        let got = first_line(long.as_bytes());
        assert!(got.len() <= 303, "bounded: {}", got.len());
        assert!(got.ends_with("..."));
    }

    #[test]
    fn resolve_errors_render_actionable_messages() {
        let cases = [
            (ResolveError::EmptyScope, "empty scope"),
            (
                ResolveError::RelativeLabel {
                    scope: ":c".to_owned(),
                },
                ":c",
            ),
            (
                ResolveError::ExternalScope {
                    scope: "@r//p".to_owned(),
                },
                "@r//p",
            ),
            (
                ResolveError::OutsideWorkspace {
                    scope: "../x".to_owned(),
                },
                "../x",
            ),
            (
                ResolveError::PathNotFound {
                    scope: "n.py".to_owned(),
                },
                "n.py",
            ),
            (
                ResolveError::NotFileOrDir {
                    scope: "s".to_owned(),
                },
                "unsupported path",
            ),
            (
                ResolveError::NotAPackage {
                    scope: "docs/g.md".to_owned(),
                },
                "not a package",
            ),
            (
                ResolveError::UnsupportedName {
                    scope: "n".to_owned(),
                },
                "control characters",
            ),
            (
                ResolveError::NoOwner {
                    file: "f.py".to_owned(),
                    label: "//:f.py".to_owned(),
                },
                "f.py",
            ),
            (
                ResolveError::QueryFailed {
                    label: "//:f.py".to_owned(),
                    detail: "boom".to_owned(),
                },
                "boom",
            ),
        ];
        for (error, needle) in cases {
            assert!(error.to_string().contains(needle), "{error:?}");
        }
    }

    #[test]
    fn file_labels_use_the_nearest_enclosing_package() {
        let workspace = temp_workspace("pkg-labels");
        write(&workspace, "pkg/BUILD.bazel", "");
        write(&workspace, "pkg/a.py", "x = 1\n");
        write(&workspace, "pkg/src/deep/b.py", "x = 1\n");
        write(&workspace, "BUILD.bazel", "");
        write(&workspace, "top.py", "x = 1\n");
        assert_eq!(
            file_label(&workspace, "pkg/a.py", "pkg/a.py").expect("label"),
            "//pkg:a.py"
        );
        assert_eq!(
            file_label(&workspace, "pkg/src/deep/b.py", "pkg/src/deep/b.py").expect("label"),
            "//pkg:src/deep/b.py"
        );
        assert_eq!(
            file_label(&workspace, "top.py", "top.py").expect("label"),
            "//:top.py"
        );
        assert_eq!(dir_pattern(""), "//...");
        assert_eq!(dir_pattern("src"), "//src/...");
        assert_eq!(normalize_rel("./pkg/./a.py").expect("dots"), "pkg/a.py");
        assert_eq!(normalize_rel("pkg//a.py").expect("doubles"), "pkg/a.py");
        cleanup(&workspace);
    }

    #[test]
    fn bare_build_marker_and_nearest_package_win() {
        let workspace = temp_workspace("pkg-markers");
        write(&workspace, "legacy/BUILD", "");
        write(&workspace, "legacy/a.py", "x = 1\n");
        write(&workspace, "outer/BUILD.bazel", "");
        write(&workspace, "outer/inner/BUILD.bazel", "");
        write(&workspace, "outer/inner/a.py", "x = 1\n");
        write(&workspace, "outer/loose.py", "x = 1\n");
        assert_eq!(
            file_label(&workspace, "legacy/a.py", "legacy/a.py").expect("label"),
            "//legacy:a.py"
        );
        assert_eq!(
            file_label(&workspace, "outer/inner/a.py", "outer/inner/a.py").expect("label"),
            "//outer/inner:a.py"
        );
        assert_eq!(
            file_label(&workspace, "outer/loose.py", "outer/loose.py").expect("label"),
            "//outer:loose.py"
        );
        cleanup(&workspace);
    }

    #[test]
    fn files_without_any_enclosing_package_fail() {
        let workspace = temp_workspace("no-package");
        write(&workspace, "docs/guide.md", "# guide\n");
        write(&workspace, "other/BUILD.bazel", "");
        let err =
            resolve(&scopes(&["docs/guide.md"]), &workspace, &NeverQuery).expect_err("no package");
        assert_eq!(
            err,
            ResolveError::NotAPackage {
                scope: "docs/guide.md".to_owned(),
            }
        );
        assert!(err.to_string().contains("not a package"), "{err}");
        cleanup(&workspace);
    }

    #[test]
    fn test_mapping_queries_transitive_test_owners() {
        let workspace = temp_workspace("test-map");
        let query = FakeQuery::new(vec![FakeQuery::ok("//pkg:unit\n//pkg:e2e\n//pkg:unit\n")]);
        let got = map_owners_to_tests(&scopes(&["//pkg:lib"]), &workspace, &query).expect("map");
        assert_eq!(got, scopes(&["//pkg:e2e", "//pkg:unit"]));
        let calls = query.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1, workspace, "mapping runs in the workspace");
        assert_eq!(
            calls[0].0,
            scopes(&[
                "bazel",
                "--nohome_rc",
                "--nosystem_rc",
                "query",
                "--",
                "kind('.*_test rule', rdeps(//..., set(\"//pkg:lib\")))",
            ])
        );
        cleanup(&workspace);
    }

    #[test]
    fn test_mapping_sorts_owners_in_set_expression() {
        let workspace = temp_workspace("test-map-order");
        let query = FakeQuery::new(vec![FakeQuery::ok("//t:t\n")]);
        map_owners_to_tests(&scopes(&["//z:lib", "//a:lib"]), &workspace, &query).expect("map");
        let calls = query.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].0.last().expect("expression"),
            "kind('.*_test rule', rdeps(//..., set(\"//a:lib\" \"//z:lib\")))"
        );
        cleanup(&workspace);
    }

    #[test]
    fn empty_test_mapping_suggests_explicit_label() {
        let workspace = temp_workspace("test-map-empty");
        let query = FakeQuery::new(vec![FakeQuery::ok("\n")]);
        let err = map_owners_to_tests(&scopes(&["//pkg:lib"]), &workspace, &query)
            .expect_err("empty mapping");
        assert_eq!(
            err,
            ResolveError::NoTests {
                owners: scopes(&["//pkg:lib"]),
            }
        );
        assert!(
            err.to_string().contains("explicit test label"),
            "actionable: {err}"
        );
        cleanup(&workspace);
    }

    #[test]
    fn test_mapping_failures_report_the_first_bazel_line() {
        let workspace = temp_workspace("test-map-fail");
        let query = FakeQuery::new(vec![FakeQuery::failed("\n  query failed: blah  \nmore\n")]);
        let err =
            map_owners_to_tests(&scopes(&["//pkg:lib"]), &workspace, &query).expect_err("failed");
        assert_eq!(
            err,
            ResolveError::QueryFailed {
                label: "kind('.*_test rule', rdeps(//..., set(\"//pkg:lib\")))".to_owned(),
                detail: "query failed: blah".to_owned(),
            }
        );
        cleanup(&workspace);
    }

    #[test]
    fn empty_owners_map_to_no_tests_without_query() {
        let workspace = temp_workspace("test-map-no-query");
        let query = NeverQuery;
        let got = map_owners_to_tests(&[], &workspace, &query).expect("map");
        assert!(got.is_empty());
        cleanup(&workspace);
    }

    #[test]
    fn no_tests_error_renders_owners_and_guidance() {
        let error = ResolveError::NoTests {
            owners: scopes(&["//a:lib", "//b:lib"]),
        };
        let text = error.to_string();
        assert!(text.contains("//a:lib //b:lib"), "{text}");
        assert!(text.contains("//pkg/..."), "{text}");
    }

    #[test]
    fn run_labels_pass_through_without_query() {
        let workspace = temp_workspace("run-labels");
        let query = NeverQuery;
        let got = resolve_run(&scopes(&["//app:bin"]), &workspace, &query).expect("resolve");
        assert_eq!(got, scopes(&["//app:bin"]));
        cleanup(&workspace);
    }

    #[test]
    fn run_empty_scope_is_a_usage_error() {
        let workspace = temp_workspace("run-empty");
        let query = NeverQuery;
        let err = resolve_run(&[], &workspace, &query).expect_err("empty");
        assert_eq!(err, ResolveError::EmptyScope);
        cleanup(&workspace);
    }

    #[test]
    fn run_file_resolves_single_binary_owner() {
        let workspace = temp_workspace("run-file");
        write(&workspace, "app/BUILD.bazel", "");
        write(&workspace, "app/main.py", "x = 1\n");
        let query = FakeQuery::new(vec![FakeQuery::ok("//app:bin\n")]);
        let got = resolve_run(&scopes(&["app/main.py"]), &workspace, &query).expect("resolve");
        assert_eq!(got, scopes(&["//app:bin"]));
        let calls = query.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].0.last().expect("expression"),
            "kind('.*_binary rule', rdeps(//..., \"//app:main.py\", 1))"
        );
        cleanup(&workspace);
    }

    #[test]
    fn run_file_without_binary_reports_no_runnable() {
        let workspace = temp_workspace("run-no-bin");
        write(&workspace, "pkg/BUILD.bazel", "");
        write(&workspace, "pkg/a.py", "x = 1\n");
        // No `_binary` owner, but a plain owner exists for guidance.
        let query = FakeQuery::new(vec![FakeQuery::ok("\n"), FakeQuery::ok("//pkg:lib\n")]);
        let err = resolve_run(&scopes(&["pkg/a.py"]), &workspace, &query).expect_err("no runnable");
        assert_eq!(
            err,
            ResolveError::NoRunnable {
                scopes: scopes(&["pkg/a.py"]),
            }
        );
        assert!(err.to_string().contains("no executable target"), "{err}");
        cleanup(&workspace);
    }

    #[test]
    fn run_file_without_any_owner_reports_no_owner() {
        let workspace = temp_workspace("run-no-owner");
        write(&workspace, "pkg/BUILD.bazel", "");
        write(&workspace, "pkg/a.py", "x = 1\n");
        let query = FakeQuery::new(vec![FakeQuery::ok("\n"), FakeQuery::ok("\n")]);
        let err = resolve_run(&scopes(&["pkg/a.py"]), &workspace, &query).expect_err("no owner");
        assert_eq!(
            err,
            ResolveError::NoOwner {
                file: "pkg/a.py".to_owned(),
                label: "//pkg:a.py".to_owned(),
            }
        );
        cleanup(&workspace);
    }

    #[test]
    fn run_dir_with_two_binaries_reports_ambiguous_candidates() {
        let workspace = temp_workspace("run-ambiguous");
        std::fs::create_dir_all(workspace.join("app")).expect("dir");
        let query = FakeQuery::new(vec![FakeQuery::ok("//app:two\n//app:one\n")]);
        let err = resolve_run(&scopes(&["app"]), &workspace, &query).expect_err("ambiguous");
        assert_eq!(
            err,
            ResolveError::AmbiguousRunnable {
                candidates: scopes(&["//app:one", "//app:two"]),
            }
        );
        assert!(
            err.to_string().contains("multiple executable targets"),
            "{err}"
        );
        let calls = query.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].0.last().expect("expression"),
            "kind('.*_binary rule', //app/...)"
        );
        cleanup(&workspace);
    }

    #[test]
    fn test_scope_maps_files_to_tests_and_keeps_patterns() {
        let workspace = temp_workspace("test-scope");
        write(&workspace, "pkg/BUILD.bazel", "");
        write(&workspace, "pkg/a.py", "x = 1\n");
        let query = FakeQuery::new(vec![
            FakeQuery::ok("//pkg:lib\n"),
            FakeQuery::ok("//pkg:unit\n"),
        ]);
        let got = resolve_for_test(&scopes(&["pkg/a.py", "//other/..."]), &workspace, &query)
            .expect("resolve");
        assert_eq!(
            got.targets,
            scopes(&["//other/...", "//pkg:unit"]),
            "owners are replaced by mapped tests"
        );
        assert_eq!(
            query.calls().len(),
            2,
            "one ownership query plus one mapping"
        );
        cleanup(&workspace);
    }

    #[test]
    fn test_scope_without_files_matches_plain_resolve() {
        let workspace = temp_workspace("test-scope-labels");
        let query = NeverQuery;
        let got = resolve_for_test(&scopes(&["//a:one", "//b/..."]), &workspace, &query)
            .expect("resolve");
        assert_eq!(got.targets, scopes(&["//a:one", "//b/..."]));
        assert_eq!(got.scope, Scope::Labels(scopes(&["//a:one", "//b/..."])));
        cleanup(&workspace);
    }

    struct FailIo;

    impl QueryRunner for FailIo {
        fn run_query(&self, _argv: &[String], _cwd: &Path) -> io::Result<QueryResult> {
            Err(io::Error::other("boom"))
        }
    }

    #[test]
    fn query_io_errors_become_query_failed() {
        let workspace = temp_workspace("query-io");
        write(&workspace, "pkg/BUILD.bazel", "");
        write(&workspace, "pkg/a.py", "x = 1\n");
        let err = resolve(&scopes(&["pkg/a.py"]), &workspace, &FailIo).expect_err("io");
        assert!(matches!(err, ResolveError::QueryFailed { .. }), "{err:?}");
        let err = resolve_run(&scopes(&["pkg/a.py"]), &workspace, &FailIo).expect_err("io");
        assert!(matches!(err, ResolveError::QueryFailed { .. }), "{err:?}");
        let err =
            map_owners_to_tests(&scopes(&["//pkg:lib"]), &workspace, &FailIo).expect_err("io");
        assert!(matches!(err, ResolveError::QueryFailed { .. }), "{err:?}");
        cleanup(&workspace);
    }

    #[test]
    fn runnable_query_failures_report_first_line() {
        let workspace = temp_workspace("run-query-fail");
        write(&workspace, "app/BUILD.bazel", "");
        write(&workspace, "app/main.py", "x = 1\n");
        let query = FakeQuery::new(vec![FakeQuery::failed("nope\n")]);
        let err = resolve_run(&scopes(&["app/main.py"]), &workspace, &query).expect_err("fail");
        assert!(matches!(err, ResolveError::QueryFailed { .. }), "{err:?}");
        cleanup(&workspace);
    }

    #[test]
    fn not_a_directory_maps_to_query_failed() {
        let workspace = temp_workspace("not-a-dir");
        write(&workspace, "pkg", "file, not dir\n");
        let query = NeverQuery;
        let err = resolve(&scopes(&["pkg/a.py"]), &workspace, &query).expect_err("enotdir");
        assert!(matches!(err, ResolveError::QueryFailed { .. }), "{err:?}");
        let err =
            resolve_for_test(&scopes(&["pkg/a.py"]), &workspace, &query).expect_err("enotdir");
        assert!(matches!(err, ResolveError::QueryFailed { .. }), "{err:?}");
        let err = resolve_run(&scopes(&["pkg/a.py"]), &workspace, &query).expect_err("enotdir");
        assert!(matches!(err, ResolveError::QueryFailed { .. }), "{err:?}");
        cleanup(&workspace);
    }

    #[test]
    fn test_scope_rejects_external_and_relative() {
        let workspace = temp_workspace("test-scope-reject");
        let query = NeverQuery;
        assert_eq!(
            resolve_for_test(&scopes(&["@r//p"]), &workspace, &query).expect_err("ext"),
            ResolveError::ExternalScope {
                scope: "@r//p".to_owned(),
            }
        );
        assert_eq!(
            resolve_for_test(&scopes(&[":c"]), &workspace, &query).expect_err("rel"),
            ResolveError::RelativeLabel {
                scope: ":c".to_owned(),
            }
        );
        assert_eq!(
            resolve_run(&scopes(&["@r//p"]), &workspace, &query).expect_err("ext"),
            ResolveError::ExternalScope {
                scope: "@r//p".to_owned(),
            }
        );
        assert_eq!(
            resolve_run(&scopes(&[":c"]), &workspace, &query).expect_err("rel"),
            ResolveError::RelativeLabel {
                scope: ":c".to_owned(),
            }
        );
        cleanup(&workspace);
    }

    #[test]
    fn test_scope_missing_files_fail() {
        let workspace = temp_workspace("test-scope-missing");
        let query = NeverQuery;
        assert_eq!(
            resolve_for_test(&scopes(&["nope.py"]), &workspace, &query).expect_err("missing"),
            ResolveError::PathNotFound {
                scope: "nope.py".to_owned(),
            }
        );
        assert_eq!(
            resolve_run(&scopes(&["nope.py"]), &workspace, &query).expect_err("missing"),
            ResolveError::PathNotFound {
                scope: "nope.py".to_owned(),
            }
        );
        cleanup(&workspace);
    }

    #[test]
    fn test_scope_dir_only_resolves_without_query() {
        let workspace = temp_workspace("test-scope-dir");
        std::fs::create_dir_all(workspace.join("app")).expect("dir");
        let query = NeverQuery;
        let got = resolve_for_test(&scopes(&["app"]), &workspace, &query).expect("dir");
        assert_eq!(got.targets, scopes(&["//app/..."]));
        cleanup(&workspace);
    }

    #[test]
    fn run_mixed_label_and_file_skips_label_in_second_pass() {
        let workspace = temp_workspace("run-mixed");
        write(&workspace, "app/BUILD.bazel", "");
        write(&workspace, "app/main.py", "x = 1\n");
        let query = FakeQuery::new(vec![FakeQuery::ok("//app:bin\n")]);
        let got =
            resolve_run(&scopes(&["//app:bin", "app/main.py"]), &workspace, &query).expect("mixed");
        assert_eq!(got, scopes(&["//app:bin"]));
        assert_eq!(query.calls().len(), 1);
        cleanup(&workspace);
    }

    #[test]
    fn symlinks_are_neither_file_nor_dir() {
        let workspace = temp_workspace("symlink-kind");
        write(&workspace, "target.txt", "x\n");
        #[cfg(unix)]
        std::os::unix::fs::symlink(workspace.join("target.txt"), workspace.join("link"))
            .expect("link");
        #[cfg(unix)]
        {
            let query = NeverQuery;
            assert!(matches!(
                resolve(&scopes(&["link"]), &workspace, &query).expect_err("link"),
                ResolveError::NotFileOrDir { .. }
            ));
            assert!(matches!(
                resolve_for_test(&scopes(&["link"]), &workspace, &query).expect_err("link"),
                ResolveError::NotFileOrDir { .. }
            ));
            assert!(matches!(
                resolve_run(&scopes(&["link"]), &workspace, &query).expect_err("link"),
                ResolveError::NotFileOrDir { .. }
            ));
        }
        cleanup(&workspace);
    }

    #[test]
    fn control_chars_in_names_are_rejected() {
        let workspace = temp_workspace("control-names");
        let dir = "app\x01";
        let file = "app\x01/main.py";
        std::fs::create_dir_all(workspace.join(dir)).expect("dir");
        write(&workspace, file, "x = 1\n");
        write(&workspace, "app\x01/BUILD.bazel", "");
        let query = NeverQuery;
        assert!(matches!(
            resolve_run(&scopes(&[dir]), &workspace, &query).expect_err("dir"),
            ResolveError::UnsupportedName { .. }
        ));
        assert!(matches!(
            resolve_run(&scopes(&[file]), &workspace, &query).expect_err("file"),
            ResolveError::UnsupportedName { .. }
        ));
        cleanup(&workspace);
    }
}
