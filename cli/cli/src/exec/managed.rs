//! Managed generation staging and commit (`codegen`/`env`/`setup`) execution.

use super::common::*;
use crate::args::{Command, Invocation};
use crate::plan::{bep_path, plan_managed};
use dx_bep::{collect, CollectorConfig};
use dx_output::OutputMode;
use dx_process::ForwardError;
use std::collections::BTreeMap;
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};

/// Collects BEP-reported artifacts for one managed output group.
/// Split from [`collect_managed_codegen`] and [`collect_managed_env`]
/// so each selection proves its own group transport without touching
/// the other group's stream.
fn collect_managed_group(
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

/// Validated managed codegen collection: raw group outputs plus the
/// merged plan and its read-only projection. The raw outputs stay
/// alongside so exact-scope callers can tell a capability-absent side
/// (no contributing target) from a present but empty plan.
type ManagedCodegenCollection = (
    Vec<dx_bep::TargetOutput>,
    dx_codegen::CollectedPlan,
    Vec<dx_codegen::ProjectionEntry>,
);

/// Validated managed environment collection, mirroring
/// [`ManagedCodegenCollection`] over the env output group.
type ManagedEnvCollection = (
    Vec<dx_bep::TargetOutput>,
    dx_env_plan::CollectedPlan,
    Vec<dx_env_plan::ProjectionEntry>,
);

/// Collects and validates one managed codegen plan: decodes shards,
/// rejects conflicts, merges deterministically, and plans the read-only
/// projection through the same index collection validates. An empty
/// shard set validates as an empty plan.
fn collect_managed_codegen(bep: &Path) -> Result<ManagedCodegenCollection, (String, String)> {
    let outputs = collect_managed_group(bep, dx_codegen::OUTPUT_GROUP)?;
    let plan = dx_codegen::collect_plan(&outputs).map_err(|err| {
        (
            CODE_INVALID_RESULT.to_owned(),
            format!("invalid codegen plan: {err}"),
        )
    })?;
    let projection = dx_codegen::plan_projection(&plan.records, &outputs).map_err(|err| {
        // LCOV_EXCL_START - reason: defense-in-depth; collect_plan runs the identical artifact-index validation over the same outputs, so projection cannot fail after a successful collect; retained so a future divergence fails closed as invalid_result rather than panicking.
        (
            CODE_INVALID_RESULT.to_owned(),
            format!("invalid codegen plan: {err}"),
        )
        // LCOV_EXCL_STOP - reason: end of unreachable projection-failure mapping.
    })?;
    Ok((outputs, plan, projection))
}

/// Collects and validates one managed environment plan, mirroring
/// [`collect_managed_codegen`] over the env output group.
fn collect_managed_env(bep: &Path) -> Result<ManagedEnvCollection, (String, String)> {
    let outputs = collect_managed_group(bep, dx_env_plan::OUTPUT_GROUP)?;
    let plan = dx_env_plan::collect_plan(&outputs).map_err(|err| {
        (
            CODE_INVALID_RESULT.to_owned(),
            format!("invalid env plan: {err}"),
        )
    })?;
    let projection = dx_env_plan::plan_projection(&plan.records, &outputs).map_err(|err| {
        // LCOV_EXCL_START - reason: defense-in-depth; collect_plan runs the identical artifact-index validation over the same outputs, so projection cannot fail after a successful collect; retained so a future divergence fails closed as invalid_result rather than panicking.
        (
            CODE_INVALID_RESULT.to_owned(),
            format!("invalid env plan: {err}"),
        )
        // LCOV_EXCL_STOP - reason: end of unreachable projection-failure mapping.
    })?;
    Ok((outputs, plan, projection))
}

/// Managed empty environment identity: the deterministic empty-plan
/// digest (`"[]"` fingerprint) pairing a first independent codegen
/// selection with a real immutable identity, per
/// `docs/environments/managed-state.md`.
fn empty_env_id() -> Result<dx_setup::GenerationId, (String, String)> {
    // LCOV_EXCL_START - reason: defense-in-depth; plan_hex always renders a valid generation id, so construction cannot fail; retained so a future divergence fails closed as invalid_result rather than panicking.
    dx_setup::GenerationId::new(&dx_env_plan::plan_hex("[]")).map_err(|err| {
        (
            CODE_INVALID_RESULT.to_owned(),
            format!("invalid empty env plan digest: {err}"),
        )
    })
    // LCOV_EXCL_STOP - reason: end of unreachable digest-construction exclusion.
}

/// Managed empty generated-code identity, mirroring [`empty_env_id`].
fn empty_generated_id() -> Result<dx_setup::GenerationId, (String, String)> {
    // LCOV_EXCL_START - reason: defense-in-depth; plan_hex always renders a valid generation id, so construction cannot fail; retained so a future divergence fails closed as invalid_result rather than panicking.
    dx_setup::GenerationId::new(&dx_codegen::plan_hex("[]")).map_err(|err| {
        (
            CODE_INVALID_RESULT.to_owned(),
            format!("invalid empty codegen plan digest: {err}"),
        )
    })
    // LCOV_EXCL_STOP - reason: end of unreachable digest-construction exclusion.
}

/// Ensures the hash-addressed generation directory exists as a managed
/// directory. A present file, symlink, or other non-directory fails
/// closed so foreign state is never adopted; any other inspection
/// failure falls through to creation, which fails closed with the
/// underlying error.
fn ensure_generation_dir(
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
fn symlink_leaf(target: &Path, link: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(target, link)
    }
    #[cfg(not(windows))]
    {
        std::os::unix::fs::symlink(target, link)
    }
}

/// Rejects workspace-absolute, escaping, or empty logical paths before
/// any mutation.
fn validate_logical_path(logical_path: &str) -> Result<(), String> {
    if logical_path.is_empty() {
        return Err("generated logical path is empty".to_owned());
    }
    let path = Path::new(logical_path);
    if path.is_absolute() {
        return Err(format!(
            "generated logical path {logical_path:?} is absolute"
        ));
    }
    if path.components().any(|c| {
        matches!(
            c,
            std::path::Component::ParentDir | std::path::Component::Prefix(_)
        )
    }) {
        return Err(format!(
            "generated logical path {logical_path:?} escapes its generation"
        ));
    }
    Ok(())
}

/// Stages one immutable codegen generation: validates every mirror leaf
/// against the current BEP result, refuses logical paths colliding with
/// checked-in sources, and installs deterministic symlinks to Bazel-owned
/// artifacts. Missing artifacts fail before selection; Bazel owns remote
/// materialization and the CLI performs no fetch. Leaves install
/// idempotently so concurrent preparation of one generation never fails;
/// a leaf pointing elsewhere is reconstructed.
fn stage_codegen_generation(
    workspace: &Path,
    id: &dx_setup::GenerationId,
    projection: &[dx_codegen::ProjectionEntry],
) -> Result<(), (String, String)> {
    let dir = ensure_generation_dir(workspace, dx_setup::GENERATED_DIR_NAME, id.as_str())?;
    let mut seen: BTreeMap<&str, &str> = BTreeMap::new();
    for entry in projection {
        if let Some(previous) = seen.insert(entry.logical_path.as_str(), entry.artifact.as_str()) {
            if previous != entry.artifact.as_str() {
                return Err((
                    CODE_INVALID_RESULT.to_owned(),
                    format!(
                        "generated logical path {:?} maps to multiple artifacts",
                        entry.logical_path
                    ),
                ));
            }
        }
    }
    for entry in projection {
        validate_logical_path(&entry.logical_path).map_err(|reason| {
            (
                CODE_INVALID_RESULT.to_owned(),
                format!("invalid codegen plan: {reason}"),
            )
        })?;
        if workspace
            .join(&entry.logical_path)
            .symlink_metadata()
            .is_ok()
        {
            return Err((
                CODE_INVALID_RESULT.to_owned(),
                format!(
                    "invalid codegen plan: generated logical path {:?} collides with a workspace source",
                    entry.logical_path
                ),
            ));
        }
        let artifact = Path::new(&entry.artifact);
        if artifact.symlink_metadata().is_err() {
            return Err((
                CODE_INVALID_RESULT.to_owned(),
                format!(
                    "invalid codegen plan: referenced artifact {:?} is missing; Bazel owns materialization",
                    entry.artifact
                ),
            ));
        }
        let leaf = dir.join(&entry.logical_path);
        if let Some(parent) = leaf.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                (
                    CODE_MANAGED_COMMIT_FAILED.to_owned(),
                    format!("cannot create {}: {e}", parent.display()),
                )
            })?;
        }
        // Reuse is exact-identity reuse: a leaf already pointing at the
        // current BEP-reported artifact stays; any other existing leaf is
        // reconstructed so a stale or foreign leaf never survives
        // selection. An unreadable leaf falls through to replacement,
        // which fails closed below when the filesystem is unusable.
        let needs_link = match std::fs::symlink_metadata(&leaf) {
            Ok(meta) => {
                if meta.file_type().is_dir() && !meta.file_type().is_symlink() {
                    return Err((
                        CODE_INVALID_RESULT.to_owned(),
                        format!(
                            "invalid codegen plan: generated logical path {:?} collides within its generation",
                            entry.logical_path
                        ),
                    ));
                }
                match std::fs::read_link(&leaf) {
                    Ok(current) if current == *artifact => false,
                    _ => {
                        std::fs::remove_file(&leaf).map_err(|e| {
                            (
                                CODE_MANAGED_COMMIT_FAILED.to_owned(),
                                format!("cannot replace {}: {e}", leaf.display()),
                            )
                        })?;
                        true
                    }
                }
            }
            Err(_) => true,
        };
        if needs_link {
            symlink_leaf(artifact, &leaf).map_err(|e| {
                (
                    CODE_MANAGED_COMMIT_FAILED.to_owned(),
                    format!("cannot link {}: {e}", leaf.display()),
                )
            })?;
        }
    }
    Ok(())
}

