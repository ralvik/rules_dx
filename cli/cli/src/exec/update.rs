//! Update command execution: planning is live, live execution stays deferred.

use super::common::*;
use crate::args::{Command, Invocation};
use crate::reports::plan_reports;
use dx_output::{command_finished, command_started, write_event, FinishedCounts, OutputMode};

/// Runs `dx update` planning (M26 WP2 slice 1): dependency-set and
/// package selectors through `dx_update`, mutating without
/// confirmation. `--dry-run` prints the planned selection and exits
/// `0`; live execution fails closed with `update_deferred` because
/// resolver backends and per-set reporting land in later M26 slices
/// (O12).
pub(crate) fn execute_update(invocation: &Invocation, env: Env<'_>) -> i32 {
    debug_assert!(
        invocation.command == Command::Update,
        "update dispatch guards commands"
    );
    let Env { out, err, .. } = env;
    match plan_reports(
        invocation.command,
        &invocation.reports,
        &invocation.output,
        invocation.dry_run,
    ) {
        Ok(_) => {}
        Err(error) => return pre_exec(err, &error.to_string()),
    }
    let request = dx_update::UpdateRequest::plan(&invocation.targets);
    let summary = match request.selection() {
        dx_update::UpdateSelection::AllSets => "Running update for all dependency sets".to_owned(),
        dx_update::UpdateSelection::Selected(selectors) => {
            format!("Running update for {}", selectors.join(", "))
        }
    };
    let verbose =
        matches!(invocation.output, OutputMode::Text { quiet: false }) && !invocation.quiet;
    if invocation.dry_run {
        if invocation.output == OutputMode::Json {
            if let Ok(event) = command_started(invocation.command.name(), true, "default") {
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
    operational(
        invocation,
        out,
        err,
        CODE_UPDATE_DEFERRED,
        "update resolver execution is deferred: backend operation and per-set reporting land in later M26 slices (O12); use --dry-run for planning",
    )
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;

    #[test]
    fn update_dry_run_plans_selection_without_launching() {
        let harness = Harness::new("update-dryrun");
        let (code, out, err) = harness.run(&["update", "--dry-run"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(
            out.contains("Running update for all dependency sets"),
            "{out}"
        );
        assert_eq!(err, "", "{err}");
        assert!(
            harness.seen_env.borrow().is_empty(),
            "dry-run launches nothing"
        );

        let harness = Harness::new("update-dryrun-selected");
        let (code, out, err) = harness.run(&["update", "crates", "--dry-run"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("Running update for crates"), "{out}");
        assert_eq!(err, "", "{err}");
        assert!(
            harness.seen_env.borrow().is_empty(),
            "dry-run launches nothing"
        );
    }

    #[test]
    fn update_live_is_deferred_operational() {
        let harness = Harness::new("update-live");
        let (code, out, err) = harness.run(&["update"]);
        assert_eq!(code, 1, "{out}{err}");
        assert!(err.contains("update_deferred"), "{err}");
        assert!(
            harness.seen_env.borrow().is_empty(),
            "deferred run launches nothing"
        );
    }
}
