//! Build/test/coverage workflow execution with JUnit/LCOV collection and report rendering.

use super::common::*;
use super::run::execute_run;
use crate::args::{Command, Invocation};
use crate::plan::{bep_path, plan_workflow, WorkflowVerb};
use crate::reports::{
    coverage_line_rate, junit_infrastructure_case, parse_test_xml, plan_reports, render_junit,
    validate_lcov, Destination, JunitCase, PlannedReport,
};
use crate::resolve::{resolve, resolve_for_test};
use dx_apply::{FileSystem, RealFileSystem};
use dx_bep::{collect_test_outputs, ArtifactReader};
use dx_output::{
    command_finished, command_started, report_event, write_event, FinishedCounts, OutputMode,
};
use dx_process::ForwardError;
use std::collections::BTreeMap;
use std::io::{BufReader, Write};
use std::path::Path;

/// Workflow dispatch: `build`/`test`/`coverage` preserve Bazel status
/// with JUnit/LCOV collection; `run` preserves the application status
/// verbatim. Pre-execution usage failures exit 2; operational failures
/// exit 1.
pub(crate) fn execute_workflow(invocation: &Invocation, env: Env<'_>) -> i32 {
    if invocation.command == Command::Run {
        return execute_run(invocation, env);
    }
    let Env {
        workspace,
        runner,
        query_runner,
        temp_dir,
        pid,
        nonce,
        out,
        err,
        ci: _,
    } = env;
    let planned_reports = match plan_reports(
        invocation.command,
        &invocation.reports,
        &invocation.output,
        invocation.dry_run,
    ) {
        Ok(planned) => planned,
        Err(error) => return pre_exec(err, &error.to_string()),
    };
    let stdout_report = planned_reports
        .iter()
        .any(|report| report.destination == Destination::Stdout);
    let resolved = match invocation.command {
        Command::Build => resolve(&invocation.targets, workspace, query_runner),
        Command::Test | Command::Coverage => {
            resolve_for_test(&invocation.targets, workspace, query_runner)
        }
        _ => {
            return pre_exec(
                err,
                &ForwardError::UnsupportedCommand {
                    command: invocation.command.name().to_owned(),
                }
                .to_string(),
            );
        }
    };
    let resolved = match resolved {
        Ok(resolved) => resolved,
        Err(error) => return pre_exec(err, &error.to_string()),
    };
    let Some(verb) = WorkflowVerb::of(invocation.command) else {
        return pre_exec(
            err,
            &ForwardError::UnsupportedCommand {
                command: invocation.command.name().to_owned(),
            }
            .to_string(),
        );
    };
    let bep = bep_path(temp_dir, pid, nonce);
    let bep_text = bep.to_str().map(ToString::to_string);
    let Some(bep_text) = bep_text else {
        return operational(
            invocation,
            out,
            err,
            CODE_UNREADABLE_BEP,
            "temporary event path is not UTF-8",
        );
    };
    let bep_arg = if verb.collects_reports() {
        Some(bep_text.as_str())
    } else {
        None
    };
    let plan = match plan_workflow(verb, &resolved, &invocation.bazel_options, bep_arg) {
        Ok(plan) => plan,
        Err(error) => return pre_exec(err, &format!("{error:?}")),
    };
    if invocation.dry_run {
        if invocation.output == OutputMode::Json {
            if let Ok(event) = command_started(invocation.command.name(), true, "default") {
                let _ = write_event(out, &event);
            }
            let _ = write_event(out, &command_finished(0, &FinishedCounts::default()));
        } else if !matches!(invocation.output, OutputMode::Diff)
            && !matches!(invocation.output, OutputMode::Text { quiet: true })
            && !stdout_report
            && !invocation.quiet
        {
            let _ = writeln!(out, "{}", plan.summary);
        }
        return 0;
    }
    if invocation.output == OutputMode::Json {
        if let Ok(event) = command_started(invocation.command.name(), false, "default") {
            let _ = write_event(out, &event);
        }
    } else if matches!(invocation.output, OutputMode::Text { quiet: false })
        && !stdout_report
        && !invocation.quiet
    {
        let _ = writeln!(out, "{}", plan.summary);
    }
    let status = match runner.run(&plan.argv, workspace, &[]) {
        Ok(status) => status,
        Err(error) => {
            return operational(
                invocation,
                out,
                err,
                CODE_LAUNCH_FAILED,
                &format!("failed to launch Bazel: {error}"),
            );
        }
    };
    let Some(bazel_code) = status.code else {
        return operational(
            invocation,
            out,
            err,
            CODE_BAZEL_SIGNALLED,
            "Bazel terminated by signal",
        );
    };
    if verb == WorkflowVerb::Build {
        if invocation.output == OutputMode::Json {
            let _ = write_event(
                out,
                &command_finished(bazel_code, &FinishedCounts::default()),
            );
        }
        return bazel_code;
    }
    execute_test_reports(TestReportsRequest {
        invocation,
        workspace,
        out,
        err,
        verb,
        bep: &bep,
        planned_reports: &planned_reports,
        stdout_report,
        bazel_code,
    })
}

