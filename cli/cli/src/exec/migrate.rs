//! Migrate command execution: major-release-only planning plus
//! fail-closed execution.

use super::common::*;
use crate::args::{Command, Invocation};
use crate::reports::plan_reports;
use dx_output::{
    command_finished, command_started, notice_event, write_event, FinishedCounts, NoticeEvent,
    OutputMode,
};

/// Runs `dx migrate --from <version> --to <version> [scope ...]`
///: validates the version pair through
/// `dx_adopt::plan_migrate` (Cargo-flavor semver, major-release-only
/// gate, one manifest per major hop
/// `migrate-v<from_major>-to-v<to_major>.json`), then fails closed
/// because no manifests exist yet (module at `0.0.0`, no releases
/// cut). `--dry-run` plans without touching the tree; live execution
/// reports `migrate_failed` (exit 1) with no writes. Usage errors
/// (missing `--from`/`--to`, non-semver, non-major bumps) exit `2`
/// before any write.
pub(crate) fn execute_migrate(invocation: &Invocation, env: Env<'_>) -> i32 {
    debug_assert!(
        invocation.command == Command::Migrate,
        "migrate dispatch guards commands"
    );
    let Env { out, err, .. } = env;
    match plan_reports(
        invocation.command,
        &invocation.reports,
        &invocation.output,
        invocation.dry_run,
    ) {
        Ok(_) => {}
        Err(error) => return pre_exec(err, &error.to_string()),
    }
    let (Some(from), Some(to)) = (invocation.from.as_deref(), invocation.to.as_deref()) else {
        return pre_exec(err, "migrate needs --from <version> --to <version>");
    };
    let plan = match dx_adopt::plan_migrate(from, to) {
        Ok(plan) => plan,
        Err(error) => return pre_exec(err, &error.to_string()),
    };
    let summary = format!(
        "Would migrate {} -> {} via {}",
        plan.from, plan.to, plan.manifest
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
                code: "migrate_planned".to_owned(),
                message: summary,
                related_command: Some("migrate".to_owned()),
                scope: Some(vec![plan.manifest.clone()]),
                path: Some(plan.manifest.clone()),
                language: None,
                import: None,
            }) {
                let _ = write_event(out, &event);
            }
            let finished = command_finished(0, &FinishedCounts::default());
            let _ = write_event(out, &finished);
        } else if verbose {
            let _ = writeln!(out, "{summary}");
        }
        return 0;
    }
    if invocation.output == OutputMode::Json {
        if let Ok(event) = command_started(invocation.command.name(), false, "default") {
            let _ = write_event(out, &event);
        }
    } else if verbose {
        let _ = writeln!(out, "{summary}");
    }
    // Live: no manifests exist yet (module at `0.0.0`), so fail closed
    // with no writes — the same discipline as the deferred audit/update
    // paths before backends landed.
    operational(
        invocation,
        out,
        err,
        CODE_MIGRATE_FAILED,
        &format!(
            "no migrate manifest {} yet (module at 0.0.0, no major releases cut)",
            plan.manifest
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;

    #[test]
    fn dry_run_plans_major_bump_without_writing() {
        let harness = Harness::new("migrate-dryrun");
        let (code, out, err) = harness.run(&["migrate", "--from=1.2.3", "--to=2.0.0", "--dry-run"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("migrate-v1-to-v2.json"), "{out}");
        assert!(out.contains("1.2.3 -> 2.0.0"), "{out}");
        assert_eq!(err, "", "{err}");
    }

    #[test]
    fn dry_run_json_emits_planned_and_finished() {
        let harness = Harness::new("migrate-dryrun-json");
        let (code, out, err) = harness.run(&[
            "migrate",
            "--from=1.2.3",
            "--to=2.0.0",
            "--dry-run",
            "--output=json",
        ]);
        assert_eq!(code, 0, "{out}{err}");
        let events: Vec<serde_json::Value> = out
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
        assert_eq!(err, "", "{err}");
    }

    #[test]
    fn live_fails_closed_with_migrate_failed() {
        let harness = Harness::new("migrate-live-closed");
        let (code, _, err) = harness.run(&["migrate", "--from=1.2.3", "--to=2.0.0"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("migrate_failed"), "{err}");
        assert!(err.contains("migrate-v1-to-v2.json"), "{err}");
    }

    #[test]
    fn live_json_fails_closed_with_error_and_finished() {
        let harness = Harness::new("migrate-live-json");
        let (code, out, err) =
            harness.run(&["migrate", "--from=1.2.3", "--to=2.0.0", "--output=json"]);
        assert_eq!(code, 1, "{out}{err}");
        assert!(err.contains("migrate_failed"), "{err}");
        let events: Vec<serde_json::Value> = out
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
    fn missing_versions_and_non_major_are_pre_exec() {
        // Missing `--to` never reaches execution: `parse` rejects it
        // with `MissingValue` (exit 2) before any write.
        use crate::args::parse;
        fn args(words: &[&str]) -> Vec<String> {
            words.iter().map(ToString::to_string).collect()
        }
        assert!(matches!(
            parse(&args(&["migrate", "--from=1.2.3", "--dry-run"])),
            Err(crate::args::ArgsError::MissingValue { .. })
        ));
        // Well-formed versions that fail the major gate reach execution
        // and fail pre-exec with the stable gate diagnostic.
        let harness = Harness::new("migrate-nonmajor");
        let (code, _, err) = harness.run(&["migrate", "--from=1.2.3", "--to=1.3.0", "--dry-run"]);
        assert_eq!(code, 2, "{err}");
        assert!(err.contains("major-release-only"), "{err}");
        let harness = Harness::new("migrate-badsemver");
        let (code, _, err) = harness.run(&["migrate", "--from=abc", "--to=2.0.0", "--dry-run"]);
        assert_eq!(code, 2, "{err}");
    }

    #[test]
    fn from_to_belong_to_migrate_only() {
        use crate::args::parse;
        fn args(words: &[&str]) -> Vec<String> {
            words.iter().map(ToString::to_string).collect()
        }
        assert!(matches!(
            parse(&args(&["lint", "--from=1.0.0"])),
            Err(crate::args::ArgsError::UnsupportedOption { .. })
        ));
        assert!(matches!(
            parse(&args(&["build", "//a:one", "--to=2.0.0"])),
            Err(crate::args::ArgsError::UnsupportedOption { .. })
        ));
    }
}
