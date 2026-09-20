//! Managed-state cleanup (`dx clean`) execution.

use super::common::*;
use crate::args::Invocation;
use dx_clean::{
    apply_plan, bazel_forward_argv, collect_inventory_with_scan, measure_prune_bytes,
    render_dry_run, RECOVERY_GUIDANCE,
};
use dx_output::OutputMode;

/// Runs `dx clean [--dry-run] [--bazel]`: collects the
/// workspace managed-state inventory with the process scan (live shells
/// or actions holding `.dx` paths pin their hexes as active), plans the
/// prune set over validated unselected records and generations, and
/// either renders the `--dry-run` listing with reclaimable bytes
/// (deleting nothing, holding no lock) or applies the prune set under
/// the shared commit lock. An explicit `--bazel` additionally forwards
/// exactly `bazel clean` after pruning and prints the dangling-link
/// recovery guidance; under `--dry-run` the forward is listed, never
/// run. Argument parsing guarantees text output with no scopes,
/// reports, or quality options on this path.
///
/// Exits `0` on success (including an empty prune set), `1` on
/// inventory, lock, or prune failures, and propagates the Bazel exit
/// code for the explicit forward.
pub(crate) fn execute_clean(invocation: &Invocation, env: Env<'_>) -> i32 {
    let Env {
        workspace,
        runner,
        out,
        err,
        ..
    } = env;
    let inventory = match collect_inventory_with_scan(workspace) {
        Ok(inventory) => inventory,
        Err(error) => {
            return operational(invocation, out, err, CODE_CLEAN_FAILED, &error.to_string());
        }
    };
    let plan = inventory.plan();
    // Reclaimable bytes measure the planned prune set before any lock or
    // deletion: symlinks count, targets never do, and vanished entries
    // measure zero so measure and idempotent apply agree.
    let bytes = match measure_prune_bytes(workspace, &plan) {
        Ok(bytes) => bytes,
        Err(error) => {
            return operational(invocation, out, err, CODE_CLEAN_FAILED, &error.to_string());
        }
    };
    // Human prose is the only output on this path: `--dry-run` and the
    // prune summary print in text mode unless `--quiet` suppresses them.
    let verbose =
        matches!(invocation.output, OutputMode::Text { quiet: false }) && !invocation.quiet;
    if invocation.dry_run {
        if verbose {
            let _ = writeln!(out, "{}", render_dry_run(&plan, &bytes));
            if invocation.bazel_clean {
                let _ = writeln!(out, "would forward: bazel clean");
            }
        }
        return 0;
    }
    let outcome = match apply_plan(workspace, &plan) {
        Ok(outcome) => outcome,
        Err(error) => {
            return operational(invocation, out, err, CODE_CLEAN_FAILED, &error.to_string());
        }
    };
    if verbose {
        if outcome.removed_setup_records.is_empty() && outcome.removed_generations.is_empty() {
            let _ = writeln!(out, "dx clean: nothing to prune");
        } else {
            let _ = writeln!(
                out,
                "dx clean: pruned {} setup records and {} generations ({} bytes reclaimed)",
                outcome.removed_setup_records.len(),
                outcome.removed_generations.len(),
                bytes.reclaimed(&outcome)
            );
        }
    }
    if !invocation.bazel_clean {
        return 0;
    }
    // The explicit forward is exactly `bazel clean` (never any other
    // verb): the argv pins to the frozen `dx_clean` forward shape.
    let mut argv = vec!["bazel".to_owned()];
    argv.extend(bazel_forward_argv());
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
    if verbose {
        let _ = writeln!(out, "{}", RECOVERY_GUIDANCE);
    }
    bazel_code
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use dx_setup::{read_current_pair, setup_hex};

    #[test]
    fn clean_dry_run_empty_workspace_reports_nothing() {
        let harness = Harness::new("clean-dryrun-empty");
        let (code, out, err) = harness.run(&["clean", "--dry-run"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("nothing to prune"), "{out}");
        assert_eq!(err, "", "{err}");
        assert!(
            harness.seen_env.borrow().is_empty(),
            "dry-run launches nothing"
        );
        assert!(
            !harness.workspace.join(".dx").exists(),
            "dry-run creates no managed state"
        );
    }

    #[test]
    fn clean_dry_run_lists_stale_record_and_deletes_nothing() {
        let harness = Harness::new("clean-dryrun-list");
        // Commit order selects the last pair: stale first, current last.
        let stale = commit_clean_pair(&harness, '3', '4');
        let current = commit_clean_pair(&harness, '1', '2');
        let (code, out, err) = harness.run(&["clean", "--dry-run"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(
            out.contains(&format!("prune setup record: .dx/setups/{stale}")),
            "{out}"
        );
        // The listing reports per-entry reclaimable bytes plus the
        // total; the stale record holds pair links (nonzero) while the
        // empty generation dirs measure zero.
        assert!(out.contains("bytes)"), "{out}");
        assert!(out.contains("reclaimable total:"), "{out}");
        assert!(
            out.contains(&format!("preserve current: .dx/setups/{current}")),
            "{out}"
        );
        assert!(
            !out.contains(&format!("prune setup record: .dx/setups/{current}")),
            "{out}"
        );
        assert!(
            harness.workspace.join(".dx/setups").join(&stale).exists(),
            "dry-run deletes nothing"
        );
        assert!(harness.seen_env.borrow().is_empty());
    }

    #[test]
    fn clean_apply_prunes_stale_record_and_orphaned_generations() {
        let harness = Harness::new("clean-apply");
        let stale = commit_clean_pair(&harness, '3', '4');
        let current = commit_clean_pair(&harness, '1', '2');
        let (code, out, err) = harness.run(&["clean"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(
            out.contains("pruned 1 setup records and 2 generations"),
            "{out}"
        );
        assert!(out.contains("bytes reclaimed"), "{out}");
        let dx_dir = harness.workspace.join(".dx");
        assert!(!dx_dir.join("setups").join(&stale).exists());
        assert!(dx_dir.join("setups").join(&current).exists());
        assert!(!dx_dir
            .join("environments")
            .join('3'.to_string().repeat(64))
            .exists());
        assert!(!dx_dir
            .join("generated")
            .join('4'.to_string().repeat(64))
            .exists());
        assert!(
            dx_dir
                .join("environments")
                .join('1'.to_string().repeat(64))
                .exists(),
            "current generations survive"
        );
        assert_eq!(
            read_current_pair(&harness.workspace).expect("read current"),
            read_current_pair(&harness.workspace).expect("reread current"),
            "selection read is stable"
        );
        let live = read_current_pair(&harness.workspace).expect("live pair");
        let live_pair = live.expect("current still selected");
        assert_eq!(setup_hex(&live_pair), current);
    }

    #[test]
    fn clean_apply_preserves_generations_shared_with_current() {
        let harness = Harness::new("clean-apply-shared");
        commit_clean_pair(&harness, '3', '2');
        commit_clean_pair(&harness, '1', '2');
        let (code, out, err) = harness.run(&["clean"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(
            out.contains("pruned 1 setup records and 1 generations"),
            "{out}"
        );
        assert!(out.contains("bytes reclaimed"), "{out}");
        assert!(
            harness
                .workspace
                .join(".dx/generated")
                .join('2'.to_string().repeat(64))
                .exists(),
            "generation shared with current survives"
        );
    }

    #[test]
    fn clean_bazel_forward_runs_after_prune_and_propagates_code() {
        let harness = Harness::new("clean-bazel");
        let (code, out, err) = harness.run(&["clean", "--bazel"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("nothing to prune"), "{out}");
        assert!(out.contains("re-run `dx setup`"), "{out}");
        assert_eq!(
            harness.seen_env.borrow().len(),
            1,
            "exactly one Bazel launch for the explicit forward"
        );

        let mut failing = Harness::new("clean-bazel-fail");
        failing.bazel_code = 3;
        let (code, _, err) = failing.run(&["clean", "--bazel"]);
        assert_eq!(code, 3, "{err}");

        let dry = Harness::new("clean-bazel-dryrun");
        let (code, out, err) = dry.run(&["clean", "--dry-run", "--bazel"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("would forward: bazel clean"), "{out}");
        assert!(
            dry.seen_env.borrow().is_empty(),
            "dry-run lists the forward without launching"
        );
    }

    #[test]
    fn clean_malformed_current_fails_closed_without_pruning() {
        let harness = Harness::new("clean-bad-current");
        let stale = commit_clean_pair(&harness, '3', '4');
        let pointer = harness.workspace.join(".dx/setups/current");
        std::fs::remove_file(&pointer).expect("remove pointer");
        std::fs::write(&pointer, "not a symlink").expect("file pointer");
        for args in [&["clean", "--dry-run"][..], &["clean"][..]] {
            let (code, out, err) = harness.run(args);
            assert_eq!(code, 1, "{out}{err}");
            let combined = format!("{out}{err}");
            assert!(combined.contains("clean_failed"), "{combined}");
        }
        assert!(
            harness.workspace.join(".dx/setups").join(&stale).exists(),
            "failure prunes nothing"
        );
    }

    #[test]
    fn clean_unmanaged_paths_are_refused_and_preserved() {
        let harness = Harness::new("clean-unmanaged");
        let setups = harness.workspace.join(".dx/setups");
        std::fs::create_dir_all(setups.join("notes")).expect("unmanaged record");
        let environments = harness.workspace.join(".dx/environments");
        std::fs::create_dir_all(&environments).expect("environments dir");
        std::fs::write(environments.join("README"), "operator notes").expect("unmanaged file");
        let (code, out, err) = harness.run(&["clean", "--dry-run"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("refuse unmanaged path: README"), "{out}");
        assert!(out.contains("refuse unmanaged path: notes"), "{out}");
        let (code, out, err) = harness.run(&["clean"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("nothing to prune"), "{out}");
        assert!(setups.join("notes").exists(), "unmanaged paths survive");
        assert!(environments.join("README").exists());
    }
}
