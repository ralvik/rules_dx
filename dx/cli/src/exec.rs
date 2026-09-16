//! Quality command execution (M07 WP3): dispatch root across command
//! families. Each family lives in its own module under `exec/`:
//! quality, workflow, bazel, generate, clean, managed, audit, update,
//! run, and the check/fix umbrella. Shared plumbing (error codes,
//! environment, BEP collection, mutation helpers) lives in [`common`];
//! unit-test fakes live in `test_support`.
//!
//! Contract: `docs/cli/commands/quality.md`,
//! `docs/cli/output-protocol.md`, and `docs/cli/standard-reports.md`.

mod audit;
mod bazel;
mod clean;
mod common;
mod generate;
mod managed;
mod quality;
mod run;
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
