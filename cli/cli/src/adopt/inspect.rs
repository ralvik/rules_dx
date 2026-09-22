//! Adoption inspect execution (`owners`/`deps`/`why`).
//!
//! Split from `super` (`adopt.rs`): owns [`execute_inspect`] plus the
//! `bazel query` forwarding helper and the two-step `why` resolution
//! (file owner then `somepath`). Re-exported through `super` so the
//! dispatch path stays `crate::adopt::execute_adoption`.

use std::io::Write;

use crate::args::{Command, Invocation};
use crate::resolve::QueryRunner;

use super::{operational, pre_exec, summaries_suppressed};

/// Runs one inspect command (`owners`/`deps`/`why`) via `bazel query`
/// forwarding. `why` resolves the file owner first, then explains one
/// path from the resolved owner to the target. `--dry-run` plans without
/// launching Bazel.
pub(crate) fn execute_inspect(
    invocation: &Invocation,
    workspace: &std::path::Path,
    query_runner: &dyn QueryRunner,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    if invocation.command == Command::Why {
        return execute_why(invocation, workspace, query_runner, out, err);
    }
    let kind = invocation.command.name();
    // Dry-run plans each scope without launching: validate the plan for
    // usage errors, then print the would-run summary.
    if invocation.dry_run {
        for scope in &invocation.targets {
            if let Err(error) = dx_adopt::plan_inspect(kind, scope, invocation.configured) {
                return pre_exec(err, &error.to_string());
            }
        }
        if !summaries_suppressed(invocation) {
            for scope in &invocation.targets {
                if let Ok(plan) = dx_adopt::plan_inspect(kind, scope, invocation.configured) {
                    let _ = writeln!(out, "would run bazel {} {}", plan.verb, plan.expr);
                }
            }
        }
        return 0;
    }
    let mut code = 0;
    for scope in &invocation.targets {
        let plan = match dx_adopt::plan_inspect(kind, scope, invocation.configured) {
            Ok(plan) => plan,
            Err(error) => return pre_exec(err, &error.to_string()),
        };
        let step = run_inspect_query(&plan.verb, &plan.expr, workspace, query_runner, out, err);
        if step != 0 {
            code = step;
        }
    }
    code
}

/// Runs one planned inspect query as `bazel <verb> <expr>` and prints
/// bytewise-sorted deduplicated labels. The verb and expression stay
/// separate argv elements so the expression is never double-wrapped
/// in a second `query` invocation.
fn run_inspect_query(
    verb: &str,
    expr: &str,
    workspace: &std::path::Path,
    query_runner: &dyn QueryRunner,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let argv = vec!["bazel".to_owned(), verb.to_owned(), expr.to_owned()];
    match query_runner.run_query(&argv, workspace) {
        Ok(result) => {
            if result.code != Some(0) {
                return operational(
                    out,
                    err,
                    &format!(
                        "query failed: bazel {verb} {expr} exited with code {}",
                        result.code.unwrap_or(-1)
                    ),
                );
            }
            let text = String::from_utf8_lossy(&result.stdout);
            let mut lines: Vec<&str> = text.lines().collect();
            lines.sort_unstable();
            lines.dedup();
            for line in lines {
                let _ = writeln!(out, "{line}");
            }
            0
        }
        Err(error) => operational(out, err, &error.to_string()),
    }
}

