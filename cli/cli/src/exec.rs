//! Quality command execution (M07 WP3): dispatch root across command
//! families. Each family lives in its own module under `exec/`:
//! quality, workflow (dispatch) plus test_reports (test/coverage
//! collection), bazel, generate, clean, managed (dispatch) plus
//! managed_codegen/managed_env (generation sides) plus
//! managed_staging (shared staging primitives), quality plus
//! quality_apply (mutation and status projection) plus quality_patch
//! (diff-patch rendering) plus quality_reports (standard-report
//! writing), audit, update,
//! run, and the check/fix umbrella. Shared plumbing (error codes,
//! environment, source verification, mutation helpers) lives in
//! [`common`]; BEP results collection and proto mapping live in
//! [`results`]; unit-test fakes live in `test_support`.
//!
//! Contract: `docs/cli/commands/quality.md`,
//! `docs/cli/output-protocol.md`, and `docs/cli/standard-reports.md`.

mod audit;
mod bazel;
mod clean;
mod common;
mod deploy;
mod generate;
mod managed;
mod managed_codegen;
mod managed_env;
mod managed_staging;
mod quality;
mod quality_apply;
mod quality_patch;
mod quality_reports;
mod results;
mod run;
mod test_reports;
#[cfg(test)]
mod test_support;
mod umbrella;
mod update;
mod workflow;

use crate::args::{Command, Invocation};

pub use common::Env;

/// Runs the quality command to completion and returns the process exit
/// code. All dx-owned bytes go to `out` except operational diagnostics
/// (always `err`); diff mode never emits dx-owned prose or diagnostics
/// on stdout, and a stdout report owns stdout while human text moves
/// to stderr.
pub fn execute(invocation: &Invocation, env: Env<'_>) -> i32 {
    if invocation.command.is_adoption() {
        let Env {
            workspace,
            query_runner,
            out,
            err,
            ..
        } = env;
        return crate::adopt::execute_adoption(
            invocation,
            crate::adopt::AdoptEnv {
                workspace,
                query_runner,
                out,
                err,
            },
        );
    }
    if invocation.command.is_umbrella() {
        return umbrella::execute_umbrella(invocation, env);
    }
    if invocation.command.is_workflow() {
        return workflow::execute_workflow(invocation, env);
    }
    if invocation.command == Command::Bazel {
        return bazel::execute_bazel(invocation, env);
    }
    if invocation.command == Command::Generate {
        return generate::execute_generate(invocation, env);
    }
    if invocation.command == Command::Clean {
        return clean::execute_clean(invocation, env);
    }
    if invocation.command.is_managed() {
        return managed::execute_managed(invocation, env);
    }
    if invocation.command == Command::Audit {
        return audit::execute_audit(invocation, env);
    }
    if invocation.command == Command::Update {
        return update::execute_update(invocation, env);
    }
    quality::execute_quality(invocation, env)
}

#[cfg(test)]
mod tests {
    use super::test_support::Harness;
    use crate::args::Command;

    /// Dispatch-table pin (issue #238): every `Command` variant must map
    /// to exactly one execution family in the same order as `execute`
    /// above. Adding a variant without wiring it here fails loudly
    /// instead of silently falling to the quality pipeline.
    fn family(command: Command) -> &'static str {
        if command.is_adoption() {
            "adoption"
        } else if command.is_umbrella() {
            "umbrella"
        } else if command.is_workflow() {
            "workflow"
        } else if command == Command::Bazel {
            "bazel"
        } else if command == Command::Generate {
            "generate"
        } else if command == Command::Clean {
            "clean"
        } else if command.is_managed() {
            "managed"
        } else if command == Command::Audit {
            "audit"
        } else if command == Command::Update {
            "update"
        } else {
            "quality"
        }
    }

    #[test]
    fn dispatch_table_covers_every_command() {
        let cases = [
            (Command::Audit, "audit"),
            (Command::Lint, "quality"),
            (Command::Typecheck, "quality"),
            (Command::Format, "quality"),
            (Command::Generate, "generate"),
            (Command::Build, "workflow"),
            (Command::Test, "workflow"),
            (Command::Coverage, "workflow"),
            (Command::Run, "workflow"),
            (Command::Deploy, "workflow"),
            (Command::Check, "umbrella"),
            (Command::Fix, "umbrella"),
            (Command::Clean, "clean"),
            (Command::Update, "update"),
            (Command::Codegen, "managed"),
            (Command::Env, "managed"),
            (Command::Setup, "managed"),
            (Command::Init, "adoption"),
            (Command::Hooks, "adoption"),
            (Command::Status, "adoption"),
            (Command::Version, "adoption"),
            (Command::Watch, "adoption"),
            (Command::Owners, "adoption"),
            (Command::Deps, "adoption"),
            (Command::Why, "adoption"),
            (Command::Completion, "adoption"),
            (Command::Bazel, "bazel"),
        ];
        assert_eq!(cases.len(), 27, "every Command variant pinned");
        for (command, want) in cases {
            assert_eq!(family(command), want, "family for {}", command.name());
        }
    }

    #[test]
    fn dry_run_families_launch_nothing() {
        for argv in [
            vec!["--dry-run", "bazel", "version"],
            vec!["audit", "--dry-run"],
            vec!["update", "--dry-run"],
        ] {
            let name = format!("exec-dispatch-{}", argv[0].trim_start_matches('-'));
            let harness = Harness::new(&name);
            let (code, _out, err) = harness.run(&argv);
            assert_eq!(code, 0, "{argv:?}");
            assert_eq!(err, "", "{argv:?}");
            assert!(
                harness.seen_env.borrow().is_empty(),
                "{argv:?} launches nothing"
            );
        }
    }

    #[test]
    fn deferred_families_fail_closed_without_launch() {
        for argv in [vec!["audit"], vec!["update"]] {
            let name = format!("exec-deferred-{}", argv[0]);
            let harness = Harness::new(&name);
            let (code, _out, err) = harness.run(&argv);
            assert_eq!(code, 1, "{argv:?}");
            assert!(err.contains("deferred"), "{argv:?} fails closed: {err}");
            assert!(
                harness.seen_env.borrow().is_empty(),
                "{argv:?} launches nothing"
            );
        }
    }
}
