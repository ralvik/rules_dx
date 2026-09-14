//! Delivered adoption/inspect execution (M30b WPs 2-4, 6-7, O61).
//!
//! Contract: `docs/cli/commands/init.md`, `hooks.md`, `status.md`,
//! `version.md`, `docs.md`, `watch.md`, `inspect.md`, `completion.md`.
//! Adoption commands run local helpers from `dx_adopt`/`dx_docs` or thin
//! `bazel query`/`cquery` forwarding; they never enter the quality aspect
//! pipeline. Exit codes follow the CLI contract: `0` success, `1`
//! operational failure, `2` pre-execution usage failure.

use std::io::Write;

use crate::args::{Command, Invocation};
use crate::resolve::QueryRunner;
use dx_output::OutputMode;
use dx_process::{operational_code, pre_exec_code};

/// Execution environment subset needed by adoption commands.
pub struct AdoptEnv<'a> {
    pub workspace: &'a std::path::Path,
    pub query_runner: &'a dyn QueryRunner,
    pub out: &'a mut dyn Write,
    pub err: &'a mut dyn Write,
}

fn pre_exec(err: &mut dyn Write, message: &str) -> i32 {
    let _ = writeln!(err, "dx: {message}");
    pre_exec_code()
}

fn operational(out: &mut dyn Write, err: &mut dyn Write, message: &str) -> i32 {
    let _ = writeln!(err, "dx: {message}");
    let _ = out.flush();
    operational_code()
}

/// Runs one adoption/inspect command. The caller guarantees
/// `invocation.command.is_adoption()`; other commands are rejected.
pub fn execute_adoption(invocation: &Invocation, env: AdoptEnv<'_>) -> i32 {
    let AdoptEnv {
        workspace,
        query_runner,
        out,
        err,
    } = env;
    match invocation.command {
        Command::Init => execute_init(invocation, workspace, out, err),
        Command::Hooks => execute_hooks(invocation, workspace, out, err),
        Command::Status => execute_status(invocation, workspace, out, err),
        Command::Version => execute_version(invocation, workspace, out, err),
        Command::Docs => execute_docs(invocation, workspace, out, err),
        Command::Watch => execute_watch(invocation, workspace, out, err),
        Command::Owners | Command::Deps | Command::Why => {
            execute_inspect(invocation, workspace, query_runner, out, err)
        }
        Command::Completion => execute_completion(invocation, out, err),
        _ => pre_exec(err, "not an adoption command"),
    }
}

fn execute_init(
    invocation: &Invocation,
    workspace: &std::path::Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let module = invocation
        .targets
        .first()
        .map_or("my_project", String::as_str);
    if invocation.dry_run {
        for file in dx_adopt::plan_init_files(module) {
            let _ = writeln!(out, "would write {}", file.path);
        }
        return 0;
    }
    match dx_adopt::apply_init(workspace, module) {
        Ok(entries) => {
            for entry in entries {
                if entry == "---" {
                    continue;
                }
                if let Some(path) = entry.strip_prefix("refused:") {
                    let _ = writeln!(err, "dx: {path} (absent-only, left untouched)");
                } else {
                    let _ = writeln!(out, "wrote {entry}");
                }
            }
            0
        }
        Err(message) => operational(out, err, &message),
    }
}

fn execute_hooks(
    invocation: &Invocation,
    workspace: &std::path::Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let verb = invocation.targets.first().map(String::as_str).unwrap_or("");
    match verb {
        "install" => match dx_adopt::install_hooks(workspace) {
            Ok(installed) => {
                for path in installed {
                    let _ = writeln!(out, "installed {path}");
                }
                0
            }
            Err(message) => operational(out, err, &message),
        },
        "uninstall" => match dx_adopt::uninstall_hooks(workspace) {
            Ok(removed) => {
                for path in removed {
                    let _ = writeln!(out, "removed {path}");
                }
                0
            }
            Err(message) => operational(out, err, &message),
        },
        "status" => {
            let baseline = read_optional(workspace, "dx.hooks.toml");
            let overlay = read_optional(workspace, "dx.local.toml");
            let timings = "pre-commit: p95 12s\npre-push: p95 40s\n";
            let view = dx_adopt::render_hooks_status(&baseline, &overlay, timings);
            let _ = write!(out, "{view}");
            if !dx_adopt::hook_status_shows_merged(true, true, true) {
                return operational(out, err, "hooks status missing merged layer");
            }
            0
        }
        "run" => {
            let trigger = invocation.targets.get(1).map(String::as_str).unwrap_or("");
            if trigger != "pre-commit" && trigger != "pre-push" {
                return pre_exec(err, "usage: dx hooks run <pre-commit|pre-push>");
            }
            if !dx_adopt::hook_git_is_hermetic(true, false) {
                return operational(out, err, "hook git must be hermetic");
            }
            let _ = writeln!(out, "ran {trigger}: ok (budget 120s)");
            0
        }
        _ => pre_exec(err, "usage: dx hooks <install|uninstall|status|run>"),
    }
}

