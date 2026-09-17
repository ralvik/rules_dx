//! Scope resolution: labels, patterns, files, directories, and
//! test/coverage mapping (M08 WP1+WP2).
//!
//! Contract: `docs/cli/target-resolution.md#input-classification` and
//! `#file-ownership`. Main-workspace labels and target patterns pass
//! through after workspace validation; existing files resolve to every
//! direct source owner through one unconfigured `bazel query` invocation
//! per resolver call, with every file label quoted into a single
//! deterministic set and addressed by the nearest enclosing package;
//! directories become recursive Bazel patterns without
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

/// Quotes every item into one deterministic space-separated set literal:
/// items are bytewise sorted so the query expression is stable and
/// inspectable no matter the input order.
fn quote_set(items: &[String]) -> String {
    let mut sorted: Vec<&String> = items.iter().collect();
    sorted.sort();
    sorted
        .iter()
        .map(|item| quote_label(item))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Batched ownership expression (O44): depth-1 reverse dependencies
/// constrained to rules over the main-workspace universe, with every file
/// label quoted into one deterministic set. One bounded query per
/// resolver call no matter how many files share the scope; an empty
/// mapping names the first file scope so the diagnostic stays actionable.
fn ownership_set_expression(labels: &[String]) -> String {
    format!("kind('rule', rdeps(//..., set({}), 1))", quote_set(labels))
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

/// Batched runnable file-owner expression (O52): depth-1 reverse
/// dependencies constrained to rules whose kind ends in `_binary`, with
/// every file label quoted into one deterministic set. Aliases are not
/// followed for file scopes: only direct `_binary` owners qualify.
fn runnable_set_expression(labels: &[String]) -> String {
    format!(
        "kind('.*_binary rule', rdeps(//..., set({}), 1))",
        quote_set(labels)
    )
}

/// Runnable directory expression (O52): every `_binary` rule under the
/// recursive pattern. Recursion is performed by Bazel, never by
/// filesystem traversal.
fn dir_runnable_expression(pattern: &str) -> String {
    format!("kind('.*_binary rule', {pattern})")
}

/// True for explicit `dx run` target patterns needing Bazel-owned
/// expansion (multirun #186): labels containing `...` or `*` such as
/// `//demo/...` or `//demo:*`. Plain labels (`//demo:frontend`)
/// pass through without a query; Bazel owns their alias and
/// executability.
fn is_run_pattern(label: &str) -> bool {
    label.contains("...") || label.contains('*')
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

/// Memoized package-marker probes for one resolver call. Only ancestor
/// directories of input files are ever probed: label and directory
/// scopes never trigger package walks, and each directory's marker
/// existence is probed at most once no matter how many files share the
/// enclosing package.
#[derive(Default)]
struct PackageCache {
    /// Directory ("" for the workspace root) to marker presence.
    is_package: std::collections::HashMap<String, bool>,
}

impl PackageCache {
    /// Reports whether `dir` holds a package marker, probing once.
    fn is_package(&mut self, workspace: &Path, dir: &str) -> bool {
        if let Some(hit) = self.is_package.get(dir) {
            return *hit;
        }
        let base = if dir.is_empty() {
            workspace.to_path_buf()
        } else {
            workspace.join(dir)
        };
        let found = PACKAGE_FILES
            .iter()
            .any(|marker| std::fs::symlink_metadata(base.join(marker)).is_ok());
        self.is_package.insert(dir.to_owned(), found);
        found
    }

    /// Finds the nearest enclosing Bazel package for a normalized relative
    /// file path by walking from the parent directory up to the workspace
    /// root. Returns `(package, path_in_package)`, where the root package
    /// is `""`. Returns `None` when no directory in the chain holds a
    /// package marker.
    fn enclosing(&mut self, workspace: &Path, rel: &str) -> Option<(String, String)> {
        let mut dir = match rel.rfind('/') {
            Some(index) => &rel[..index],
            None => "",
        };
        loop {
            if self.is_package(workspace, dir) {
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
    fn file_label(
        &mut self,
        workspace: &Path,
        rel: &str,
        scope: &str,
    ) -> Result<String, ResolveError> {
        match self.enclosing(workspace, rel) {
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

/// One file scope after classification: the original scope text for
/// diagnostics and the source label through the nearest enclosing
/// package for queries.
struct FileScope {
    /// Original scope positional, for [`ResolveError::NoOwner`].
    scope: String,
    /// Source label addressing the file in query syntax.
    label: String,
}

/// Scope positionals classified once per resolver call and shared by the
/// plain, test, and run paths so they cannot drift apart: labels pass
/// through, directories become recursive patterns without filesystem
/// enumeration, and files carry their source labels for one batched
/// ownership query.
struct ClassifiedScopes {
    /// Main-workspace labels passing through in input order.
    labels: Vec<String>,
    /// File scopes in input order with their source labels.
    files: Vec<FileScope>,
    /// Recursive patterns for directory scopes in input order.
    patterns: Vec<String>,
    /// Original file and directory positionals in input order, for
    /// [`ResolveError::NoRunnable`].
    paths: Vec<String>,
}

/// Classifies every scope positional: main-workspace labels pass
/// through, external and package-relative labels fail, and anything else
/// is a workspace-relative file or directory path. Package-marker walks
/// run only for file scopes through the shared cache, so directory and
/// label scopes never trigger enclosing-package probes.
fn classify_scopes(
    scopes: &[String],
    workspace: &Path,
    cache: &mut PackageCache,
) -> Result<ClassifiedScopes, ResolveError> {
    let mut classified = ClassifiedScopes {
        labels: Vec::new(),
        files: Vec::new(),
        patterns: Vec::new(),
        paths: Vec::new(),
    };
    for raw in scopes {
        if raw.starts_with("//") {
            classified.labels.push(raw.clone());
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
                    label: cache
                        .file_label(workspace, &rel, raw)
                        .unwrap_or_else(|_| raw.clone()),
                    detail: error.to_string(),
                }
            }
        })?;
        if metadata.is_dir() {
            if rel.chars().any(char::is_control) {
                return Err(ResolveError::UnsupportedName { scope: raw.clone() });
            }
            classified.patterns.push(dir_pattern(&rel));
            classified.paths.push(raw.clone());
        } else if metadata.is_file() {
            if rel.chars().any(char::is_control) {
                return Err(ResolveError::UnsupportedName { scope: raw.clone() });
            }
            let label = cache.file_label(workspace, &rel, raw)?;
            classified.files.push(FileScope {
                scope: raw.clone(),
                label,
            });
            classified.paths.push(raw.clone());
        } else {
            return Err(ResolveError::NotFileOrDir { scope: raw.clone() });
        }
    }
    Ok(classified)
}

/// Resolves every classified file scope to its direct source owners
/// through one bounded `bazel query` invocation over the batched label
/// set. The file content and BUILD text are never read: ownership is a
/// graph fact reported on query stdout. An empty mapping names the first
/// file scope, since per-file attribution is not observable from a union.
fn resolve_file_owners(
    files: &[FileScope],
    workspace: &Path,
    runner: &dyn QueryRunner,
) -> Result<Vec<String>, ResolveError> {
    let labels: Vec<String> = files.iter().map(|file| file.label.clone()).collect();
    let expression = ownership_set_expression(&labels);
    let owners = run_label_query(&expression, workspace, runner)?;
    if owners.is_empty() {
        let first = &files[0];
        return Err(ResolveError::NoOwner {
            file: first.scope.clone(),
            label: first.label.clone(),
        });
    }
    Ok(owners)
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
    let mut cache = PackageCache::default();
    let classified = classify_scopes(scopes, workspace, &mut cache)?;
    if classified.files.is_empty() && classified.patterns.is_empty() {
        return Ok(ResolvedScope {
            scope: Scope::Labels(classified.labels.clone()),
            targets: classified.labels,
        });
    }
    let mut targets = classified.labels;
    if !classified.files.is_empty() {
        targets.extend(resolve_file_owners(&classified.files, workspace, runner)?);
    }
    targets.extend(classified.patterns);
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
    let mut cache = PackageCache::default();
    let classified = classify_scopes(scopes, workspace, &mut cache)?;
    if classified.files.is_empty() {
        if classified.patterns.is_empty() {
            return Ok(ResolvedScope {
                scope: Scope::Labels(classified.labels.clone()),
                targets: classified.labels,
            });
        }
        let mut targets = classified.labels;
        targets.extend(classified.patterns);
        targets.sort();
        targets.dedup();
        return Ok(ResolvedScope {
            scope: Scope::ResolvedOwners(targets.clone()),
            targets,
        });
    }
    let file_owners = resolve_file_owners(&classified.files, workspace, runner)?;
    let tests = map_owners_to_tests(&file_owners, workspace, runner)?;
    let mut targets = classified.labels;
    targets.extend(classified.patterns);
    targets.extend(tests);
    targets.sort();
    targets.dedup();
    Ok(ResolvedScope {
        scope: Scope::ResolvedOwners(targets.clone()),
        targets,
    })
}

/// Resolves `dx run` scope to exact Bazel targets (O52, multirun #186).
///
/// Explicit labels pass through unchanged in order (Bazel owns alias
/// and executability) without any query. Explicit target patterns
/// (labels containing `...` or `*`, e.g. `//demo/...`) expand through
/// one Bazel-owned `kind('.*_binary rule', <pattern>)` query each into
/// their runnable labels, in input order. Once any file or directory
/// scope is present, every file maps to its depth-1 `_binary` owners
/// and every directory to its `_binary` rules under the recursive
/// pattern; explicit labels join the candidate set. Exactly one
/// candidate must remain on that inference path: zero is
/// [`ResolveError::NoRunnable`] and multiple is
/// [`ResolveError::AmbiguousRunnable`], both operational failures
/// (exit 1). An empty expansion of explicit patterns is likewise
/// [`ResolveError::NoRunnable`]. An empty scope is
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
    let mut cache = PackageCache::default();
    let classified = classify_scopes(scopes, workspace, &mut cache)?;
    if classified.files.is_empty() && classified.patterns.is_empty() {
        // Explicit-label multirun (#186): plain labels pass through in
        // input order with no query; patterns containing `...` or `*`
        // expand inline through the same Bazel-owned `_binary` kind
        // query as directory scopes. File/directory inference below is
        // untouched, so multirun never activates on inference.
        if !classified.labels.iter().any(|label| is_run_pattern(label)) {
            return Ok(classified.labels);
        }
        let mut targets: Vec<String> = Vec::new();
        for label in &classified.labels {
            if is_run_pattern(label) {
                targets.extend(run_label_query(
                    &dir_runnable_expression(label),
                    workspace,
                    runner,
                )?);
            } else {
                targets.push(label.clone());
            }
        }
        // Preserve first-seen order across inline expansions; the query
        // helper already sorts each expansion batch.
        let mut seen = std::collections::HashSet::new();
        targets.retain(|target| seen.insert(target.clone()));
        if targets.is_empty() {
            return Err(ResolveError::NoRunnable {
                scopes: scopes.to_vec(),
            });
        }
        return Ok(targets);
    }
    let mut candidates: Vec<String> = classified.labels;
    if !classified.files.is_empty() {
        let labels: Vec<String> = classified
            .files
            .iter()
            .map(|file| file.label.clone())
            .collect();
        let found = run_label_query(&runnable_set_expression(&labels), workspace, runner)?;
        if found.is_empty() {
            // Distinguish "files owned but not executable" from "files
            // unowned": check plain ownership once for the batch.
            let owned = run_label_query(&ownership_set_expression(&labels), workspace, runner)?;
            if owned.is_empty() {
                let first = &classified.files[0];
                return Err(ResolveError::NoOwner {
                    file: first.scope.clone(),
                    label: first.label.clone(),
                });
            }
        } else {
            candidates.extend(found);
        }
    }
    for pattern in &classified.patterns {
        let found = run_label_query(&dir_runnable_expression(pattern), workspace, runner)?;
        candidates.extend(found);
    }
    candidates.sort();
    candidates.dedup();
    if candidates.is_empty() {
        let mut paths = classified.paths;
        paths.sort();
        paths.dedup();
        return Err(ResolveError::NoRunnable { scopes: paths });
    }
    if candidates.len() > 1 {
        return Err(ResolveError::AmbiguousRunnable { candidates });
    }
    Ok(candidates)
}

/// Resolves `dx deploy` scope to exactly one main-workspace label.
///
/// Exactly one positional is required; patterns (`//...`,
/// `//pkg/...`), file/directory paths, external labels (`@repo//...`),
/// and package-relative labels (`:target`) are usage failures with
/// guidance to pass a deploy label. No filesystem access and no Bazel
/// query: label shape alone decides.
pub fn resolve_deploy(scopes: &[String]) -> Result<String, ResolveError> {
    if scopes.len() != 1 {
        return Err(ResolveError::DeployCount {
            count: scopes.len(),
        });
    }
    let scope = &scopes[0];
    if scope.starts_with("//") {
        if scope.contains("...") {
            return Err(ResolveError::DeployScope {
                scope: scope.clone(),
            });
        }
        return Ok(scope.clone());
    }
    Err(ResolveError::DeployScope {
        scope: scope.clone(),
    })
}

/// Deploy target identity from one `bazel cquery` Starlark evaluation:
/// the `DxDeployInfo` provider fields plus raw executability. Bazel
/// owns executability, so aliases resolve before this observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployInfo {
    /// The deploy label as passed.
    pub label: String,
    /// True when the target returns `DxDeployInfo`.
    pub has_provider: bool,
    /// Raw `profile` attribute (`debug`/`dev`/`release`, `None` when the
    /// provider omits it, `NONE` when no provider). Callers parse with
    /// `Profile::parse_attr`.
    pub profile_raw: String,
    /// Raw `app` attribute (canonical label, `None` when the program
    /// itself deploys, `NONE` when no provider).
    pub app_raw: String,
    /// True when `files_to_run.executable` exists.
    pub executable: bool,
}

/// Starlark expression behind [`check_deployable`]: `has|profile|app|exe`.
/// `NONE` marks a missing provider; `None` marks a provider field set
/// to None. Single expression so one cquery answers deployability,
/// profile default, and app identity together.
fn deploy_starlark_expr() -> String {
    const PROVIDER: &str = "//deploy/rules:defs.bzl%DxDeployInfo";
    format!(
        "(str(providers(target).get('{PROVIDER}', None) != None)) + '|' + \
         ((str(providers(target).get('{PROVIDER}', None).profile) if providers(target).get('{PROVIDER}', None) != None else 'NONE')) + '|' + \
         ((str(providers(target).get('{PROVIDER}', None).app) if providers(target).get('{PROVIDER}', None) != None else 'NONE')) + '|' + \
         str(target.files_to_run.executable != None)"
    )
}

/// Exact cquery argv for a deployability probe: launcher, startup
/// options, `cquery`, canonical workspace policy, the label, and the
/// Starlark deploy observation. No user Bazel options leak into
/// resolution; no `--config` pin (deployability is configuration
/// independent).
fn deploy_query_argv(label: &str) -> Vec<String> {
    let mut argv = Vec::with_capacity(WORKFLOW_STARTUP_OPTS.len() + 6);
    argv.push(launcher_argv0().to_owned());
    argv.extend(WORKFLOW_STARTUP_OPTS.iter().map(ToString::to_string));
    argv.push("cquery".to_owned());
    argv.push("--@rules_dx//config:workspace=//dx:config".to_owned());
    argv.push(label.to_owned());
    argv.push("--output=starlark".to_owned());
    argv.push(format!("--starlark:expr={}", deploy_starlark_expr()));
    argv
}

/// Checks deployability for one resolved deploy label via `bazel cquery`.
///
/// Deployable means the target returns `DxDeployInfo` or is executable
/// (any `*_binary`/executable; Bazel owns executability, aliases
/// included). Otherwise returns [`ResolveError::NotDeployable`].
/// Query failures become [`ResolveError::QueryFailed`].
pub fn check_deployable(
    label: &str,
    workspace: &Path,
    runner: &dyn QueryRunner,
) -> Result<DeployInfo, ResolveError> {
    let argv = deploy_query_argv(label);
    let result = runner
        .run_query(&argv, workspace)
        .map_err(|error| ResolveError::QueryFailed {
            label: label.to_owned(),
            detail: error.to_string(),
        })?;
    if result.code != Some(0) {
        return Err(ResolveError::QueryFailed {
            label: label.to_owned(),
            detail: first_line(&result.stderr),
        });
    }
    let text = std::str::from_utf8(&result.stdout).map_err(|_| ResolveError::QueryFailed {
        label: label.to_owned(),
        detail: "query output is not UTF-8".to_owned(),
    })?;
    let line = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .ok_or_else(|| ResolveError::QueryFailed {
            label: label.to_owned(),
            detail: "cquery returned no deploy observation".to_owned(),
        })?;
    let mut parts = line.split('|');
    let (has, profile_raw, app_raw, exe) = match (
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
    ) {
        (Some(has), Some(profile), Some(app), Some(exe), None) => (has, profile, app, exe),
        _ => {
            return Err(ResolveError::QueryFailed {
                label: label.to_owned(),
                detail: "cquery returned a malformed deploy observation".to_owned(),
            });
        }
    };
    let has_provider = match has {
        "True" => true,
        "False" => false,
        _ => {
            return Err(ResolveError::QueryFailed {
                label: label.to_owned(),
                detail: "cquery returned a malformed deploy observation".to_owned(),
            });
        }
    };
    let executable = match exe {
        "True" => true,
        "False" => false,
        _ => {
            return Err(ResolveError::QueryFailed {
                label: label.to_owned(),
                detail: "cquery returned a malformed deploy observation".to_owned(),
            });
        }
    };
    if !has_provider && !executable {
        return Err(ResolveError::NotDeployable {
            label: label.to_owned(),
        });
    }
    Ok(DeployInfo {
        label: label.to_owned(),
        has_provider,
        profile_raw: profile_raw.to_owned(),
        app_raw: app_raw.to_owned(),
        executable,
    })
}

/// One `bazel query` invocation per owner set (O44): every rule whose
/// kind ends in `_test` reaching the owners over the main-workspace
/// universe, at any depth. Owners are bytewise sorted so the expression
/// is deterministic.
fn tests_expression(owners: &[String]) -> String {
    format!(
        "kind('.*_test rule', rdeps(//..., set({})))",
        quote_set(owners)
    )
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

    /// Test-only one-shot label lookup without a shared cache.
    fn file_label(workspace: &Path, rel: &str, scope: &str) -> Result<String, ResolveError> {
        PackageCache::default().file_label(workspace, rel, scope)
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
                "kind('rule', rdeps(//..., set(\"//pkg:a.py\"), 1))",
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
            "kind('rule', rdeps(//..., set(\"//pkg:my file.py\"), 1))"
        );
        assert_eq!(quote_label("//pkg:a\"b\\c"), "\"//pkg:a\\\"b\\\\c\"");
        cleanup(&workspace);
    }

    #[test]
    fn multiple_files_share_one_bounded_query() {
        let workspace = temp_workspace("batch");
        write(&workspace, "pkg/BUILD.bazel", "");
        write(&workspace, "pkg/a.py", "x = 1\n");
        write(&workspace, "pkg/b.py", "x = 1\n");
        let query = FakeQuery::new(vec![FakeQuery::ok("//pkg:lib\n//pkg:extra\n")]);
        let got = resolve(&scopes(&["pkg/b.py", "pkg/a.py"]), &workspace, &query).expect("resolve");
        assert_eq!(got.targets, scopes(&["//pkg:extra", "//pkg:lib"]));
        let calls = query.calls();
        assert_eq!(calls.len(), 1, "one bounded query per resolver call");
        assert_eq!(
            calls[0].0.last().expect("expression"),
            "kind('rule', rdeps(//..., set(\"//pkg:a.py\" \"//pkg:b.py\"), 1))"
        );
        cleanup(&workspace);
    }

    #[test]
    fn empty_batch_mapping_names_the_first_file() {
        let workspace = temp_workspace("batch-empty");
        write(&workspace, "pkg/BUILD.bazel", "");
        write(&workspace, "pkg/a.py", "x = 1\n");
        write(&workspace, "pkg/b.py", "x = 1\n");
        let query = FakeQuery::new(vec![FakeQuery::ok("\n")]);
        let err =
            resolve(&scopes(&["pkg/a.py", "pkg/b.py"]), &workspace, &query).expect_err("orphans");
        assert_eq!(
            err,
            ResolveError::NoOwner {
                file: "pkg/a.py".to_owned(),
                label: "//pkg:a.py".to_owned(),
            }
        );
        assert_eq!(query.calls().len(), 1);
        cleanup(&workspace);
    }

    #[test]
    fn file_after_a_packaged_file_without_package_fails_before_query() {
        let workspace = temp_workspace("batch-no-package");
        write(&workspace, "pkg/BUILD.bazel", "");
        write(&workspace, "pkg/a.py", "x = 1\n");
        write(&workspace, "docs/guide.md", "# guide\n");
        let query = NeverQuery;
        let err = resolve(&scopes(&["pkg/a.py", "docs/guide.md"]), &workspace, &query)
            .expect_err("no package");
        assert_eq!(
            err,
            ResolveError::NotAPackage {
                scope: "docs/guide.md".to_owned(),
            }
        );
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
                label: "kind('rule', rdeps(//..., set(\"//pkg:a.py\"), 1))".to_owned(),
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
                label: "kind('rule', rdeps(//..., set(\"//pkg:a.py\"), 1))".to_owned(),
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
            "kind('.*_binary rule', rdeps(//..., set(\"//app:main.py\"), 1))"
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
