//! Docs site build/check/serve execution over the Bazel-cached pipeline.
//!
//! Contract: `docs/cli/commands/docs.md`.
//!
//! Check selects extraction plus shared validation without rendering;
//! the default build validates and renders; `--serve` previews the last
//! build outputs locally without caching of its own. Both modes share
//! the same validation and reject the same invalid IR and references.
//! See: `docs/documentation/site.md`.

use super::common::*;
use crate::args::Invocation;
use crate::resolve::resolve;
use dx_output::{
    command_finished, command_started, error_event, operation_event, write_event, FinishedCounts,
    OutputMode,
};

/// Fixture-scale docs site targets proving the extract to render chain.
/// Bare scope selects the repository docs site; explicit scopes resolve
/// through the shared workflow resolution instead.
/// See: `docs/documentation/site.md`.
const DOCS_CHECK_TARGET: &str = "//docs/site:demo_aggregate";
const DOCS_BUILD_TARGET: &str = "//docs/site:demo_site";

/// Default preview port when `--serve` runs without `--port`.
/// See: `docs/cli/commands/docs.md`.
const DOCS_DEFAULT_PORT: u16 = 8000;

/// Stable operational error code for `dx docs --serve` preview failures:
/// the local preview server exited nonzero after a successful build.
// See: `docs/cli/output-protocol.md#operational-error`.
pub(crate) const CODE_SERVE_FAILED: &str = "serve_failed";

