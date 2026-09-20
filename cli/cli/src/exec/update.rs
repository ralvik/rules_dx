//! Update command execution: live resolver backends with continuation
//! plus the vendored preset fragment.

use super::common::*;
use crate::args::{Command, Invocation};
use crate::reports::plan_reports;
use dx_output::{
    command_finished, command_started, error_event, notice_event, write_event, FinishedCounts,
    NoticeEvent, OutputMode,
};
use std::collections::BTreeMap;

/// Runs `dx update`  with the preset fragment:
/// default mode updates dependency-set/package/target selectors through
/// `dx_update` (mutating without confirmation) plus the preset fragment
/// atomically; `--check` is the non-mutating preset stale gate (exit `0`
/// clean / `1` stale, copying the `generate --check` exit contract) and
/// ignores selectors. `--dry-run` plans without launching or touching
/// the tree. Usage errors exit `2` before any launch.
pub(crate) fn execute_update(invocation: &Invocation, env: Env<'_>) -> i32 {
    debug_assert!(
        invocation.command == Command::Update,
        "update dispatch guards commands"
    );
    match plan_reports(
        invocation.command,
        &invocation.reports,
        &invocation.output,
        invocation.dry_run,
    ) {
        Ok(_) => {}
        Err(error) => return pre_exec(env.err, &error.to_string()),
    }
    let verbose =
        matches!(invocation.output, OutputMode::Text { quiet: false }) && !invocation.quiet;
    // Preset stale gate: non-mutating, preset-only, no backend launches.
    if invocation.check {
        let Env {
            workspace,
            out,
            err,
            ..
        } = env;
        let mode = "check";
        if invocation.dry_run {
            if invocation.output == OutputMode::Json {
                if let Ok(event) = command_started(invocation.command.name(), true, mode) {
                    let _ = write_event(out, &event);
                }
                let finished = command_finished(0, &FinishedCounts::default());
                let _ = write_event(out, &finished);
            } else if verbose {
                let _ = writeln!(out, "Would check preset fragment");
            }
            return 0;
        }
        if invocation.output == OutputMode::Json {
            if let Ok(event) = command_started(invocation.command.name(), false, mode) {
                let _ = write_event(out, &event);
            }
        } else if verbose {
            let _ = writeln!(out, "Running update --check for preset");
        }
        match dx_adopt::check_preset(workspace) {
            Ok(()) => {
                if invocation.output == OutputMode::Json {
                    let finished = command_finished(0, &FinishedCounts::default());
                    let _ = write_event(out, &finished);
                } else if verbose {
                    let _ = writeln!(out, "preset clean");
                }
                0
            }
            Err(dx_adopt::PresetError::Stale { detail }) => {
                if invocation.output == OutputMode::Json {
                    if let Ok(event) =
                        error_event(CODE_UPDATE_FAILED, &detail, None, None, Some("execute"))
                    {
                        let _ = write_event(out, &event);
                    }
                    let finished = command_finished(1, &FinishedCounts::default());
                    let _ = write_event(out, &finished);
                } else {
                    // Check failure (like `generate --check`): report the
                    // diff to stdout, exit 1, no stderr.
                    let _ = writeln!(out, "{detail}");
                }
                1
            }
            Err(error) => {
                // Owned collisions and I/O failures are operational.
                operational(invocation, out, err, CODE_UPDATE_FAILED, &error.to_string())
            }
        }
    } else {
        execute_update_default(invocation, env, verbose)
    }
}

