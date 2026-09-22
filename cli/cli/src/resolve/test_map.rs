//! Source-owner to test-target mapping.
//!
//! Split from `super` (`resolve.rs`): owns [`map_owners_to_tests`] and
//! its `kind(..._test ... rdeps(...))` expression helper. Re-exported
//! through `super` so the public path stays
//! [`crate::resolve::map_owners_to_tests`]. Shares the query plumbing
//! ([`super::QueryRunner`], [`super::QueryResult`],
//! [`super::quote_set`], [`super::parse_owners`], [`super::first_line`])
//! with the scope classification in `super`.

use std::path::Path;

use dx_process::{launcher_argv0, WORKFLOW_STARTUP_OPTS};

use super::{first_line, parse_owners, quote_set, QueryRunner, ResolveError};

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
    use crate::resolve::NeverQuery;
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

    // Shared test guard: `crate::resolve::NeverQuery` (see `types.rs`, issue #914).

    fn scopes(words: &[&str]) -> Vec<String> {
        words.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn test_mapping_queries_transitive_test_owners() {
        let scratch = dx_test_scratch::scratch("dx-resolve-test-test-map-");
        let workspace = scratch.path().to_path_buf();
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
    }

    #[test]
    fn test_mapping_sorts_owners_in_set_expression() {
        let scratch = dx_test_scratch::scratch("dx-resolve-test-test-map-order-");
        let workspace = scratch.path().to_path_buf();
        let query = FakeQuery::new(vec![FakeQuery::ok("//t:t\n")]);
        map_owners_to_tests(&scopes(&["//z:lib", "//a:lib"]), &workspace, &query).expect("map");
        let calls = query.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].0.last().expect("expression"),
            "kind('.*_test rule', rdeps(//..., set(\"//a:lib\" \"//z:lib\")))"
        );
    }

    #[test]
    fn empty_test_mapping_suggests_explicit_label() {
        let scratch = dx_test_scratch::scratch("dx-resolve-test-test-map-empty-");
        let workspace = scratch.path().to_path_buf();
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
    }

    #[test]
    fn test_mapping_failures_report_the_first_bazel_line() {
        let scratch = dx_test_scratch::scratch("dx-resolve-test-test-map-fail-");
        let workspace = scratch.path().to_path_buf();
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
    }

    #[test]
    fn empty_owners_map_to_no_tests_without_query() {
        let scratch = dx_test_scratch::scratch("dx-resolve-test-test-map-no-query-");
        let workspace = scratch.path().to_path_buf();
        let query = NeverQuery;
        let got = map_owners_to_tests(&[], &workspace, &query).expect("map");
        assert!(got.is_empty());
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
}