/// Runs `dx docs [--check] [--serve [--port <n>]] [scope ...]`: resolves
/// the scope through the shared workflow resolution (bare scope selects
/// the repository docs site), builds the Bazel-cached extract to
/// aggregate to render chain, and optionally previews the last build
/// outputs locally. The build is non-mutating: only Bazel outputs and
/// cache entries are written, never sources or committed IR.
/// See: `docs/cli/commands/docs.md`.
pub(crate) fn execute_docs(invocation: &Invocation, env: Env<'_>) -> i32 {
    let Env {
        workspace,
        runner,
        query_runner,
        out,
        err,
        ..
    } = env;
    let json = invocation.output == OutputMode::Json;
    let mode = if invocation.check { "check" } else { "default" };
    // Scope reuses the shared workflow resolution; bare scope selects
    // the repository docs site targets below.
    // See: `docs/documentation/site.md`.
    let (labels, scope_text) = if invocation.targets.is_empty() {
        let target = if invocation.check {
            DOCS_CHECK_TARGET
        } else {
            DOCS_BUILD_TARGET
        };
        (vec![target.to_owned()], "//...".to_owned())
    } else {
        match resolve(&invocation.targets, workspace, query_runner) {
            Ok(resolved) => {
                let scope_text = if resolved.targets.is_empty() {
                    "//...".to_owned()
                } else {
                    resolved.targets.join(" ")
                };
                (resolved.targets, scope_text)
            }
            Err(error) => return pre_exec(err, &error.to_string()),
        }
    };
    let actions: &[&str] = if invocation.check {
        &["extract", "aggregate"]
    } else {
        &["extract", "aggregate", "render"]
    };
    let summary = if invocation.check {
        format!("Running docs check for {scope_text} (extract+aggregate, no render)")
    } else {
        format!("Running docs build for {scope_text} (extract+aggregate+render)")
    };
    if json {
        if let Ok(event) = command_started(invocation.command.name(), invocation.dry_run, mode) {
            let _ = write_event(out, &event);
        }
        for action in actions {
            if let Ok(event) = operation_event(invocation.command.name(), action, Some(&labels)) {
                let _ = write_event(out, &event);
            }
        }
    }
    if invocation.dry_run {
        if json {
            let _ = write_event(out, &command_finished(0, &FinishedCounts::default()));
        } else if matches!(invocation.output, OutputMode::Text { quiet: false })
            && !invocation.quiet
        {
            let _ = writeln!(out, "{summary}");
            if invocation.serve {
                let port = invocation.port.unwrap_or(DOCS_DEFAULT_PORT);
                let _ = writeln!(out, "would serve at http://127.0.0.1:{port}/");
            }
        }
        return 0;
    }
    if !json && matches!(invocation.output, OutputMode::Text { quiet: false }) && !invocation.quiet
    {
        let _ = writeln!(out, "{summary}");
    }
    let mut argv = vec![
        "bazel".to_owned(),
        "--nohome_rc".to_owned(),
        "--nosystem_rc".to_owned(),
        "build".to_owned(),
        crate::plan::workspace_flag(),
    ];
    argv.extend(labels.clone());
    let status = match runner.run(&argv, workspace, &[]) {
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
    if bazel_code != 0 {
        if json {
            if let Ok(event) = error_event(
                "bazel_failed",
                &format!(
                    "Bazel docs {} failed with exit {bazel_code} (see stderr diagnostics; unit docs/site:demo, pinned mdBook 0.4.43)",
                    if invocation.check { "check" } else { "build" },
                ),
                None,
                None,
                Some("execute"),
            ) {
                let _ = write_event(out, &event);
            }
            let _ = write_event(
                out,
                &command_finished(bazel_code, &FinishedCounts::default()),
            );
        }
        return bazel_code;
    }
    if !invocation.serve {
        if json {
            let _ = write_event(out, &command_finished(0, &FinishedCounts::default()));
        }
        return 0;
    }
    let port = invocation.port.unwrap_or(DOCS_DEFAULT_PORT);
    let serve_dir = workspace.join("bazel-bin/docs/site");
    let serve_dir_text = serve_dir.display().to_string();
    if !json && matches!(invocation.output, OutputMode::Text { quiet: false }) && !invocation.quiet
    {
        let _ = writeln!(
            out,
            "Serving docs at http://127.0.0.1:{port}/ ({serve_dir_text})"
        );
    }
    let serve_argv = vec![
        "python3".to_owned(),
        "-m".to_owned(),
        "http.server".to_owned(),
        port.to_string(),
        "--directory".to_owned(),
        serve_dir_text,
    ];
    let serve_status = match runner.run(&serve_argv, workspace, &[]) {
        Ok(status) => status,
        Err(error) => {
            return operational(
                invocation,
                out,
                err,
                CODE_LAUNCH_FAILED,
                &format!("failed to launch docs preview: {error}"),
            );
        }
    };
    let Some(serve_code) = serve_status.code else {
        return operational(
            invocation,
            out,
            err,
            CODE_BAZEL_SIGNALLED,
            "Docs preview terminated by signal",
        );
    };
    if json {
        if serve_code != 0 {
            if let Ok(event) = error_event(
                CODE_SERVE_FAILED,
                &format!("Docs preview exited with {serve_code} (see stderr diagnostics)"),
                None,
                None,
                Some("serve"),
            ) {
                let _ = write_event(out, &event);
            }
        }
        let _ = write_event(
            out,
            &command_finished(serve_code, &FinishedCounts::default()),
        );
    }
    serve_code
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use super::CODE_SERVE_FAILED;

    #[test]
    fn docs_check_builds_aggregate_without_render() {
        let harness = Harness::new("docs-check");
        let (code, out, _) = harness.run(&["docs", "--check", "--output=text"]);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("Running docs check"), "{out}");
        assert!(out.contains("no render"), "{out}");
    }

    #[test]
    fn docs_build_validates_and_renders() {
        let harness = Harness::new("docs-build");
        let (code, out, _) = harness.run(&["docs", "--output=text"]);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("Running docs build"), "{out}");
        assert!(out.contains("render"), "{out}");
    }

    #[test]
    fn docs_port_requires_serve() {
        let args: Vec<String> = ["docs", "--port=8080"]
            .iter()
            .map(ToString::to_string)
            .collect();
        let err = crate::args::parse(&args).expect_err("port without serve must fail");
        assert!(err.to_string().contains("--port"), "{err}");
    }

    #[test]
    fn docs_serve_previews_after_build() {
        let harness = Harness::new("docs-serve");
        let (code, out, _) = harness.run(&["docs", "--serve", "--port=8080"]);
        assert_eq!(code, 0, "{out}");
        assert!(
            out.contains("Serving docs at http://127.0.0.1:8080/"),
            "{out}"
        );
    }

    #[test]
    fn docs_dry_run_plans_without_launch() {
        let harness = Harness::new("docs-dry");
        let (code, out, _) = harness.run(&["docs", "--check", "--dry-run"]);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("Running docs check"), "{out}");
        assert!(
            harness.seen_env.borrow().is_empty(),
            "dry-run launches nothing"
        );
    }

    #[test]
    fn docs_json_streams_started_operation_finished() {
        let harness = Harness::new("docs-json");
        let (code, out, _) = harness.run(&["docs", "--check", "--output=json"]);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("command_started"), "{out}");
        assert!(out.contains("operation"), "{out}");
        assert!(out.contains("command_finished"), "{out}");
        for line in out.lines() {
            serde_json::from_str::<serde_json::Value>(line).expect("NDJSON line");
        }
    }

    #[test]
    fn docs_bazel_failure_names_unit_and_pin() {
        let mut harness = Harness::new("docs-fail");
        harness.bazel_code = 3;
        let (code, out, _) = harness.run(&["docs", "--output=json"]);
        assert_eq!(code, 3, "{out}");
        assert!(out.contains("bazel_failed"), "{out}");
        assert!(out.contains("docs/site:demo"), "{out}");
        assert!(out.contains("0.4.43"), "{out}");
    }

    #[test]
    fn serve_failed_code_is_stable_single_source() {
        // Fixture pins the stable wire code so output-protocol drift
        // fails here, not in automation matching on `code`.
        // See: `docs/cli/output-protocol.md#operational-error`.
        assert_eq!(CODE_SERVE_FAILED, "serve_failed");
    }

    #[test]
    fn docs_serve_failure_emits_serve_failed() {
        use std::io;
        struct ServeFailRunner;
        impl dx_process::Runner for ServeFailRunner {
            fn run(
                &self,
                argv: &[String],
                _cwd: &std::path::Path,
                _env: &[(&str, &str)],
            ) -> io::Result<dx_process::ChildStatus> {
                if argv.first().is_some_and(|first| first == "bazel") {
                    Ok(dx_process::ChildStatus { code: Some(0) })
                } else {
                    Ok(dx_process::ChildStatus { code: Some(3) })
                }
            }
        }
        let harness = Harness::new("docs-serve-fail");
        let inv = invocation(&["docs", "--serve", "--port=8080", "--output=json"]);
        let runner = ServeFailRunner;
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = super::execute_docs(
            &inv,
            super::super::common::Env {
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
        assert_eq!(code, 3, "{code}");
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
        let error = events
            .iter()
            .find(|event| event["event"] == serde_json::json!("error"))
            .expect("serve error");
        assert_eq!(error["code"], serde_json::json!(CODE_SERVE_FAILED));
    }
}
