//! Managed generation staging and commit (`codegen`/`env`/`setup`) execution.
//!
//! Codegen collection/staging lives in [`super::managed_codegen`]
//! and env collection/staging in [`super::managed_env`]; this module
//! keeps the managed dispatch ([`execute_managed`]) plus the shared
//! staging primitives both sides build on.

use super::common::*;
use super::managed_codegen::{collect_managed_codegen, empty_generated_id, stage_codegen_side};
use super::managed_env::{collect_managed_env, empty_env_id, stage_env_side};
use crate::args::{Command, Invocation};
use crate::plan::{bep_path, plan_managed};
use dx_bep::{collect, CollectorConfig};
use dx_output::OutputMode;
use dx_process::ForwardError;
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};

/// Collects BEP-reported artifacts for one managed output group.
/// Shared by [`collect_managed_codegen`](super::managed_codegen::collect_managed_codegen)
/// and [`collect_managed_env`](super::managed_env::collect_managed_env)
/// so each selection proves its own group transport without touching
/// the other group's stream.
pub(crate) fn collect_managed_group(
    bep: &Path,
    group: &str,
) -> Result<Vec<dx_bep::TargetOutput>, (String, String)> {
    let file = std::fs::File::open(bep).map_err(|err| {
        (
            CODE_UNREADABLE_BEP.to_owned(),
            format!("failed to read build events: {err}"),
        )
    })?;
    let config = CollectorConfig::new(group).map_err(|err| {
        (
            CODE_INVALID_BEP.to_owned(),
            format!("invalid BEP config: {err}"),
        )
    })?;
    collect(BufReader::new(file), &config, &FsArtifacts).map_err(|err| {
        (
            CODE_INVALID_BEP.to_owned(),
            format!("invalid build events: {err}"),
        )
    })
}

/// Ensures the hash-addressed generation directory exists as a managed
/// directory. A present file, symlink, or other non-directory fails
/// closed so foreign state is never adopted; any other inspection
/// failure falls through to creation, which fails closed with the
/// underlying error.
pub(crate) fn ensure_generation_dir(
    workspace: &Path,
    dir_name: &str,
    hex: &str,
) -> Result<PathBuf, (String, String)> {
    if !workspace.is_dir() {
        return Err((
            CODE_MANAGED_COMMIT_FAILED.to_owned(),
            format!("workspace root {} is not a directory", workspace.display()),
        ));
    }
    let dir = workspace.join(".dx").join(dir_name).join(hex);
    match std::fs::symlink_metadata(&dir) {
        Ok(meta) => {
            if !meta.file_type().is_dir() || meta.file_type().is_symlink() {
                return Err((
                    CODE_MANAGED_COMMIT_FAILED.to_owned(),
                    format!(
                        "{} is not a managed generation directory; refusing to adopt foreign state",
                        dir.display()
                    ),
                ));
            }
        }
        Err(_) => {
            std::fs::create_dir_all(&dir).map_err(|e| {
                (
                    CODE_MANAGED_COMMIT_FAILED.to_owned(),
                    format!("cannot create {}: {e}", dir.display()),
                )
            })?;
        }
    }
    Ok(dir)
}

/// Platform symlink primitive for generation mirror leaves: one entry
/// point with the OS primitive selected inside, instead of two
/// cfg-gated twin functions with identical call shapes.
pub(crate) fn symlink_leaf(target: &Path, link: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(target, link)
    }
    #[cfg(not(windows))]
    {
        std::os::unix::fs::symlink(target, link)
    }
}

