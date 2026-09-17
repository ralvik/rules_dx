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
//!
//! Run/deploy resolution (`resolve_run`, `resolve_deploy`,
//! `check_deployable`) lives in the [`run_deploy`](self::run_deploy)
//! domain submodule (issue #236 resolve/plan/run/deploy unscramble,
//! handoff from issue #237); the public path stays
//! `crate::resolve::{...}` via the re-exports below. See the matching
//! note in `plan.rs` for the planning carve.

pub mod classify;
pub mod run_deploy;
pub mod test_map;

pub(crate) use classify::{
    classify_scopes, first_line, parse_owners, resolve_file_owners, PackageCache,
};
pub use run_deploy::{check_deployable, resolve_deploy, resolve_run, DeployInfo};
pub use test_map::map_owners_to_tests;

use std::io;
use std::path::Path;

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
pub(crate) fn quote_set(items: &[String]) -> String {
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
pub(crate) fn ownership_set_expression(labels: &[String]) -> String {
    format!("kind('rule', rdeps(//..., set({}), 1))", quote_set(labels))
}

/// Quotes a label as a double-quoted query string literal, escaping
/// backslashes and quotes. Rejects control characters, which cannot
/// round-trip through line-oriented query output.
pub(crate) fn quote_label(label: &str) -> String {
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
pub(crate) fn run_label_query(
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

    fn temp_workspace(name: &str) -> dx_test_scratch::TempDir {
        dx_test_scratch::scratch(&format!("dx-resolve-test-{name}-"))
    }

    fn write(workspace: &Path, rel: &str, text: &str) {
        let full = workspace.join(rel);
        std::fs::create_dir_all(full.parent().expect("parent")).expect("parent dir");
        std::fs::write(full, text).expect("write file");
    }

    #[test]
    fn query_failures_report_the_first_bazel_line() {
        let scratch = temp_workspace("query-fail");
        let workspace = scratch.path().to_path_buf();
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
    fn test_scope_maps_files_to_tests_and_keeps_patterns() {
        let scratch = temp_workspace("test-scope");
        let workspace = scratch.path().to_path_buf();
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
    }

    #[test]
    fn test_scope_without_files_matches_plain_resolve() {
        let scratch = temp_workspace("test-scope-labels");
        let workspace = scratch.path().to_path_buf();
        let query = NeverQuery;
        let got = resolve_for_test(&scopes(&["//a:one", "//b/..."]), &workspace, &query)
            .expect("resolve");
        assert_eq!(got.targets, scopes(&["//a:one", "//b/..."]));
        assert_eq!(got.scope, Scope::Labels(scopes(&["//a:one", "//b/..."])));
    }

    struct FailIo;

    impl QueryRunner for FailIo {
        fn run_query(&self, _argv: &[String], _cwd: &Path) -> io::Result<QueryResult> {
            Err(io::Error::other("boom"))
        }
    }

    #[test]
    fn query_io_errors_become_query_failed() {
        let scratch = temp_workspace("query-io");
        let workspace = scratch.path().to_path_buf();
        write(&workspace, "pkg/BUILD.bazel", "");
        write(&workspace, "pkg/a.py", "x = 1\n");
        let err = resolve(&scopes(&["pkg/a.py"]), &workspace, &FailIo).expect_err("io");
        assert!(matches!(err, ResolveError::QueryFailed { .. }), "{err:?}");
        let err = resolve_run(&scopes(&["pkg/a.py"]), &workspace, &FailIo).expect_err("io");
        assert!(matches!(err, ResolveError::QueryFailed { .. }), "{err:?}");
        let err =
            map_owners_to_tests(&scopes(&["//pkg:lib"]), &workspace, &FailIo).expect_err("io");
        assert!(matches!(err, ResolveError::QueryFailed { .. }), "{err:?}");
    }

    #[test]
    fn not_a_directory_maps_to_query_failed() {
        let scratch = temp_workspace("not-a-dir");
        let workspace = scratch.path().to_path_buf();
        write(&workspace, "pkg", "file, not dir\n");
        let query = NeverQuery;
        let err = resolve(&scopes(&["pkg/a.py"]), &workspace, &query).expect_err("enotdir");
        assert!(matches!(err, ResolveError::QueryFailed { .. }), "{err:?}");
        let err =
            resolve_for_test(&scopes(&["pkg/a.py"]), &workspace, &query).expect_err("enotdir");
        assert!(matches!(err, ResolveError::QueryFailed { .. }), "{err:?}");
        let err = resolve_run(&scopes(&["pkg/a.py"]), &workspace, &query).expect_err("enotdir");
        assert!(matches!(err, ResolveError::QueryFailed { .. }), "{err:?}");
    }

    #[test]
    fn test_scope_rejects_external_and_relative() {
        let scratch = temp_workspace("test-scope-reject");
        let workspace = scratch.path().to_path_buf();
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
    }

    #[test]
    fn test_scope_missing_files_fail() {
        let scratch = temp_workspace("test-scope-missing");
        let workspace = scratch.path().to_path_buf();
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
    }

    #[test]
    fn test_scope_dir_only_resolves_without_query() {
        let scratch = temp_workspace("test-scope-dir");
        let workspace = scratch.path().to_path_buf();
        std::fs::create_dir_all(workspace.join("app")).expect("dir");
        let query = NeverQuery;
        let got = resolve_for_test(&scopes(&["app"]), &workspace, &query).expect("dir");
        assert_eq!(got.targets, scopes(&["//app/..."]));
    }

    #[test]
    fn control_chars_in_names_are_rejected() {
        let scratch = temp_workspace("control-names");
        let workspace = scratch.path().to_path_buf();
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
    }
}
