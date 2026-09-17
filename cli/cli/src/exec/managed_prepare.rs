//! Managed side preparation and commit-error mapping (issue #236).
//!
//! Split from [`super::managed`] (`managed.rs`): owns
//! [`prepare_managed_sides`] plus [`map_commit_error`]. Re-exported
//! through `super` so the public path stays
//! `crate::exec::managed_prepare::{prepare_managed_sides,
//! map_commit_error}` via `pub(crate)`. Shares the codegen/env
//! collection and staging in [`super::managed_codegen`],
//! [`super::managed_env`], and the error codes in [`super::common`];
//! the managed dispatch ([`super::managed::execute_managed`]) calls back
//! in.

use std::path::Path;

use super::common::*;
use super::managed_codegen::{collect_managed_codegen, empty_generated_id, stage_codegen_side};
use super::managed_env::{collect_managed_env, empty_env_id, stage_env_side};
use crate::args::Command;

/// Maps a setup commit failure into the stable managed error vocabulary.
/// Only the capability error carries its own code; every other commit
/// failure preserves the prior pointer and reports `managed_commit_failed`.
pub(crate) fn map_commit_error(error: dx_setup::CommitError) -> (String, String) {
    match error {
        dx_setup::CommitError::NoCapability => (
            CODE_MANAGED_NO_CAPABILITY.to_owned(),
            "selected scope provides neither environment nor codegen capability".to_owned(),
        ),
        other => (CODE_MANAGED_COMMIT_FAILED.to_owned(), other.to_string()),
    }
}

/// Collects, validates, and stages the prepared sides for one managed
/// command without committing: independent commands always prepare
/// their own side (even an empty plan, which clears a stale selection,
/// paired downstream with the managed empty counterpart), while an
/// exact `dx setup` leaves a side with no contributing target
/// unprepared so the commit carries the current generation forward (or
/// the managed empty generation on first selection). Repository setup
/// always prepares both canonical sides. Staged-but-unselected
/// generations are ordinary retained cache, never selection state.
pub(crate) fn prepare_managed_sides(
    command: Command,
    repository: bool,
    workspace: &Path,
    bep: &Path,
) -> Result<dx_setup::PreparedSides, (String, String)> {
    let empties = || -> Result<dx_setup::PreparedSides, (String, String)> {
        Ok(dx_setup::PreparedSides {
            prepared_environment: None,
            prepared_generated: None,
            empty_environment: empty_env_id()?,
            empty_generated: empty_generated_id()?,
        })
    };
    match command {
        Command::Codegen => {
            let (_, plan, projection) = collect_managed_codegen(bep)?;
            let generated = stage_codegen_side(workspace, &plan, &projection)?;
            Ok(dx_setup::PreparedSides {
                prepared_generated: Some(generated),
                ..empties()?
            })
        }
        Command::Env => {
            let (_, plan, projection) = collect_managed_env(bep)?;
            let environment = stage_env_side(workspace, &plan, &projection)?;
            Ok(dx_setup::PreparedSides {
                prepared_environment: Some(environment),
                ..empties()?
            })
        }
        Command::Setup => {
            let (codegen_outputs, codegen_plan, codegen_projection) = collect_managed_codegen(bep)?;
            let (env_outputs, env_plan, env_projection) = collect_managed_env(bep)?;
            let prepared_generated = if repository || !codegen_outputs.is_empty() {
                Some(stage_codegen_side(
                    workspace,
                    &codegen_plan,
                    &codegen_projection,
                )?)
            } else {
                None
            };
            let prepared_environment = if repository || !env_outputs.is_empty() {
                Some(stage_env_side(workspace, &env_plan, &env_projection)?)
            } else {
                None
            };
            Ok(dx_setup::PreparedSides {
                prepared_environment,
                prepared_generated,
                ..empties()?
            })
        }
        // LCOV_EXCL_START - reason: defense-in-depth; execute routes only managed commands here, so this arm is unreachable; retained to fail closed as invalid_result instead of panicking.
        _ => {
            debug_assert!(false, "managed dispatch guards commands");
            Err((
                CODE_INVALID_RESULT.to_owned(),
                "unsupported managed command".to_owned(),
            ))
        } // LCOV_EXCL_STOP - reason: end of unreachable managed-dispatch arm.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn managed_commit_errors_map_to_stable_codes() {
        let (code, message) = map_commit_error(dx_setup::CommitError::NoCapability);
        assert_eq!(code, CODE_MANAGED_NO_CAPABILITY);
        assert!(
            message.contains("neither environment nor codegen"),
            "{message}"
        );
        let (code, _) = map_commit_error(dx_setup::CommitError::WorkspaceRoot {
            path: PathBuf::from("missing"),
        });
        assert_eq!(code, CODE_MANAGED_COMMIT_FAILED);
    }
}
