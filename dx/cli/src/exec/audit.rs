//! Audit command execution: planning is live, live execution stays deferred.

use super::common::*;
use crate::args::{Command, Invocation};
use crate::reports::plan_reports;
use dx_output::{command_finished, command_started, write_event, FinishedCounts, OutputMode};

/// Runs `dx audit` planning (M26 WP1 slice 1): family selection and
/// scope defaults through `dx_audit`, never the quality aspect
/// pipeline. `--dry-run` prints the planned families and scopes and
/// exits `0`; live execution fails closed with `audit_deferred`
/// because auditor wiring, advisory acquisition, and SARIF mapping
/// land in later M26 slices (O11/O58). Audit is non-mutating.
pub(crate) fn execute_audit(invocation: &Invocation, env: Env<'_>) -> i32 {
    debug_assert!(
        invocation.command == Command::Audit,
        "audit dispatch guards commands"
    );
    let Env { out, err, .. } = env;
    let request = match dx_audit::plan_audit(&invocation.targets) {
        Ok(request) => request,
        Err(error) => return pre_exec(err, &error.to_string()),
    };
    match plan_reports(
        invocation.command,
        &invocation.reports,
        &invocation.output,
        invocation.dry_run,
    ) {
        Ok(_) => {}
        Err(error) => return pre_exec(err, &error.to_string()),
    }
    let families = request
        .families
        .iter()
        .map(|family| family.as_str())
        .collect::<Vec<_>>()
        .join("+");
    let scopes = request.effective_scopes().join(", ");
    let summary = format!("Running audit {families} for {scopes}");
    let verbose =
        matches!(invocation.output, OutputMode::Text { quiet: false }) && !invocation.quiet;
    if invocation.dry_run {
        if invocation.output == OutputMode::Json {
            if let Ok(event) = command_started(invocation.command.name(), true, "default") {
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
    operational(
        invocation,
        out,
        err,
        CODE_AUDIT_DEFERRED,
        "audit tool execution is deferred: auditor wiring, advisory acquisition, and SARIF mapping land in later M26 slices (O11/O58); use --dry-run for planning",
    )
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;

    #[test]
    fn audit_dry_run_plans_families_without_launching() {
        let harness = Harness::new("audit-dryrun");
        let (code, out, err) = harness.run(&["audit", "--dry-run"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(
            out.contains("Running audit security+license for //..."),
            "{out}"
        );
        assert_eq!(err, "", "{err}");
        assert!(
            harness.seen_env.borrow().is_empty(),
            "dry-run launches nothing"
        );

        let harness = Harness::new("audit-dryrun-family");
        let (code, out, err) = harness.run(&["audit", "security", "--dry-run"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("Running audit security for //..."), "{out}");
        assert_eq!(err, "", "{err}");
        assert!(
            harness.seen_env.borrow().is_empty(),
            "dry-run launches nothing"
        );
    }

    #[test]
    fn audit_live_is_deferred_operational() {
        let harness = Harness::new("audit-live");
        let (code, out, err) = harness.run(&["audit"]);
        assert_eq!(code, 1, "{out}{err}");
        assert!(err.contains("audit_deferred"), "{err}");
        assert!(
            harness.seen_env.borrow().is_empty(),
            "deferred run launches nothing"
        );
    }

    #[test]
    fn audit_dry_run_json_emits_lifecycle() {
        let harness = Harness::new("audit-dryrun-json");
        let (code, out, err) = harness.run(&["audit", "--dry-run", "--output=json"]);
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
        assert_eq!(kinds, vec!["command_started", "command_finished"]);
        assert_eq!(
            events.last().expect("finished")["exit_code"],
            serde_json::json!(0)
        );
        assert!(
            harness.seen_env.borrow().is_empty(),
            "dry-run launches nothing"
        );
    }

    #[test]
    fn audit_live_json_emits_deferred_lifecycle() {
        let harness = Harness::new("audit-live-json");
        let (code, out, err) = harness.run(&["audit", "--output=json"]);
        assert_eq!(code, 1, "{out}{err}");
        assert!(err.contains("audit_deferred"), "{err}");
        let events: Vec<serde_json::Value> = out
            .lines()
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()
            .expect("NDJSON");
        let kinds: Vec<&str> = events
            .iter()
            .map(|event| event["event"].as_str().expect("event"))
            .collect();
        assert_eq!(kinds, vec!["command_started", "error", "command_finished"]);
        assert!(
            harness.seen_env.borrow().is_empty(),
            "deferred run launches nothing"
        );
    }

    #[test]
    fn audit_update_dry_run_quiet_prints_nothing() {
        for argv in [
            vec!["audit", "--dry-run", "--quiet"],
            vec!["update", "--dry-run", "--quiet"],
        ] {
            let name = format!("dryrun-quiet-{}", argv[0]);
            let harness = Harness::new(&name);
            let (code, out, err) = harness.run(&argv);
            assert_eq!(code, 0, "{out}{err}");
            assert_eq!(out, "", "{out}");
            assert_eq!(err, "", "{err}");
            assert!(
                harness.seen_env.borrow().is_empty(),
                "dry-run launches nothing"
            );
        }
    }
}
