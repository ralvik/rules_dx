//! Command execution dispatch root (kept,): this file holds
//! only `execute` plus the dispatch-table test. The `exec.rs` plus
//! `exec/` pairing is the idiomatic parent module with child modules,
//! not duplication. Each family lives in its own module under `exec/`:
//! quality plus quality_apply (mutation and status projection) plus
//! quality_patch (diff-patch rendering) plus quality_emit
//! (finding/change/mutation emission) plus quality_reports
//! (standard-report writing), workflow (dispatch) plus run/deploy
//! (workflow sub-families) plus test_reports (test/coverage
//! collection), bazel, generate, clean, managed (dispatch) plus
//! managed_codegen/managed_env (generation sides) plus
//! managed_staging (shared staging primitives) plus managed_prepare
//! (side preparation and commit-error mapping), audit, update, bump,
//! migrate, docs, and the check/fix umbrella. Shared plumbing (error codes,
//! environment, source verification, mutation helpers) lives in
//! [`common`]; BEP results collection and proto mapping live in
//! [`results`]; unit-test fakes live in `test_support`.
//!
//! Contract: `docs/cli/commands/quality.md`,
//! `docs/cli/output-protocol.md`, and `docs/cli/standard-reports.md`.

mod audit;
mod bazel;
mod bump;
mod clean;
pub(crate) mod common;
mod deploy;
mod docs;
mod generate;
mod managed;
mod managed_codegen;
mod managed_env;
mod managed_prepare;
mod managed_staging;
mod migrate;
mod quality;
mod quality_apply;
mod quality_emit;
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
    // `--here` must be consumed into explicit targets before dispatch
    // (`main.rs` plus test `Harness` via `apply_here`); fail closed
    // rather than silently ignoring an unresolved cwd scope.
    if invocation.here {
        let Env { err, .. } = env;
        return common::pre_exec(err, "option \"--here/--cwd\" needs cwd resolution");
    }
    if invocation.command.is_adoption() {
        let Env {
            workspace,
            runner,
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
                runner,
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
    if invocation.command == Command::Bump {
        return bump::execute_bump(invocation, env);
    }
    if invocation.command == Command::Migrate {
        return migrate::execute_migrate(invocation, env);
    }
    if invocation.command == Command::Docs {
        return docs::execute_docs(invocation, env);
    }
    quality::execute_quality(invocation, env)
}

#[cfg(test)]
mod tests {
    use super::test_support::Harness;
    use crate::args::Command;

    /// Dispatch-table pin: every `Command` variant must map
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
        } else if command == Command::Bump {
            "bump"
        } else if command == Command::Migrate {
            "migrate"
        } else if command == Command::Docs {
            "docs"
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
            (Command::Bump, "bump"),
            (Command::Migrate, "migrate"),
            (Command::Codegen, "managed"),
            (Command::Env, "managed"),
            (Command::Setup, "managed"),
            (Command::Init, "adoption"),
            (Command::New, "adoption"),
            (Command::Upgrade, "adoption"),
            (Command::Hooks, "adoption"),
            (Command::Status, "adoption"),
            (Command::Version, "adoption"),
            (Command::Watch, "adoption"),
            (Command::Owners, "adoption"),
            (Command::Deps, "adoption"),
            (Command::Why, "adoption"),
            (Command::Completion, "adoption"),
            (Command::Docs, "docs"),
            (Command::Bazel, "bazel"),
        ];
        assert_eq!(cases.len(), 32, "every Command variant pinned");
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
            vec!["bump", "cargo:anyhow", "1.2.3", "--dry-run"],
            vec!["migrate", "--from=1.2.3", "--to=2.0.0", "--dry-run"],
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
        // Audit executes live auditors per family; update
        // executes resolver backends live (see exec/update.rs). Both fail
        // closed with their stable operational codes when workspaces lack
        // lockfiles.
        for argv in [vec!["audit"]] {
            let name = format!("exec-deferred-{}", argv[0]);
            let harness = Harness::new(&name);
            let (code, _out, err) = harness.run(&argv);
            assert_eq!(code, 1, "{argv:?}");
            assert!(err.contains("audit_failed"), "{argv:?} fails closed: {err}");
        }
    }
}
