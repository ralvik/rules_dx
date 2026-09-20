//! `dx run` / `dx deploy` scope resolution.
//!
//! Split from `super` (`resolve.rs`): owns [`resolve_run`],
//! [`resolve_deploy`], [`DeployInfo`], and [`check_deployable`] plus the
//! run-only query-expression helpers. Re-exported through `super` so the
//! public paths stay `crate::resolve::{resolve_run, resolve_deploy,
//! check_deployable, DeployInfo}`. Shares the query core in `super`
//! ([`super::PackageCache`], [`super::classify_scopes`],
//! [`super::run_label_query`], [`super::QueryRunner`]); the
//! `dx_run`/`dx_deploy` planning carve in `plan.rs` moves with the same
//! unscramble.

use std::path::Path;

use dx_process::{launcher_argv0, WORKFLOW_STARTUP_OPTS};

use super::classify::classify_scopes;
use super::packages::PackageCache;
use super::{
    first_line, ownership_set_expression, quote_set, run_label_query, QueryRunner, ResolveError,
};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::QueryResult;
    use std::cell::RefCell;
    use std::io;
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

    /// Query runner that fails the test on any call: label scopes
    /// must resolve without touching Bazel.
    struct NeverQuery;

    // LCOV_EXCL_START - reason: test-only guard; NeverQuery is never called.
    impl QueryRunner for NeverQuery {
        fn run_query(&self, _argv: &[String], _cwd: &Path) -> io::Result<QueryResult> {
            panic!("label scopes must not run queries");
        }
    }
    // LCOV_EXCL_STOP - reason: end of NeverQuery exclusion.

    fn scopes(words: &[&str]) -> Vec<String> {
        words.iter().map(ToString::to_string).collect()
    }

    fn temp_workspace(name: &str) -> dx_test_scratch::TempDir {
        dx_test_scratch::scratch(&format!("dx-resolve-run-test-{name}-"))
    }

    fn write(workspace: &Path, rel: &str, text: &str) {
        let full = workspace.join(rel);
        std::fs::create_dir_all(full.parent().expect("parent")).expect("parent dir");
        std::fs::write(full, text).expect("write file");
    }

    #[test]
    fn run_labels_pass_through_without_query() {
        let scratch = temp_workspace("run-labels");
        let workspace = scratch.path().to_path_buf();
        let query = NeverQuery;
        let got = resolve_run(&scopes(&["//app:bin"]), &workspace, &query).expect("resolve");
        assert_eq!(got, scopes(&["//app:bin"]));
    }

    #[test]
    fn run_empty_scope_is_a_usage_error() {
        let scratch = temp_workspace("run-empty");
        let workspace = scratch.path().to_path_buf();
        let query = NeverQuery;
        let err = resolve_run(&[], &workspace, &query).expect_err("empty");
        assert_eq!(err, ResolveError::EmptyScope);
    }

    #[test]
    fn run_file_resolves_single_binary_owner() {
        let scratch = temp_workspace("run-file");
        let workspace = scratch.path().to_path_buf();
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
    }

    #[test]
    fn run_file_without_binary_reports_no_runnable() {
        let scratch = temp_workspace("run-no-bin");
        let workspace = scratch.path().to_path_buf();
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
    }

    #[test]
    fn run_file_without_any_owner_reports_no_owner() {
        let scratch = temp_workspace("run-no-owner");
        let workspace = scratch.path().to_path_buf();
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
    }

    #[test]
    fn run_dir_with_two_binaries_reports_ambiguous_candidates() {
        let scratch = temp_workspace("run-ambiguous");
        let workspace = scratch.path().to_path_buf();
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
    }

    #[test]
    fn runnable_query_failures_report_first_line() {
        let scratch = temp_workspace("run-query-fail");
        let workspace = scratch.path().to_path_buf();
        write(&workspace, "app/BUILD.bazel", "");
        write(&workspace, "app/main.py", "x = 1\n");
        let query = FakeQuery::new(vec![FakeQuery::failed("nope\n")]);
        let err = resolve_run(&scopes(&["app/main.py"]), &workspace, &query).expect_err("fail");
        assert!(matches!(err, ResolveError::QueryFailed { .. }), "{err:?}");
    }

    #[test]
    fn run_mixed_label_and_file_skips_label_in_second_pass() {
        let scratch = temp_workspace("run-mixed");
        let workspace = scratch.path().to_path_buf();
        write(&workspace, "app/BUILD.bazel", "");
        write(&workspace, "app/main.py", "x = 1\n");
        let query = FakeQuery::new(vec![FakeQuery::ok("//app:bin\n")]);
        let got =
            resolve_run(&scopes(&["//app:bin", "app/main.py"]), &workspace, &query).expect("mixed");
        assert_eq!(got, scopes(&["//app:bin"]));
        assert_eq!(query.calls().len(), 1);
    }
}