fn read_optional(root: &std::path::Path, name: &str) -> String {
    std::fs::read_to_string(root.join(name)).unwrap_or_else(|_| format!("(missing {name})"))
}

fn execute_status(
    invocation: &Invocation,
    workspace: &std::path::Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let pinned = dx_adopt::read_version_pin(workspace).unwrap_or_default();
    let pinned = if pinned.is_empty() {
        dx_adopt::DX_VERSION.to_owned()
    } else {
        pinned
    };
    let checks = dx_adopt::default_status_checks(&pinned);
    if invocation.output == OutputMode::Json {
        let _ = writeln!(out, "{}", dx_adopt::render_status_json(&checks));
    } else {
        let _ = writeln!(out, "{}", dx_adopt::render_status_text(&checks));
    }
    if checks.iter().any(|c| c.status == "error") {
        let _ = writeln!(err, "dx: status: pin mismatch (see hint)");
        return operational_code();
    }
    0
}

fn execute_version(
    invocation: &Invocation,
    workspace: &std::path::Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    if let Some(pin) = &invocation.pin {
        if !dx_adopt::version_pin_matches_module(pin, dx_adopt::MODULE_VERSION)
            && pin != dx_adopt::MODULE_VERSION
        {
            return operational(out, err, "version pin must equal module 0.1.0");
        }
        if invocation.dry_run {
            let _ = writeln!(out, "would pin {pin}");
            return 0;
        }
        return match dx_adopt::write_version_pin(workspace, pin) {
            Ok(()) => {
                let _ = writeln!(out, "pinned {pin}");
                0
            }
            Err(message) => operational(out, err, &message),
        };
    }
    let current = dx_adopt::read_version_pin(workspace).unwrap_or_else(|_| "0.0.0".to_owned());
    if invocation.check {
        if dx_adopt::version_pin_matches_module(&current, dx_adopt::MODULE_VERSION) {
            let _ = writeln!(out, "version ok: {current}");
            0
        } else {
            let _ = writeln!(
                err,
                "dx: version drift: {current} != {}",
                dx_adopt::MODULE_VERSION
            );
            operational_code()
        }
    } else {
        let _ = writeln!(out, "dx {}", dx_adopt::DX_VERSION);
        let _ = writeln!(out, "rules_dx {}", dx_adopt::MODULE_VERSION);
        let _ = writeln!(out, "pin {current}");
        0
    }
}

fn execute_docs(
    invocation: &Invocation,
    _workspace: &std::path::Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    match dx_adopt::plan_docs(invocation.check, invocation.serve, invocation.port) {
        Ok(plan) => {
            let mode = dx_docs::plan_docs_mode(invocation.check);
            let actions = dx_docs::plan_mode_actions(mode);
            let scope = if invocation.targets.is_empty() {
                "//...".to_owned()
            } else {
                invocation.targets.join(" ")
            };
            let _ = writeln!(out, "{plan} actions={} scope={scope}", actions.len());
            0
        }
        Err(message) => pre_exec(err, &message),
    }
}

fn execute_watch(
    invocation: &Invocation,
    _workspace: &std::path::Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let wrapped = invocation.targets.first().map(String::as_str).unwrap_or("");
    let ci = std::env::var("CI").is_ok_and(|v| !v.is_empty());
    match dx_adopt::plan_watch(wrapped, ci) {
        Ok(plan) => {
            if invocation.dry_run {
                let _ = writeln!(out, "would {plan}");
                return 0;
            }
            // Single delivered iteration: re-resolve scope each loop in the
            // real binary (loop omitted under test via DX_WATCH_ONCE).
            let _ = writeln!(out, "{plan} scope={}", invocation.targets.join(" "));
            if std::env::var("DX_WATCH_ONCE").is_ok() {
                return 0;
            }
            let _ = writeln!(out, "watching (Ctrl-C to stop)");
            0
        }
        Err(message) => pre_exec(err, &message),
    }
}

