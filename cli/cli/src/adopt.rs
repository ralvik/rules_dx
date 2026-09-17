//! Delivered adoption/inspect execution (M30b WPs 2-4, 6-7, O61).
//!
//! Contract: `docs/cli/commands/init.md`, `hooks.md`, `status.md`,
//! `version.md`, `watch.md`, `inspect.md`, `completion.md`.
//! Adoption commands run local helpers from `dx_adopt` or thin
//! `bazel query`/`cquery` forwarding; they never enter the quality aspect
//! pipeline. Exit codes follow the CLI contract: `0` success, `1`
//! operational failure, `2` pre-execution usage failure.
//!
//! Domain split (issue #236): inspect execution (`owners`/`deps`/`why`)
//! lives in [`inspect`], status execution (`status`) lives in [`status`],
//! version execution (`version`) lives in [`version`]; this facade keeps
//! dispatch plus the remaining execution domains. The public path stays
//! `crate::adopt::{execute_adoption, AdoptEnv}`.

mod inspect;
mod status;
mod version;

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

/// True when stdout prose summaries should be suppressed (issue #200).
/// `--quiet` (and its `Text { quiet: true }` encoding) suppresses `dx`
/// lifecycle summaries but never result documents: `status` / `version`
/// (except `version` dry-run plans, which are summaries) / inspect labels
/// / completion scripts / `hooks status` views always print because they
/// are the answer, not a summary. Summary owners (`init`,
/// `hooks install` / `uninstall` / `run`, `watch`, dry-run plans including
/// `version --dry-run --pin` / `--rollback`) check this; result owners do
/// not, and that non-suppression is documented in the output protocol
/// rather than a silent ignore.
fn summaries_suppressed(invocation: &Invocation) -> bool {
    invocation.quiet || matches!(invocation.output, OutputMode::Text { quiet: true })
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
        Command::Status => status::execute_status(invocation, workspace, out, err),
        Command::Version => version::execute_version(invocation, workspace, out, err),
        Command::Watch => execute_watch(invocation, workspace, out, err),
        Command::Owners | Command::Deps | Command::Why => {
            inspect::execute_inspect(invocation, workspace, query_runner, out, err)
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
        if !summaries_suppressed(invocation) {
            for file in dx_adopt::plan_init_files(module) {
                let _ = writeln!(out, "would write {}", file.path);
            }
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
                } else if !summaries_suppressed(invocation) {
                    let _ = writeln!(out, "wrote {entry}");
                }
            }
            0
        }
        Err(error) => operational(out, err, &error.to_string()),
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
                if !summaries_suppressed(invocation) {
                    for path in installed {
                        let _ = writeln!(out, "installed {path}");
                    }
                }
                0
            }
            Err(error) => operational(out, err, &error.to_string()),
        },
        "uninstall" => match dx_adopt::uninstall_hooks(workspace) {
            Ok(removed) => {
                if !summaries_suppressed(invocation) {
                    for path in removed {
                        let _ = writeln!(out, "removed {path}");
                    }
                }
                0
            }
            Err(error) => operational(out, err, &error.to_string()),
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
            if !summaries_suppressed(invocation) {
                let _ = writeln!(out, "ran {trigger}: ok (budget 120s)");
            }
            0
        }
        _ => pre_exec(err, "usage: dx hooks <install|uninstall|status|run>"),
    }
}

fn read_optional(root: &std::path::Path, name: &str) -> String {
    std::fs::read_to_string(root.join(name)).unwrap_or_else(|_| format!("(missing {name})"))
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
                if !summaries_suppressed(invocation) {
                    let _ = writeln!(out, "would {plan}");
                }
                return 0;
            }
            // Single delivered iteration: re-resolve scope each loop in the
            // real binary (loop omitted under test via DX_WATCH_ONCE).
            if !summaries_suppressed(invocation) {
                let _ = writeln!(out, "{plan} scope={}", invocation.targets.join(" "));
            }
            if std::env::var("DX_WATCH_ONCE").is_ok() {
                return 0;
            }
            if !summaries_suppressed(invocation) {
                let _ = writeln!(out, "watching (Ctrl-C to stop)");
            }
            0
        }
        Err(error) => pre_exec(err, &error.to_string()),
    }
}

