//! Sequential `dx check` / `dx fix` umbrella over the quality phases plus generate.

use super::common::*;
use super::execute;
use super::generate::execute_generate;
use crate::args::{Command, Invocation, ReportRequest};
use crate::plan::spec;
use crate::reports::plan_reports;
use dx_apply::{FileSystem, RealFileSystem};
use dx_output::{
    command_finished, command_started, report_event, write_event, FinishedCounts, OutputMode,
};
use serde_json::{json, Value};
use std::path::PathBuf;

/// Umbrella phases in contract order (WP4): format, lint,
/// typecheck, then generate freshness or mutation.
const UMBRELLA_PHASES: [Command; 4] = [
    Command::Format,
    Command::Lint,
    Command::Typecheck,
    Command::Generate,
];

/// One executed umbrella phase: the phase identity plus the
/// phase-private SARIF capture when the phase supports the format.
struct UmbrellaPhase {
    command: Command,
    sarif_capture: Option<PathBuf>,
}

/// Sequential `dx check` / `dx fix` umbrella (WP4): each phase
/// reuses its wrapped command's scope resolution, Bazel invocation,
/// result collection, mutation, reporting, and exit-status behavior
/// verbatim through [`execute`] with captured streams. The first
/// nonzero phase stops the umbrella; its exit code is preserved. One
/// umbrella `command_started`/`command_finished` pair brackets the
/// verbatim per-phase streams in phase order, `--output diff`
/// concatenates each executed phase's validated patch in phase order,
/// and each SARIF request merges the executed SARIF-capable phases'
/// `runs` in phase order into one document.
pub(crate) fn execute_umbrella(invocation: &Invocation, env: Env<'_>) -> i32 {
    let Env {
        workspace,
        runner,
        query_runner,
        temp_dir,
        pid,
        nonce,
        out,
        err,
        ci,
    } = env;
    let umbrella_check = invocation.command == Command::Check;
    // The umbrella kind dictates the phase mode; an explicit `--check`
    // additionally forces check mode under `fix` (passthrough).
    let phase_check = umbrella_check || invocation.check;
    let mode = if phase_check { "check" } else { "default" };
    // Report planning reuses the umbrella registry (SARIF only in):
    // dry-run conflicts, unsupported formats, and duplicates fail here
    // before any phase starts.
    if let Err(error) = plan_reports(
        invocation.command,
        &invocation.reports,
        &invocation.output,
        invocation.dry_run,
    ) {
        return pre_exec(err, &error.to_string());
    }
    // A stdout report destination would let every phase claim the
    // reserved stdout document: fail closed before execution.
    for request in &invocation.reports {
        if request.destination == "-" {
            return pre_exec(
                err,
                &format!(
                    "option \"--report={}={}\" is not supported by dx {}: phases share one stdout document",
                    request.format,
                    request.destination,
                    invocation.command.name(),
                ),
            );
        }
    }
    if invocation.output == OutputMode::Json {
        if let Ok(event) = command_started(invocation.command.name(), invocation.dry_run, mode) {
            let _ = write_event(out, &event);
        }
    }
    let mut executed: Vec<UmbrellaPhase> = Vec::new();
    let mut stop_code: Option<i32> = None;
    for (index, phase) in UMBRELLA_PHASES.iter().enumerate() {
        // Phases share the temporary directory, so each phase derives
        // its own nonce: BEP streams and intended manifests must never
        // alias across phases, including after a failed phase that
        // leaves its artifacts behind.
        let phase_nonce = nonce.wrapping_add(index as u64);
        // Route SARIF requests through one phase-private capture when
        // the phase registry supports the format; other phases
        // contribute nothing to that request.
        let mut phase_reports = Vec::new();
        let mut sarif_capture: Option<PathBuf> = None;
        for request in &invocation.reports {
            if !spec(*phase).reports.contains(&request.format.as_str()) {
                continue;
            }
            if sarif_capture.is_none() {
                let capture = temp_dir.join(format!(
                    "umbrella-{}-{pid}-{phase_nonce}.sarif",
                    phase.name()
                ));
                let Some(capture_text) = capture.to_str() else {
                    return pre_exec(err, "temporary report path is not UTF-8"); // LCOV_EXCL_LINE - reason: defense-in-depth; capture paths join the ASCII temporary directory with ASCII phase names, so non-UTF-8 paths are unreachable.
                };
                phase_reports.push(ReportRequest {
                    format: request.format.clone(),
                    destination: capture_text.to_owned(),
                });
                sarif_capture = Some(capture);
            } // LCOV_EXCL_LINE - reason: closing brace of a fully covered guard carries no executable region of its own.
        }
        let phase_invocation = Invocation {
            command: *phase,
            check: phase_check,
            // Umbrella phases (format/lint/typecheck/generate) take no
            // profile flags; the parent check/fix rejects them at parse.
            debug: false,
            release: false,
            workspace: invocation.workspace.clone(),
            dry_run: invocation.dry_run,
            quiet: invocation.quiet,
            verbose: invocation.verbose,
            output: invocation.output,
            reports: phase_reports,
            fail_on: invocation.fail_on,
            min_coverage: invocation.min_coverage,
            targets: invocation.targets.clone(),
            bazel_options: invocation.bazel_options.clone(),
            bazel_clean: false,
            pin: None,
            rollback: false,
            configured: false,
        };
        let mut phase_out = Vec::new();
        let mut phase_err = Vec::new();
        let code = {
            let phase_env = Env {
                workspace,
                runner,
                query_runner,
                temp_dir,
                pid,
                nonce: phase_nonce,
                out: &mut phase_out,
                err: &mut phase_err,
                ci,
            };
            if *phase == Command::Generate {
                execute_generate(&phase_invocation, phase_env)
            } else {
                execute(&phase_invocation, phase_env)
            }
        };
        let _ = out.write_all(&phase_out);
        let _ = err.write_all(&phase_err);
        executed.push(UmbrellaPhase {
            command: *phase,
            sarif_capture,
        });
        if code != 0 {
            stop_code = Some(code);
            break;
        }
    }
    // Merged standard reports: one document per request over the
    // executed phases only. Absent or unparsable captures contribute
    // nothing: a completed phase always leaves a validated capture,
    // while a stopped phase's partial follows its own collection rule.
    let complete = stop_code.is_none();
    let fs = RealFileSystem;
    let mut reports_ok = true;
    for request in &invocation.reports {
        let mut runs: Vec<Value> = Vec::new();
        let mut schema = json!("https://json.schemastore.org/sarif-2.1.0.json");
        let mut version = json!("2.1.0");
        for phase in &executed {
            if !spec(phase.command)
                .reports
                .contains(&request.format.as_str())
            {
                continue;
            }
            let Some(capture) = &phase.sarif_capture else {
                continue; // LCOV_EXCL_LINE - reason: by construction a phase supporting the requested format always carries its capture, so this fallback never fires.
            }; // LCOV_EXCL_LINE - reason: closing brace of a fully covered guard carries no executable region of its own.
            let Ok(bytes) = std::fs::read(capture) else {
                continue; // LCOV_EXCL_LINE - reason: captures are written atomically by executed phases and removed only after the merge, so a missing capture file is unreachable without external interference.
            };
            let Ok(document) = serde_json::from_slice::<Value>(&bytes) else {
                continue; // LCOV_EXCL_LINE - reason: captures are rendered by render_sarif, which always emits valid JSON, so an unparsable capture is unreachable.
            };
            if runs.is_empty() {
                if let Some(value) = document.get("$schema") {
                    schema = value.clone();
                }
                if let Some(value) = document.get("version") {
                    version = value.clone();
                }
            }
            if let Some(Value::Array(phase_runs)) = document.get("runs").cloned() {
                runs.extend(phase_runs);
            }
        }
        let document = json!({
            "version": version,
            "$schema": schema,
            "runs": runs,
        })
        .to_string();
        let target = workspace.join(&request.destination);
        let parent_ok = target
            .parent()
            .is_none_or(|parent| parent.as_os_str().is_empty() || parent.is_dir());
        if !(parent_ok && fs.write_atomic(&target, document.as_bytes()).is_ok()) {
            reports_ok = false;
            let detail = format!(
                "failed to write {} report to {}",
                request.format, request.destination
            );
            let _ = writeln!(err, "dx: report_failed: {detail}");
            if invocation.output == OutputMode::Json {
                if let Ok(event) =
                    dx_output::error_event(CODE_REPORT_FAILED, &detail, None, None, None)
                {
                    let _ = write_event(out, &event);
                }
            } // LCOV_EXCL_LINE - reason: closing brace of a fully covered error-reporting guard carries no executable region of its own.
            continue;
        }
        if invocation.output == OutputMode::Json {
            if let Ok(event) = report_event(&request.format, &request.destination, complete) {
                let _ = write_event(out, &event);
            }
        } else if matches!(invocation.output, OutputMode::Text { .. }) {
            let _ = writeln!(
                out,
                "Wrote {} report to {}.",
                request.format, request.destination
            );
        } else if invocation.output == OutputMode::Diff {
            let _ = writeln!(
                err,
                "Wrote {} report to {}.",
                request.format, request.destination
            );
        }
    }
    for phase in &executed {
        if let Some(capture) = &phase.sarif_capture {
            let _ = std::fs::remove_file(capture);
        }
    }
    let code = match stop_code {
        Some(phase_code) if reports_ok => phase_code,
        Some(_) => 1,
        None if reports_ok => 0,
        None => 1,
    };
    if invocation.output == OutputMode::Json {
        let _ = write_event(out, &command_finished(code, &FinishedCounts::default()));
    }
    code
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use crate::plan::GENERATE_ENV_INTENDED;

    #[test]
    fn check_clean_runs_every_phase_in_order() {
        let harness = umbrella_clean("umbrella-check-clean");
        let (code, out, err) = harness.run(&["check", "--output=text"]);
        assert_eq!(code, 0, "{out}{err}");
        let format = out
            .find("Running format analysis for //...")
            .expect("format");
        let lint = out.find("Running lint analysis for //...").expect("lint");
        let typecheck = out
            .find("Running typecheck analysis for //...")
            .expect("typecheck");
        let generate = out.find("Running generate for //...").expect("generate");
        assert!(
            format < lint && lint < typecheck && typecheck < generate,
            "{out}"
        );
        assert_eq!(err, "", "{err}");
        assert_eq!(
            harness.seen_env.borrow().len(),
            4,
            "one Bazel launch per phase"
        );
    }

    #[test]
    fn check_json_brackets_phase_lifecycles() {
        let harness = umbrella_clean("umbrella-check-json");
        let (code, out, err) = harness.run(&["check", "--output=json"]);
        assert_eq!(code, 0, "{out}{err}");
        let events: Vec<serde_json::Value> = out
            .lines()
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()
            .expect("NDJSON");
        assert_eq!(events.first().expect("first")["event"], "command_started");
        assert_eq!(events.first().expect("first")["command"], "check");
        assert_eq!(events.last().expect("last")["event"], "command_finished");
        assert_eq!(events.last().expect("last")["exit_code"], 0);
        let started: Vec<&str> = events
            .iter()
            .filter(|event| event["event"] == "command_started")
            .map(|event| event["command"].as_str().expect("command"))
            .collect();
        assert_eq!(
            started,
            vec!["check", "format", "lint", "typecheck", "generate"]
        );
    }

    #[test]
    fn check_stops_at_first_failing_phase() {
        let harness = umbrella_findings("umbrella-stop");
        let (code, out, _) = harness.run(&["check", "--output=text"]);
        assert_eq!(code, 1, "{out}");
        assert!(out.contains("Running format analysis for //..."), "{out}");
        assert!(!out.contains("Running lint analysis"), "{out}");
        assert!(!out.contains("Running typecheck analysis"), "{out}");
        assert!(!out.contains("Running generate"), "{out}");
        assert_eq!(
            std::fs::read(harness.workspace.join("src/a.py")).expect("source"),
            b"x = 1\n",
            "check mode never mutates"
        );
        assert_eq!(
            harness.seen_env.borrow().len(),
            1,
            "later phases never launch"
        );
    }

    #[test]
    fn check_json_stop_reports_umbrella_failure() {
        let harness = umbrella_findings("umbrella-stop-json");
        let (code, out, _) = harness.run(&["check", "--output=json"]);
        assert_eq!(code, 1, "{out}");
        let events: Vec<serde_json::Value> = out
            .lines()
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()
            .expect("NDJSON");
        assert_eq!(events.first().expect("first")["command"], "check");
        assert_eq!(events.last().expect("last")["event"], "command_finished");
        assert_eq!(events.last().expect("last")["exit_code"], 1);
        let started: Vec<&str> = events
            .iter()
            .filter(|event| event["event"] == "command_started")
            .map(|event| event["command"].as_str().expect("command"))
            .collect();
        assert_eq!(started, vec!["check", "format"]);
    }

    #[test]
    fn fix_check_flag_forces_check_mode() {
        let harness = umbrella_findings("umbrella-fix-check");
        let (code, out, _) = harness.run(&["fix", "--check", "--output=text"]);
        assert_eq!(code, 1, "{out}");
        assert!(out.contains("Running format analysis for //..."), "{out}");
        assert_eq!(
            std::fs::read(harness.workspace.join("src/a.py")).expect("source"),
            b"x = 1\n",
            "forced check mode never mutates"
        );
        assert_eq!(
            harness.seen_env.borrow().len(),
            1,
            "later phases never launch"
        );
    }

    #[test]
    fn fix_applies_generate_mutation_after_clean_quality() {
        let mut harness = Harness::new("umbrella-fix");
        harness.write_source("src/a.py", "x = 1\n");
        harness.write_source("rust/tests/fixtures/hello/BUILD.bazel", "xyz\n");
        harness.results.insert(
            "//test:corpus".to_owned(),
            harness.valid_result(vec![], vec![]),
        );
        harness.intended = Some(intended_witness(
            "default",
            true,
            &intended_modify("rust/tests/fixtures/hello/BUILD.bazel", b"abc\n", b"xyz\n"),
            "",
        ));
        let (code, out, err) = harness.run(&["fix", "--output=text"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(
            out.contains("Modified rust/tests/fixtures/hello/BUILD.bazel"),
            "{out}"
        );
        assert_eq!(
            std::fs::read(harness.workspace.join("src/a.py")).expect("source"),
            b"x = 1\n"
        );
    }

    #[test]
    fn fix_stops_when_later_phase_goes_stale() {
        let harness = umbrella_findings("umbrella-stale");
        let (code, out, _) = harness.run(&["fix", "--output=text"]);
        assert_eq!(code, 1, "{out}");
        assert!(out.contains("Running format analysis for //..."), "{out}");
        assert!(out.contains("Running lint analysis for //..."), "{out}");
        assert!(!out.contains("Running typecheck analysis"), "{out}");
        assert!(!out.contains("Running generate"), "{out}");
        assert_eq!(
            std::fs::read(harness.workspace.join("src/a.py")).expect("source"),
            b"y = 1\n",
            "format applied before lint went stale"
        );
        let launches = harness.seen_env.borrow();
        assert_eq!(launches.len(), 2, "typecheck and generate never launch");
        assert!(
            launches
                .iter()
                .flatten()
                .all(|(key, _)| key != GENERATE_ENV_INTENDED),
            "generate dispatch never ran"
        );
    }

    #[test]
    fn umbrella_stdout_report_is_rejected_pre_exec() {
        let harness = umbrella_clean("umbrella-stdout-report");
        let (code, _, err) = harness.run(&["check", "--report=sarif=-", "--output=text"]);
        assert_eq!(code, 2, "{err}");
        assert!(err.contains("share one stdout document"), "{err}");
        assert!(harness.seen_env.borrow().is_empty(), "no phase launched");
    }

    #[test]
    fn umbrella_unknown_report_is_rejected_pre_exec() {
        let harness = umbrella_clean("umbrella-unknown-report");
        let (code, _, err) = harness.run(&["check", "--report=junit=out.xml"]);
        assert_eq!(code, 2, "{err}");
        assert!(err.contains("unsupported report format"), "{err}");
        assert!(harness.seen_env.borrow().is_empty(), "no phase launched");
    }

    /// Only lint and typecheck carry SARIF in their registries:
    /// format and generate contribute no runs to the merged
    /// document, which concatenates one run per executed
    /// SARIF-capable phase in phase order.
    #[test]
    fn umbrella_sarif_merges_executed_phases_in_order() {
        let harness = umbrella_clean("umbrella-sarif");
        let (code, out, err) = harness.run(&["check", "--output=json", "--report=sarif=out.sarif"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("\"format\":\"sarif\""), "{out}");
        let document: serde_json::Value = serde_json::from_slice(
            &std::fs::read(harness.workspace.join("out.sarif")).expect("sarif"),
        )
        .expect("SARIF JSON");
        assert_eq!(
            document["runs"].as_array().expect("runs").len(),
            2,
            "lint and typecheck each contribute one run"
        );
        assert_eq!(
            document["runs"][0], document["runs"][1],
            "both phases replay the shared clean fixture"
        );

        let (code, out, _) = harness.run(&["check", "--output=text", "--report=sarif=out.sarif"]);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("Wrote sarif report to out.sarif."), "{out}");
    }

    #[test]
    fn umbrella_sarif_covers_only_executed_phases() {
        // The format phase fails before any SARIF-capable phase
        // runs, so the merged document is a valid empty run list.
        let harness = umbrella_findings("umbrella-sarif-stop");
        let (code, _, _) = harness.run(&["check", "--output=json", "--report=sarif=out.sarif"]);
        assert_eq!(code, 1);
        let document: serde_json::Value = serde_json::from_slice(
            &std::fs::read(harness.workspace.join("out.sarif")).expect("sarif"),
        )
        .expect("SARIF JSON");
        assert_eq!(
            document["runs"].as_array().expect("runs").len(),
            0,
            "no executed SARIF-capable phase, no runs"
        );

        let (code, _, err) = harness.run(&["check", "--output=diff", "--report=sarif=out.sarif"]);
        assert_eq!(code, 1);
        assert!(err.contains("Wrote sarif report to out.sarif."), "{err}");
    }

    #[test]
    fn umbrella_report_write_failure_fails() {
        let harness = umbrella_clean("umbrella-report-fail");
        let (code, _, err) = harness.run(&[
            "check",
            "--output=text",
            "--report=sarif=missing-dir/out.sarif",
        ]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("report_failed"), "{err}");

        let (code, out, _) = harness.run(&[
            "check",
            "--output=json",
            "--report=sarif=missing-dir/out.sarif",
        ]);
        assert_eq!(code, 1, "{out}");
        assert!(out.contains("\"code\":\"report_failed\""), "{out}");
    }

    #[test]
    fn umbrella_failed_phase_and_failed_report_still_fails() {
        let harness = umbrella_findings("umbrella-report-fail-stop");
        let (code, _, err) = harness.run(&[
            "check",
            "--output=text",
            "--report=sarif=missing-dir/out.sarif",
        ]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("report_failed"), "{err}");
    }

    #[test]
    fn fix_json_reports_umbrella_lifecycle() {
        let mut harness = umbrella_clean("umbrella-fix-json");
        harness.intended = Some(intended_witness("default", true, "", ""));
        let (code, out, err) = harness.run(&["fix", "--output=json"]);
        assert_eq!(code, 0, "{out}{err}");
        let events: Vec<serde_json::Value> = out
            .lines()
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()
            .expect("NDJSON");
        assert_eq!(events.first().expect("first")["command"], "fix");
        assert_eq!(
            events.first().expect("first")["mode"],
            "default",
            "fix mutates without --check"
        );
        assert_eq!(events.last().expect("last")["exit_code"], 0);
    }

    #[test]
    fn umbrella_diff_stops_after_first_phase_patch() {
        let harness = umbrella_findings("umbrella-diff");
        let (code, out, _) = harness.run(&["check", "--output=diff"]);
        assert_eq!(code, 1, "{out}");
        assert!(out.contains("src/a.py"), "{out}");
        assert_eq!(
            harness.seen_env.borrow().len(),
            1,
            "later phases never launch"
        );
    }
}