#[allow(clippy::too_many_lines)]
fn execute_update_default(invocation: &Invocation, env: Env<'_>, verbose: bool) -> i32 {
    let Env {
        workspace,
        runner,
        out,
        err,
        ..
    } = env;
    let resolved = match dx_update::selector::resolve(&invocation.targets) {
        Ok(resolved) => resolved,
        Err(error) => return pre_exec(err, &error.to_string()),
    };
    let summary = display_summary(&resolved);
    if invocation.dry_run {
        if invocation.output == OutputMode::Json {
            if let Ok(event) = command_started(invocation.command.name(), true, "default") {
                let _ = write_event(out, &event);
            }
            let finished = command_finished(0, &FinishedCounts::default());
            let _ = write_event(out, &finished);
        } else if verbose {
            let _ = writeln!(out, "{summary}");
            let _ = writeln!(out, "Would update preset fragment");
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
    // Preset first, atomically, preserving overrides (fail fast on
    // collisions; independent of dependency backends).
    if let Err(error) = dx_adopt::update_preset(workspace) {
        return operational(invocation, out, err, CODE_UPDATE_FAILED, &error.to_string());
    }
    if verbose && invocation.output != OutputMode::Json {
        let _ = writeln!(out, "updated preset (tools/bazelrc/preset.bazelrc)");
    }
    // Live: run backends in sorted set order, continuing independent sets
    // after failures. V1 sets are independent (distinct locks), so the
    // depends-on relation is empty; `aggregate` still derives `Blocked`
    // for any future dependent that lacks a result.
    let mut attempted: Vec<dx_update::outcome::SetOutcome> = Vec::new();
    let mut details: BTreeMap<dx_update::sets::SetId, SetDetail> = BTreeMap::new();
    for (set, request) in &resolved {
        let plan = match dx_update::backend::plan(*set, request) {
            Ok(plan) => plan,
            Err(error) => {
                let reason = match error {
                    dx_update::backend::BackendError::Unsupported { reason, .. } => reason,
                };
                let detail = format!("unsupported update: {reason}");
                attempted.push(dx_update::outcome::SetOutcome {
                    set: set.name().to_owned(),
                    status: dx_update::outcome::SetStatus::Failed,
                });
                details.insert(
                    *set,
                    SetDetail::Failed {
                        message: format!("failed to update {}: {detail}", set.name()),
                    },
                );
                continue;
            }
        };
        match plan {
            dx_update::backend::BackendPlan::Noop => {
                attempted.push(dx_update::outcome::SetOutcome {
                    set: set.name().to_owned(),
                    status: dx_update::outcome::SetStatus::Success,
                });
                details.insert(
                    *set,
                    SetDetail::Success {
                        message: success_line(*set, request),
                    },
                );
            }
            dx_update::backend::BackendPlan::Run { argv, env: extra } => {
                let env_refs: Vec<(&str, &str)> = extra
                    .iter()
                    .map(|(key, value)| (key.as_str(), value.as_str()))
                    .collect();
                match runner.run(&argv, workspace, &env_refs) {
                    Err(error) => {
                        attempted.push(dx_update::outcome::SetOutcome {
                            set: set.name().to_owned(),
                            status: dx_update::outcome::SetStatus::Failed,
                        });
                        details.insert(
                            *set,
                            SetDetail::Failed {
                                message: format!(
                                    "failed to update {}: failed to launch updater: {error}",
                                    set.name()
                                ),
                            },
                        );
                    }
                    Ok(status) => match status.code {
                        Some(0) => {
                            attempted.push(dx_update::outcome::SetOutcome {
                                set: set.name().to_owned(),
                                status: dx_update::outcome::SetStatus::Success,
                            });
                            details.insert(
                                *set,
                                SetDetail::Success {
                                    message: success_line(*set, request),
                                },
                            );
                        }
                        Some(code) => {
                            attempted.push(dx_update::outcome::SetOutcome {
                                set: set.name().to_owned(),
                                status: dx_update::outcome::SetStatus::Failed,
                            });
                            details.insert(
                                *set,
                                SetDetail::Failed {
                                    message: format!(
                                        "failed to update {}: updater exited {code}",
                                        set.name()
                                    ),
                                },
                            );
                        }
                        None => {
                            attempted.push(dx_update::outcome::SetOutcome {
                                set: set.name().to_owned(),
                                status: dx_update::outcome::SetStatus::Failed,
                            });
                            details.insert(
                                *set,
                                SetDetail::Failed {
                                    message: format!(
                                        "failed to update {}: updater terminated by signal",
                                        set.name()
                                    ),
                                },
                            );
                        }
                    },
                }
            }
        }
    }
    let selected: Vec<String> = resolved.keys().map(|set| set.name().to_owned()).collect();
    let depends: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let report = match dx_update::outcome::aggregate(&selected, &attempted, &depends) {
        Ok(report) => report,
        Err(error) => {
            return operational(invocation, out, err, CODE_UPDATE_FAILED, &error.to_string());
        }
    };
    let exit = dx_update::report::exit_code(&report);
    if invocation.output == OutputMode::Json {
        for outcome in &report.outcomes {
            let set_name = outcome.set.as_str();
            match outcome.status {
                dx_update::outcome::ReportedStatus::Success => {
                    let message = details
                        .get(&parse_set(set_name))
                        .and_then(|detail| match detail {
                            SetDetail::Success { message } => Some(message.clone()),
                            SetDetail::Failed { .. } => None,
                        })
                        .unwrap_or_else(|| format!("updated {set_name}"));
                    if let Ok(event) = notice_event(&NoticeEvent {
                        level: "info".to_owned(),
                        code: "update_set_success".to_owned(),
                        message,
                        related_command: Some("update".to_owned()),
                        scope: Some(vec![set_name.to_owned()]),
                        path: None,
                        language: None,
                        import: None,
                    }) {
                        let _ = write_event(out, &event);
                    }
                }
                dx_update::outcome::ReportedStatus::Failed => {
                    let message = details
                        .get(&parse_set(set_name))
                        .and_then(|detail| match detail {
                            SetDetail::Failed { message } => Some(message.clone()),
                            SetDetail::Success { .. } => None,
                        })
                        .unwrap_or_else(|| format!("failed to update {set_name}"));
                    if let Ok(event) =
                        error_event(CODE_UPDATE_FAILED, &message, None, None, Some("execute"))
                    {
                        let _ = write_event(out, &event);
                    }
                    let _ = writeln!(err, "dx: {CODE_UPDATE_FAILED}: {message}");
                }
                dx_update::outcome::ReportedStatus::Blocked => {
                    let message = format!("blocked {set_name} (depends on a failed update)");
                    if let Ok(event) = notice_event(&NoticeEvent {
                        level: "warning".to_owned(),
                        code: "update_set_blocked".to_owned(),
                        message: message.clone(),
                        related_command: Some("update".to_owned()),
                        scope: Some(vec![set_name.to_owned()]),
                        path: None,
                        language: None,
                        import: None,
                    }) {
                        let _ = write_event(out, &event);
                    }
                    if verbose {
                        let _ = writeln!(out, "{message}");
                    }
                }
            }
        }
        let finished = command_finished(
            exit,
            &FinishedCounts {
                results_complete: Some(true),
                ..FinishedCounts::default()
            },
        );
        let _ = write_event(out, &finished);
        return exit;
    }
    // Text mode: successes to stdout when verbose, failures always to
    // stderr, plus a final aggregate summary when verbose.
    for outcome in &report.outcomes {
        match outcome.status {
            dx_update::outcome::ReportedStatus::Success => {
                if verbose {
                    if let Some(SetDetail::Success { message }) =
                        details.get(&parse_set(outcome.set.as_str()))
                    {
                        let _ = writeln!(out, "{message}");
                    }
                }
            }
            dx_update::outcome::ReportedStatus::Failed => {
                if let Some(SetDetail::Failed { message }) =
                    details.get(&parse_set(outcome.set.as_str()))
                {
                    let _ = writeln!(err, "dx: {CODE_UPDATE_FAILED}: {message}");
                }
            }
            dx_update::outcome::ReportedStatus::Blocked => {
                if verbose {
                    let _ = writeln!(out, "blocked {} (depends on a failed update)", outcome.set);
                }
            }
        }
    }
    if verbose {
        let succeeded = report
            .outcomes
            .iter()
            .filter(|outcome| outcome.status == dx_update::outcome::ReportedStatus::Success)
            .count();
        let failed = report
            .outcomes
            .iter()
            .filter(|outcome| outcome.status == dx_update::outcome::ReportedStatus::Failed)
            .count();
        let blocked = report
            .outcomes
            .iter()
            .filter(|outcome| outcome.status == dx_update::outcome::ReportedStatus::Blocked)
            .count();
        let _ = writeln!(
            out,
            "dx update: {succeeded} succeeded, {failed} failed, {blocked} blocked"
        );
    }
    exit
}

enum SetDetail {
    Success { message: String },
    Failed { message: String },
}

fn parse_set(name: &str) -> dx_update::sets::SetId {
    // Aggregate outcomes use canonical set names produced above; fall back
    // to Cargo only to keep reporting total (unreachable in practice).
    dx_update::sets::SetId::parse(name).unwrap_or(dx_update::sets::SetId::Cargo)
}

fn display_summary(
    resolved: &BTreeMap<dx_update::sets::SetId, dx_update::selector::SetRequest>,
) -> String {
    let all_full = resolved.len() == dx_update::sets::SetId::ALL.len()
        && resolved
            .values()
            .all(|request| *request == dx_update::selector::SetRequest::Full);
    if all_full {
        return "Running update for all dependency sets".to_owned();
    }
    let mut parts = Vec::new();
    for (set, request) in resolved {
        match request {
            dx_update::selector::SetRequest::Full => parts.push(set.name().to_owned()),
            dx_update::selector::SetRequest::Packages(packages) => {
                for package in packages {
                    parts.push(format!("{}:{package}", set.name()));
                }
            }
        }
    }
    format!("Running update for {}", parts.join(", "))
}

fn success_line(set: dx_update::sets::SetId, request: &dx_update::selector::SetRequest) -> String {
    match request {
        dx_update::selector::SetRequest::Full => match set {
            dx_update::sets::SetId::Cargo => {
                "updated cargo (rust/tests/fixtures/hello/Cargo.lock, cargo-bazel-lock.json)"
                    .to_owned()
            }
            dx_update::sets::SetId::Npm => "updated npm (pnpm-lock.yaml)".to_owned(),
            dx_update::sets::SetId::Maven => {
                "updated maven (third_party/jvm/maven_install.json)".to_owned()
            }
            dx_update::sets::SetId::NuGet => {
                "updated nuget (third_party/dotnet/paket.lock)".to_owned()
            }
            dx_update::sets::SetId::Go => "updated go (nothing to update)".to_owned(),
        },
        dx_update::selector::SetRequest::Packages(packages) => {
            let locks = set.locks().join(", ");
            let selections: Vec<String> = packages
                .iter()
                .map(|package| format!("{}:{package}", set.name()))
                .collect();
            if locks.is_empty() {
                format!("updated {} (nothing to update)", selections.join(", "))
            } else {
                format!("updated {} ({locks})", selections.join(", "))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use dx_process::{ChildStatus, Runner};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::io;
    use std::path::Path;
    use std::rc::Rc;

    struct ScriptRunner {
        calls: Rc<RefCell<Vec<Vec<String>>>>,
        codes: RefCell<HashMap<String, Option<i32>>>,
        io_error: bool,
    }

    impl ScriptRunner {
        fn new(codes: &[(&str, Option<i32>)]) -> Self {
            ScriptRunner {
                calls: Rc::new(RefCell::new(Vec::new())),
                codes: RefCell::new(
                    codes
                        .iter()
                        .map(|(key, code)| ((*key).to_owned(), *code))
                        .collect(),
                ),
                io_error: false,
            }
        }

        fn key_for(argv: &[String]) -> String {
            if argv.contains(&"//rust/tests/fixtures/hello:hello".to_owned()) {
                "cargo".to_owned()
            } else if argv.contains(&"@pnpm//:pnpm".to_owned()) {
                "npm".to_owned()
            } else if argv.contains(&"@maven//:pin".to_owned()) {
                "maven".to_owned()
            } else if argv.iter().any(|arg| arg.contains("paket2bazel")) {
                "nuget".to_owned()
            } else {
                argv.join(" ")
            }
        }
    }

    impl Runner for ScriptRunner {
        fn run(
            &self,
            argv: &[String],
            _cwd: &Path,
            _env: &[(&str, &str)],
        ) -> io::Result<ChildStatus> {
            if self.io_error {
                return Err(io::Error::other("fake launch failure"));
            }
            self.calls.borrow_mut().push(argv.to_vec());
            let key = Self::key_for(argv);
            let code = self.codes.borrow().get(&key).copied().unwrap_or(Some(0));
            Ok(ChildStatus { code })
        }
    }

    fn run_with(argv: &[&str], runner: &ScriptRunner) -> (i32, String, String) {
        use crate::args::parse;
        use crate::exec::{execute, Env};
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let words: Vec<String> = argv.iter().map(|word| (*word).to_owned()).collect();
        let invocation = parse(&words).expect("parse");
        let workspace_guard = temp_dir(&format!("update-live-{id}"));
        let temp_guard = temp_dir(&format!("update-live-tmp-{id}"));
        let query = ScriptQuery {
            calls: RefCell::new(Vec::new()),
            outputs: RefCell::new(Vec::new()),
        };
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = execute(
            &invocation,
            Env {
                workspace: workspace_guard.path(),
                runner,
                query_runner: &query,
                temp_dir: temp_guard.path(),
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
    fn dry_run_resolves_without_launching() {
        let harness = Harness::new("update-dryrun-live");
        let (code, out, err) = harness.run(&["update", "--dry-run"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(
            out.contains("Running update for all dependency sets"),
            "{out}"
        );
        assert_eq!(err, "", "{err}");
        assert!(
            harness.seen_env.borrow().is_empty(),
            "dry-run launches nothing"
        );

        let harness = Harness::new("update-dryrun-npm");
        let (code, out, err) = harness.run(&["update", "npm:jest", "--dry-run"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("Running update for npm:jest"), "{out}");
        assert_eq!(err, "", "{err}");
    }

    #[test]
    fn dry_run_rejects_unknown_and_unowned() {
        let harness = Harness::new("update-dryrun-unknown");
        let (code, _, err) = harness.run(&["update", "crates", "--dry-run"]);
        assert_eq!(code, 2, "{err}");
        let harness = Harness::new("update-dryrun-unowned");
        let (code, _, err) = harness.run(&[
            "update",
            "python/tests/fixtures/hello/hello.py",
            "--dry-run",
        ]);
        assert_eq!(code, 2, "{err}");
    }

    #[test]
    fn live_all_success_reports_per_set_and_exits_zero() {
        let runner = ScriptRunner::new(&[]);
        let (code, out, err) = run_with(&["update"], &runner);
        assert_eq!(code, 0, "{out}{err}");
        assert!(
            out.contains("Running update for all dependency sets"),
            "{out}"
        );
        assert!(out.contains("updated cargo ("), "{out}");
        assert!(out.contains("updated npm ("), "{out}");
        assert!(out.contains("updated maven ("), "{out}");
        assert!(out.contains("updated nuget ("), "{out}");
        assert!(out.contains("updated go (nothing to update)"), "{out}");
        assert!(out.contains("5 succeeded, 0 failed, 0 blocked"), "{out}");
        assert_eq!(err, "", "{err}");
        // Go is a no-op with no launch; the four Bazel backends launch.
        assert_eq!(runner.calls.borrow().len(), 4);
    }

    #[test]
    fn live_independent_failure_preserves_success_and_exits_one() {
        let runner = ScriptRunner::new(&[("maven", Some(1))]);
        let (code, out, err) = run_with(&["update"], &runner);
        assert_eq!(code, 1, "{out}{err}");
        assert!(out.contains("updated cargo ("), "{out}");
        assert!(out.contains("updated npm ("), "{out}");
        assert!(out.contains("4 succeeded, 1 failed, 0 blocked"), "{out}");
        assert!(err.contains("update_failed"), "{err}");
        assert!(err.contains("failed to update maven"), "{err}");
        assert_eq!(runner.calls.borrow().len(), 4);
    }

    #[test]
    fn live_selective_npm_runs_once_with_packages() {
        let runner = ScriptRunner::new(&[]);
        let (code, out, err) = run_with(&["update", "npm:jest", "npm:react"], &runner);
        assert_eq!(code, 0, "{out}{err}");
        assert!(
            out.contains("Running update for npm:jest, npm:react"),
            "{out}"
        );
        assert!(
            out.contains("updated npm:jest, npm:react (pnpm-lock.yaml)"),
            "{out}"
        );
        assert_eq!(runner.calls.borrow().len(), 1);
        assert!(runner.calls.borrow()[0].contains(&"jest".to_owned()));
        assert!(runner.calls.borrow()[0].contains(&"react".to_owned()));
    }

    #[test]
    fn live_unsupported_selective_fails_without_launch() {
        let runner = ScriptRunner::new(&[]);
        let (code, out, err) = run_with(&["update", "cargo:anyhow"], &runner);
        assert_eq!(code, 1, "{out}{err}");
        assert!(err.contains("update_failed"), "{err}");
        assert!(err.contains("unsupported"), "{err}");
        assert!(runner.calls.borrow().is_empty());
    }

    #[test]
    fn live_target_resolves_to_owning_set_only() {
        let runner = ScriptRunner::new(&[]);
        let (code, out, err) = run_with(&["update", "//go/tests/fixtures/hello:hello"], &runner);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("Running update for go"), "{out}");
        assert!(out.contains("updated go (nothing to update)"), "{out}");
        assert!(runner.calls.borrow().is_empty());
    }

    #[test]
    fn live_json_emits_per_set_notices_and_finished() {
        let runner = ScriptRunner::new(&[("npm", Some(2))]);
        let (code, out, err) = run_with(&["update", "--output=json"], &runner);
        assert_eq!(code, 1, "{out}{err}");
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
        assert_eq!(
            events.last().expect("finished")["exit_code"],
            serde_json::json!(1)
        );
        assert!(kinds.contains(&"notice"));
        assert!(kinds.contains(&"error"));
        assert!(err.contains("update_failed"), "{err}");
    }

    #[test]
    fn live_dry_run_json_still_plans_without_per_set() {
        let harness = Harness::new("update-dryrun-json-live");
        let (code, out, err) = harness.run(&["update", "--dry-run", "--output=json"]);
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
    }

    #[test]
    fn check_clean_passes_without_launching() {
        let harness = Harness::new("update-check-clean");
        harness.write_source(
            ".bazelrc",
            "import %workspace%/tools/bazelrc/preset.bazelrc\ntry-import %workspace%/user.bazelrc\n",
        );
        harness.write_source(
            "tools/bazelrc/preset.bazelrc",
            &dx_adopt::render_preset_fragment(),
        );
        let (code, out, err) = harness.run(&["update", "--check"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("Running update --check for preset"), "{out}");
        assert!(out.contains("preset clean"), "{out}");
        assert_eq!(err, "", "{err}");
        assert!(
            harness.seen_env.borrow().is_empty(),
            "check launches nothing"
        );
    }

    #[test]
    fn check_stale_fails_with_diff_and_no_mutation() {
        let harness = Harness::new("update-check-stale");
        harness.write_source(
            ".bazelrc",
            "import %workspace%/tools/bazelrc/preset.bazelrc\ntry-import %workspace%/user.bazelrc\n",
        );
        harness.write_source("tools/bazelrc/preset.bazelrc", "# dirty\n");
        let (code, out, err) = harness.run(&["update", "--check"]);
        assert_eq!(code, 1, "{out}{err}");
        assert!(out.contains("stale"), "{out}");
        assert!(out.contains("checked-in"), "{out}");
        assert_eq!(err, "", "{err}");
        // Check never writes.
        assert_eq!(
            std::fs::read_to_string(harness.workspace.join("tools/bazelrc/preset.bazelrc"))
                .expect("read"),
            "# dirty\n"
        );
        assert!(harness.seen_env.borrow().is_empty());
    }

    #[test]
    fn check_missing_fails_closed() {
        let harness = Harness::new("update-check-missing");
        harness.write_source(
            ".bazelrc",
            "import %workspace%/tools/bazelrc/preset.bazelrc\n",
        );
        let (code, out, err) = harness.run(&["update", "--check"]);
        assert_eq!(code, 1, "{out}{err}");
        assert!(out.contains("stale"), "{out}");
        assert_eq!(err, "", "{err}");
    }

    #[test]
    fn check_collision_fails_operational() {
        let harness = Harness::new("update-check-collision");
        harness.write_source(
            ".bazelrc",
            "import %workspace%/tools/bazelrc/preset.bazelrc\ncommon --enable_bzlmod\n",
        );
        harness.write_source(
            "tools/bazelrc/preset.bazelrc",
            &dx_adopt::render_preset_fragment(),
        );
        let (code, _, err) = harness.run(&["update", "--check"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("update_failed"), "{err}");
        assert!(err.contains("duplicates preset"), "{err}");
    }

    #[test]
    fn check_json_reports_stale_and_clean() {
        let clean = Harness::new("update-check-json-clean");
        clean.write_source(
            ".bazelrc",
            "import %workspace%/tools/bazelrc/preset.bazelrc\n",
        );
        clean.write_source(
            "tools/bazelrc/preset.bazelrc",
            &dx_adopt::render_preset_fragment(),
        );
        let (code, out, err) = clean.run(&["update", "--check", "--output=json"]);
        assert_eq!(code, 0, "{out}{err}");
        let events: Vec<serde_json::Value> = out
            .lines()
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()
            .expect("NDJSON");
        assert_eq!(events[0]["event"], serde_json::json!("command_started"));
        assert_eq!(events[0]["mode"], serde_json::json!("check"));
        assert_eq!(
            events.last().expect("finished")["exit_code"],
            serde_json::json!(0)
        );

        let dirty = Harness::new("update-check-json-stale");
        dirty.write_source(
            ".bazelrc",
            "import %workspace%/tools/bazelrc/preset.bazelrc\n",
        );
        dirty.write_source("tools/bazelrc/preset.bazelrc", "# dirty\n");
        let (code, out, err) = dirty.run(&["update", "--check", "--output=json"]);
        assert_eq!(code, 1, "{out}{err}");
        assert!(out.contains("\"event\":\"error\""), "{out}");
        assert!(out.contains("update_failed"), "{out}");
    }

    #[test]
    fn default_updates_preset_atomically() {
        // Go is a no-op backend (no launch), so the preset fix is the
        // only mutation; proves default mode regenerates the fragment.
        let harness = Harness::new("update-default-preset");
        harness.write_source(
            ".bazelrc",
            "import %workspace%/tools/bazelrc/preset.bazelrc\ntry-import %workspace%/user.bazelrc\n",
        );
        harness.write_source("tools/bazelrc/preset.bazelrc", "# dirty\n");
        let (code, out, err) = harness.run(&["update", "go"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(
            out.contains("updated preset (tools/bazelrc/preset.bazelrc)"),
            "{out}"
        );
        assert_eq!(
            std::fs::read_to_string(harness.workspace.join("tools/bazelrc/preset.bazelrc"))
                .expect("read"),
            dx_adopt::render_preset_fragment()
        );
        assert_eq!(err, "", "{err}");
    }

    #[test]
    fn update_json_never_emits_change_or_mutation() {
        // Issue #586 wont-fix: backends provide no committed-change
        // manifest and Git/BUILD inference is forbidden, so update JSON
        // never emits change/mutation events in any mode.
        let runner = ScriptRunner::new(&[]);
        let (code, out, err) = run_with(&["update", "--output=json"], &runner);
        assert_eq!(code, 0, "{out}{err}");
        assert!(
            !out.contains("\"event\":\"change\""),
            "update must not emit change events: {out}"
        );
        assert!(
            !out.contains("\"event\":\"mutation\""),
            "update must not emit mutation events: {out}"
        );
        assert!(
            !out.contains("\"event\":\"diagnostic\""),
            "update must not emit diagnostics: {out}"
        );
        assert!(
            !out.contains("\"event\":\"operation\""),
            "update must not emit operations: {out}"
        );
        let events: Vec<serde_json::Value> = out
            .lines()
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()
            .expect("NDJSON");
        for event in &events {
            let kind = event["event"].as_str().expect("event");
            assert!(
                kind == "command_started"
                    || kind == "notice"
                    || kind == "error"
                    || kind == "command_finished",
                "unexpected update event {kind}: {out}"
            );
        }
    }

    #[test]
    fn update_json_completeness_is_per_set_plus_finished() {
        // Issue #586 event-completeness contract: exactly one terminal
        // per-set event per selected set in sorted order, then exactly
        // one command_finished. Preceding per-set events stay true with
        // no rollback; nothing is inferred for unattempted sets.
        let runner = ScriptRunner::new(&[("maven", Some(1))]);
        let (code, out, err) = run_with(&["update", "--output=json"], &runner);
        assert_eq!(code, 1, "{out}{err}");
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
        // Five selected sets means five terminal per-set events.
        let per_set = &events[1..events.len() - 1];
        assert_eq!(per_set.len(), 5, "{out}");
        let scopes: Vec<String> = per_set
            .iter()
            .map(|event| {
                let kind = event["event"].as_str().expect("event");
                if kind == "error" {
                    // Issue #586: per-set failures ride `update_failed`
                    // errors without a scope; the message names the set.
                    assert_eq!(
                        event["code"].as_str().expect("code"),
                        "update_failed",
                        "{out}"
                    );
                    let message = event["message"].as_str().expect("message");
                    assert!(message.contains("maven"), "{out}");
                    "maven".to_owned()
                } else {
                    assert_eq!(kind, "notice", "{out}");
                    assert_eq!(
                        event["code"].as_str().expect("code"),
                        "update_set_success",
                        "{out}"
                    );
                    event["scope"][0].as_str().expect("scope").to_owned()
                }
            })
            .collect();
        let mut sorted = scopes.clone();
        sorted.sort();
        assert_eq!(scopes, sorted, "per-set events use sorted set order: {out}");
        // Finished keeps results_complete (every set reached a terminal
        // report, including the failure) and omits file-level counts.
        let finished = events.last().expect("finished");
        assert_eq!(finished["exit_code"], serde_json::json!(1));
        assert_eq!(finished["results_complete"], serde_json::json!(true));
        assert!(finished.get("changes").is_none(), "{out}");
        assert!(finished.get("mutations").is_none(), "{out}");
        assert!(finished.get("diagnostics").is_none(), "{out}");
    }

    #[test]
    fn update_json_check_and_dryrun_emit_no_file_events_or_counts() {
        // Issue #586: the wont-fix holds for --check and --dry-run too;
        // neither emits file-level events nor counts.
        let harness = Harness::new("update-586-check-json");
        harness.write_source(
            ".bazelrc",
            "import %workspace%/tools/bazelrc/preset.bazelrc\n",
        );
        harness.write_source(
            "tools/bazelrc/preset.bazelrc",
            &dx_adopt::render_preset_fragment(),
        );
        let (code, out, err) = harness.run(&["update", "--check", "--output=json"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(!out.contains("\"event\":\"change\""), "{out}");
        assert!(!out.contains("\"event\":\"mutation\""), "{out}");
        let events: Vec<serde_json::Value> = out
            .lines()
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()
            .expect("NDJSON");
        let finished = events.last().expect("finished");
        assert!(finished.get("changes").is_none(), "{out}");
        assert!(finished.get("mutations").is_none(), "{out}");
        assert!(finished.get("diagnostics").is_none(), "{out}");

        let dry = Harness::new("update-586-dryrun-json");
        let (code, out, err) = dry.run(&["update", "--dry-run", "--output=json"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(!out.contains("\"event\":\"change\""), "{out}");
        assert!(!out.contains("\"event\":\"mutation\""), "{out}");
    }
}
