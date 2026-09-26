use super::common::*;
use super::managed_staging::{collect_managed_group, ensure_generation_dir, symlink_leaf};
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) type ManagedCodegenCollection = (
    Vec<dx_bep::TargetOutput>,
    dx_codegen::CollectedPlan,
    Vec<dx_codegen::ProjectionEntry>,
);

pub(crate) fn collect_managed_codegen(
    bep: &Path,
    workspace: &Path,
) -> Result<ManagedCodegenCollection, (String, String)> {
    let outputs = collect_managed_group(bep, dx_codegen::OUTPUT_GROUP, workspace)?;
    let plan = dx_codegen::collect_plan(&outputs).map_err(|err| {
        (
            CODE_INVALID_RESULT.to_owned(),
            format!("invalid codegen plan: {err}"),
        )
    })?;
    let projection = dx_codegen::plan_projection(&plan.records, &outputs).map_err(|err| {
        // LCOV_EXCL_START - reason: defensive diverge, issue: 1055, policy: docs/testing/strategy-details.md#coverage
        (
            CODE_INVALID_RESULT.to_owned(),
            format!("invalid codegen plan: {err}"),
        )
        // LCOV_EXCL_STOP - reason: end defensive diverge, issue: 1055, policy: docs/testing/strategy-details.md#coverage
    })?;
    Ok((outputs, plan, projection))
}

pub(crate) fn empty_generated_id() -> Result<dx_setup::GenerationId, (String, String)> {
    // LCOV_EXCL_START - reason: defensive diverge, issue: 1055, policy: docs/testing/strategy-details.md#coverage
    dx_setup::GenerationId::new(&dx_codegen::plan_hex("[]")).map_err(|err| {
        (
            CODE_INVALID_RESULT.to_owned(),
            format!("invalid empty codegen plan digest: {err}"),
        )
    })
    // LCOV_EXCL_STOP - reason: end defensive diverge, issue: 1055, policy: docs/testing/strategy-details.md#coverage
}

fn validate_logical_path(logical_path: &str) -> Result<(), ExecError> {
    if logical_path.is_empty() {
        return Err(ExecError::EmptyLogicalPath);
    }
    let path = Path::new(logical_path);
    if path.is_absolute() {
        return Err(ExecError::AbsoluteLogicalPath {
            path: logical_path.to_owned(),
        });
    }
    if path.components().any(|c| {
        matches!(
            c,
            std::path::Component::ParentDir | std::path::Component::Prefix(_)
        )
    }) {
        return Err(ExecError::EscapingLogicalPath {
            path: logical_path.to_owned(),
        });
    }
    Ok(())
}

pub(crate) fn stage_codegen_generation(
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
        if !entry.replaces.is_empty() && entry.replaces != entry.logical_path {
            return Err((
                CODE_INVALID_RESULT.to_owned(),
                format!(
                    "invalid codegen plan: entry {:?} carries a replacement contract for {:?}, want the logical path itself",
                    entry.logical_path, entry.replaces,
                ),
            ));
        }
        if workspace
            .join(&entry.logical_path)
            .symlink_metadata()
            .is_ok()
            && entry.replaces != entry.logical_path
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

pub(crate) fn stage_codegen_side(
    workspace: &Path,
    plan: &dx_codegen::CollectedPlan,
    projection: &[dx_codegen::ProjectionEntry],
) -> Result<dx_setup::GenerationId, (String, String)> {
    let id = dx_setup::GenerationId::new(&plan.hex()).map_err(|err| {
        // LCOV_EXCL_START - reason: defensive diverge, issue: 1055, policy: docs/testing/strategy-details.md#coverage
        (
            CODE_INVALID_RESULT.to_owned(),
            format!("invalid codegen plan digest: {err}"),
        )
        // LCOV_EXCL_STOP - reason: end defensive diverge, issue: 1055, policy: docs/testing/strategy-details.md#coverage
    })?;
    stage_codegen_generation(workspace, &id, projection)?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use super::*;
    use dx_setup::{read_current_pair, GENERATED_DIR_NAME};

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
    fn managed_stage_codegen_mirrors_and_reuses_leaves() {
        let fixture = managed_stage_fixture("managed-codegen-mirror");
        let workspace = fixture.workspace().to_path_buf();
        let first = fixture.first.clone();
        let second = fixture.second.clone();
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
        let fixture = managed_stage_fixture("managed-codegen-reject");
        let workspace = fixture.workspace().to_path_buf();
        let first = fixture.first.clone();
        let second = fixture.second.clone();
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
    fn managed_stage_codegen_replacement_contract_allows_declared_collision() {
        let fixture = managed_stage_fixture("managed-codegen-replaces");
        let workspace = fixture.workspace().to_path_buf();
        let first = fixture.first.clone();
        let id = empty_generated_id().expect("empty digest");
        // A checked-in source at the generated logical path fails closed
        // without the contract.
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
        // The same collision succeeds when the entry carries the explicit
        // replacement contract identifying the replaced source
        // (`replaces` equal to the logical path) and the backing artifact.
        stage_codegen_generation(
            &workspace,
            &id,
            &[codegen_replacement_entry("gen/owned.txt", &first)],
        )
        .expect("contracted collision stages");
        let dir = workspace
            .join(".dx")
            .join(GENERATED_DIR_NAME)
            .join(id.as_str());
        assert_eq!(
            std::fs::read_link(dir.join("gen/owned.txt")).expect("leaf"),
            first
        );
        // The checked-in source itself is untouched: the mirror stages
        // under the generation directory, never beside sources.
        assert_eq!(
            std::fs::read(workspace.join("gen/owned.txt")).expect("source"),
            b"source"
        );
        // A cross-path contract never waives the collision: `replaces`
        // must equal the logical path itself.
        let bad = dx_codegen::ProjectionEntry {
            logical_path: "gen/owned.txt".to_owned(),
            artifact: first.to_string_lossy().into_owned(),
            import_root: String::new(),
            namespace: String::new(),
            replaces: "gen/other.txt".to_owned(),
        };
        let (code, message) =
            stage_codegen_generation(&workspace, &id, &[bad]).expect_err("cross-path contract");
        assert_eq!(code, CODE_INVALID_RESULT);
        assert!(message.contains("replacement contract"), "{message}");
    }

    #[test]
    fn managed_stage_codegen_filesystem_failures_fail_closed() {
        let fixture = managed_stage_fixture("managed-codegen-fs");
        let workspace = fixture.workspace().to_path_buf();
        let first = fixture.first.clone();
        let second = fixture.second.clone();
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
}
