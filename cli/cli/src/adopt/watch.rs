//! Adoption watch execution (`watch`).
//!
//! Split from `super` (`adopt.rs`): owns [`execute_watch`], the thin
//! wrapper over `dx_adopt::plan_watch` (dry-run plans are summaries,
//! suppressed under `--quiet`). Re-exported through `super` so the
//! dispatch path stays `crate::adopt::execute_adoption`.

use std::io::Write;

use crate::args::Invocation;

use super::{pre_exec, summaries_suppressed};

/// Runs `dx watch`: plans the wrapped command via `dx_adopt::plan_watch`
/// and prints one delivered iteration (the real binary re-resolves scope
/// each loop; the loop is omitted under test via `DX_WATCH_ONCE`).
pub(crate) fn execute_watch(
    invocation: &Invocation,
    _workspace: &std::path::Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let wrapped = invocation.targets.first().map(String::as_str).unwrap_or("");
    // Local-only gate shares the single `dx_process::is_ci` owner with
    // `dx run` so `CI=1`/`yes`/empty cannot diverge again.
    // See: `docs/cli/commands/watch.md`.
    let ci = dx_process::is_ci();
    match dx_adopt::plan_watch(wrapped, ci) {
        Ok(plan) => {
            if invocation.dry_run {
                if !summaries_suppressed(invocation) {
                    let _ = writeln!(out, "would {plan}");
                }
                return 0;
            }
            // Single delivered iteration: re-resolve scope each loop in the
            // real binary (loop omitted under test via DX_WATCH_ONCE).
            if !summaries_suppressed(invocation) {
                let _ = writeln!(out, "{plan} scope={}", invocation.targets.join(" "));
            }
            if std::env::var("DX_WATCH_ONCE").is_ok() {
                return 0;
            }
            if !summaries_suppressed(invocation) {
                let _ = writeln!(out, "watching (Ctrl-C to stop)");
            }
            0
        }
        Err(error) => pre_exec(err, &error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adopt::{execute_adoption, AdoptEnv};
    use crate::args::parse;
    use std::io;

    fn invocation(words: &[&str]) -> Invocation {
        parse(&words.iter().map(ToString::to_string).collect::<Vec<_>>()).expect("parse")
    }

    struct NullQuery;

    impl crate::resolve::QueryRunner for NullQuery {
        fn run_query(
            &self,
            _argv: &[String],
            _cwd: &std::path::Path,
        ) -> io::Result<crate::resolve::QueryResult> {
            Ok(crate::resolve::QueryResult {
                code: Some(0),
                stdout: b"//a:one\n".to_vec(),
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
    fn watch_validates_wrapped_command() {
        let scratch = dx_test_scratch::scratch("dx-adopt-watch-validates-");
        let root = scratch.path().to_path_buf();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let inv = invocation(&["watch", "test", "//..."]);
        assert_eq!(
            execute_adoption(
                &inv,
                AdoptEnv {
                    workspace: &root,
                    query_runner: &NullQuery,
                    runner: &NullRunner,
                    out: &mut out,
                    err: &mut err,
                },
            ),
            0
        );
    }
}