fn execute_why(
    invocation: &Invocation,
    workspace: &std::path::Path,
    query_runner: &dyn QueryRunner,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    // Parsing guarantees exactly `<file> <label>`, but `execute` is also
    // reachable with a hand-built `Invocation`: fail closed via `get`
    // instead of panicking on direct indexing.
    let Some(file) = invocation.targets.first() else {
        return pre_exec(err, "why needs exactly <file> <label>");
    };
    let Some(label) = invocation.targets.get(1) else {
        return pre_exec(err, "why needs exactly <file> <label>");
    };
    if invocation.targets.len() != 2 {
        return pre_exec(err, "why needs exactly <file> <label>");
    }
    // Dry-run plans without launching: validate both plan shapes, then
    // print the would-run summary (the `somepath` leg needs the resolved
    // owner, so live resolution is skipped).
    if invocation.dry_run {
        let owner_plan = match dx_adopt::plan_inspect("owners", file, invocation.configured) {
            Ok(plan) => plan,
            Err(error) => return pre_exec(err, &error.to_string()),
        };
        if !summaries_suppressed(invocation) {
            let _ = writeln!(
                out,
                "would run bazel {} {} then somepath to {label}",
                owner_plan.verb, owner_plan.expr
            );
        }
        return 0;
    }
    // Step 1: resolve the file's depth-1 owner. `why` never resolves
    // the raw file path against the target graph: Bazel `somepath`
    // needs rule-to-rule endpoints.
    let owner_plan = match dx_adopt::plan_inspect("owners", file, invocation.configured) {
        Ok(plan) => plan,
        Err(error) => return pre_exec(err, &error.to_string()),
    };
    let owner_argv = vec![
        "bazel".to_owned(),
        owner_plan.verb.clone(),
        owner_plan.expr.clone(),
    ];
    let owner = match query_runner.run_query(&owner_argv, workspace) {
        Ok(result) => {
            if result.code != Some(0) {
                return operational(
                    out,
                    err,
                    &format!(
                        "query failed: bazel {} {} for file {file} exited with code {}",
                        owner_plan.verb,
                        owner_plan.expr,
                        result.code.unwrap_or(-1)
                    ),
                );
            }
            let text = String::from_utf8_lossy(&result.stdout);
            let mut labels: Vec<&str> = text.lines().collect();
            labels.sort_unstable();
            labels.dedup();
            match labels.into_iter().next() {
                Some(owner) => owner.to_owned(),
                None => {
                    return operational(
                        out,
                        err,
                        &format!(
                            "no owner for {file} via bazel {} {}",
                            owner_plan.verb, owner_plan.expr
                        ),
                    );
                }
            }
        }
        Err(error) => return operational(out, err, &error.to_string()),
    };
    // Step 2: explain one path from the resolved owner to the target.
    let leg = match dx_adopt::plan_somepath(&owner, label, invocation.configured) {
        Ok(leg) => leg,
        Err(error) => return pre_exec(err, &error.to_string()),
    };
    run_inspect_query(&leg.verb, &leg.expr, workspace, query_runner, out, err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adopt::{execute_adoption, AdoptEnv};
    use crate::args::parse;
    use crate::resolve::QueryResult;
    use std::io;

    fn invocation(words: &[&str]) -> Invocation {
        parse(&words.iter().map(ToString::to_string).collect::<Vec<_>>()).expect("parse")
    }

    struct ScriptedQuery {
        calls: std::cell::RefCell<Vec<Vec<String>>>,
        outputs: Vec<Vec<u8>>,
        code: Option<i32>,
    }

    impl ScriptedQuery {
        fn with(outputs: &[&str]) -> Self {
            Self {
                calls: std::cell::RefCell::new(Vec::new()),
                outputs: outputs
                    .iter()
                    .map(|text| text.as_bytes().to_vec())
                    .collect(),
                code: Some(0),
            }
        }
    }

    impl QueryRunner for ScriptedQuery {
        fn run_query(&self, argv: &[String], _cwd: &std::path::Path) -> io::Result<QueryResult> {
            let mut calls = self.calls.borrow_mut();
            let stdout = self.outputs.get(calls.len()).cloned().unwrap_or_default();
            calls.push(argv.to_vec());
            Ok(QueryResult {
                code: self.code,
                stdout,
                stderr: Vec::new(),
            })
        }
    }

    struct NullRunner;

    impl dx_process::Runner for NullRunner {
        fn run(
            &self,
            _argv: &[String],
            _cwd: &std::path::Path,
            _env: &[(&str, &str)],
        ) -> io::Result<dx_process::ChildStatus> {
            Ok(dx_process::ChildStatus { code: Some(0) })
        }
    }

    #[test]
    fn inspect_forwards_single_unwrapped_query() {
        let runner = ScriptedQuery::with(&["//z:two\n//a:one\n//z:two\n"]);
        let inv = invocation(&["owners", "//a:one"]);
        let scratch = dx_test_scratch::scratch("dx-adopt-inspect-single-");
        let root = scratch.path().to_path_buf();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = execute_adoption(
            &inv,
            AdoptEnv {
                workspace: &root,
                query_runner: &runner,
                runner: &NullRunner,
                out: &mut out,
                err: &mut err,
            },
        );
        assert_eq!(code, 0);
        // Sorted and deduplicated.
        assert_eq!(String::from_utf8(out).expect("out"), "//a:one\n//z:two\n");
        // Exactly one query expression: never double-wrapped in a
        // second `query` invocation and never shell-quoted.
        let calls = runner.calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0],
            vec![
                "bazel".to_owned(),
                "query".to_owned(),
                "kind('rule', rdeps(//..., //a:one, 1))".to_owned(),
            ]
        );
        assert!(String::from_utf8(err).expect("err").is_empty());
    }

    #[test]
    fn inspect_configured_uses_cquery() {
        let runner = ScriptedQuery::with(&["//a:one\n"]);
        let inv = invocation(&["deps", "--configured", "//a:one"]);
        let scratch = dx_test_scratch::scratch("dx-adopt-inspect-configured-");
        let root = scratch.path().to_path_buf();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = execute_adoption(
            &inv,
            AdoptEnv {
                workspace: &root,
                query_runner: &runner,
                runner: &NullRunner,
                out: &mut out,
                err: &mut err,
            },
        );
        assert_eq!(code, 0);
        let calls = runner.calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0][1], "cquery");
        assert_eq!(calls[0][2], "deps(//a:one)");
    }

    #[test]
    fn why_resolves_owner_then_somepath() {
        let runner = ScriptedQuery::with(&["//owner:lib\n", "//owner:lib\n//app:server\n"]);
        let inv = invocation(&["why", "src/lib.rs", "//app:server"]);
        let scratch = dx_test_scratch::scratch("dx-adopt-inspect-why-");
        let root = scratch.path().to_path_buf();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = execute_adoption(
            &inv,
            AdoptEnv {
                workspace: &root,
                query_runner: &runner,
                runner: &NullRunner,
                out: &mut out,
                err: &mut err,
            },
        );
        assert_eq!(code, 0);
        let calls = runner.calls.borrow();
        assert_eq!(calls.len(), 2);
        // Step 1 resolves the file owner; step 2 explains from the
        // resolved owner, never from the raw file path.
        assert_eq!(calls[0][2], "kind('rule', rdeps(//..., src/lib.rs, 1))");
        assert_eq!(
            calls[1],
            vec![
                "bazel".to_owned(),
                "query".to_owned(),
                "somepath(//owner:lib, //app:server)".to_owned(),
            ]
        );
        assert!(String::from_utf8(out)
            .expect("out")
            .contains("//app:server"));
    }

    #[test]
    fn why_without_owner_is_operational() {
        let runner = ScriptedQuery::with(&[""]);
        let inv = invocation(&["why", "src/orphan.rs", "//app:server"]);
        let scratch = dx_test_scratch::scratch("dx-adopt-inspect-why-orphan-");
        let root = scratch.path().to_path_buf();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = execute_adoption(
            &inv,
            AdoptEnv {
                workspace: &root,
                query_runner: &runner,
                runner: &NullRunner,
                out: &mut out,
                err: &mut err,
            },
        );
        assert_eq!(code, 1);
        assert!(String::from_utf8(err).expect("err").contains("no owner"));
        assert_eq!(runner.calls.borrow().len(), 1);
    }

    #[test]
    fn inspect_dry_run_plans_without_query() {
        let runner = ScriptedQuery::with(&["//a:one\n"]);
        let inv = invocation(&["owners", "//a:one", "--dry-run"]);
        let scratch = dx_test_scratch::scratch("dx-adopt-inspect-dry-");
        let root = scratch.path().to_path_buf();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = execute_adoption(
            &inv,
            AdoptEnv {
                workspace: &root,
                query_runner: &runner,
                runner: &NullRunner,
                out: &mut out,
                err: &mut err,
            },
        );
        assert_eq!(code, 0);
        assert!(String::from_utf8(out)
            .expect("out")
            .contains("would run bazel"));
        assert_eq!(runner.calls.borrow().len(), 0);
        let runner = ScriptedQuery::with(&["//owner:lib\n"]);
        let inv = invocation(&["why", "src/lib.rs", "//app:server", "--dry-run"]);
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = execute_adoption(
            &inv,
            AdoptEnv {
                workspace: &root,
                query_runner: &runner,
                runner: &NullRunner,
                out: &mut out,
                err: &mut err,
            },
        );
        assert_eq!(code, 0);
        assert!(String::from_utf8(out)
            .expect("out")
            .contains("would run bazel"));
        assert_eq!(runner.calls.borrow().len(), 0);
    }

    #[test]
    fn why_malformed_invocation_fails_closed_without_panic() {
        // `execute` is reachable with a hand-built `Invocation`: malformed
        // `why` targets must fail pre-exec (exit 2) instead of panicking
        // on direct indexing.
        let base = invocation(&["why", "src/lib.rs", "//app:server"]);
        for targets in [
            Vec::new(),
            vec!["only-one".to_owned()],
            vec![
                "src/lib.rs".to_owned(),
                "//app:server".to_owned(),
                "//extra:lib".to_owned(),
            ],
        ] {
            let mut bad = base.clone();
            bad.targets = targets;
            let runner = ScriptedQuery::with(&["//owner:lib\n"]);
            let scratch = dx_test_scratch::scratch("dx-adopt-inspect-why-malformed-");
            let root = scratch.path().to_path_buf();
            let mut out = Vec::new();
            let mut err = Vec::new();
            let code = execute_adoption(
                &bad,
                AdoptEnv {
                    workspace: &root,
                    query_runner: &runner,
                    runner: &NullRunner,
                    out: &mut out,
                    err: &mut err,
                },
            );
            assert_eq!(code, 2, "targets: {:?}", bad.targets);
            assert!(
                String::from_utf8(err)
                    .expect("err")
                    .contains("why needs exactly"),
                "targets: {:?}",
                bad.targets
            );
            assert_eq!(runner.calls.borrow().len(), 0);
        }
    }
}