/// Inputs to [`execute_test_reports`]: CLI plumbing plus the report plan
/// and Bazel outcome. Grouped so the 9-argument dispatch takes one value
/// instead of nine positional arguments.
struct TestReportsRequest<'a> {
    invocation: &'a Invocation,
    workspace: &'a Path,
    out: &'a mut dyn Write,
    err: &'a mut dyn Write,
    verb: WorkflowVerb,
    bep: &'a Path,
    planned_reports: &'a [PlannedReport],
    stdout_report: bool,
    bazel_code: i32,
}

/// bytes, renders requested reports, and selects the workflow exit
/// code: Bazel's exact nonzero code is preserved; success with
/// incomplete collection or failed reports exits 1.
fn execute_test_reports(request: TestReportsRequest<'_>) -> i32 {
    let TestReportsRequest {
        invocation,
        workspace,
        out,
        err,
        verb,
        bep,
        planned_reports,
        stdout_report,
        bazel_code,
    } = request;
    let outputs = match std::fs::File::open(bep).map_err(|err| {
        (
            CODE_UNREADABLE_BEP.to_owned(),
            format!("failed to read build events: {err}"),
        )
    }) {
        Ok(file) => match collect_test_outputs(BufReader::new(file)) {
            Ok(outputs) => outputs,
            Err(error) => {
                let _ = std::fs::remove_file(bep);
                return operational(
                    invocation,
                    out,
                    err,
                    CODE_INVALID_BEP,
                    &format!("invalid build events: {error:?}"),
                );
            }
        },
        Err((code, message)) => {
            let _ = std::fs::remove_file(bep);
            return operational(invocation, out, err, &code, &message);
        }
    };
    let _ = std::fs::remove_file(bep);
    let reader = FsArtifacts;
    let mut complete = bazel_code == 0;
    let mut detail = String::new();
    let mut suites: Vec<(String, Vec<JunitCase>)> = Vec::new();
    let mut lcov_documents: Vec<String> = Vec::new();
    if verb == WorkflowVerb::Test {
        let mut grouped: BTreeMap<String, Vec<JunitCase>> = BTreeMap::new();
        for output in &outputs {
            if output.name != "test.xml" {
                continue;
            }
            let bytes = match reader.read_artifact(&output.exec_path) {
                Ok(bytes) => bytes,
                Err(error) => {
                    complete = false;
                    if detail.is_empty() {
                        detail = format!("unreadable {}: {error}", output.exec_path.display());
                    }
                    continue;
                }
            };
            let shard = output.shard.saturating_sub(1);
            let attempt = output.attempt.saturating_sub(1);
            match parse_test_xml(&bytes, shard, attempt) {
                Ok(cases) => grouped
                    .entry(output.label.clone())
                    .or_default()
                    .extend(cases),
                Err(error) => {
                    complete = false;
                    if detail.is_empty() {
                        detail = format!("invalid {}: {error}", output.exec_path.display());
                    }
                }
            }
        }
        if grouped.is_empty() {
            complete = false;
            if detail.is_empty() {
                detail = "no test.xml artifacts were reported".to_owned();
            }
        }
        suites = grouped.into_iter().collect();
        if !complete {
            suites.push(junit_infrastructure_case(&detail));
        }
    } else {
        for output in &outputs {
            if output.name != "coverage.dat" && output.name != "test.lcov" {
                continue;
            }
            let bytes = match reader.read_artifact(&output.exec_path) {
                Ok(bytes) => bytes,
                Err(error) => {
                    complete = false;
                    if detail.is_empty() {
                        detail = format!("unreadable {}: {error}", output.exec_path.display());
                    }
                    continue;
                }
            };
            match validate_lcov(&bytes) {
                Ok(()) => lcov_documents.push(String::from_utf8_lossy(&bytes).into_owned()),
                Err(error) => {
                    complete = false;
                    if detail.is_empty() {
                        detail = format!("invalid {}: {error}", output.exec_path.display());
                    }
                }
            }
        }
        if lcov_documents.is_empty() {
            complete = false;
            if detail.is_empty() {
                detail = "no coverage.dat artifacts were reported".to_owned();
            }
        }
    }
    let fs = RealFileSystem;
    let mut reports_ok = true;
    for planned in planned_reports {
        let document = match verb {
            WorkflowVerb::Test => Some(render_junit(&suites)),
            WorkflowVerb::Coverage => {
                if lcov_documents.is_empty() {
                    None
                } else {
                    let mut combined = lcov_documents.join("\n");
                    if !combined.ends_with('\n') {
                        combined.push('\n');
                    }
                    Some(combined)
                }
            }
            _ => None, // LCOV_EXCL_LINE - reason: defense-in-depth; execute_test_reports is only reached for Test/Coverage (Build returns early, Run returns earlier), so Build/Run arms are unreachable
        };
        let Some(document) = document else {
            reports_ok = false;
            let detail = format!(
                "failed to render {} report: {detail}",
                planned.format.name()
            );
            let _ = writeln!(err, "dx: report_failed: {detail}");
            if invocation.output == OutputMode::Json {
                if let Ok(event) =
                    dx_output::error_event(CODE_REPORT_FAILED, &detail, None, None, None)
                {
                    let _ = write_event(out, &event);
                }
            }
            continue;
        };
        let written = match &planned.destination {
            Destination::Stdout => out
                .write_all(document.as_bytes())
                .and_then(|()| out.write_all(b"\n"))
                .is_ok(),
            Destination::File(destination) => {
                let target = workspace.join(destination);
                let parent_ok = target
                    .parent()
                    .is_none_or(|parent| parent.as_os_str().is_empty() || parent.is_dir());
                parent_ok && fs.write_atomic(&target, document.as_bytes()).is_ok()
            }
        };
        if !written {
            reports_ok = false;
            let detail = format!(
                "failed to write {} report to {}",
                planned.format.name(),
                planned.destination.display()
            );
            let _ = writeln!(err, "dx: report_failed: {detail}");
            if invocation.output == OutputMode::Json {
                if let Ok(event) =
                    dx_output::error_event(CODE_REPORT_FAILED, &detail, None, None, None)
                {
                    let _ = write_event(out, &event);
                }
            }
            continue;
        }
        if invocation.output == OutputMode::Json {
            if let Ok(event) = report_event(
                planned.format.name(),
                planned.destination.display(),
                complete && reports_ok,
            ) {
                let _ = write_event(out, &event);
            }
        } else if matches!(invocation.output, OutputMode::Text { .. }) && !stdout_report {
            let _ = writeln!(
                out,
                "Wrote {} report to {}.",
                planned.format.name(),
                planned.destination.display()
            );
        } else if invocation.output == OutputMode::Diff {
            let _ = writeln!(
                err,
                "Wrote {} report to {}.",
                planned.format.name(),
                planned.destination.display()
            );
        }
    }
    if !complete && !detail.is_empty() && planned_reports.is_empty() {
        if invocation.output == OutputMode::Json {
            if let Ok(event) =
                dx_output::error_event("incomplete_results", &detail, None, None, None)
            {
                let _ = write_event(out, &event);
            }
        } else {
            let _ = writeln!(err, "dx: incomplete_results: {detail}");
        }
    }
    let mut threshold_ok = true;
    if verb == WorkflowVerb::Coverage {
        if let Some(minimum) = invocation.min_coverage {
            let summary = match coverage_line_rate(&lcov_documents, &|path| {
                std::fs::read_to_string(workspace.join(path)).ok()
            }) {
                Ok((covered, eligible)) if eligible > 0 => {
                    let percent = 100.0 * covered as f64 / eligible as f64;
                    let passed = covered * 100 >= u64::from(minimum) * eligible;
                    threshold_ok = passed;
                    if passed {
                        format!(
                            "coverage {percent:.2}% ({covered}/{eligible} lines) meets minimum {minimum}%"
                        )
                    } else {
                        format!(
                            "coverage_below_minimum: coverage {percent:.2}% ({covered}/{eligible} lines) below minimum {minimum}%"
                        )
                    }
                }
                Ok(_) => {
                    threshold_ok = false;
                    "coverage_below_minimum: no executable lines in the collected LCOV".to_owned()
                }
                Err(error) => {
                    threshold_ok = false;
                    format!("coverage_below_minimum: {error}")
                }
            };
            if invocation.output == OutputMode::Json {
                if !threshold_ok {
                    if let Ok(event) = dx_output::error_event(
                        CODE_COVERAGE_BELOW_MINIMUM,
                        &summary,
                        None,
                        None,
                        None,
                    ) {
                        let _ = write_event(out, &event);
                    }
                }
            } else {
                let _ = writeln!(err, "dx: {summary}");
            }
        }
    }
    let code = if bazel_code != 0 {
        bazel_code
    } else if complete && reports_ok && threshold_ok {
        0
    } else {
        1
    };
    if invocation.output == OutputMode::Json {
        let _ = write_event(
            out,
            &command_finished(
                code,
                &FinishedCounts {
                    results_complete: Some(complete && reports_ok),
                    ..FinishedCounts::default()
                },
            ),
        );
    }
    code
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    use std::path::PathBuf;

    const MINIMAL_TEST_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?><testsuites><testsuite name="s"><testcase name="passes" classname="c" time="0.1"/></testsuite></testsuites>"#;

    const MINIMAL_LCOV: &str = "SF:src/a.py\nDA:1,1\nend_of_record\n";

    #[test]
    fn build_preserves_bazel_status_verbatim() {
        let harness = Harness::new("build-ok");
        let (code, out, _) = harness.run(&["build", "--output=text"]);
        assert_eq!(code, 0);
        assert!(out.contains("Running build for //..."), "{out}");
        let harness = Harness {
            bazel_code: 3,
            ..Harness::new("build-fails")
        };
        let (code, _, _) = harness.run(&["build", "--output=text"]);
        assert_eq!(code, 3);
    }

    #[test]
    fn test_junit_file_report_succeeds() {
        let harness = Harness::new("test-junit");
        let uri = write_bep_artifact(&harness, "test.xml", MINIMAL_TEST_XML.as_bytes());
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.xml"), uri)],
            )]),
            ..harness
        };
        let (code, out, _) = harness.run(&["test", "--output=text", "--report=junit=out.xml"]);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("Wrote junit report to out.xml."));
        let document = std::fs::read(harness.workspace.join("out.xml")).expect("junit");
        let text = String::from_utf8(document).expect("utf8");
        assert!(text.contains("<testsuites name=\"dx\""), "{text}");
        assert!(text.contains("<testsuite name=\"//a:t\""), "{text}");
    }

    #[test]
    fn test_stdout_report_owns_stdout() {
        let harness = Harness::new("test-stdout");
        let uri = write_bep_artifact(&harness, "test-stdout.xml", MINIMAL_TEST_XML.as_bytes());
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.xml"), uri)],
            )]),
            ..harness
        };
        let (code, out, _) = harness.run(&["test", "--output=text", "--report=junit=-"]);
        assert_eq!(code, 0);
        assert!(out.contains("<testsuites name=\"dx\""), "{out}");
        assert!(!out.contains("Running test"), "{out}");
    }

    #[test]
    fn test_missing_artifacts_are_incomplete() {
        let harness = Harness {
            raw_bep: Some(vec![String::from(
                "{\"id\": {\"testResult\": {\"label\": \"//a:t\"}}, \"testResult\": {\"status\": \"PASSED\"}}",
            )]),
            ..Harness::new("test-empty")
        };
        let (code, _, err) = harness.run(&["test", "--output=text"]);
        assert_eq!(code, 1);
        assert!(err.contains("incomplete_results"), "{err}");
    }

    #[test]
    fn test_preserves_bazel_failure_code() {
        let harness = Harness::new("test-bazel-fails");
        let uri = write_bep_artifact(&harness, "fail.xml", MINIMAL_TEST_XML.as_bytes());
        let harness = Harness {
            bazel_code: 4,
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.xml"), uri)],
            )]),
            ..harness
        };
        let (code, _, _) = harness.run(&["test", "--output=text"]);
        assert_eq!(code, 4);
    }

    #[test]
    fn coverage_lcov_file_report_succeeds() {
        let harness = Harness::new("cov-ok");
        let uri = write_bep_artifact(&harness, "coverage.dat", MINIMAL_LCOV.as_bytes());
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.lcov"), uri)],
            )]),
            ..harness
        };
        let (code, out, _) = harness.run(&["coverage", "--output=text", "--report=lcov=out.lcov"]);
        assert_eq!(code, 0, "{out}");
        let document = std::fs::read(harness.workspace.join("out.lcov")).expect("lcov");
        assert_eq!(document, MINIMAL_LCOV.as_bytes());
    }

    #[test]
    fn coverage_invalid_tracefile_fails() {
        let harness = Harness::new("cov-bad");
        let uri = write_bep_artifact(&harness, "bad.dat", b"not lcov");
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.lcov"), uri)],
            )]),
            ..harness
        };
        let (code, _, err) = harness.run(&["coverage", "--output=text"]);
        assert_eq!(code, 1);
        assert!(err.contains("incomplete_results"), "{err}");
    }

    const HALF_LCOV: &str = "SF:src/a.py\nDA:1,1\nDA:2,0\nend_of_record\n";

    #[test]
    fn coverage_min_coverage_passes_at_threshold() {
        let harness = coverage_harness("cov-threshold-ok", MINIMAL_LCOV.as_bytes());
        let (code, _, err) = harness.run(&["coverage", "--output=text", "--min-coverage=100"]);
        assert_eq!(code, 0, "{err}");
        assert!(err.contains("meets minimum 100%"), "{err}");
    }

    #[test]
    fn coverage_min_coverage_fails_below_threshold() {
        let harness = coverage_harness("cov-threshold-low", HALF_LCOV.as_bytes());
        let (code, _, err) = harness.run(&["coverage", "--output=text", "--min-coverage=80"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("coverage_below_minimum"), "{err}");
        assert!(err.contains("50.00% (1/2 lines)"), "{err}");
    }

    #[test]
    fn coverage_min_coverage_failure_reports_json_event() {
        let harness = coverage_harness("cov-threshold-json", HALF_LCOV.as_bytes());
        let (code, out, _) = harness.run(&["coverage", "--output=json", "--min-coverage=80"]);
        assert_eq!(code, 1, "{out}");
        assert!(out.contains("coverage_below_minimum"), "{out}");
    }

    #[test]
    fn coverage_min_coverage_fails_without_executable_lines() {
        let harness = coverage_harness("cov-threshold-empty", b"SF:src/a.py\nend_of_record\n");
        let (code, _, err) = harness.run(&["coverage", "--output=text", "--min-coverage=80"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("no executable lines"), "{err}");
    }

    #[test]
    fn coverage_min_coverage_rejects_invalid_markers() {
        let harness = Harness::new("cov-threshold-markers");
        let source = harness.workspace.join("src/lib.rs");
        std::fs::create_dir_all(source.parent().expect("parent")).expect("mkdir");
        std::fs::write(&source, "// LCOV_EXCL_LINE\nfn a() {}\n").expect("write");
        let uri = write_bep_artifact(
            &harness,
            "coverage.dat",
            b"SF:src/lib.rs\nDA:1,1\nDA:2,1\nend_of_record\n",
        );
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.lcov"), uri)],
            )]),
            ..harness
        };
        let (code, _, err) = harness.run(&["coverage", "--output=text", "--min-coverage=80"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("coverage_below_minimum"), "{err}");
    }

    #[test]
    fn workflow_bad_report_is_pre_exec() {
        let harness = Harness::new("wf-bad-report");
        let (code, _, err) = harness.run(&["build", "--report=junit=a.xml"]);
        assert_eq!(code, 2, "{err}");
    }

    #[test]
    fn workflow_bad_scope_is_pre_exec() {
        let harness = Harness::new("wf-bad-scope");
        let (code, _, err) = harness.run(&["build", "nope.py"]);
        assert_eq!(code, 2, "{err}");
    }

    #[test]
    fn workflow_conflicting_option_is_pre_exec() {
        let harness = Harness::new("wf-conflict");
        let (code, _, err) = harness.run(&[
            "build",
            "--",
            "--@rules_dx//config:workspace=//other:config",
        ]);
        assert_eq!(code, 2, "{err}");
    }

    #[test]
    #[cfg(unix)]
    fn workflow_non_utf8_bep_is_operational() {
        let mut harness = Harness::new("wf-nonutf8");
        harness.temp = PathBuf::from(OsString::from_vec(vec![0xff]));
        let (code, _, err) = harness.run(&["build", "--output=text"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("unreadable_bep"), "{err}");
    }

    #[test]
    fn workflow_dry_run_text_and_json() {
        let harness = Harness::new("wf-dry-text");
        let (code, out, _) = harness.run(&["build", "--dry-run", "--output=text"]);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("Running build"), "{out}");
        let harness = Harness::new("wf-dry-json");
        let (code, out, _) = harness.run(&["build", "--dry-run", "--output=json"]);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("command_started"), "{out}");
    }

    #[test]
    fn workflow_json_lifecycle_and_build_status() {
        let harness = Harness::new("wf-json");
        let (code, out, _) = harness.run(&["build", "--output=json"]);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("command_started"), "{out}");
        assert!(out.contains("command_finished"), "{out}");
    }

    #[test]
    fn workflow_launch_failure_is_operational() {
        let mut harness = Harness::new("wf-launch");
        harness.io_error = true;
        let (code, _, err) = harness.run(&["build", "--output=text"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("launch_failed"), "{err}");
    }

    #[test]
    fn workflow_signalled_is_operational() {
        let mut harness = Harness::new("wf-signal");
        harness.signalled = true;
        let (code, _, err) = harness.run(&["build", "--output=text"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("bazel_signalled"), "{err}");
    }

    #[test]
    fn test_unreadable_bep_is_operational() {
        let mut harness = Harness::new("test-nobep");
        harness.skip_bep = true;
        let (code, _, err) = harness.run(&["test", "--output=text"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("unreadable_bep"), "{err}");
    }

    #[test]
    fn test_invalid_bep_is_operational() {
        let harness = Harness {
            raw_bep: Some(vec![String::from("not json")]),
            ..Harness::new("test-badbep")
        };
        let (code, _, err) = harness.run(&["test", "--output=text"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("invalid_bep"), "{err}");
    }

    #[test]
    fn test_ignores_non_xml_entries() {
        let harness = Harness::new("test-ignore-log");
        let xml = write_bep_artifact(&harness, "ok.xml", MINIMAL_TEST_XML.as_bytes());
        std::fs::write(harness.temp.join("ok.log"), b"log").expect("log");
        let log_uri = format!("file://{}", harness.temp.join("ok.log").display());
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[
                    (String::from("test.log"), log_uri),
                    (String::from("test.xml"), xml),
                ],
            )]),
            ..harness
        };
        let (code, _, _) = harness.run(&["test", "--output=text"]);
        assert_eq!(code, 0);
    }

    #[test]
    fn test_unreadable_artifact_is_incomplete() {
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(
                    String::from("test.xml"),
                    String::from("file:///nonexistent/a.xml"),
                )],
            )]),
            ..Harness::new("test-unreadable")
        };
        let (code, _, err) = harness.run(&["test", "--output=text"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("incomplete_results"), "{err}");
    }

    #[test]
    fn test_invalid_xml_is_incomplete() {
        let harness = Harness::new("test-badxml");
        let uri = write_bep_artifact(&harness, "bad.xml", b"not xml");
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.xml"), uri)],
            )]),
            ..harness
        };
        let (code, _, err) = harness.run(&["test", "--output=text"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("incomplete_results"), "{err}");
    }

    #[test]
    fn coverage_ignores_non_lcov_and_reports_missing() {
        let harness = Harness::new("cov-ignore");
        let xml = write_bep_artifact(&harness, "x.xml", MINIMAL_TEST_XML.as_bytes());
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.xml"), xml)],
            )]),
            ..harness
        };
        let (code, _, err) = harness.run(&["coverage", "--output=text"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("no coverage.dat"), "{err}");
    }

    #[test]
    fn coverage_unreadable_is_incomplete() {
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(
                    String::from("coverage.dat"),
                    String::from("file:///nonexistent/c.dat"),
                )],
            )]),
            ..Harness::new("cov-unreadable")
        };
        let (code, _, err) = harness.run(&["coverage", "--output=text"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("incomplete_results"), "{err}");
    }

    #[test]
    fn coverage_report_failed_when_incomplete() {
        let harness = Harness::new("cov-repfail");
        let uri = write_bep_artifact(&harness, "bad2.dat", b"not lcov");
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.lcov"), uri)],
            )]),
            ..harness
        };
        let (code, _, err) = harness.run(&["coverage", "--output=text", "--report=lcov=out.lcov"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("report_failed"), "{err}");
    }

    #[test]
    fn test_report_write_failure_is_operational() {
        let harness = Harness::new("test-writefail");
        let uri = write_bep_artifact(&harness, "w.xml", MINIMAL_TEST_XML.as_bytes());
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.xml"), uri)],
            )]),
            ..harness
        };
        let (code, _, err) = harness.run(&[
            "test",
            "--output=text",
            "--report=junit=missing-dir/out.xml",
        ]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("report_failed"), "{err}");
    }

    #[test]
    fn test_report_json_emits_report_event() {
        let harness = Harness::new("test-jsonrep");
        let uri = write_bep_artifact(&harness, "j.xml", MINIMAL_TEST_XML.as_bytes());
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.xml"), uri)],
            )]),
            ..harness
        };
        let (code, out, _) = harness.run(&["test", "--output=json", "--report=junit=out.xml"]);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("report"), "{out}");
        assert!(out.contains("command_finished"), "{out}");
    }

    #[test]
    fn test_report_diff_goes_to_stderr() {
        let harness = Harness::new("test-diffrep");
        let uri = write_bep_artifact(&harness, "d.xml", MINIMAL_TEST_XML.as_bytes());
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.xml"), uri)],
            )]),
            ..harness
        };
        let (code, _, err) = harness.run(&["test", "--output=diff", "--report=junit=out.xml"]);
        assert_eq!(code, 0, "{err}");
        assert!(err.contains("Wrote junit report"), "{err}");
    }

    #[test]
    fn test_incomplete_json_reports_incomplete_event() {
        let harness = Harness {
            raw_bep: Some(vec![String::from(
                "{\"id\": {\"testResult\": {\"label\": \"//a:t\"}}, \"testResult\": {\"status\": \"PASSED\"}}",
            )]),
            ..Harness::new("test-incjson")
        };
        let (code, out, _) = harness.run(&["test", "--output=json"]);
        assert_eq!(code, 1, "{out}");
        assert!(out.contains("incomplete_results"), "{out}");
        assert!(out.contains("command_finished"), "{out}");
    }

    #[test]
    fn coverage_lcov_without_trailing_newline_gets_newline() {
        let harness = Harness::new("cov-nonl");
        let raw = b"SF:src/a.py\nDA:1,1\nend_of_record";
        let uri = write_bep_artifact(&harness, "nonl.dat", raw);
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.lcov"), uri)],
            )]),
            ..harness
        };
        let (code, out, _) = harness.run(&["coverage", "--output=text", "--report=lcov=out.lcov"]);
        assert_eq!(code, 0, "{out}");
        let document = std::fs::read(harness.workspace.join("out.lcov")).expect("lcov");
        assert!(document.ends_with(b"\n"), "{document:?}");
    }

    #[test]
    fn coverage_render_failure_json_reports_error_event() {
        // No lcov documents with a requested report triggers render failure;
        // JSON output must emit the `report_failed` error event.
        let harness = Harness::new("cov-render-json");
        let xml = write_bep_artifact(&harness, "x.xml", MINIMAL_TEST_XML.as_bytes());
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.xml"), xml)],
            )]),
            ..harness
        };
        let (code, out, _) = harness.run(&["coverage", "--output=json", "--report=lcov=out.lcov"]);
        assert_eq!(code, 1, "{out}");
        assert!(out.contains("report_failed"), "{out}");
    }

    #[test]
    fn test_report_write_failure_json_reports_error_event() {
        let harness = Harness::new("test-writefail-json");
        let uri = write_bep_artifact(&harness, "w2.xml", MINIMAL_TEST_XML.as_bytes());
        let harness = Harness {
            raw_bep: Some(vec![test_result_line(
                "//a:t",
                &[(String::from("test.xml"), uri)],
            )]),
            ..harness
        };
        let (code, out, _) = harness.run(&[
            "test",
            "--output=json",
            "--report=junit=missing-dir/out.xml",
        ]);
        assert_eq!(code, 1, "{out}");
        assert!(out.contains("report_failed"), "{out}");
    }
}