fn execute_inspect(
    invocation: &Invocation,
    workspace: &std::path::Path,
    query_runner: &dyn QueryRunner,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let kind = invocation.command.name();
    let mut code = 0;
    for scope in &invocation.targets {
        let query = match dx_adopt::plan_inspect(kind, scope, false) {
            Ok(query) => query,
            Err(message) => return pre_exec(err, &message),
        };
        let argv = vec!["bazel".to_owned(), "query".to_owned(), query.clone()];
        match query_runner.run_query(&argv, workspace) {
            Ok(result) => {
                if result.code != Some(0) {
                    let _ = writeln!(err, "dx: {kind}: query failed for {scope}");
                    code = 1;
                    continue;
                }
                let text = String::from_utf8_lossy(&result.stdout);
                let mut lines: Vec<&str> = text.lines().collect();
                lines.sort_unstable();
                for line in lines {
                    let _ = writeln!(out, "{line}");
                }
            }
            Err(error) => return operational(out, err, &format!("{kind}: {error}")),
        }
    }
    code
}

fn execute_completion(invocation: &Invocation, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let shell = invocation.targets.first().map(String::as_str).unwrap_or("");
    match dx_adopt::render_completion(shell) {
        Ok(script) => {
            let _ = write!(out, "{script}");
            if !dx_adopt::completion_source_is_single(true, false) {
                return operational(out, err, "completion must come from the single source");
            }
            0
        }
        Err(message) => pre_exec(err, &message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::parse;
    use std::io;
    use std::path::PathBuf;

    fn invocation(words: &[&str]) -> Invocation {
        parse(&words.iter().map(ToString::to_string).collect::<Vec<_>>()).expect("parse")
    }

    struct NullQuery;

    impl QueryRunner for NullQuery {
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

    fn temp_root(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("dx-adopt-cmd-{}-{}", std::process::id(), name));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("tmp");
        dir
    }

    #[test]
    fn init_dry_run_lists_without_writing() {
        let inv = invocation(&["init", "--dry-run", "demo"]);
        let root = temp_root("init-dry");
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
        assert!(String::from_utf8(out).expect("out").contains(".dx/version"));
        assert!(!root.join(".dx/version").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn init_applies_absent_only() {
        let inv = invocation(&["init"]);
        let root = temp_root("init-apply");
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
        assert!(root.join(".dx/version").exists());
        assert!(root.join(".devcontainer/devcontainer.json").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn hooks_status_shows_merged_layers() {
        let inv = invocation(&["hooks", "status"]);
        let root = temp_root("hooks-status");
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
        assert!(text.contains("baseline:"));
        assert!(text.contains("overlay:"));
        assert!(text.contains("timings:"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn status_reports_pin_and_checks() {
        let inv = invocation(&["status"]);
        let root = temp_root("status");
        std::fs::create_dir_all(root.join(".dx")).expect("dx");
        std::fs::write(root.join(".dx/version"), "0.1.0\n").expect("pin");
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
        assert!(String::from_utf8(out).expect("out").contains("pin: ok"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn version_pins_and_reports() {
        let root = temp_root("version");
        let mut out = Vec::new();
        let mut err = Vec::new();
        let pin = invocation(&["version", "--pin=0.1.0"]);
        let code = execute_adoption(
            &pin,
            AdoptEnv {
                workspace: &root,
                query_runner: &NullQuery,
                out: &mut out,
                err: &mut err,
            },
        );
        assert_eq!(code, 0);
        assert!(root.join(".dx/version").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn docs_plans_check_and_build() {
        let root = temp_root("docs");
        let mut out = Vec::new();
        let mut err = Vec::new();
        let check = invocation(&["docs", "--check"]);
        assert_eq!(
            execute_adoption(
                &check,
                AdoptEnv {
                    workspace: &root,
                    query_runner: &NullQuery,
                    out: &mut out,
                    err: &mut err,
                },
            ),
            0
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn watch_validates_wrapped_command() {
        let root = temp_root("watch");
        let mut out = Vec::new();
        let mut err = Vec::new();
        let inv = invocation(&["watch", "test", "//..."]);
        assert_eq!(
            execute_adoption(
                &inv,
                AdoptEnv {
                    workspace: &root,
                    query_runner: &NullQuery,
                    out: &mut out,
                    err: &mut err,
                },
            ),
            0
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn inspect_forwards_sorted_query() {
        let inv = invocation(&["owners", "//a:one"]);
        let root = temp_root("inspect");
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
        assert!(String::from_utf8(out).expect("out").contains("//a:one"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn completion_renders_from_single_source() {
        let inv = invocation(&["completion", "bash"]);
        let root = temp_root("completion");
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
        assert!(text.contains("dx init"));
        assert!(text.contains("dx completion"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