/// Maps a setup commit failure into the stable managed error vocabulary.
/// Only the capability error carries its own code; every other commit
/// failure preserves the prior pointer and reports `managed_commit_failed`.
fn map_commit_error(error: dx_setup::CommitError) -> (String, String) {
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
fn prepare_managed_sides(
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

/// Runs `dx codegen`, `dx env`, and `dx setup` (M25 WP3/WP5):
/// validates the label-only scope through the shared setup scope rules,
/// plans the Bazel collection request with [`plan_managed`], and either
/// renders the `--dry-run` summary (planning nothing else, launching
/// nothing) or runs the live Bazel build, collects and validates the
/// plan shards, stages the immutable generations, and commits the
/// selection through one atomic `.dx/setups/current` replacement under
/// the shared O36 commit lock. Independent commits re-read the current
/// pair under the lock, so a concurrently completed opposite side is
/// carried forward instead of lost. Build, staging, validation, or
/// commit failure leaves the current setup unchanged; staged but
/// unselected generations may remain as retained cache. Argument
/// parsing guarantees text output with no quality-only options on this
/// path.
///
/// Exits `0` on `--dry-run` and on committed selection (including an
/// already-current reselection), the Bazel exit code verbatim when the
/// live build fails, `1` on operational, collection, staging, or commit
/// failures (including a scope with neither capability), and `2` on
/// scope or policy conflicts found before execution.
pub(crate) fn execute_managed(invocation: &Invocation, env: Env<'_>) -> i32 {
    debug_assert!(
        invocation.command.is_managed(),
        "managed dispatch guards commands"
    );
    let Env {
        workspace,
        runner,
        temp_dir,
        pid,
        nonce,
        out,
        err,
        ..
    } = env;
    if !invocation.command.is_managed() {
        return pre_exec(
            err,
            &ForwardError::UnsupportedCommand {
                command: invocation.command.name().to_owned(),
            }
            .to_string(),
        );
    }
    let scope = match dx_setup::resolve_scope(&invocation.targets) {
        Ok(scope) => scope,
        Err(error) => return pre_exec(err, &error.to_string()),
    };
    let bep = bep_path(temp_dir, pid, nonce);
    let Some(bep_text) = bep.to_str() else {
        return operational(
            invocation,
            out,
            err,
            CODE_UNREADABLE_BEP,
            "temporary event path is not UTF-8",
        );
    };
    let plan = match plan_managed(
        invocation.command,
        &scope,
        &invocation.bazel_options,
        bep_text,
    ) {
        Ok(plan) => plan,
        Err(error) => return pre_exec(err, &format!("{error}")),
    };
    // Human prose is the only output on this path: the planned
    // operation prints unless `--quiet` suppresses it, in both
    // `--dry-run` and live modes.
    let verbose =
        matches!(invocation.output, OutputMode::Text { quiet: false }) && !invocation.quiet;
    if invocation.dry_run {
        if verbose {
            let _ = writeln!(out, "{}", plan.summary);
        }
        return 0;
    }
    if verbose {
        let _ = writeln!(out, "{}", plan.summary);
    }
    let status = match runner.run(&plan.argv, workspace, &[]) {
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
        let _ = std::fs::remove_file(&bep);
        return operational(
            invocation,
            out,
            err,
            CODE_BAZEL_SIGNALLED,
            "Bazel terminated by signal",
        );
    };
    if bazel_code != 0 {
        let _ = std::fs::remove_file(&bep);
        return bazel_code;
    }
    let repository = matches!(scope, dx_setup::SetupScope::Repository);
    let sides = match prepare_managed_sides(invocation.command, repository, workspace, &bep) {
        Ok(sides) => sides,
        Err((code, message)) => {
            let _ = std::fs::remove_file(&bep);
            return operational(invocation, out, err, &code, &message);
        }
    };
    let (pair, outcome) = match dx_setup::commit_prepared(workspace, sides) {
        Ok(committed) => committed,
        Err(error) => {
            let (code, message) = map_commit_error(error);
            let _ = std::fs::remove_file(&bep);
            return operational(invocation, out, err, &code, &message);
        }
    };
    let _ = std::fs::remove_file(&bep);
    if verbose {
        let setup = dx_setup::setup_hex(&pair);
        if outcome == dx_setup::CommitOutcome::AlreadyCurrent {
            let _ = writeln!(
                out,
                "dx {}: already selected setup {setup} (environment {}, generated {})",
                invocation.command.name(),
                pair.environment.as_str(),
                pair.generated.as_str(),
            );
        } else {
            let _ = writeln!(
                out,
                "dx {}: selected setup {setup} (environment {}, generated {})",
                invocation.command.name(),
                pair.environment.as_str(),
                pair.generated.as_str(),
            );
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use super::*;
    use dx_setup::{read_current_pair, ENVIRONMENTS_DIR_NAME, GENERATED_DIR_NAME};
    use std::path::PathBuf;

    #[test]
    fn managed_dry_run_prints_summary_without_launching() {
        for command in ["codegen", "env", "setup"] {
            let name = format!("managed-dryrun-{command}");
            let harness = Harness::new(&name);
            let (code, out, err) = harness.run(&[command, "--dry-run"]);
            assert_eq!(code, 0, "{out}{err}");
            assert!(
                out.contains(&format!("Running {command} for //...")),
                "{out}"
            );
            assert_eq!(err, "", "{err}");
            assert!(
                harness.seen_env.borrow().is_empty(),
                "dry-run launches nothing"
            );
        }
    }

    #[test]
    fn managed_dry_run_exact_scope_selects_label() {
        let harness = Harness::new("managed-dryrun-exact");
        let (code, out, err) = harness.run(&["env", "//a:one", "--dry-run"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("Running env for //a:one"), "{out}");
        assert_eq!(err, "", "{err}");
        assert!(
            harness.seen_env.borrow().is_empty(),
            "dry-run launches nothing"
        );
    }

    #[test]
    fn managed_dry_run_quiet_prints_nothing() {
        let harness = Harness::new("managed-dryrun-quiet");
        let (code, out, err) = harness.run(&["setup", "--dry-run", "--quiet"]);
        assert_eq!(code, 0, "{out}{err}");
        assert_eq!(out, "", "{out}");
        assert_eq!(err, "", "{err}");
    }

    #[test]
    fn managed_live_empty_selection_commits_with_empty_counterparts() {
        for command in ["codegen", "env", "setup"] {
            let name = format!("managed-commit-{command}");
            let harness = Harness::new(&name);
            let (code, out, err) = harness.run(&[command]);
            assert_eq!(code, 0, "{out}{err}");
            assert!(
                out.contains(&format!("Running {command} for //...")),
                "{out}"
            );
            assert!(out.contains("selected setup "), "{out}");
            assert_eq!(err, "", "{err}");
            assert_eq!(
                harness.seen_env.borrow().len(),
                1,
                "committed selection launches one Bazel build"
            );
            // An empty plan stages the managed empty generations, so the
            // first selection pairs each prepared side with the managed
            // empty counterpart from the same digest family. Only staged
            // sides materialize; the record links to an unstaged empty
            // counterpart dangle with ordinary missing-target behavior.
            let pair = read_current_pair(&harness.workspace)
                .expect("read current")
                .expect("selection committed");
            assert_eq!(pair.environment, empty_env_id().expect("empty digest"));
            assert_eq!(pair.generated, empty_generated_id().expect("empty digest"));
            for side in match command {
                "codegen" => vec![GENERATED_DIR_NAME],
                "env" => vec![ENVIRONMENTS_DIR_NAME],
                _ => vec![ENVIRONMENTS_DIR_NAME, GENERATED_DIR_NAME],
            } {
                let id = match side {
                    GENERATED_DIR_NAME => pair.generated.as_str(),
                    _ => pair.environment.as_str(),
                };
                assert!(
                    harness.workspace.join(".dx").join(side).join(id).is_dir(),
                    "{command} stages its {side} generation"
                );
            }
            // Reselection is a no-op success reporting the current setup.
            let (code, out, err) = harness.run(&[command]);
            assert_eq!(code, 0, "{out}{err}");
            assert!(out.contains("already selected setup "), "{out}");
            assert_eq!(err, "", "{err}");
        }
    }

    #[test]
    fn managed_live_exact_sides_commit_with_empty_counterparts() {
        for command in ["codegen", "env"] {
            let name = format!("managed-exact-{command}");
            let harness = Harness::new(&name);
            let (code, out, err) = harness.run(&[command, "//a:one"]);
            assert_eq!(code, 0, "{out}{err}");
            assert!(out.contains("selected setup "), "{out}");
            let pair = read_current_pair(&harness.workspace)
                .expect("read current")
                .expect("selection committed");
            assert_eq!(pair.environment, empty_env_id().expect("empty digest"));
            assert_eq!(pair.generated, empty_generated_id().expect("empty digest"));
        }
    }

    #[test]
    fn managed_live_exact_setup_without_capability_fails_closed() {
        let harness = Harness::new("managed-setup-nocap");
        let (code, out, err) = harness.run(&["setup", "//a:one"]);
        assert_eq!(code, 1, "{out}{err}");
        assert!(err.contains("dx: no_capability:"), "{err}");
        assert!(out.contains("Running setup for //a:one"), "{out}");
        assert_eq!(
            read_current_pair(&harness.workspace).expect("read current"),
            None,
            "capability failure commits nothing"
        );
    }

    #[test]
    fn managed_live_quiet_commit_prints_nothing() {
        let harness = Harness::new("managed-quiet-commit");
        let (code, out, err) = harness.run(&["codegen", "--quiet"]);
        assert_eq!(code, 0, "{out}{err}");
        assert_eq!(out, "", "{out}");
        assert_eq!(err, "", "{err}");
        assert!(
            read_current_pair(&harness.workspace)
                .expect("read current")
                .is_some(),
            "quiet still commits"
        );
    }

    #[test]
    fn managed_live_malformed_current_fails_commit_without_mutation() {
        let harness = Harness::new("managed-bad-current");
        let (code, _, _) = harness.run(&["codegen"]);
        assert_eq!(code, 0);
        let generations = harness.workspace.join(".dx").join(GENERATED_DIR_NAME);
        assert!(generations.is_dir(), "first commit stages generations");
        let pointer = harness.workspace.join(".dx/setups/current");
        std::fs::remove_file(&pointer).expect("remove pointer");
        std::fs::write(&pointer, "not a symlink").expect("file pointer");
        let (code, _, err) = harness.run(&["codegen"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("dx: managed_commit_failed:"), "{err}");
        assert!(
            generations.is_dir(),
            "commit failure preserves staged cache"
        );
        assert_eq!(
            std::fs::read(&pointer).expect("pointer bytes"),
            b"not a symlink",
            "commit failure leaves the malformed pointer untouched"
        );
    }

    #[test]
    fn managed_group_config_failure_is_operational() {
        let harness = Harness::new("managed-bad-group");
        let bep = harness.temp.join("empty.json");
        std::fs::write(&bep, "").expect("bep");
        let (code, message) = collect_managed_group(&bep, "").expect_err("empty group");
        assert_eq!(code, CODE_INVALID_BEP);
        assert!(message.contains("invalid BEP config"), "{message}");
    }

    #[test]
    fn managed_empty_sides_derive_the_managed_empty_identities() {
        let workspace = temp_dir("managed-empty-sides-ws");
        let codegen_plan = dx_codegen::collect_plan(&[]).expect("empty codegen plan");
        let staged = stage_codegen_side(&workspace, &codegen_plan, &[]).expect("stage");
        assert_eq!(staged, empty_generated_id().expect("empty digest"));
        let env_plan = dx_env_plan::collect_plan(&[]).expect("empty env plan");
        let staged = stage_env_side(&workspace, &env_plan, &[]).expect("stage");
        assert_eq!(staged, empty_env_id().expect("empty digest"));
        let values = std::fs::read_to_string(
            workspace
                .join(".dx")
                .join(ENVIRONMENTS_DIR_NAME)
                .join(empty_env_id().expect("empty digest").as_str())
                .join("values.json"),
        )
        .expect("values");
        assert_eq!(values, "{}");
    }

    #[test]
    fn managed_generation_dir_guards_foreign_state() {
        let workspace = temp_dir("managed-gendir-ws");
        let hex = empty_generated_id().expect("empty digest");
        let first =
            ensure_generation_dir(&workspace, GENERATED_DIR_NAME, hex.as_str()).expect("create");
        assert!(first.is_dir());
        let second =
            ensure_generation_dir(&workspace, GENERATED_DIR_NAME, hex.as_str()).expect("reuse");
        assert_eq!(first, second);
        // A present non-directory is never adopted.
        let other = "0".repeat(64);
        std::fs::write(
            workspace.join(".dx").join(GENERATED_DIR_NAME).join(&other),
            "foreign",
        )
        .expect("foreign file");
        let (code, message) =
            ensure_generation_dir(&workspace, GENERATED_DIR_NAME, &other).expect_err("refuse");
        assert_eq!(code, CODE_MANAGED_COMMIT_FAILED);
        assert!(
            message.contains("not a managed generation directory"),
            "{message}"
        );
        // Creation failures surface the underlying error.
        let file_workspace = workspace.join("ws-file");
        std::fs::write(&file_workspace, "not a dir").expect("workspace file");
        let (code, message) = ensure_generation_dir(&file_workspace, GENERATED_DIR_NAME, &other)
            .expect_err("workspace file");
        assert_eq!(code, CODE_MANAGED_COMMIT_FAILED);
        assert!(message.contains("is not a directory"), "{message}");
    }

    #[test]
    fn managed_generation_dir_create_failure_surfaces() {
        let workspace = temp_dir("managed-gendir-create-ws");
        // `.dx/generated` as a file makes directory creation fail.
        std::fs::create_dir_all(workspace.join(".dx")).expect("dx dir");
        std::fs::write(workspace.join(".dx").join(GENERATED_DIR_NAME), "file").expect("blocker");
        let (code, message) =
            ensure_generation_dir(&workspace, GENERATED_DIR_NAME, &"1".repeat(64))
                .expect_err("create fails");
        assert_eq!(code, CODE_MANAGED_COMMIT_FAILED);
        assert!(message.contains("cannot create"), "{message}");
    }

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

    #[test]
    fn managed_live_launch_failure_is_operational() {
        let harness = Harness {
            io_error: true,
            ..Harness::new("managed-launch-failed")
        };
        let (code, _, err) = harness.run(&["codegen"]);
        assert_eq!(code, 1);
        assert!(err.contains("dx: launch_failed: failed to launch Bazel"));
    }

    #[test]
    fn managed_live_signalled_bazel_is_operational() {
        let harness = Harness {
            signalled: true,
            ..Harness::new("managed-signalled")
        };
        let (code, _, err) = harness.run(&["env"]);
        assert_eq!(code, 1);
        assert!(err.contains("dx: bazel_signalled: Bazel terminated by signal"));
    }

    #[test]
    fn managed_live_bazel_failure_returns_exit_verbatim() {
        let harness = Harness {
            bazel_code: 3,
            ..Harness::new("managed-bazel-failed")
        };
        let (code, out, err) = harness.run(&["setup"]);
        assert_eq!(code, 3, "{out}{err}");
        assert!(out.contains("Running setup for //..."), "{out}");
    }

    #[test]
    fn managed_live_missing_bep_is_operational() {
        let harness = Harness {
            skip_bep: true,
            ..Harness::new("managed-missing-bep")
        };
        let (code, _, err) = harness.run(&["codegen"]);
        assert_eq!(code, 1);
        assert!(err.contains("dx: unreadable_bep: failed to read build events"));
    }

    #[test]
    fn managed_live_malformed_bep_is_operational() {
        let harness = Harness {
            raw_bep: Some(vec!["{not json".to_owned()]),
            ..Harness::new("managed-bad-bep")
        };
        let (code, _, err) = harness.run(&["env"]);
        assert_eq!(code, 1);
        assert!(err.contains("dx: invalid_bep: invalid build events"));
    }

    #[test]
    fn managed_policy_conflict_fails_before_execution() {
        let harness = Harness::new("managed-conflict");
        let (code, _, _) = harness.run(&["setup", "--", "--aspects=//other.bzl%aspect"]);
        assert_eq!(code, 2);
        assert!(
            harness.seen_env.borrow().is_empty(),
            "policy conflict launches nothing"
        );
    }
}