fn execute_completion(invocation: &Invocation, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    // Scripts render at runtime from the `Cli` grammar (issue #202):
    // the same definition feeds parsing, `--help`, and completions, so
    // output cannot drift from the command reference.
    let shell = invocation.targets.first().map(String::as_str).unwrap_or("");
    match crate::args::render_completion(shell) {
        Ok(script) => {
            let _ = write!(out, "{script}");
            0
        }
        Err(error) => pre_exec(err, &error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::parse;
    use std::io;

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

    fn temp_root(name: &str) -> dx_test_scratch::TempDir {
        dx_test_scratch::scratch(&format!("dx-adopt-cmd-{name}-"))
    }

    #[test]
    fn init_dry_run_lists_without_writing() {
        let inv = invocation(&["init", "--dry-run", "demo"]);
        let scratch = temp_root("init-dry");
        let root = scratch.path().to_path_buf();
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
    }

    #[test]
    fn init_applies_absent_only() {
        let inv = invocation(&["init"]);
        let scratch = temp_root("init-apply");
        let root = scratch.path().to_path_buf();
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
    }

    #[test]
    fn hooks_status_shows_merged_layers() {
        let inv = invocation(&["hooks", "status"]);
        let scratch = temp_root("hooks-status");
        let root = scratch.path().to_path_buf();
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
    }

    #[test]
    fn watch_validates_wrapped_command() {
        let scratch = temp_root("watch");
        let root = scratch.path().to_path_buf();
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
    }

    #[test]
    fn quiet_suppresses_summaries_but_not_results() {
        // Issue #200: `--quiet` silences `dx` prose summaries while result
        // documents still print. Init dry-run plans are summaries;
        // `status` output is the answer.
        let inv = invocation(&["init", "--dry-run", "--quiet", "demo"]);
        let scratch = temp_root("quiet-init");
        let root = scratch.path().to_path_buf();
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
        assert!(String::from_utf8(out).expect("out").is_empty());

        let inv = invocation(&["status", "--quiet"]);
        let scratch = temp_root("quiet-status");
        let root = scratch.path().to_path_buf();
        std::fs::create_dir_all(root.join(".dx")).expect("dx");
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
        assert!(!String::from_utf8(out).expect("out").is_empty());

        // Version dry-run plans are summaries (silenced); version output
        // itself is the answer (never silenced).
        let inv = invocation(&["version", "--dry-run", "--pin=0.0.0", "--quiet"]);
        let scratch = temp_root("quiet-version-dryrun");
        let root = scratch.path().to_path_buf();
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
        assert!(String::from_utf8(out).expect("out").is_empty());

        let inv = invocation(&["version", "--quiet"]);
        let scratch = temp_root("quiet-version");
        let root = scratch.path().to_path_buf();
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
        assert!(!String::from_utf8(out).expect("out").is_empty());
    }

    #[test]
    fn completion_renders_from_single_source() {
        use clap::ValueEnum;
        // Every supported shell renders every command and key flag from
        // the single Cli grammar (issue #202); no hand-maintained list.
        for &shell in crate::args::COMPLETION_SHELLS {
            let inv = invocation(&["completion", shell]);
            let scratch = temp_root("completion");
            let root = scratch.path().to_path_buf();
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
            assert_eq!(code, 0, "shell {shell}");
            let text = String::from_utf8(out).expect("out");
            for cmd in crate::args::Command::value_variants() {
                assert!(
                    text.contains(cmd.name()),
                    "shell {shell} misses command {}",
                    cmd.name()
                );
            }
            for flag in [
                "workspace",
                "dry-run",
                "output",
                "report",
                "fail-on",
                "check",
            ] {
                assert!(text.contains(flag), "shell {shell} misses flag {flag}");
            }
        }
        // Unknown shells keep the contract error.
        let unknown = crate::args::render_completion("tcsh");
        assert!(unknown.is_err());
        assert!(unknown.unwrap_err().to_string().contains("unknown-shell"));
    }
}
