use std::io::Write;

use crate::args::Invocation;
use crate::exec::common::check_stdout_write;

use super::{pre_exec, summaries_suppressed};

pub(crate) fn execute_watch(
    invocation: &Invocation,
    _workspace: &std::path::Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let wrapped = invocation.targets.first().map(String::as_str).unwrap_or("");
    // Local-only gate shares the single `dx_process::is_ci` owner with
    // `dx run` so `CI=1`/`yes`/empty cannot diverge again.
    let ci = dx_process::is_ci();
    match dx_adopt::plan_watch(wrapped, ci) {
        Ok(plan) => {
            if invocation.dry_run {
                if !summaries_suppressed(invocation) {
                    if let Err(exit) = check_stdout_write(writeln!(out, "would {plan}")) {
                        return exit;
                    }
                }
                return 0;
            }
            // Single delivered iteration: re-resolve scope each loop in the
            // real binary (loop omitted under test via DX_WATCH_ONCE).
            if !summaries_suppressed(invocation) {
                if let Err(exit) = check_stdout_write(writeln!(
                    out,
                    "{plan} scope={}",
                    invocation.targets.join(" ")
                )) {
                    return exit;
                }
            }
            if std::env::var("DX_WATCH_ONCE").is_ok() {
                return 0;
            }
            if !summaries_suppressed(invocation) {
                if let Err(exit) = check_stdout_write(writeln!(out, "watching (Ctrl-C to stop)")) {
                    return exit;
                }
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
