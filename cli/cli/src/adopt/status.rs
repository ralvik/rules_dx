//! Adoption status execution (`status`).
//!
//! Split from `super` (`adopt.rs`): owns [`execute_status`], the result
//! document that always prints even under `--quiet` (quiet suppresses
//! summaries, not answers; dry-run plans are summaries, suppressed under
//! `--quiet`). JSON streams the NDJSON envelope via `write_event`.
//! Re-exported through `super` so the dispatch path stays
//! `crate::adopt::execute_adoption`.

use std::io::Write;

use crate::args::Invocation;
use dx_output::{
    command_finished, command_started, status_event, write_event, FinishedCounts, OutputMode,
    StatusEvent,
};
use dx_process::operational_code;

use super::summaries_suppressed;

/// Runs `dx status`: prints the pin plus default status checks as NDJSON
/// vs text (the only mode branch; `--output=diff` is rejected at parse
/// time because status has no patch to emit). JSON streams
/// `command_started` plus one `status` event per check plus
/// `command_finished` via `write_event`; `--dry-run` plans without
/// reading the pin or computing checks. Returns operational failure
/// when any check reports `error`.
pub(crate) fn execute_status(
    invocation: &Invocation,
    workspace: &std::path::Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    // Dry-run plans instead of executing: no pin read, no check
    // computation (See: `docs/cli/output-protocol.md#dry-run`).
    if invocation.dry_run {
        if invocation.output == OutputMode::Json {
            if let Ok(event) = command_started(invocation.command.name(), true, "default") {
                let _ = write_event(out, &event);
            }
            let _ = write_event(out, &command_finished(0, &FinishedCounts::default()));
        } else if !summaries_suppressed(invocation) {
            let _ = writeln!(out, "would report status");
        }
        return 0;
    }
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
        if let Ok(event) = command_started(invocation.command.name(), false, "default") {
            let _ = write_event(out, &event);
        }
        for check in &checks {
            if let Ok(event) = status_event(&StatusEvent {
                name: check.name.clone(),
                status: check.status.clone(),
                detail: check.detail.clone(),
                hint: check.hint.clone(),
            }) {
                let _ = write_event(out, &event);
            }
        }
        let failed = checks.iter().any(|c| c.status == "error");
        let code = if failed { operational_code() } else { 0 };
        let _ = write_event(out, &command_finished(code, &FinishedCounts::default()));
        if failed {
            let _ = writeln!(err, "dx: status: pin mismatch (see hint)");
            return operational_code();
        }
        return 0;
    }
    let _ = writeln!(out, "{}", dx_adopt::render_status_text(&checks));
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
                runner: &NullRunner,
                out: &mut out,
                err: &mut err,
            },
        );
        assert_eq!(code, 0);
        assert!(String::from_utf8(out).expect("out").contains("pin: ok"));
    }

    #[test]
    fn status_json_streams_envelope() {
        let inv = invocation(&["status", "--output=json"]);
        let scratch = dx_test_scratch::scratch("dx-adopt-status-json-");
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
                runner: &NullRunner,
                out: &mut out,
                err: &mut err,
            },
        );
        assert_eq!(code, 0);
        let text = String::from_utf8(out).expect("out");
        let events: Vec<serde_json::Value> = text
            .lines()
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()
            .expect("NDJSON");
        assert!(!events.is_empty());
        for event in &events {
            assert!(event.get("schema").is_some(), "{event}");
            assert!(event.get("event").is_some(), "{event}");
        }
        let kinds: Vec<&str> = events
            .iter()
            .map(|event| event["event"].as_str().expect("event"))
            .collect();
        assert_eq!(kinds[0], "command_started");
        assert_eq!(kinds[kinds.len() - 1], "command_finished");
        assert!(kinds.contains(&"status"), "{kinds:?}");
        assert_eq!(
            events[0]["dry_run"],
            serde_json::Value::Bool(false),
            "{events:?}"
        );
        assert_eq!(
            events.last().expect("finished")["exit_code"],
            serde_json::json!(0)
        );
        let status = events
            .iter()
            .find(|event| {
                event["event"] == serde_json::json!("status")
                    && event["name"] == serde_json::json!("pin")
            })
            .expect("pin status event");
        assert_eq!(status["status"], serde_json::json!("ok"));
        assert!(String::from_utf8(err).expect("err").is_empty());
    }

    #[test]
    fn status_pin_mismatch_fails_operational() {
        for words in [vec!["status"], vec!["status", "--output=json"]] {
            let inv = invocation(&words);
            let scratch = dx_test_scratch::scratch("dx-adopt-status-mismatch-");
            let root = scratch.path().to_path_buf();
            std::fs::create_dir_all(root.join(".dx")).expect("dx");
            std::fs::write(root.join(".dx/version"), "9.9.9\n").expect("pin");
            let mut out = Vec::new();
            let mut err = Vec::new();
            let code = execute_adoption(
                &inv,
                AdoptEnv {
                    workspace: &root,
                    query_runner: &NullQuery,
                    runner: &NullRunner,
                    out: &mut out,
                    err: &mut err,
                },
            );
            assert_eq!(code, 1, "words: {words:?}");
            assert!(
                String::from_utf8(err)
                    .expect("err")
                    .contains("pin mismatch"),
                "words: {words:?}"
            );
            if words.contains(&"--output=json") {
                let text = String::from_utf8(out).expect("out");
                let events: Vec<serde_json::Value> = text
                    .lines()
                    .map(serde_json::from_str)
                    .collect::<Result<_, _>>()
                    .expect("NDJSON");
                assert_eq!(
                    events.last().expect("finished")["exit_code"],
                    serde_json::json!(1)
                );
            }
        }
    }

    #[test]
    fn status_dry_run_plans_without_executing() {
        let inv = invocation(&["status", "--dry-run"]);
        let scratch = dx_test_scratch::scratch("dx-adopt-status-dry-");
        let root = scratch.path().to_path_buf();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = execute_adoption(
            &inv,
            AdoptEnv {
                workspace: &root,
                query_runner: &NullQuery,
                runner: &NullRunner,
                out: &mut out,
                err: &mut err,
            },
        );
        assert_eq!(code, 0);
        assert!(String::from_utf8(out)
            .expect("out")
            .contains("would report status"));
        let inv = invocation(&["status", "--dry-run", "--quiet"]);
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = execute_adoption(
            &inv,
            AdoptEnv {
                workspace: &root,
                query_runner: &NullQuery,
                runner: &NullRunner,
                out: &mut out,
                err: &mut err,
            },
        );
        assert_eq!(code, 0);
        assert!(String::from_utf8(out).expect("out").is_empty());
    }

    #[test]
    fn status_dry_run_json_emits_lifecycle_only() {
        let inv = invocation(&["status", "--dry-run", "--output=json"]);
        let scratch = dx_test_scratch::scratch("dx-adopt-status-dry-json-");
        let root = scratch.path().to_path_buf();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = execute_adoption(
            &inv,
            AdoptEnv {
                workspace: &root,
                query_runner: &NullQuery,
                runner: &NullRunner,
                out: &mut out,
                err: &mut err,
            },
        );
        assert_eq!(code, 0);
        let text = String::from_utf8(out).expect("out");
        let events: Vec<serde_json::Value> = text
            .lines()
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()
            .expect("NDJSON");
        let kinds: Vec<&str> = events
            .iter()
            .map(|event| event["event"].as_str().expect("event"))
            .collect();
        assert_eq!(kinds, vec!["command_started", "command_finished"]);
        assert_eq!(events[0]["dry_run"], serde_json::json!(true));
        assert_eq!(
            events.last().expect("finished")["exit_code"],
            serde_json::json!(0)
        );
    }
}
