use super::common::*;
use super::managed_staging::{collect_managed_group, ensure_generation_dir, symlink_leaf};
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) type ManagedEnvCollection = (
    Vec<dx_bep::TargetOutput>,
    dx_env_plan::CollectedPlan,
    Vec<dx_env_plan::ProjectionEntry>,
);

pub(crate) fn collect_managed_env(
    bep: &Path,
    workspace: &Path,
) -> Result<ManagedEnvCollection, (String, String)> {
    let outputs = collect_managed_group(bep, dx_env_plan::OUTPUT_GROUP, workspace)?;
    let plan = dx_env_plan::collect_plan(&outputs).map_err(|err| {
        (
            CODE_INVALID_RESULT.to_owned(),
            format!("invalid env plan: {err}"),
        )
    })?;
    let projection = dx_env_plan::plan_projection(&plan.records, &outputs).map_err(|err| {
        // LCOV_EXCL_START - reason: defensive diverge, issue: 1055, policy: docs/testing/strategy-details.md#coverage
        (
            CODE_INVALID_RESULT.to_owned(),
            format!("invalid env plan: {err}"),
        )
        // LCOV_EXCL_STOP - reason: end defensive diverge, issue: 1055, policy: docs/testing/strategy-details.md#coverage
    })?;
    Ok((outputs, plan, projection))
}

pub(crate) fn empty_env_id() -> Result<dx_setup::GenerationId, (String, String)> {
    // LCOV_EXCL_START - reason: defensive diverge, issue: 1055, policy: docs/testing/strategy-details.md#coverage
    dx_setup::GenerationId::new(&dx_env_plan::plan_hex("[]")).map_err(|err| {
        (
            CODE_INVALID_RESULT.to_owned(),
            format!("invalid empty env plan digest: {err}"),
        )
    })
    // LCOV_EXCL_STOP - reason: end defensive diverge, issue: 1055, policy: docs/testing/strategy-details.md#coverage
}

fn validate_env_key(key: &str) -> Result<(), ExecError> {
    if key.is_empty() {
        return Err(ExecError::EmptyEnvKey);
    }
    if key.contains('/') || key.contains('\\') || key == "." || key == ".." {
        return Err(ExecError::BadEnvKey {
            key: key.to_owned(),
        });
    }
    Ok(())
}

pub(crate) fn stage_env_generation(
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
    // escaping and `dx_atomic_fs` owns crash-safe publishing.
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

pub(crate) fn stage_env_side(
    workspace: &Path,
    plan: &dx_env_plan::CollectedPlan,
    projection: &[dx_env_plan::ProjectionEntry],
) -> Result<dx_setup::GenerationId, (String, String)> {
    let id = dx_setup::GenerationId::new(&plan.hex()).map_err(|err| {
        // LCOV_EXCL_START - reason: defensive diverge, issue: 1055, policy: docs/testing/strategy-details.md#coverage
        (
            CODE_INVALID_RESULT.to_owned(),
            format!("invalid env plan digest: {err}"),
        )
        // LCOV_EXCL_STOP - reason: end defensive diverge, issue: 1055, policy: docs/testing/strategy-details.md#coverage
    })?;
    stage_env_generation(workspace, &id, projection)?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use super::*;
    use dx_setup::{read_current_pair, ENVIRONMENTS_DIR_NAME};

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
    fn managed_env_keys_validate() {
        assert!(validate_env_key("key.json").is_ok());
        for bad in ["", "a/b", "a\\b", ".", ".."] {
            assert!(validate_env_key(bad).is_err(), "env key {bad:?} must fail");
        }
    }

    #[test]
    fn managed_stage_env_mirrors_leaves_and_values() {
        let fixture = managed_stage_fixture("managed-env-mirror");
        let workspace = fixture.workspace().to_path_buf();
        let first = fixture.first.clone();
        let second = fixture.second.clone();
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
        let fixture = managed_stage_fixture("managed-env-reject");
        let workspace = fixture.workspace().to_path_buf();
        let first = fixture.first.clone();
        let second = fixture.second.clone();
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
        // A stale `values.json.next` directory (pre- staging
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
        let fixture = managed_stage_fixture("managed-env-fs");
        let workspace = fixture.workspace().to_path_buf();
        let first = fixture.first.clone();
        let second = fixture.second.clone();
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
}
