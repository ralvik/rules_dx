//! Delivered adoption/inspect execution (WPs 2-4, 6-7).
//!
//! Contract: `docs/cli/commands/init.md`, `hooks.md`, `status.md`,
//! `version.md`, `watch.md`, `inspect.md`, `completion.md`.
//! Adoption commands run local helpers from `dx_adopt` or thin
//! `bazel query`/`cquery` forwarding; they never enter the quality aspect
//! pipeline. Exit codes follow the CLI contract: `0` success, `1`
//! operational failure, `2` pre-execution usage failure.
//!
//! Domain split: each execution domain lives in its own
//! module — [`inspect`] (`owners`/`deps`/`why`), [`status`], [`version`],
//! [`watch`], [`completion`], [`init`], [`hooks`]; this facade keeps
//! dispatch plus shared helpers. The public path stays
//! `crate::adopt::{execute_adoption, AdoptEnv}`.

mod completion;
mod hooks;
mod init;
mod inspect;
mod status;
mod version;
mod watch;

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

/// True when stdout prose summaries should be suppressed.
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
        Command::Init => init::execute_init(invocation, workspace, out, err),
        Command::Hooks => hooks::execute_hooks(invocation, workspace, out, err),
        Command::Status => status::execute_status(invocation, workspace, out, err),
        Command::Version => version::execute_version(invocation, workspace, out, err),
        Command::Watch => watch::execute_watch(invocation, workspace, out, err),
        Command::Owners | Command::Deps | Command::Why => {
            inspect::execute_inspect(invocation, workspace, query_runner, out, err)
        }
        Command::Completion => completion::execute_completion(invocation, out, err),
        _ => pre_exec(err, "not an adoption command"),
    }
}

fn read_optional(root: &std::path::Path, name: &str) -> String {
    std::fs::read_to_string(root.join(name)).unwrap_or_else(|_| format!("(missing {name})"))
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
    fn quiet_suppresses_summaries_but_not_results() {
        // `--quiet` silences `dx` prose summaries while result
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
}
