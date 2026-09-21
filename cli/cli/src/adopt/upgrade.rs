//! Adoption upgrade execution (`upgrade`).
//!
//! Split from `super` (`adopt.rs`): owns [`execute_upgrade`], the
//! one-shot pin plus migrate plus setup composition (dry-run plans are
//! summaries, suppressed under `--quiet`). Re-exported through `super`
//! so the dispatch path stays `crate::adopt::execute_adoption`.
//!
//! See: `docs/cli/commands/new-upgrade.md`.

use std::io::Write;

use crate::args::Invocation;
use dx_output::{
    command_finished, command_started, error_event, notice_event, write_event, FinishedCounts,
    NoticeEvent, OutputMode,
};
use dx_process::operational_code;

use super::{operational, pre_exec, summaries_suppressed};

/// Runs `dx upgrade --from <version> --to <version>`: validates the pair
/// through `dx_adopt::plan_upgrade` (Cargo-flavor semver, upgrade-only
/// gate, migrate manifest selection), then fails closed because no
/// manifests exist yet (module at `0.0.0`, no releases cut).
/// `--dry-run` plans the pin plus migrate plus setup composition without
/// touching the tree; live execution reports `upgrade_failed` (exit 1)
/// with the recovery pointer and no writes. Usage errors (missing
/// `--from`/`--to`, non-semver, downgrades/equal versions) exit `2`
/// before any write.
pub(crate) fn execute_upgrade(
    invocation: &Invocation,
    _workspace: &std::path::Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let (Some(from), Some(to)) = (invocation.from.as_deref(), invocation.to.as_deref()) else {
        return pre_exec(err, "upgrade needs --from <version> --to <version>");
    };
    let plan = match dx_adopt::plan_upgrade(from, to) {
        Ok(plan) => plan,
        Err(error) => return pre_exec(err, &error.to_string()),
    };
    let summary = format!(
        "Would upgrade {} -> {} via {} (pin {}, migrate, setup)",
        plan.from, plan.to, plan.manifest, plan.to
    );
    let verbose =
        matches!(invocation.output, OutputMode::Text { quiet: false }) && !invocation.quiet;
    if invocation.dry_run {
        if invocation.output == OutputMode::Json {
            if let Ok(event) = command_started(invocation.command.name(), true, "default") {
                let _ = write_event(out, &event);
            }
            if let Ok(event) = notice_event(&NoticeEvent {
                level: "info".to_owned(),
                code: "upgrade_planned".to_owned(),
                message: summary,
                related_command: Some("upgrade".to_owned()),
                scope: Some(vec![plan.manifest.clone()]),
                path: Some(plan.manifest.clone()),
                language: None,
                import: None,
            }) {
                let _ = write_event(out, &event);
            }
            let finished = command_finished(0, &FinishedCounts::default());
            let _ = write_event(out, &finished);
        } else if !summaries_suppressed(invocation) {
            let _ = writeln!(out, "{summary}");
        }
        return 0;
    }
    let live_summary = format!(
        "Upgrade {} -> {} via {} (pin {}, migrate, setup)",
        plan.from, plan.to, plan.manifest, plan.to
    );
    let message = format!(
        "no upgrade manifest {} yet (module at 0.0.0, no releases cut); {}",
        plan.manifest, plan.message
    );
    if invocation.output == OutputMode::Json {
        if let Ok(event) = command_started(invocation.command.name(), false, "default") {
            let _ = write_event(out, &event);
        }
        let _ = writeln!(err, "dx: upgrade_failed: {message}");
        if let Ok(event) = error_event("upgrade_failed", &message, None, None, None) {
            let _ = write_event(out, &event);
        }
        let finished = command_finished(
            operational_code(),
            &FinishedCounts {
                results_complete: Some(false),
                ..FinishedCounts::default()
            },
        );
        let _ = write_event(out, &finished);
        return operational_code();
    }
    if verbose {
        let _ = writeln!(out, "{live_summary}");
    }
    operational(out, err, &format!("upgrade_failed: {message}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adopt::{execute_adoption, AdoptEnv};
    use crate::args::parse;
    use dx_process::pre_exec_code;
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

    #[test]
    fn upgrade_dry_run_plans_composition_without_writing() {
        let inv = invocation(&["upgrade", "--from=1.2.3", "--to=2.0.0", "--dry-run"]);
        let scratch = dx_test_scratch::scratch("dx-adopt-upgrade-dry-");
        let root = scratch.path().to_path_buf();
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
        let text = String::from_utf8(out).expect("out");
        assert!(text.contains("1.2.3 -> 2.0.0"), "{text}");
        assert!(text.contains("migrate-v1-to-v2.json"), "{text}");
        assert!(text.contains("pin 2.0.0"), "{text}");
        assert!(text.contains("setup"), "{text}");
    }

    #[test]
    fn upgrade_dry_run_json_emits_planned_and_finished() {
        let inv = invocation(&[
            "upgrade",
            "--from=1.2.3",
            "--to=2.0.0",
            "--dry-run",
            "--output=json",
        ]);
        let scratch = dx_test_scratch::scratch("dx-adopt-upgrade-dry-json-");
        let root = scratch.path().to_path_buf();
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
        assert_eq!(kinds[0], "command_started");
        assert_eq!(kinds[kinds.len() - 1], "command_finished");
        assert!(kinds.contains(&"notice"), "{kinds:?}");
        assert_eq!(
            events.last().expect("finished")["exit_code"],
            serde_json::json!(0)
        );
    }

    #[test]
    fn upgrade_live_fails_closed_with_recovery_pointer() {
        let inv = invocation(&["upgrade", "--from=1.2.3", "--to=2.0.0"]);
        let scratch = dx_test_scratch::scratch("dx-adopt-upgrade-live-");
        let root = scratch.path().to_path_buf();
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
        assert_eq!(code, 1);
        let err_text = String::from_utf8(err).expect("err");
        assert!(err_text.contains("upgrade_failed"), "{err_text}");
        assert!(err_text.contains("migrate-v1-to-v2.json"), "{err_text}");
        assert!(
            err_text.contains("dx upgrade --from 1.2.3 --to 2.0.0"),
            "{err_text}"
        );
        assert!(err_text.contains("dx setup"), "{err_text}");
    }

    #[test]
    fn upgrade_live_json_fails_closed_with_error_and_finished() {
        let inv = invocation(&["upgrade", "--from=1.2.3", "--to=2.0.0", "--output=json"]);
        let scratch = dx_test_scratch::scratch("dx-adopt-upgrade-live-json-");
        let root = scratch.path().to_path_buf();
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
        assert_eq!(code, 1);
        assert!(String::from_utf8(err)
            .expect("err")
            .contains("upgrade_failed"));
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
        assert_eq!(kinds[0], "command_started");
        assert!(kinds.contains(&"error"), "{kinds:?}");
        assert_eq!(kinds[kinds.len() - 1], "command_finished");
    }

    #[test]
    fn upgrade_rejects_downgrade_pre_exec() {
        let inv = invocation(&["upgrade", "--from=2.0.0", "--to=1.0.0", "--dry-run"]);
        let scratch = dx_test_scratch::scratch("dx-adopt-upgrade-downgrade-");
        let root = scratch.path().to_path_buf();
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
        assert_eq!(code, pre_exec_code());
        assert!(String::from_utf8(err)
            .expect("err")
            .contains("upgrade-only"));
    }
}
