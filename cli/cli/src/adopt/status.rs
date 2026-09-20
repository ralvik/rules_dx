//! Adoption status execution (`status`).
//!
//! Split from `super` (`adopt.rs`): owns [`execute_status`], the result
//! document that always prints even under `--quiet` (quiet suppresses
//! summaries, not answers). Re-exported through `super` so the dispatch
//! path stays `crate::adopt::execute_adoption`.

use std::io::Write;

use crate::args::Invocation;
use dx_output::OutputMode;
use dx_process::operational_code;

/// Runs `dx status`: prints the pin plus default status checks as JSON
/// vs text (the only mode branch; `--output=diff` is rejected at parse
/// time because status has no patch to emit). Returns operational failure
/// when any check reports `error`.
pub(crate) fn execute_status(
    invocation: &Invocation,
    workspace: &std::path::Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let pinned = dx_adopt::read_version_pin(workspace).unwrap_or_default();
    let pinned = if pinned.is_empty() {
        dx_adopt::DX_VERSION.to_owned()
    } else {
        pinned
    };
    let checks = dx_adopt::default_status_checks(&pinned);
    // Result document: always prints even under `--quiet` (quiet suppresses
    // summaries, not answers; see `super::summaries_suppressed` and the output
    // protocol). JSON vs text is the only mode branch here; `--output=diff`
    // is rejected at parse time because status has no patch to emit.
    if invocation.output == OutputMode::Json {
        let _ = writeln!(out, "{}", dx_adopt::render_status_json(&checks));
    } else {
        let _ = writeln!(out, "{}", dx_adopt::render_status_text(&checks));
    }
    if checks.iter().any(|c| c.status == "error") {
        let _ = writeln!(err, "dx: status: pin mismatch (see hint)");
        return operational_code();
    }
    0
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

    struct NullQuery;

    impl crate::resolve::QueryRunner for NullQuery {
        fn run_query(&self, _argv: &[String], _cwd: &std::path::Path) -> io::Result<QueryResult> {
            Ok(QueryResult {
                code: Some(0),
                stdout: b"//a:one\n".to_vec(),
                stderr: Vec::new(),
            })
        }
    }

    #[test]
    fn status_reports_pin_and_checks() {
        let inv = invocation(&["status"]);
        let scratch = dx_test_scratch::scratch("dx-adopt-status-status-");
        let root = scratch.path().to_path_buf();
        std::fs::create_dir_all(root.join(".dx")).expect("dx");
        std::fs::write(root.join(".dx/version"), "0.0.0\n").expect("pin");
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = execute_adoption(
            &inv,
            AdoptEnv {
                workspace: &root,
                query_runner: &NullQuery,
                out: &mut out,
                err: &mut err,
            },
        );
        assert_eq!(code, 0);
        assert!(String::from_utf8(out).expect("out").contains("pin: ok"));
    }
}
