//! `dx run` single-target launch execution.

use super::common::*;
use crate::args::Invocation;
use crate::plan::plan_run;
use crate::reports::plan_reports;
use crate::resolve::{resolve_run, ResolveError};
use dx_output::{command_finished, command_started, write_event, FinishedCounts, OutputMode};

/// Stable operational codes for workflow failures.
const CODE_NO_RUNNABLE: &str = "no_runnable";
const CODE_AMBIGUOUS_RUNNABLE: &str = "ambiguous_runnable";
const CODE_NO_TESTS: &str = "no_tests";

fn resolve_code(error: &ResolveError) -> &'static str {
    match error {
        ResolveError::NoRunnable { .. } => CODE_NO_RUNNABLE,
        ResolveError::AmbiguousRunnable { .. } => CODE_AMBIGUOUS_RUNNABLE,
        ResolveError::NoTests { .. } => CODE_NO_TESTS,
        _ => "scope_error",
    }
}

/// Executes `dx run`: local-only single-runnable launcher with
/// verbatim application exit codes. Lifecycle prose goes to stderr;
/// the application keeps stdout through the process runner.
pub(crate) fn execute_run(invocation: &Invocation, env: Env<'_>) -> i32 {
    let Env {
        workspace,
        runner,
        query_runner,
        temp_dir: _,
        pid: _,
        nonce: _,
        out,
        err,
        ci,
    } = env;
    if ci {
        return pre_exec(err, "dx run refuses when CI=true: local-only command");
    }
    let planned_reports = match plan_reports(
        invocation.command,
        &invocation.reports,
        &invocation.output,
        invocation.dry_run,
    ) {
        Ok(planned) => planned,
        Err(error) => return pre_exec(err, &error.to_string()),
    };
    debug_assert!(planned_reports.is_empty(), "dx run takes no --report");
    let targets = match resolve_run(&invocation.targets, workspace, query_runner) {
        Ok(targets) => targets,
        Err(error) => {
            let code = resolve_code(&error);
            let message = error.to_string();
            if matches!(
                error,
                ResolveError::NoRunnable { .. } | ResolveError::AmbiguousRunnable { .. }
            ) {
                return operational(invocation, out, err, code, &message);
            }
            return pre_exec(err, &message);
        }
    };
    let plan = if targets.len() == 1 {
        plan_run(&targets[0], &invocation.bazel_options, invocation.profile())
    } else {
        plan_run_multi(&targets, &invocation.bazel_options, invocation.profile())
    };
    if invocation.dry_run {
        if invocation.output == OutputMode::Json {
            if let Ok(event) = command_started(invocation.command.name(), true, "default") {
                let _ = write_event(out, &event);
            }
            let _ = write_event(out, &command_finished(0, &FinishedCounts::default()));
        } else if !invocation.quiet {
            let _ = writeln!(err, "{}", plan.summary);
        }
        return 0;
    }
    if !invocation.quiet {
        let _ = writeln!(err, "{}", plan.summary);
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
    let Some(code) = status.code else {
        return operational(
            invocation,
            out,
            err,
            CODE_BAZEL_SIGNALLED,
            "Bazel terminated by signal",
        );
    };
    code
}

/// Plans a label-only multi-target `dx run` argv Bazel owns.
///
/// File/directory scopes enforce single-runnable selection in
/// [`resolve_run`]; label scopes pass through unchanged, including
/// multiple labels. `bazel run` rejects multi-target requests itself,
/// so this preserves Bazel's exact diagnostic and status. Shares the
/// single [`crate::plan::plan_run_targets`] builder with [`plan_run`]
/// so the launcher, startup options, and workspace policy cannot drift.
fn plan_run_multi(
    targets: &[String],
    app_args: &[String],
    profile: crate::args::Profile,
) -> crate::plan::BuildPlan {
    crate::plan::plan_run_targets(targets, app_args, profile)
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use super::*;
    use crate::args::parse;
    use crate::args::Command;
    use crate::exec::{execute, Env};

    #[test]
    fn run_label_passthrough_preserves_status_on_stderr() {
        let harness = Harness::new("run-ok");
        let (code, out, err) = harness.run(&["run", "//app:bin", "--", "--port=8080"]);
        assert_eq!(code, 0);
        assert_eq!(out, "");
        assert!(err.contains("Running run for //app:bin"), "{err}");
        let harness = Harness {
            bazel_code: 7,
            ..Harness::new("run-fails")
        };
        let (code, _, _) = harness.run(&["run", "//app:bin"]);
        assert_eq!(code, 7);
    }

    #[test]
    fn run_empty_scope_is_pre_exec() {
        let harness = Harness::new("run-empty");
        let (code, _, err) = harness.run(&["run"]);
        assert_eq!(code, 2);
        assert!(err.contains("empty scope"), "{err}");
    }

    #[test]
    fn run_file_without_runnable_is_operational() {
        let harness = Harness::new("run-norunnable");
        harness.write_source("pkg/BUILD.bazel", "");
        harness.write_source("pkg/a.py", "x = 1\n");
        harness.query.script_owners("");
        harness.query.script_owners("//pkg:lib\n");
        let (code, _, err) = harness.run(&["run", "pkg/a.py"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("no_runnable"), "{err}");
    }

    #[test]
    fn resolve_code_maps_all_variants() {
        assert_eq!(
            resolve_code(&ResolveError::NoRunnable {
                scopes: vec!["a".to_owned()],
            }),
            "no_runnable"
        );
        assert_eq!(
            resolve_code(&ResolveError::AmbiguousRunnable {
                candidates: vec!["//a:one".to_owned(), "//a:two".to_owned()],
            }),
            "ambiguous_runnable"
        );
        assert_eq!(
            resolve_code(&ResolveError::NoTests {
                owners: vec!["//a:lib".to_owned()],
            }),
            "no_tests"
        );
        assert_eq!(resolve_code(&ResolveError::EmptyScope), "scope_error");
    }

    #[test]
    fn run_ambiguous_is_operational() {
        let harness = Harness::new("run-amb");
        std::fs::create_dir_all(harness.workspace.join("app")).expect("dir");
        harness.query.script_owners("//app:two\n//app:one\n");
        let (code, _, err) = harness.run(&["run", "app"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("ambiguous_runnable"), "{err}");
    }

    #[test]
    fn run_bad_report_is_pre_exec() {
        // `parse` rejects `--report` for `run` before execution; assert the
        // usage error directly since `Harness::run` requires parse success.
        let args: Vec<String> = ["run", "//app:bin", "--report=sarif=a.sarif"]
            .iter()
            .map(ToString::to_string)
            .collect();
        let err = parse(&args).expect_err("run report must fail parse");
        assert!(err.to_string().contains("--report"), "{err:?}");
    }

    #[test]
    fn run_dry_run_json_and_text() {
        // `parse` owns `--output=json` rejection for `run`; the text dry-run
        // exercises `execute_run` planning.
        let args: Vec<String> = ["run", "//app:bin", "--dry-run", "--output=json"]
            .iter()
            .map(ToString::to_string)
            .collect();
        let err = parse(&args).expect_err("run json must fail parse");
        assert!(err.to_string().contains("--output"), "{err:?}");
        let harness = Harness::new("run-dry-text");
        let (code, _, err) = harness.run(&["run", "//app:bin", "--dry-run"]);
        assert_eq!(code, 0, "{err}");
        assert!(err.contains("Running run"), "{err}");
    }

    #[test]
    fn run_launch_and_signal_failures() {
        let mut harness = Harness::new("run-launch");
        harness.io_error = true;
        let (code, _, err) = harness.run(&["run", "//app:bin"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("launch_failed"), "{err}");
        let mut harness = Harness::new("run-signal");
        harness.signalled = true;
        let (code, _, err) = harness.run(&["run", "//app:bin"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("bazel_signalled"), "{err}");
    }

    #[test]
    fn run_multi_target_uses_multi_plan() {
        let harness = Harness::new("run-multi");
        let (code, _, err) = harness.run(&["run", "//a:bin", "//b:bin"]);
        assert_eq!(code, 0, "{err}");
        assert!(err.contains("//a:bin //b:bin"), "{err}");
        // Multi-target with app args covers the `--` forwarding arm.
        let harness = Harness::new("run-multi-args");
        let (code, _, err) = harness.run(&["run", "//a:bin", "//b:bin", "--", "--port=8080"]);
        assert_eq!(code, 0, "{err}");
        assert!(err.contains("//a:bin //b:bin"), "{err}");
    }

    fn run_invocation(
        command: Command,
        output: OutputMode,
        reports: Vec<crate::args::ReportRequest>,
        dry_run: bool,
    ) -> Invocation {
        Invocation {
            command,
            check: false,
            debug: false,
            release: false,
            workspace: None,
            dry_run,
            quiet: false,
            output,
            reports,
            fail_on: dx_output::Threshold::Warning,
            min_coverage: None,
            targets: vec!["//app:bin".to_owned()],
            bazel_options: Vec::new(),
            bazel_clean: false,
            pin: None,
            rollback: false,
            configured: false,
        }
    }

    fn execute_with(invocation: &Invocation, harness: &Harness) -> (i32, String, String) {
        let runner = harness.runner();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = execute(
            invocation,
            Env {
                workspace: &harness.workspace,
                runner: &runner,
                query_runner: &harness.query,
                temp_dir: &harness.temp,
                pid: std::process::id(),
                nonce: 0,
                out: &mut out,
                err: &mut err,
                ci: false,
            },
        );
        (
            code,
            String::from_utf8(out).expect("stdout"),
            String::from_utf8(err).expect("stderr"),
        )
    }

    #[test]
    fn run_manual_invocations_cover_defense_branches() {
        // `parse` rejects `--report` and JSON output for `run`; construct the
        // invocation directly to cover `execute_run` defense branches.
        let harness = Harness::new("run-manual-report");
        let inv = run_invocation(
            Command::Run,
            OutputMode::Text { quiet: false },
            vec![crate::args::ReportRequest {
                format: "junit".to_owned(),
                destination: "out.xml".to_owned(),
            }],
            false,
        );
        let (code, _, err) = execute_with(&inv, &harness);
        assert_eq!(code, 2, "{err}");

        let harness = Harness::new("run-manual-json");
        let inv = run_invocation(Command::Run, OutputMode::Json, Vec::new(), true);
        let (code, out, _) = execute_with(&inv, &harness);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("command_started"), "{out}");
        assert!(out.contains("command_finished"), "{out}");
    }

    #[test]
    fn run_profile_flags_reach_bazel_argv() {
        use std::cell::RefCell;
        use std::rc::Rc;
        // End-to-end pin of the issue #179 mapping on the `run` path:
        // parse selects the profile and execution injects the matching
        // `--config=dx_*` (bare means `dx_dev`).
        for (words, flag) in [
            (vec!["run", "//app:bin"], "--config=dx_dev"),
            (vec!["run", "--debug", "//app:bin"], "--config=dx_debug"),
            (vec!["run", "//app:bin", "--release"], "--config=dx_release"),
        ] {
            let harness = Harness::new("run-profile");
            let seen = Rc::new(RefCell::new(Vec::new()));
            let probe = ArgvProbe {
                code: Some(0),
                seen: Rc::clone(&seen),
            };
            let inv = invocation(&words);
            let mut out = Vec::new();
            let mut err = Vec::new();
            let code = execute(
                &inv,
                Env {
                    workspace: &harness.workspace,
                    runner: &probe,
                    query_runner: &harness.query,
                    temp_dir: &harness.temp,
                    pid: std::process::id(),
                    nonce: 0,
                    out: &mut out,
                    err: &mut err,
                    ci: false,
                },
            );
            assert_eq!(code, 0, "{words:?}");
            let seen = seen.borrow();
            assert_eq!(seen.len(), 1, "{words:?}");
            assert!(
                seen[0].contains(&flag.to_owned()),
                "{words:?} argv missing {flag}: {:?}",
                seen[0]
            );
        }
    }

    #[test]
    fn run_ci_refusal_is_pre_exec() {
        // The refusal bit travels inside `Env`, never through
        // process-global environment: parallel test threads share one
        // process, so `set_var("CI", ...)` here used to flake
        // unrelated `run` tests with spurious CI refusals.
        let harness = Harness::new("run-ci");
        let (code, _, err) = harness.run_with_ci(&["run", "//app:bin"], true);
        assert_eq!(code, 2, "{err}");
        assert!(err.contains("CI=true"), "{err}");
        // The same invocation without the bit proceeds past the gate
        // (here: into ambiguous-runnable resolution).
        std::fs::create_dir_all(harness.workspace.join("app")).expect("dir");
        harness.query.script_owners("//app:two\n//app:one\n");
        let (code, _, err) = harness.run(&["run", "app"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("ambiguous_runnable"), "{err}");
    }
}