/// Rejects env keys that are not safe single-path filenames before any
/// mutation.
fn validate_env_key(key: &str) -> Result<(), String> {
    if key.is_empty() {
        return Err("env identity key is empty".to_owned());
    }
    if key.contains('/') || key.contains('\\') || key == "." || key == ".." {
        return Err(format!("env identity key {key:?} is not a single filename"));
    }
    Ok(())
}

/// Stages one immutable environment generation: validates every backing
/// leaf against the current BEP result and installs deterministic
/// symlinks to Bazel-owned artifacts under `artifacts/` plus a
/// deterministic `values.json` carrying the key-to-value identity
/// inputs. Layout mirrors the codegen mirror leaf shape so selection
/// commits one deterministic link tree; language-native facades and
/// `.dx/bin` refresh stay outside this layer.
fn stage_env_generation(
    workspace: &Path,
    id: &dx_setup::GenerationId,
    projection: &[dx_env_plan::ProjectionEntry],
) -> Result<(), (String, String)> {
    let dir = ensure_generation_dir(workspace, dx_setup::ENVIRONMENTS_DIR_NAME, id.as_str())?;
    let mut seen: BTreeMap<&str, (&str, &str)> = BTreeMap::new();
    for entry in projection {
        validate_env_key(&entry.key).map_err(|reason| {
            (
                CODE_INVALID_RESULT.to_owned(),
                format!("invalid env plan: {reason}"),
            )
        })?;
        if let Some((value, artifact)) = seen.insert(
            entry.key.as_str(),
            (entry.value.as_str(), entry.artifact.as_str()),
        ) {
            if value != entry.value.as_str() || artifact != entry.artifact.as_str() {
                return Err((
                    CODE_INVALID_RESULT.to_owned(),
                    format!("env identity key {:?} maps to multiple inputs", entry.key),
                ));
            }
        }
    }
    let artifacts = dir.join("artifacts");
    std::fs::create_dir_all(&artifacts).map_err(|e| {
        (
            CODE_MANAGED_COMMIT_FAILED.to_owned(),
            format!("cannot create {}: {e}", artifacts.display()),
        )
    })?;
    for entry in projection {
        let artifact = Path::new(&entry.artifact);
        if artifact.symlink_metadata().is_err() {
            return Err((
                CODE_INVALID_RESULT.to_owned(),
                format!(
                    "invalid env plan: referenced artifact {:?} is missing; Bazel owns materialization",
                    entry.artifact
                ),
            ));
        }
        // Reuse mirrors the codegen mirror leaves: a leaf already
        // pointing at the current BEP-reported artifact stays, any other
        // existing leaf is reconstructed, and an unreadable leaf falls
        // through to replacement, which fails closed below.
        let leaf = artifacts.join(&entry.key);
        let needs_link = match std::fs::symlink_metadata(&leaf) {
            Ok(meta) => {
                if meta.file_type().is_dir() && !meta.file_type().is_symlink() {
                    return Err((
                        CODE_INVALID_RESULT.to_owned(),
                        format!(
                            "invalid env plan: env identity key {:?} collides within its generation",
                            entry.key
                        ),
                    ));
                }
                match std::fs::read_link(&leaf) {
                    Ok(current) if current == *artifact => false,
                    _ => {
                        std::fs::remove_file(&leaf).map_err(|e| {
                            (
                                CODE_MANAGED_COMMIT_FAILED.to_owned(),
                                format!("cannot replace {}: {e}", leaf.display()),
                            )
                        })?;
                        true
                    }
                }
            }
            Err(_) => true,
        };
        if needs_link {
            symlink_leaf(artifact, &leaf).map_err(|e| {
                (
                    CODE_MANAGED_COMMIT_FAILED.to_owned(),
                    format!("cannot link {}: {e}", leaf.display()),
                )
            })?;
        }
    }
    // `seen` is already a `BTreeMap`, so serializing the key/value
    // projection preserves sorted keys; `serde_json` owns string
    // escaping and `dx_atomic_fs` owns crash-safe publishing (#227).
    let values: BTreeMap<&str, &str> = seen
        .iter()
        .map(|(key, (value, _))| (*key, *value))
        .collect();
    let rendered = serde_json::to_string(&values).map_err(|e| {
        (
            CODE_MANAGED_COMMIT_FAILED.to_owned(),
            format!("cannot render values.json: {e}"),
        )
    })?;
    let target = dir.join("values.json");
    dx_atomic_fs::write_atomic(&target, rendered.as_bytes()).map_err(|e| {
        (
            CODE_MANAGED_COMMIT_FAILED.to_owned(),
            format!("cannot publish {}: {e}", target.display()),
        )
    })?;
    Ok(())
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

/// Stages one validated codegen side: derives its immutable identity
/// from the plan digest and installs the mirror leaves. Returns the
/// prepared generation identity.
fn stage_codegen_side(
    workspace: &Path,
    plan: &dx_codegen::CollectedPlan,
    projection: &[dx_codegen::ProjectionEntry],
) -> Result<dx_setup::GenerationId, (String, String)> {
    let id = dx_setup::GenerationId::new(&plan.hex()).map_err(|err| {
        // LCOV_EXCL_START - reason: defense-in-depth; plan digests always render valid generation ids, so construction cannot fail; retained so a future divergence fails closed as invalid_result rather than panicking.
        (
            CODE_INVALID_RESULT.to_owned(),
            format!("invalid codegen plan digest: {err}"),
        )
        // LCOV_EXCL_STOP - reason: end of unreachable digest-construction exclusion.
    })?;
    stage_codegen_generation(workspace, &id, projection)?;
    Ok(id)
}

/// Stages one validated environment side, mirroring
/// [`stage_codegen_side`] over the env generation layout.
fn stage_env_side(
    workspace: &Path,
    plan: &dx_env_plan::CollectedPlan,
    projection: &[dx_env_plan::ProjectionEntry],
) -> Result<dx_setup::GenerationId, (String, String)> {
    let id = dx_setup::GenerationId::new(&plan.hex()).map_err(|err| {
        // LCOV_EXCL_START - reason: defense-in-depth; plan digests always render valid generation ids, so construction cannot fail; retained so a future divergence fails closed as invalid_result rather than panicking.
        (
            CODE_INVALID_RESULT.to_owned(),
            format!("invalid env plan digest: {err}"),
        )
        // LCOV_EXCL_STOP - reason: end of unreachable digest-construction exclusion.
    })?;
    stage_env_generation(workspace, &id, projection)?;
    Ok(id)
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
    fn managed_live_invalid_codegen_shard_fails_closed() {
        let mut harness = Harness::new("managed-bad-codegen-shard");
        let shard = harness.temp.join("bad.dxcodegen.pb");
        std::fs::write(&shard, b"not a codegen shard").expect("shard");
        harness.raw_bep = Some(managed_shard_bep(&shard, dx_codegen::OUTPUT_GROUP));
        let (code, _, err) = harness.run(&["codegen"]);
        assert_eq!(code, 1, "{err}");
        assert!(
            err.contains("dx: invalid_result: invalid codegen plan"),
            "{err}"
        );
        assert_eq!(
            read_current_pair(&harness.workspace).expect("read current"),
            None,
            "collection failure commits nothing"
        );
    }

    #[test]
    fn managed_live_invalid_env_shard_fails_closed() {
        let mut harness = Harness::new("managed-bad-env-shard");
        let shard = harness.temp.join("bad.dxenv.pb");
        std::fs::write(&shard, b"not an env shard").expect("shard");
        harness.raw_bep = Some(managed_shard_bep(&shard, dx_env_plan::OUTPUT_GROUP));
        let (code, _, err) = harness.run(&["env"]);
        assert_eq!(code, 1, "{err}");
        assert!(
            err.contains("dx: invalid_result: invalid env plan"),
            "{err}"
        );
        assert_eq!(
            read_current_pair(&harness.workspace).expect("read current"),
            None,
            "collection failure commits nothing"
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
    fn managed_logical_paths_validate() {
        assert!(validate_logical_path("gen/out.rs").is_ok());
        assert!(validate_logical_path("a/./b").is_ok());
        for bad in ["", "/absolute", "../escape", "a/../../escape"] {
            assert!(
                validate_logical_path(bad).is_err(),
                "logical path {bad:?} must fail"
            );
        }
    }

    #[test]
    fn managed_env_keys_validate() {
        assert!(validate_env_key("key.json").is_ok());
        for bad in ["", "a/b", "a\\b", ".", ".."] {
            assert!(validate_env_key(bad).is_err(), "env key {bad:?} must fail");
        }
    }

    #[test]
    fn managed_stage_codegen_mirrors_and_reuses_leaves() {
        let (workspace, first, second) = managed_stage_fixture("managed-codegen-mirror");
        let id = empty_generated_id().expect("empty digest");
        let projection = vec![
            codegen_entry("gen/a.txt", &first),
            codegen_entry("nested/b.txt", &second),
        ];
        stage_codegen_generation(&workspace, &id, &projection).expect("stage");
        let dir = workspace
            .join(".dx")
            .join(GENERATED_DIR_NAME)
            .join(id.as_str());
        assert_eq!(
            std::fs::read_link(dir.join("gen/a.txt")).expect("leaf"),
            first
        );
        assert_eq!(
            std::fs::read_link(dir.join("nested/b.txt")).expect("leaf"),
            second
        );
        // Restaging is exact-identity reuse: nothing changes.
        stage_codegen_generation(&workspace, &id, &projection).expect("restage");
        assert_eq!(
            std::fs::read_link(dir.join("gen/a.txt")).expect("leaf"),
            first
        );
        // A stale leaf pointing elsewhere is reconstructed.
        std::fs::remove_file(dir.join("gen/a.txt")).expect("remove leaf");
        symlink_leaf(&second, &dir.join("gen/a.txt")).expect("stale leaf");
        stage_codegen_generation(&workspace, &id, &projection).expect("repair stale");
        assert_eq!(
            std::fs::read_link(dir.join("gen/a.txt")).expect("leaf"),
            first
        );
        // A foreign regular file at a leaf is reconstructed too.
        std::fs::remove_file(dir.join("gen/a.txt")).expect("remove leaf");
        std::fs::write(dir.join("gen/a.txt"), "foreign").expect("foreign leaf");
        stage_codegen_generation(&workspace, &id, &projection).expect("repair foreign");
        assert_eq!(
            std::fs::read_link(dir.join("gen/a.txt")).expect("leaf"),
            first
        );
        // Duplicate entries resolving to the same artifact are one leaf.
        let doubled = vec![
            codegen_entry("gen/a.txt", &first),
            codegen_entry("gen/a.txt", &first),
        ];
        stage_codegen_generation(&workspace, &id, &doubled).expect("identical duplicates");
    }

    #[test]
    fn managed_stage_codegen_rejects_bad_plans() {
        let (workspace, first, second) = managed_stage_fixture("managed-codegen-reject");
        let id = empty_generated_id().expect("empty digest");
        for projection in [
            vec![codegen_entry("", &first)],
            vec![codegen_entry("/absolute", &first)],
            vec![codegen_entry("../escape", &first)],
        ] {
            let (code, message) =
                stage_codegen_generation(&workspace, &id, &projection).expect_err("bad path");
            assert_eq!(code, CODE_INVALID_RESULT);
            assert!(message.contains("invalid codegen plan"), "{message}");
        }
        let (code, message) = stage_codegen_generation(
            &workspace,
            &id,
            &[
                codegen_entry("gen/a.txt", &first),
                codegen_entry("gen/a.txt", &second),
            ],
        )
        .expect_err("conflict");
        assert_eq!(code, CODE_INVALID_RESULT);
        assert!(message.contains("multiple artifacts"), "{message}");
        // A logical path colliding with a checked-in source fails.
        std::fs::create_dir_all(workspace.join("gen")).expect("source dir");
        std::fs::write(workspace.join("gen/owned.txt"), "source").expect("source");
        let (code, message) =
            stage_codegen_generation(&workspace, &id, &[codegen_entry("gen/owned.txt", &first)])
                .expect_err("workspace collision");
        assert_eq!(code, CODE_INVALID_RESULT);
        assert!(
            message.contains("collides with a workspace source"),
            "{message}"
        );
        // Missing artifacts fail before selection; Bazel owns materialization.
        let missing = workspace.join("no-such-artifact.txt");
        let (code, message) = stage_codegen_generation(
            &workspace,
            &id,
            &[codegen_entry("gen/missing.txt", &missing)],
        )
        .expect_err("missing artifact");
        assert_eq!(code, CODE_INVALID_RESULT);
        assert!(message.contains("Bazel owns materialization"), "{message}");
        // A real directory at a leaf collides within its generation.
        let dir_id = dx_setup::GenerationId::new(&"2".repeat(64)).expect("fixture id");
        let dir = ensure_generation_dir(&workspace, GENERATED_DIR_NAME, dir_id.as_str())
            .expect("gen dir");
        std::fs::create_dir_all(dir.join("gen/blocked")).expect("blocking dir");
        let (code, message) =
            stage_codegen_generation(&workspace, &dir_id, &[codegen_entry("gen/blocked", &first)])
                .expect_err("generation collision");
        assert_eq!(code, CODE_INVALID_RESULT);
        assert!(
            message.contains("collides within its generation"),
            "{message}"
        );
        // A file where an intermediate directory belongs fails creation.
        let parent_id = dx_setup::GenerationId::new(&"3".repeat(64)).expect("fixture id");
        let parent_dir = ensure_generation_dir(&workspace, GENERATED_DIR_NAME, parent_id.as_str())
            .expect("gen dir");
        std::fs::write(parent_dir.join("sub"), "file").expect("blocking file");
        let (code, message) = stage_codegen_generation(
            &workspace,
            &parent_id,
            &[codegen_entry("sub/leaf.txt", &first)],
        )
        .expect_err("parent creation");
        assert_eq!(code, CODE_MANAGED_COMMIT_FAILED);
        assert!(message.contains("cannot create"), "{message}");
    }

    #[test]
    fn managed_stage_codegen_filesystem_failures_fail_closed() {
        let (workspace, first, second) = managed_stage_fixture("managed-codegen-fs");
        let id = empty_generated_id().expect("empty digest");
        // Top-level leaves so the generation directory itself is the
        // leaf parent under test.
        stage_codegen_generation(&workspace, &id, &[codegen_entry("a.txt", &first)])
            .expect("stage");
        let dir = workspace
            .join(".dx")
            .join(GENERATED_DIR_NAME)
            .join(id.as_str());
        set_mode(&dir, 0o555);
        // Replacing a stale leaf without write permission fails.
        let (code, message) =
            stage_codegen_generation(&workspace, &id, &[codegen_entry("a.txt", &second)])
                .expect_err("cannot replace");
        assert_eq!(code, CODE_MANAGED_COMMIT_FAILED);
        assert!(message.contains("cannot replace"), "{message}");
        // Linking a fresh leaf without write permission fails.
        let (code, message) = stage_codegen_generation(
            &workspace,
            &id,
            &[
                codegen_entry("a.txt", &first),
                codegen_entry("fresh.txt", &second),
            ],
        )
        .expect_err("cannot link");
        assert_eq!(code, CODE_MANAGED_COMMIT_FAILED);
        assert!(message.contains("cannot link"), "{message}");
        set_mode(&dir, 0o755);
    }

    #[test]
    fn managed_stage_env_mirrors_leaves_and_values() {
        let (workspace, first, second) = managed_stage_fixture("managed-env-mirror");
        let id = empty_env_id().expect("empty digest");
        let projection = vec![
            env_entry("k2", "x\"y", &second),
            env_entry("k1", "v1", &first),
        ];
        stage_env_generation(&workspace, &id, &projection).expect("stage");
        let dir = workspace
            .join(".dx")
            .join(ENVIRONMENTS_DIR_NAME)
            .join(id.as_str());
        assert_eq!(
            std::fs::read_link(dir.join("artifacts/k1")).expect("leaf"),
            first
        );
        assert_eq!(
            std::fs::read_link(dir.join("artifacts/k2")).expect("leaf"),
            second
        );
        // `values.json` is deterministic over sorted keys with JSON escaping.
        let values = std::fs::read_to_string(dir.join("values.json")).expect("values");
        assert_eq!(values, "{\"k1\":\"v1\",\"k2\":\"x\\\"y\"}");
        assert!(
            !dir.join("values.json.next").exists(),
            "staging file is always published"
        );
        // Restaging reuses exact-identity leaves and republishes values.
        stage_env_generation(&workspace, &id, &projection).expect("restage");
        assert_eq!(
            std::fs::read_link(dir.join("artifacts/k1")).expect("leaf"),
            first
        );
        // A stale leaf is reconstructed.
        std::fs::remove_file(dir.join("artifacts/k1")).expect("remove leaf");
        symlink_leaf(&second, &dir.join("artifacts/k1")).expect("stale leaf");
        stage_env_generation(&workspace, &id, &projection).expect("repair stale");
        assert_eq!(
            std::fs::read_link(dir.join("artifacts/k1")).expect("leaf"),
            first
        );
        // Identical duplicates are one leaf.
        let doubled = vec![env_entry("k1", "v1", &first), env_entry("k1", "v1", &first)];
        stage_env_generation(&workspace, &id, &doubled).expect("identical duplicates");
    }

    #[test]
    fn managed_stage_env_rejects_bad_plans() {
        let (workspace, first, second) = managed_stage_fixture("managed-env-reject");
        let id = empty_env_id().expect("empty digest");
        for key in ["", "a/b", ".", ".."] {
            let (code, message) =
                stage_env_generation(&workspace, &id, &[env_entry(key, "v", &first)])
                    .expect_err("bad key");
            assert_eq!(code, CODE_INVALID_RESULT);
            assert!(message.contains("invalid env plan"), "{message}");
        }
        for projection in [
            vec![env_entry("k", "v1", &first), env_entry("k", "v2", &first)],
            vec![env_entry("k", "v", &first), env_entry("k", "v", &second)],
        ] {
            let (code, message) =
                stage_env_generation(&workspace, &id, &projection).expect_err("conflict");
            assert_eq!(code, CODE_INVALID_RESULT);
            assert!(message.contains("multiple inputs"), "{message}");
        }
        let missing = workspace.join("no-such-artifact.txt");
        let (code, message) =
            stage_env_generation(&workspace, &id, &[env_entry("k", "v", &missing)])
                .expect_err("missing artifact");
        assert_eq!(code, CODE_INVALID_RESULT);
        assert!(message.contains("Bazel owns materialization"), "{message}");
        // A real directory at a leaf collides within its generation.
        let dir_id = dx_setup::GenerationId::new(&"4".repeat(64)).expect("fixture id");
        let dir = ensure_generation_dir(&workspace, ENVIRONMENTS_DIR_NAME, dir_id.as_str())
            .expect("gen dir");
        std::fs::create_dir_all(dir.join("artifacts")).expect("artifacts dir");
        std::fs::create_dir_all(dir.join("artifacts/k")).expect("blocking dir");
        let (code, message) =
            stage_env_generation(&workspace, &dir_id, &[env_entry("k", "v", &first)])
                .expect_err("generation collision");
        assert_eq!(code, CODE_INVALID_RESULT);
        assert!(
            message.contains("collides within its generation"),
            "{message}"
        );
        // A file where `artifacts/` belongs fails creation.
        let blocked_id = dx_setup::GenerationId::new(&"5".repeat(64)).expect("fixture id");
        let blocked_dir =
            ensure_generation_dir(&workspace, ENVIRONMENTS_DIR_NAME, blocked_id.as_str())
                .expect("gen dir");
        std::fs::write(blocked_dir.join("artifacts"), "file").expect("blocking file");
        let (code, message) =
            stage_env_generation(&workspace, &blocked_id, &[env_entry("k", "v", &first)])
                .expect_err("artifacts creation");
        assert_eq!(code, CODE_MANAGED_COMMIT_FAILED);
        assert!(message.contains("cannot create"), "{message}");
        // A stale `values.json.next` directory (pre-#227 staging
        // leftover) no longer blocks: atomic staging uses OS-random
        // sibling names, so the legacy path is ignored.
        let staging_id = dx_setup::GenerationId::new(&"6".repeat(64)).expect("fixture id");
        let staging_dir =
            ensure_generation_dir(&workspace, ENVIRONMENTS_DIR_NAME, staging_id.as_str())
                .expect("gen dir");
        std::fs::create_dir_all(staging_dir.join("values.json.next")).expect("blocking dir");
        stage_env_generation(&workspace, &staging_id, &[env_entry("k", "v", &first)])
            .expect("legacy staging leftover ignored");
        // A directory at `values.json` fails the publish rename.
        let publish_id = dx_setup::GenerationId::new(&"7".repeat(64)).expect("fixture id");
        let publish_dir =
            ensure_generation_dir(&workspace, ENVIRONMENTS_DIR_NAME, publish_id.as_str())
                .expect("gen dir");
        std::fs::create_dir_all(publish_dir.join("values.json")).expect("blocking dir");
        let (code, message) =
            stage_env_generation(&workspace, &publish_id, &[env_entry("k", "v", &first)])
                .expect_err("values publish");
        assert_eq!(code, CODE_MANAGED_COMMIT_FAILED);
        assert!(message.contains("cannot publish"), "{message}");
    }

    #[test]
    fn managed_stage_env_filesystem_failures_fail_closed() {
        let (workspace, first, second) = managed_stage_fixture("managed-env-fs");
        let id = empty_env_id().expect("empty digest");
        stage_env_generation(&workspace, &id, &[env_entry("k", "v", &first)]).expect("stage");
        let dir = workspace
            .join(".dx")
            .join(ENVIRONMENTS_DIR_NAME)
            .join(id.as_str());
        // Leaves live under `artifacts/`, so that directory is the leaf
        // parent under test; `values.json` still publishes above it.
        set_mode(&dir.join("artifacts"), 0o555);
        let (code, message) =
            stage_env_generation(&workspace, &id, &[env_entry("k", "v", &second)])
                .expect_err("cannot replace");
        assert_eq!(code, CODE_MANAGED_COMMIT_FAILED);
        assert!(message.contains("cannot replace"), "{message}");
        let (code, message) =
            stage_env_generation(&workspace, &id, &[env_entry("fresh", "v", &second)])
                .expect_err("cannot link");
        assert_eq!(code, CODE_MANAGED_COMMIT_FAILED);
        assert!(message.contains("cannot link"), "{message}");
        set_mode(&dir.join("artifacts"), 0o755);
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
