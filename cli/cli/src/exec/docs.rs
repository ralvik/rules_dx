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

/// Default preview bind host when `--serve` runs without `--host`.
/// Loopback only so authoring previews never bind publicly by default.
/// See: `docs/cli/commands/docs.md`.
const DOCS_DEFAULT_HOST: &str = "127.0.0.1";

/// Stable operational error code for `dx docs --serve` preview failures:
/// the local preview server failed to launch or exited nonzero after a
/// successful build (including bind failures like a port in use).
// See: `docs/cli/output-protocol.md#operational-error`.
pub(crate) const CODE_SERVE_FAILED: &str = "serve_failed";

/// Preview URL for `host`/`port` (`http://<host>:<port>/`).
fn preview_url(host: &str, port: u16) -> String {
    format!("http://{host}:{port}/")
}

/// Browser opener argv reusing the preview `python3` dependency:
/// `webbrowser.open` works cross-platform without `xdg-open`/`open`.
fn opener_argv(url: &str) -> Vec<String> {
    vec![
        "python3".to_owned(),
        "-c".to_owned(),
        "import sys, webbrowser; webbrowser.open(sys.argv[1])".to_owned(),
        url.to_owned(),
    ]
}

/// Runs `dx docs [--check] [--serve [--port <n>] [--host <addr>] [--open]] [scope ...]`: resolves
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
                let host = invocation.host.as_deref().unwrap_or(DOCS_DEFAULT_HOST);
                let url = preview_url(host, port);
                let _ = writeln!(out, "would serve at {url}");
                if invocation.open {
                    let _ = writeln!(out, "would open {url}");
                }
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
    let host = invocation.host.as_deref().unwrap_or(DOCS_DEFAULT_HOST);
    let url = preview_url(host, port);
    let serve_dir = workspace.join("bazel-bin/docs/site");
    let serve_dir_text = serve_dir.display().to_string();
    if !json && matches!(invocation.output, OutputMode::Text { quiet: false }) && !invocation.quiet
    {
        let _ = writeln!(out, "Serving docs at {url} ({serve_dir_text})");
        if invocation.open {
            let _ = writeln!(out, "Opening {url}");
        }
    }
    if invocation.open {
        let open_argv = opener_argv(&url);
        match runner.run(&open_argv, workspace, &[]) {
            Ok(status) if status.code == Some(0) => {}
            Ok(status) => {
                let _ = writeln!(
                    err,
                    "dx: warning: browser open exited with {:?} for {url} (continuing preview)",
                    status.code,
                );
            }
            Err(error) => {
                let _ = writeln!(
                    err,
                    "dx: warning: failed to open browser for {url}: {error} (continuing preview)",
                );
            }
        }
    }
    let serve_argv = vec![
        "python3".to_owned(),
        "-m".to_owned(),
        "http.server".to_owned(),
        port.to_string(),
        "--directory".to_owned(),
        serve_dir_text,
        "--bind".to_owned(),
        host.to_owned(),
    ];
    let serve_status = match runner.run(&serve_argv, workspace, &[]) {
        Ok(status) => status,
        Err(error) => {
            return operational(
                invocation,
                out,
                err,
                CODE_SERVE_FAILED,
                &format!("failed to launch docs preview at {url}: {error} (port in use? retry another --port)"),
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
                &format!("Docs preview exited with {serve_code} at {url} (port in use? retry another --port; see stderr diagnostics)"),
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
        for words in [
            vec!["docs", "--host=example.test"],
            vec!["docs", "--open"],
            vec!["docs", "--port=0", "--serve"],
        ] {
            let args: Vec<String> = words.iter().map(ToString::to_string).collect();
            assert!(crate::args::parse(&args).is_err(), "{words:?}");
        }
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
    fn docs_serve_host_and_open() {
        let harness = Harness::new("docs-serve-host");
        let (code, out, _) = harness.run(&[
            "docs",
            "--serve",
            "--port=8080",
            "--host=example.test",
            "--open",
        ]);
        assert_eq!(code, 0, "{out}");
        assert!(
            out.contains("Serving docs at http://example.test:8080/"),
            "{out}"
        );
        assert!(out.contains("Opening http://example.test:8080/"), "{out}");
    }

    #[test]
    fn docs_serve_binds_host_and_opens_browser() {
        use std::cell::RefCell;
        use std::io;
        use std::rc::Rc;
        struct Probe {
            seen: Rc<RefCell<Vec<Vec<String>>>>,
        }
        impl dx_process::Runner for Probe {
            fn run(
                &self,
                argv: &[String],
                _cwd: &std::path::Path,
                _env: &[(&str, &str)],
            ) -> io::Result<dx_process::ChildStatus> {
                self.seen.borrow_mut().push(argv.to_vec());
                Ok(dx_process::ChildStatus { code: Some(0) })
            }
        }
        let harness = Harness::new("docs-serve-bind");
        let inv = invocation(&[
            "docs",
            "--serve",
            "--port=8080",
            "--host=example.test",
            "--open",
        ]);
        let seen = Rc::new(RefCell::new(Vec::new()));
        let runner = Probe {
            seen: Rc::clone(&seen),
        };
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
        assert_eq!(code, 0);
        let calls = seen.borrow();
        // Bazel build plus browser open plus preview server.
        assert_eq!(calls.len(), 3, "{calls:?}");
        let serve = calls
            .iter()
            .find(|argv| argv.contains(&"http.server".to_owned()));
        let serve = serve.expect("serve argv");
        assert!(serve.contains(&"8080".to_owned()), "{serve:?}");
        assert!(serve.contains(&"--bind".to_owned()), "{serve:?}");
        assert!(serve.contains(&"example.test".to_owned()), "{serve:?}");
        let open = calls
            .iter()
            .find(|argv| argv.iter().any(|arg| arg.contains("webbrowser")));
        assert!(open.is_some(), "opener must run for --open: {calls:?}");
    }

    #[test]
    fn docs_serve_launch_failure_maps_to_serve_failed() {
        use std::io;
        struct LaunchFailRunner;
        impl dx_process::Runner for LaunchFailRunner {
            fn run(
                &self,
                argv: &[String],
                _cwd: &std::path::Path,
                _env: &[(&str, &str)],
            ) -> io::Result<dx_process::ChildStatus> {
                if argv.first().is_some_and(|first| first == "bazel") {
                    Ok(dx_process::ChildStatus { code: Some(0) })
                } else {
                    Err(io::Error::other("fake bind failure: address in use"))
                }
            }
        }
        let harness = Harness::new("docs-serve-bind-fail");
        let inv = invocation(&["docs", "--serve", "--port=8080", "--output=json"]);
        let runner = LaunchFailRunner;
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
        assert_eq!(code, 1, "launch failure is operational");
        let err_text = String::from_utf8(err).expect("stderr");
        assert!(err_text.contains("serve_failed"), "{err_text}");
        assert!(!err_text.contains("launch_failed"), "{err_text}");
        let text = String::from_utf8(out).expect("out");
        assert!(text.contains("serve_failed"), "{text}");
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
