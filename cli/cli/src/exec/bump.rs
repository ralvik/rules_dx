//! Bump command execution: explicit widen-one-requirement plus automatic refresh.

use super::common::*;
use crate::args::{Command, Invocation};
use crate::reports::plan_reports;
use dx_output::{
    command_finished, command_started, error_event, notice_event, write_event, FinishedCounts,
    NoticeEvent, OutputMode,
};

/// Runs `dx bump <selector> <version>`: validates the single
/// `set:package` selector plus new version through `dx_bump`, mutating
/// without confirmation. `--dry-run` plans the widen plus the automatic
/// refresh and exits `0` without touching the tree or launching; live
/// execution rewrites exactly one declared requirement atomically (never
/// batch) then chains the resolver-owned refresh automatically
/// (`dx update cargo` full, `dx update npm:<pkg>` selective,
/// `dx update go` noop, `dx update maven` full, `dx update nuget` full
/// for Cargo/npm/Go/Maven/NuGet; preset flag-diff review plus build
/// stays file-only for Bazel/GitHub Actions with no launch).
/// Usage errors exit `2` before any write; widen failures exit `1` with
/// `bump_failed`; refresh failures exit `1` with `update_failed` with the
/// widen kept (no rollback).
pub(crate) fn execute_bump(invocation: &Invocation, env: Env<'_>) -> i32 {
    debug_assert!(
        invocation.command == Command::Bump,
        "bump dispatch guards commands"
    );
    let Env {
        workspace,
        runner,
        out,
        err,
        ..
    } = env;
    match plan_reports(
        invocation.command,
        &invocation.reports,
        &invocation.output,
        invocation.dry_run,
    ) {
        Ok(_) => {}
        Err(error) => return pre_exec(err, &error.to_string()),
    }
    if invocation.targets.len() != 2 {
        return pre_exec(err, "bump needs exactly <selector> <version>");
    }
    let request = match dx_bump::BumpRequest::parse(&invocation.targets[0], &invocation.targets[1])
    {
        Ok(request) => request,
        Err(error) => return pre_exec(err, &error.to_string()),
    };
    let summary = request.summary();
    let verbose =
        matches!(invocation.output, OutputMode::Text { quiet: false }) && !invocation.quiet;
    if invocation.dry_run {
        if invocation.output == OutputMode::Json {
            if let Ok(event) = command_started(invocation.command.name(), true, "default") {
                let _ = write_event(out, &event);
            }
            if let Ok(event) = notice_event(&NoticeEvent {
                level: "info".to_owned(),
                code: "bump_planned".to_owned(),
                message: summary,
                related_command: Some("bump".to_owned()),
                scope: Some(vec![request.selector.clone()]),
                path: Some(request.target_manifest().to_owned()),
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
    // Live: read the owning manifest, plan the single-requirement edit
    // over its bytes, and commit atomically. Any failure leaves the tree
    // untouched and reports `bump_failed` (exit 1).
    let manifest = request.target_manifest();
    let original = match std::fs::read(workspace.join(manifest)) {
        Ok(bytes) => bytes,
        Err(_) => {
            return operational(
                invocation,
                out,
                err,
                CODE_BUMP_FAILED,
                &format!(
                    "failed to widen {}: cannot read {manifest}",
                    request.selector
                ),
            );
        }
    };
    let text = match String::from_utf8(original) {
        Ok(text) => text,
        Err(_) => {
            return operational(
                invocation,
                out,
                err,
                CODE_BUMP_FAILED,
                &format!(
                    "failed to widen {}: {manifest} is not valid UTF-8",
                    request.selector
                ),
            );
        }
    };
    let widened = match request.plan_edit(&text) {
        Ok(widened) => widened,
        Err(error) => {
            return operational(
                invocation,
                out,
                err,
                CODE_BUMP_FAILED,
                &format!("failed to widen {}: {error}", request.selector),
            );
        }
    };
    if dx_atomic_fs::write_atomic(&workspace.join(manifest), widened.as_bytes()).is_err() {
        return operational(
            invocation,
            out,
            err,
            CODE_BUMP_FAILED,
            &format!(
                "failed to widen {}: cannot write {manifest}",
                request.selector
            ),
        );
    }
    // Automatic chaining (issue #638): the widen is committed, then the
    // resolver-owned refresh runs without a manual second step. File-only
    // sets (Bazel, GitHub Actions) have no refresh launch; Cargo/npm/Go
    // refresh through the approved `dx_update::backend` operations.
    if !request.needs_update_refresh() {
        let message = format!(
            "widened {} to {} in {manifest} (then run preset flag-diff review plus `bazel build //...`)",
            request.selector,
            request.version.display()
        );
        if invocation.output == OutputMode::Json {
            if let Ok(event) = notice_event(&NoticeEvent {
                level: "info".to_owned(),
                code: "bump_widened".to_owned(),
                message: message.clone(),
                related_command: Some("bump".to_owned()),
                scope: Some(vec![request.selector.clone()]),
                path: Some(manifest.to_owned()),
                language: None,
                import: None,
            }) {
                let _ = write_event(out, &event);
            }
            let finished = command_finished(
                0,
                &FinishedCounts {
                    results_complete: Some(true),
                    ..FinishedCounts::default()
                },
            );
            let _ = write_event(out, &finished);
            return 0;
        }
        if verbose {
            let _ = writeln!(out, "{message}");
        }
        return 0;
    }
    // Resolver-owned refresh: Cargo full, npm selective for the widened
    // package, Go full noop, Maven full, NuGet full. Never a private
    // resolver.
    let (update_set, update_request, update_selector) = match request.set {
        dx_bump::BumpSet::Cargo => (
            dx_update::sets::SetId::Cargo,
            dx_update::selector::SetRequest::Full,
            "cargo".to_owned(),
        ),
        dx_bump::BumpSet::Npm => {
            let package = request.package.clone();
            (
                dx_update::sets::SetId::Npm,
                dx_update::selector::SetRequest::Packages(vec![package.clone()]),
                format!("npm:{package}"),
            )
        }
        dx_bump::BumpSet::Go => (
            dx_update::sets::SetId::Go,
            dx_update::selector::SetRequest::Full,
            "go".to_owned(),
        ),
        dx_bump::BumpSet::Maven => (
            dx_update::sets::SetId::Maven,
            dx_update::selector::SetRequest::Full,
            "maven".to_owned(),
        ),
        dx_bump::BumpSet::NuGet => (
            dx_update::sets::SetId::NuGet,
            dx_update::selector::SetRequest::Full,
            "nuget".to_owned(),
        ),
        // File-only sets return above; this arm is unreachable.
        dx_bump::BumpSet::Bazel | dx_bump::BumpSet::GithubActions => {
            return operational(
                invocation,
                out,
                err,
                CODE_BUMP_FAILED,
                &format!(
                    "failed to refresh {}: no refresh for file-only set",
                    request.selector
                ),
            );
        }
    };
    let plan = match dx_update::backend::plan(update_set, &update_request) {
        Ok(plan) => plan,
        Err(error) => {
            let detail = match error {
                dx_update::backend::BackendError::Unsupported { reason, .. } => reason,
            };
            return bump_refresh_failed(
                invocation,
                out,
                err,
                &request,
                manifest,
                &format!("unsupported refresh: {detail}"),
            );
        }
    };
    match plan {
        dx_update::backend::BackendPlan::Noop => {
            let widened_message = format!(
                "widened {} to {} in {manifest}",
                request.selector,
                request.version.display()
            );
            let refreshed_message = format!(
                "{widened_message} and refreshed {update_selector} via `dx update {update_selector}` automatically (pinned module lock; no-op success)"
            );
            emit_bump_refreshed(
                invocation,
                out,
                err,
                &request,
                manifest,
                &widened_message,
                &refreshed_message,
                update_set.name(),
                verbose,
            );
            0
        }
        dx_update::backend::BackendPlan::Run { argv, env: extra } => {
            let env_refs: Vec<(&str, &str)> = extra
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str()))
                .collect();
            match runner.run(&argv, workspace, &env_refs) {
                Err(error) => bump_refresh_failed(
                    invocation,
                    out,
                    err,
                    &request,
                    manifest,
                    &format!(
                        "failed to refresh {update_selector}: failed to launch updater: {error} (widen kept in {manifest})"
                    ),
                ),
                Ok(status) => match status.code {
                    Some(0) => {
                        let widened_message = format!(
                            "widened {} to {} in {manifest}",
                            request.selector,
                            request.version.display()
                        );
                        let refreshed_message = format!(
                            "{widened_message} and refreshed {update_selector} via `dx update {update_selector}` automatically (resolver-owned)"
                        );
                        emit_bump_refreshed(invocation, out, err, &request, manifest, &widened_message, &refreshed_message, update_set.name(), verbose);
                        0
                    }
                    Some(code) => bump_refresh_failed(
                        invocation,
                        out,
                        err,
                        &request,
                        manifest,
                        &format!(
                            "failed to refresh {update_selector}: updater exited {code} (widen kept in {manifest})"
                        ),
                    ),
                    None => bump_refresh_failed(
                        invocation,
                        out,
                        err,
                        &request,
                        manifest,
                        &format!(
                            "failed to refresh {update_selector}: updater terminated by signal (widen kept in {manifest})"
                        ),
                    ),
                },
            }
        }
    }
}

/// Emits the widen-plus-refresh success notices (text plus JSON).
#[allow(clippy::too_many_arguments)]
fn emit_bump_refreshed(
    invocation: &Invocation,
    out: &mut dyn std::io::Write,
    err: &mut dyn std::io::Write,
    request: &dx_bump::BumpRequest,
    manifest: &str,
    widened_message: &str,
    refreshed_message: &str,
    set_name: &str,
    verbose: bool,
) {
    let _ = err;
    if invocation.output == OutputMode::Json {
        if let Ok(event) = notice_event(&NoticeEvent {
            level: "info".to_owned(),
            code: "bump_widened".to_owned(),
            message: widened_message.to_owned(),
            related_command: Some("bump".to_owned()),
            scope: Some(vec![request.selector.clone()]),
            path: Some(manifest.to_owned()),
            language: None,
            import: None,
        }) {
            let _ = write_event(out, &event);
        }
        if let Ok(event) = notice_event(&NoticeEvent {
            level: "info".to_owned(),
            code: "update_set_success".to_owned(),
            message: refreshed_message.to_owned(),
            related_command: Some("update".to_owned()),
            scope: Some(vec![set_name.to_owned()]),
            path: None,
            language: None,
            import: None,
        }) {
            let _ = write_event(out, &event);
        }
        let finished = command_finished(
            0,
            &FinishedCounts {
                results_complete: Some(true),
                ..FinishedCounts::default()
            },
        );
        let _ = write_event(out, &finished);
    } else if verbose {
        let _ = writeln!(out, "{refreshed_message}");
    }
}

/// Reports a refresh failure after the widen is kept (exit 1, `update_failed`).
fn bump_refresh_failed(
    invocation: &Invocation,
    out: &mut dyn std::io::Write,
    err: &mut dyn std::io::Write,
    request: &dx_bump::BumpRequest,
    manifest: &str,
    message: &str,
) -> i32 {
    let _ = writeln!(err, "dx: {CODE_UPDATE_FAILED}: {message}");
    if invocation.output == OutputMode::Json {
        // The widen stays visible: emit the widen notice before the error
        // so interrupted chaining keeps the preceding widen true.
        let widened_message = format!(
            "widened {} to {} in {manifest}",
            request.selector,
            request.version.display()
        );
        if let Ok(event) = notice_event(&NoticeEvent {
            level: "info".to_owned(),
            code: "bump_widened".to_owned(),
            message: widened_message,
            related_command: Some("bump".to_owned()),
            scope: Some(vec![request.selector.clone()]),
            path: Some(manifest.to_owned()),
            language: None,
            import: None,
        }) {
            let _ = write_event(out, &event);
        }
        if let Ok(event) = error_event(CODE_UPDATE_FAILED, message, None, None, Some("execute")) {
            let _ = write_event(out, &event);
        }
        let finished = command_finished(
            1,
            &FinishedCounts {
                results_complete: Some(false),
                ..FinishedCounts::default()
            },
        );
        let _ = write_event(out, &finished);
    }
    dx_process::operational_code()
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;

    #[test]
    fn dry_run_plans_without_writing() {
        let harness = Harness::new("bump-dryrun");
        harness.write_source(
            "rust/tests/fixtures/hello/Cargo.toml",
            "[dependencies]\nanyhow = \"1\"\n",
        );
        let (code, out, err) = harness.run(&["bump", "cargo:anyhow", "1.2.3", "--dry-run"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("Widen cargo:anyhow"), "{out}");
        assert_eq!(err, "", "{err}");
        // Dry-run writes nothing.
        assert_eq!(
            std::fs::read_to_string(
                harness
                    .workspace
                    .join("rust/tests/fixtures/hello/Cargo.toml")
            )
            .expect("read"),
            "[dependencies]\nanyhow = \"1\"\n"
        );
    }

    #[test]
    fn dry_run_rejects_bad_selector_and_version() {
        let harness = Harness::new("bump-dryrun-bad-selector");
        let (code, _, err) = harness.run(&["bump", "crates", "1.2.3", "--dry-run"]);
        assert_eq!(code, 2, "{err}");
        let harness = Harness::new("bump-dryrun-bad-version");
        let (code, _, err) = harness.run(&["bump", "cargo:anyhow", "nope!!!", "--dry-run"]);
        assert_eq!(code, 2, "{err}");
    }

    #[test]
    fn live_widens_one_cargo_requirement_atomically() {
        // Issue #638: widen chains the full repin automatically (no manual
        // second step); the fake runner succeeds so the chain exits 0.
        let harness = Harness::new("bump-live-cargo");
        harness.write_source(
            "rust/tests/fixtures/hello/Cargo.toml",
            "[dependencies]\nanyhow = \"1\"\nserde = \"1\"\n",
        );
        let (code, out, err) = harness.run(&["bump", "cargo:anyhow", "1.2.3"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("widened cargo:anyhow to 1.2.3"), "{out}");
        assert!(
            out.contains("and refreshed cargo via `dx update cargo` automatically"),
            "{out}"
        );
        assert!(err.is_empty(), "{err}");
        assert_eq!(
            harness.seen_env.borrow().len(),
            1,
            "cargo chains one refresh launch"
        );
        let widened = std::fs::read_to_string(
            harness
                .workspace
                .join("rust/tests/fixtures/hello/Cargo.toml"),
        )
        .expect("read");
        assert!(widened.contains("anyhow = \"1.2.3\""), "{widened}");
        assert!(widened.contains("serde = \"1\""), "{widened}");
    }

    #[test]
    fn live_npm_chains_selective_refresh_automatically() {
        // Issue #638: npm widens then refreshes only the widened package
        // (`dx update npm:<pkg>` selective, never a silent full).
        let harness = Harness::new("bump-live-npm-chain");
        harness.write_source(
            "package.json",
            "{\n  \"dependencies\": {\n    \"jest\": \"30.2.0\",\n    \"vue\": \"3.5.42\"\n  }\n}\n",
        );
        let (code, out, err) = harness.run(&["bump", "npm:jest", "30.3.0"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("widened npm:jest to 30.3.0"), "{out}");
        assert!(
            out.contains("and refreshed npm:jest via `dx update npm:jest` automatically"),
            "{out}"
        );
        assert!(err.is_empty(), "{err}");
        assert_eq!(
            harness.seen_env.borrow().len(),
            1,
            "npm chains one selective launch"
        );
        let widened =
            std::fs::read_to_string(harness.workspace.join("package.json")).expect("read");
        assert!(widened.contains("\"jest\": \"30.3.0\""), "{widened}");
        assert!(widened.contains("\"vue\": \"3.5.42\""), "{widened}");
    }

    #[test]
    fn live_go_chains_noop_without_launch() {
        // Issue #638: Go widens then refreshes as the pinned no-op success
        // with no launch (module lock tracks Gazelle).
        let harness = Harness::new("bump-live-go-chain");
        harness.write_source(
            "third_party/go/go.mod",
            "module example.com/mod\n\nrequire example.com/mod v1.2.3\n",
        );
        let (code, out, err) = harness.run(&["bump", "go:example.com/mod", "1.3.0"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("widened go:example.com/mod"), "{out}");
        assert!(
            out.contains("and refreshed go via `dx update go` automatically"),
            "{out}"
        );
        assert!(out.contains("no-op success"), "{out}");
        assert!(err.is_empty(), "{err}");
        assert!(
            harness.seen_env.borrow().is_empty(),
            "go noop chains without launch"
        );
        let widened =
            std::fs::read_to_string(harness.workspace.join("third_party/go/go.mod")).expect("read");
        assert!(widened.contains("v1.3.0"), "{widened}");
    }

    #[test]
    fn live_maven_chains_full_refresh_automatically() {
        // Issue #638 (plus #637 widen): Maven widens one artifact then
        // chains the whole-lock pin automatically.
        let harness = Harness::new("bump-live-maven-chain");
        harness.write_source(
            "MODULE.bazel",
            "maven.install(\n    artifacts = [\n        \"junit:junit:4.13.2\",\n    ],\n)\n",
        );
        let (code, out, err) = harness.run(&["bump", "maven:junit:junit", "4.13.3"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("widened maven:junit:junit"), "{out}");
        assert!(
            out.contains("and refreshed maven via `dx update maven` automatically"),
            "{out}"
        );
        assert!(err.is_empty(), "{err}");
        assert_eq!(
            harness.seen_env.borrow().len(),
            1,
            "maven chains one full launch"
        );
        let widened =
            std::fs::read_to_string(harness.workspace.join("MODULE.bazel")).expect("read");
        assert!(widened.contains("\"junit:junit:4.13.3\""), "{widened}");
    }

    #[test]
    fn live_nuget_chains_full_refresh_automatically() {
        // Issue #638 (plus #637 widen): NuGet widens one id then chains the
        // whole-folder regen automatically.
        let harness = Harness::new("bump-live-nuget-chain");
        harness.write_source(
            "third_party/dotnet/paket.dependencies",
            "source https://api.nuget.org/v3/index.json\nnuget FSharp.Core 10.1.201\n",
        );
        let (code, out, err) = harness.run(&["bump", "nuget:FSharp.Core", "10.1.202"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("widened nuget:FSharp.Core"), "{out}");
        assert!(
            out.contains("and refreshed nuget via `dx update nuget` automatically"),
            "{out}"
        );
        assert!(err.is_empty(), "{err}");
        assert_eq!(
            harness.seen_env.borrow().len(),
            1,
            "nuget chains one full launch"
        );
        let widened = std::fs::read_to_string(
            harness
                .workspace
                .join("third_party/dotnet/paket.dependencies"),
        )
        .expect("read");
        assert!(widened.contains("nuget FSharp.Core 10.1.202"), "{widened}");
    }

    #[test]
    fn live_bazel_file_only_has_no_refresh_launch() {
        // File-only sets keep the flag-diff review with no chaining launch.
        let harness = Harness::new("bump-live-bazel-fileonly");
        harness.write_source(".bazelversion", "9.2.0\n");
        let (code, out, err) = harness.run(&["bump", "bazel:.bazelversion", "9.3.0"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(
            out.contains("widened bazel:.bazelversion to 9.3.0"),
            "{out}"
        );
        assert!(out.contains("flag-diff"), "{out}");
        assert!(!out.contains("automatically"), "{out}");
        assert!(err.is_empty(), "{err}");
        assert!(
            harness.seen_env.borrow().is_empty(),
            "file-only chains nothing"
        );
    }

    #[test]
    fn dry_run_chains_without_launch_or_write() {
        // Dry-run plans widen plus automatic refresh without touching the
        // tree or launching.
        let harness = Harness::new("bump-dryrun-chain");
        harness.write_source(
            "rust/tests/fixtures/hello/Cargo.toml",
            "[dependencies]\nanyhow = \"1\"\n",
        );
        let (code, out, err) = harness.run(&["bump", "cargo:anyhow", "1.2.3", "--dry-run"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("Widen cargo:anyhow"), "{out}");
        assert!(out.contains("dx update cargo"), "{out}");
        assert!(out.contains("automatically"), "{out}");
        assert_eq!(err, "", "{err}");
        assert!(
            harness.seen_env.borrow().is_empty(),
            "dry-run launches nothing"
        );
        assert_eq!(
            std::fs::read_to_string(
                harness
                    .workspace
                    .join("rust/tests/fixtures/hello/Cargo.toml")
            )
            .expect("read"),
            "[dependencies]\nanyhow = \"1\"\n"
        );
    }

    #[test]
    fn live_refresh_failure_keeps_widen_and_reports_update_failed() {
        // The widen stays committed (no rollback); the refresh failure
        // exits 1 with `update_failed` and the widen-kept hint.
        let mut harness = Harness::new("bump-live-refresh-fail");
        harness.bazel_code = 1;
        harness.write_source(
            "rust/tests/fixtures/hello/Cargo.toml",
            "[dependencies]\nanyhow = \"1\"\n",
        );
        let (code, out, err) = harness.run(&["bump", "cargo:anyhow", "1.2.3"]);
        assert_eq!(code, 1, "{out}{err}");
        assert!(err.contains("update_failed"), "{err}");
        assert!(err.contains("updater exited 1"), "{err}");
        assert!(err.contains("widen kept"), "{err}");
        assert_eq!(harness.seen_env.borrow().len(), 1, "refresh attempted once");
        // Widen is kept.
        let widened = std::fs::read_to_string(
            harness
                .workspace
                .join("rust/tests/fixtures/hello/Cargo.toml"),
        )
        .expect("read");
        assert!(widened.contains("anyhow = \"1.2.3\""), "{widened}");
    }

    #[test]
    fn live_refresh_launch_failure_keeps_widen() {
        let mut harness = Harness::new("bump-live-launch-fail");
        harness.io_error = true;
        harness.write_source(
            "rust/tests/fixtures/hello/Cargo.toml",
            "[dependencies]\nanyhow = \"1\"\n",
        );
        let (code, _, err) = harness.run(&["bump", "cargo:anyhow", "1.2.3"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("update_failed"), "{err}");
        assert!(err.contains("failed to launch updater"), "{err}");
        assert!(err.contains("widen kept"), "{err}");
        let widened = std::fs::read_to_string(
            harness
                .workspace
                .join("rust/tests/fixtures/hello/Cargo.toml"),
        )
        .expect("read");
        assert!(widened.contains("anyhow = \"1.2.3\""), "{widened}");
    }

    #[test]
    fn live_json_chained_emits_widened_plus_update_success() {
        // Resolver JSON carries both the widen and the refresh success
        // before `command_finished`.
        let harness = Harness::new("bump-live-json-chain");
        harness.write_source(
            "rust/tests/fixtures/hello/Cargo.toml",
            "[dependencies]\nanyhow = \"1\"\n",
        );
        let (code, out, err) = harness.run(&["bump", "cargo:anyhow", "1.2.3", "--output=json"]);
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
        let notices: Vec<&serde_json::Value> = events
            .iter()
            .filter(|event| event["event"] == serde_json::json!("notice"))
            .collect();
        assert_eq!(notices.len(), 2, "{out}");
        assert_eq!(notices[0]["code"], serde_json::json!("bump_widened"));
        assert_eq!(notices[1]["code"], serde_json::json!("update_set_success"));
        assert_eq!(
            events.last().expect("finished")["exit_code"],
            serde_json::json!(0)
        );
        assert_eq!(err, "", "{err}");
    }

    #[test]
    fn live_json_refresh_failure_emits_widened_plus_error() {
        let mut harness = Harness::new("bump-live-json-fail");
        harness.bazel_code = 2;
        harness.write_source(
            "rust/tests/fixtures/hello/Cargo.toml",
            "[dependencies]\nanyhow = \"1\"\n",
        );
        let (code, out, err) = harness.run(&["bump", "cargo:anyhow", "1.2.3", "--output=json"]);
        assert_eq!(code, 1, "{out}{err}");
        assert!(err.contains("update_failed"), "{err}");
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
        assert!(kinds.contains(&"notice"), "{out}");
        assert!(kinds.contains(&"error"), "{out}");
        assert_eq!(
            events.last().expect("finished")["exit_code"],
            serde_json::json!(1)
        );
    }

    #[test]
    fn live_missing_requirement_fails_without_writing() {
        let harness = Harness::new("bump-live-missing");
        harness.write_source(
            "rust/tests/fixtures/hello/Cargo.toml",
            "[dependencies]\nserde = \"1\"\n",
        );
        let (code, _, err) = harness.run(&["bump", "cargo:anyhow", "1.2.3"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("bump_failed"), "{err}");
        assert!(err.contains("no declared requirement"), "{err}");
        assert_eq!(
            std::fs::read_to_string(
                harness
                    .workspace
                    .join("rust/tests/fixtures/hello/Cargo.toml")
            )
            .expect("read"),
            "[dependencies]\nserde = \"1\"\n"
        );
    }

    #[test]
    fn live_github_tag_needs_sha_resolution() {
        let harness = Harness::new("bump-live-gha-tag");
        harness.write_source(
            ".github/workflows/ci.yml",
            "      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7\n",
        );
        let (code, _, err) = harness.run(&["bump", "github-actions:actions/checkout", "v5"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("bump_failed"), "{err}");
        assert!(err.contains("needs SHA resolution"), "{err}");
    }

    #[test]
    fn live_json_emits_planned_or_widened_and_finished() {
        let harness = Harness::new("bump-live-json");
        harness.write_source(".bazelversion", "9.2.0\n");
        let (code, out, err) =
            harness.run(&["bump", "bazel:.bazelversion", "9.3.0", "--output=json"]);
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
        assert!(kinds.contains(&"notice"));
        assert_eq!(
            events.last().expect("finished")["exit_code"],
            serde_json::json!(0)
        );
        assert_eq!(err, "", "{err}");
        assert_eq!(
            std::fs::read_to_string(harness.workspace.join(".bazelversion")).expect("read"),
            "9.3.0\n"
        );
    }
}
