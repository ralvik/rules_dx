//! Normalized codegen plan collection for the `dx` CLI (M25 WP1 slice 4).
//!
//! Contract: `docs/environments/codegen.md` (provider contract, private
//! `dx_codegen_plans` output group, reserved shard suffix) and
//! `docs/cli/commands/environment-codegen-setup.md` (`dx codegen` scope:
//! no argument invokes canonical `//dx:codegen`, one exact target label,
//! paths/patterns/multiple labels/profiles/language selectors rejected).
//! Merge, conflict, and fingerprint semantics mirror the frozen Starlark
//! helpers in `//generation:codegen.bzl` (`codegen_merge_records`,
//! `codegen_conflict_error`, `codegen_plan_fingerprint`) over the binary
//! `DxCodegenShard` wire schema (`//generation:codegen.proto`); shard
//! decode and per-shard validation stay in `codegen_shard`.
//!
//! The collector reads only BEP-reported artifacts in the requested output
//! group (via `dx_bep`), recognizes shards by the reserved suffix, and
//! never scans `bazel-out`. Each non-empty entry `exec_path` suffix must
//! resolve to exactly one BEP-reported non-shard artifact (suffix match
//! on "/" boundaries so output bases differ); missing, ambiguous, or
//! duplicate claims fail, and every non-shard BEP artifact must be
//! claimed, otherwise collection fails with unreported before projection
//! planning. Empty exec paths are logical-only entries requiring no
//! artifact. Byte-identical duplicate records (one record reached through
//! many transitive routes) merge silently; any other second claim on a
//! logical path fails before selection with every claimant listed. The
//! normalized fingerprint binds exec paths and is hashed with BLAKE3-256
//! through `quality_result` (no algorithm negotiation). [`plan_projection`]
//! resolves the merged plan to deterministic mirror leaves
//! (`logical_path` to full BEP artifact path) for setup to commit.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use codegen_shard::{decode_validated, Error};
use dx_bep::TargetOutput;
use dx_roots::{build_argv, invocation_targets, repository_plan, RepositoryRootPlan};
use quality_result::digest;

/// Private output group carrying collected shards plus every generated
/// artifact referenced by them. Frozen under O33; matches
/// `DX_CODEGEN_PLAN_OUTPUT_GROUP` in `//generation:codegen.bzl`.
pub const OUTPUT_GROUP: &str = "dx_codegen_plans";

/// Reserved controlled filename suffix recognizing shards among
/// BEP-reported files. Frozen under O33; matches
/// `DX_CODEGEN_SHARD_SUFFIX` in `//generation:codegen.bzl`.
pub const SHARD_SUFFIX: &str = ".dxcodegen.pb";

/// Collecting aspect applied to the selected roots. Matches
/// `dx_codegen_plan_aspect` in `//generation:codegen.bzl`.
pub const CODEGEN_ASPECT: &str = "//generation:codegen.bzl%dx_codegen_plan_aspect";

/// Canonical repository-wide codegen selection invoked by bare
/// `dx codegen`. The effective Bazel roots behind this label stay
/// provisional pending the WP4 (O34) root benchmark; this crate only
/// owns the selection identity.
pub const REPOSITORY_TARGET: &str = "//dx:codegen";

/// Selected codegen scope: the whole repository or one exact target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodegenScope {
    Repository,
    Exact(String),
}

/// Exact-target scope failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{self:?}")]
pub enum ScopeError {
    /// More than one positional target: codegen selects at most one.
    MultipleTargets { count: usize },
    /// A target pattern (`...`, `*`, `?`): patterns never select codegen.
    TargetPattern { value: String },
    /// Anything that is not an exact target label: paths, directories,
    /// profiles/flags, and language selectors.
    NotTargetLabel { value: String },
}

/// Resolves the `dx codegen` positional scope: empty selects the
/// repository (`//dx:codegen`), one exact `//` or `@` label selects its
/// configured closure, and anything else fails before execution. A
/// compatible target whose closure contributes no shards selects an
/// empty exact projection downstream, never a failure here.
pub fn resolve_scope(targets: &[String]) -> Result<CodegenScope, ScopeError> {
    match targets {
        [] => Ok(CodegenScope::Repository),
        [single] => {
            if single.contains("...") || single.contains('*') || single.contains('?') {
                Err(ScopeError::TargetPattern {
                    value: single.clone(),
                })
            } else if single.starts_with("//") || single.starts_with('@') {
                Ok(CodegenScope::Exact(single.clone()))
            } else {
                Err(ScopeError::NotTargetLabel {
                    value: single.clone(),
                })
            }
        }
        _ => Err(ScopeError::MultipleTargets {
            count: targets.len(),
        }),
    }
}

/// Bazel labels to build for `scope`: the canonical repository target or
/// the one exact label. The repository arm composes the WP4 (O34)
/// [`dx_roots::repository_plan`] (still the `//...` baseline) behind the
/// `//dx:codegen` selection identity; exact scopes bypass root selection.
pub fn scope_targets(scope: &CodegenScope) -> Vec<String> {
    match scope {
        CodegenScope::Repository => targets_for_root_plan(&repository_plan()),
        CodegenScope::Exact(label) => vec![label.clone()],
    }
}

/// Bazel labels to build for a WP4 root plan behind `//dx:codegen`: the
/// baseline plan keeps the canonical selection identity while the
/// benchmark runs; every other candidate passes its own roots through.
/// The query-pattern-file candidate carries no command-line patterns
/// (Bazel reads them from `--target_pattern_file`).
pub fn targets_for_root_plan(plan: &RepositoryRootPlan) -> Vec<String> {
    invocation_targets(plan, REPOSITORY_TARGET)
}

/// Full `bazel build` command line for a WP4 root plan: `build` plus the
/// plan roots, the collecting aspect, and the private output group (plus
/// `--target_pattern_file` when the plan carries a pattern file).
pub fn build_argv_for_plan(plan: &RepositoryRootPlan) -> Vec<String> {
    build_argv(
        plan,
        REPOSITORY_TARGET,
        &[CODEGEN_ASPECT.to_owned()],
        &[OUTPUT_GROUP.to_owned()],
    )
}

/// One normalized projection entry. Projections are always read-only
/// context, so no `read_only` bit is carried: the fingerprint renders
/// `read_only: true` unconditionally, matching `codegen_entry`.
/// `exec_path` is the BEP-matching suffix for the backing artifact, or
/// empty for a logical-only entry requiring no materialized artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenEntry {
    pub logical_path: String,
    pub import_root: String,
    pub namespace: String,
    pub exec_path: String,
}

/// One contributor record: all entries from one producer in one
/// generated file class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenRecord {
    pub producer: String,
    pub language: String,
    pub entries: Vec<CodegenEntry>,
}

/// Shard collection failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{self:?}")]
pub enum CollectError {
    /// A BEP-reported shard file fails to decode or validate.
    Shard { path: String, error: Error },
    /// Two records claim one logical path incompatibly. The message
    /// matches `codegen_conflict_error` rendering, listing every
    /// claimant: no traversal-order winner is accepted.
    Conflict(String),
    /// An entry's non-empty exec suffix matches no BEP-reported
    /// non-shard artifact.
    MissingArtifact {
        producer: String,
        logical_path: String,
        exec_path: String,
    },
    /// An exec suffix matches more than one artifact, or one artifact
    /// is claimed by more than one entry. `claimants` holds the
    /// colliding logical paths (ambiguous case) or the artifact path
    /// rendered once per claimant context; see message construction.
    DuplicateArtifact {
        exec_path: String,
        claimants: Vec<String>,
    },
    /// A BEP-reported non-shard artifact is claimed by no entry.
    UnreportedArtifact { path: String },
}

/// Reports whether a BEP-reported artifact path is a plan shard, by
/// reserved filename suffix on the raw path bytes.
pub fn is_shard_artifact(path: &Path) -> bool {
    path.as_os_str()
        .as_encoded_bytes()
        .ends_with(SHARD_SUFFIX.as_bytes())
}

/// Reports whether a BEP-reported artifact path satisfies an entry's
/// exec suffix: exact equality or a "/"-boundary suffix match, so
/// different output bases still resolve without scanning `bazel-out`.
/// Mirrors `_exec_matches` in `//generation:codegen.bzl`.
pub fn exec_matches(artifact_path: &Path, exec_path: &str) -> bool {
    if exec_path.is_empty() {
        return false;
    }
    let rendered = artifact_path.as_os_str().to_string_lossy();
    rendered.as_ref() == exec_path || rendered.ends_with(&format!("/{exec_path}"))
}

/// Decodes every shard among the BEP-reported artifacts and validates
/// the artifact index against shard declarations: each non-empty entry
/// exec suffix must resolve to exactly one non-shard BEP artifact, and
/// every non-shard BEP artifact must be claimed by exactly one entry.
/// Logical-only entries (empty exec) require no artifact. Inputs arrive
/// sorted by label from `dx_bep`; determinism comes from
/// [`merge_records`], never arrival order.
pub fn collect_shards(outputs: &[TargetOutput]) -> Result<Vec<CodegenRecord>, CollectError> {
    let generated_paths = generated_artifact_paths(outputs);
    let mut records = Vec::new();
    for output in outputs {
        for artifact in &output.artifacts {
            if !is_shard_artifact(&artifact.exec_path) {
                continue;
            }
            let shard = decode_validated(&artifact.bytes).map_err(|error| CollectError::Shard {
                path: artifact.exec_path.display().to_string(),
                error,
            })?;
            records.push(CodegenRecord {
                producer: shard.producer,
                language: shard.language,
                entries: shard
                    .entries
                    .into_iter()
                    .map(|entry| CodegenEntry {
                        logical_path: entry.logical_path,
                        import_root: entry.import_root,
                        namespace: entry.namespace,
                        exec_path: entry.exec_path,
                    })
                    .collect(),
            });
        }
    }
    let _ = index_artifacts(&records, &generated_paths)?;
    Ok(records)
}

/// Lists every BEP-reported non-shard artifact path: the generated
/// files the shard `exec_path` suffixes index into. Shard files are
/// recognized by the reserved suffix and excluded.
fn generated_artifact_paths(outputs: &[TargetOutput]) -> Vec<String> {
    let mut paths = Vec::new();
    for output in outputs {
        for artifact in &output.artifacts {
            if !is_shard_artifact(&artifact.exec_path) {
                paths.push(artifact.exec_path.display().to_string());
            }
        }
    }
    paths
}

/// Reports whether a BEP-reported artifact path satisfies an entry's
/// exec suffix: exact equality or a "/"-boundary suffix match.
fn suffix_matches(artifact: &str, exec_path: &str) -> bool {
    artifact == exec_path || artifact.ends_with(&format!("/{exec_path}"))
}

/// Validates the artifact index and resolves every backed entry to its
/// backing artifact: each non-empty entry exec suffix must resolve to
/// exactly one non-shard BEP artifact, and every non-shard BEP artifact
/// must be claimed by exactly one entry. Returns the resolution map
/// from `(producer, logical_path)` to the full BEP-reported artifact
/// path; logical-only entries (empty exec) resolve to no artifact and
/// hold no map entry.
fn index_artifacts<'a>(
    records: &'a [CodegenRecord],
    generated_paths: &[String],
) -> Result<BTreeMap<(&'a str, &'a str), String>, CollectError> {
    let claimed: Vec<(&CodegenRecord, &CodegenEntry)> = records
        .iter()
        .flat_map(|record| record.entries.iter().map(move |entry| (record, entry)))
        .filter(|(_, entry)| !entry.exec_path.is_empty())
        .collect();
    let mut resolved: BTreeMap<(&str, &str), String> = BTreeMap::new();
    for (record, entry) in &claimed {
        let matches: Vec<&String> = generated_paths
            .iter()
            .filter(|path| suffix_matches(path, &entry.exec_path))
            .collect();
        if matches.is_empty() {
            return Err(CollectError::MissingArtifact {
                producer: record.producer.clone(),
                logical_path: entry.logical_path.clone(),
                exec_path: entry.exec_path.clone(),
            });
        }
        if matches.len() > 1 {
            return Err(CollectError::DuplicateArtifact {
                exec_path: entry.exec_path.clone(),
                claimants: matches.into_iter().map(|path| (*path).clone()).collect(),
            });
        }
        resolved.insert(
            (record.producer.as_str(), entry.logical_path.as_str()),
            (*matches[0]).clone(),
        );
    }
    for path in generated_paths {
        let claimants: Vec<(&CodegenRecord, &CodegenEntry)> = claimed
            .iter()
            .filter(|(_, entry)| suffix_matches(path, &entry.exec_path))
            .map(|(record, entry)| (*record, *entry))
            .collect();
        if claimants.is_empty() {
            return Err(CollectError::UnreportedArtifact { path: path.clone() });
        }
        if claimants.len() > 1 {
            let exec = claimants[0].1.exec_path.clone();
            let rendered: Vec<String> = claimants
                .iter()
                .map(|(record, entry)| format!("{}:{}", record.producer, entry.logical_path))
                .collect();
            return Err(CollectError::DuplicateArtifact {
                exec_path: exec,
                claimants: rendered,
            });
        }
    }
    Ok(resolved)
}

/// Merge index: owner `(producer, language)` to entry key
/// `(logical_path, import_root, namespace, exec_path)` to entry.
type MergeIndex<'a> =
    BTreeMap<(&'a str, &'a str), BTreeMap<(&'a str, &'a str, &'a str, &'a str), &'a CodegenEntry>>;

/// Merges records into deterministic normalized order, mirroring
/// `codegen_merge_records`: byte-identical duplicate entries collapse,
/// surviving records sort by `(producer, language)` with entries sorted
/// by `(logical_path, import_root, namespace, exec_path)`.
pub fn merge_records(records: &[CodegenRecord]) -> Vec<CodegenRecord> {
    let mut owners: MergeIndex<'_> = BTreeMap::new();
    for record in records {
        let owned = owners
            .entry((record.producer.as_str(), record.language.as_str()))
            .or_default();
        for entry in &record.entries {
            owned
                .entry((
                    entry.logical_path.as_str(),
                    entry.import_root.as_str(),
                    entry.namespace.as_str(),
                    entry.exec_path.as_str(),
                ))
                .or_insert(entry);
        }
    }
    owners
        .into_iter()
        .map(|((producer, language), entries)| CodegenRecord {
            producer: producer.to_owned(),
            language: language.to_owned(),
            entries: entries.into_values().cloned().collect(),
        })
        .collect()
}

/// Detects incompatible logical-path claims across records, mirroring
/// `codegen_conflict_error`: byte-identical duplicates (same producer,
/// language, root, namespace, and exec path) merge silently, any other
/// second claim fails, listing every claimant. Returns `""` when
/// conflict-free.
pub fn conflict_error(records: &[CodegenRecord]) -> String {
    /// One logical-path claim: owner `(producer, language)` plus the
    /// full entry identity `(import_root, namespace, exec_path)`.
    type Claim<'a> = (&'a str, &'a str, &'a str, &'a str, &'a str);
    let mut by_path: BTreeMap<&str, BTreeSet<Claim<'_>>> = BTreeMap::new();
    for record in records {
        for entry in &record.entries {
            by_path
                .entry(entry.logical_path.as_str())
                .or_default()
                .insert((
                    record.producer.as_str(),
                    record.language.as_str(),
                    entry.import_root.as_str(),
                    entry.namespace.as_str(),
                    entry.exec_path.as_str(),
                ));
        }
    }
    let mut conflicts = Vec::new();
    for (path, claims) in &by_path {
        if claims.len() > 1 {
            let claimants: Vec<&str> = claims.iter().map(|claim| claim.0).collect();
            conflicts.push(format!(
                "logical path '{path}' claimed by {}",
                claimants.join(", ")
            ));
        }
    }
    if conflicts.is_empty() {
        String::new()
    } else {
        format!("codegen path conflict: {}", conflicts.join("; "))
    }
}

fn escape_into(out: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
}

/// Renders the normalized complete-plan hash input, mirroring
/// `codegen_plan_fingerprint`: deterministic JSON over the merged
/// records, one object per record with producer, language, and entries
/// sorted by (logical path, import root, namespace, exec path), each
/// entry binding its exec suffix. Byte-identical to the Starlark
/// rendering for the same records (pinned by the chain/prost fixture
/// fingerprints).
pub fn fingerprint(records: &[CodegenRecord]) -> String {
    let merged = merge_records(records);
    let mut out = String::from("[");
    for (index, record) in merged.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"entries\":[");
        for (entry_index, entry) in record.entries.iter().enumerate() {
            if entry_index > 0 {
                out.push(',');
            }
            out.push_str("{\"exec_path\":\"");
            escape_into(&mut out, &entry.exec_path);
            out.push_str("\",\"import_root\":\"");
            escape_into(&mut out, &entry.import_root);
            out.push_str("\",\"logical_path\":\"");
            escape_into(&mut out, &entry.logical_path);
            out.push_str("\",\"namespace\":\"");
            escape_into(&mut out, &entry.namespace);
            out.push_str("\",\"read_only\":true}");
        }
        out.push_str("],\"language\":\"");
        escape_into(&mut out, &record.language);
        out.push_str("\",\"producer\":\"");
        escape_into(&mut out, &record.producer);
        out.push_str("\"}");
    }
    out.push(']');
    out
}

/// BLAKE3-256 over the normalized fingerprint bytes: the complete-plan
/// identity, with no algorithm negotiation. Routed through the shared
/// result crate so the digest algorithm has one owner.
pub fn plan_digest(fingerprint: &str) -> [u8; 32] {
    digest(fingerprint.as_bytes())
}

/// Lowercase hex of the plan digest, for operator messaging.
pub fn plan_hex(fingerprint: &str) -> String {
    hex::encode(plan_digest(fingerprint))
}

/// The normalized complete plan: merged records plus their fingerprint
/// and digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectedPlan {
    pub records: Vec<CodegenRecord>,
    pub fingerprint: String,
    pub digest: [u8; 32],
}

impl CollectedPlan {
    /// Lowercase hex of the plan digest, for operator messaging.
    pub fn hex(&self) -> String {
        hex::encode(self.digest)
    }
}

/// Collects the normalized complete plan from BEP-reported outputs:
/// decodes shards, rejects conflicts, merges deterministically, and
/// hashes the fingerprint. An empty shard set selects an empty plan
/// (`"[]"`) rather than failing, so compatible targets with no
/// generated sources clear stale selections.
pub fn collect_plan(outputs: &[TargetOutput]) -> Result<CollectedPlan, CollectError> {
    let records = collect_shards(outputs)?;
    let conflict = conflict_error(&records);
    if !conflict.is_empty() {
        return Err(CollectError::Conflict(conflict));
    }
    let merged = merge_records(&records);
    let rendered = fingerprint(&merged);
    let digest = digest(rendered.as_bytes());
    Ok(CollectedPlan {
        records: merged,
        fingerprint: rendered,
        digest,
    })
}

/// One planned mirror leaf: the deterministic workspace-relative
/// logical path and the full BEP-reported artifact path it links to.
/// Logical-only entries (empty exec) carry no backing artifact and hold
/// no projection entry: the mirror links backed files only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionEntry {
    pub logical_path: String,
    pub artifact: String,
    pub import_root: String,
    pub namespace: String,
}

/// Plans the read-only mirror from merged records and BEP-reported
/// outputs: resolves every backed entry to its unique backing artifact
/// through the same index [`collect_shards`] validates (missing,
/// ambiguous, duplicate, or unreported artifacts fail here too),
/// skips logical-only entries, and sorts by logical path so setup
/// commits one deterministic link tree. Callers pass the merged
/// [`CollectedPlan::records`]; conflicts must already be rejected by
/// [`conflict_error`] before selection.
pub fn plan_projection(
    records: &[CodegenRecord],
    outputs: &[TargetOutput],
) -> Result<Vec<ProjectionEntry>, CollectError> {
    let generated_paths = generated_artifact_paths(outputs);
    let resolved = index_artifacts(records, &generated_paths)?;
    let mut projection: Vec<ProjectionEntry> = Vec::new();
    for record in records {
        for entry in &record.entries {
            if entry.exec_path.is_empty() {
                continue;
            }
            let artifact = resolved
                .get(&(record.producer.as_str(), entry.logical_path.as_str()))
                .cloned()
                .unwrap_or_else(|| entry.exec_path.clone());
            projection.push(ProjectionEntry {
                logical_path: entry.logical_path.clone(),
                artifact,
                import_root: entry.import_root.clone(),
                namespace: entry.namespace.clone(),
            });
        }
    }
    projection.sort_by(|left, right| left.logical_path.cmp(&right.logical_path));
    Ok(projection)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use codegen_shard::proto::{DxCodegenEntry, DxCodegenShard};
    use dx_bep::CollectedArtifact;

    fn entry(logical_path: &str, import_root: &str, namespace: &str) -> CodegenEntry {
        CodegenEntry {
            logical_path: logical_path.to_owned(),
            import_root: import_root.to_owned(),
            namespace: namespace.to_owned(),
            exec_path: String::new(),
        }
    }

    fn entry_with_exec(
        logical_path: &str,
        import_root: &str,
        namespace: &str,
        exec_path: &str,
    ) -> CodegenEntry {
        CodegenEntry {
            logical_path: logical_path.to_owned(),
            import_root: import_root.to_owned(),
            namespace: namespace.to_owned(),
            exec_path: exec_path.to_owned(),
        }
    }

    fn record(producer: &str, language: &str, entries: Vec<CodegenEntry>) -> CodegenRecord {
        CodegenRecord {
            producer: producer.to_owned(),
            language: language.to_owned(),
            entries,
        }
    }

    fn record_a() -> CodegenRecord {
        record(
            "//gen:alpha",
            "rust",
            vec![entry("src/alpha.rs", "src", "alpha")],
        )
    }

    fn record_b() -> CodegenRecord {
        record(
            "//gen:beta",
            "rust",
            vec![entry("src/beta.rs", "src", "beta")],
        )
    }

    fn shard_bytes(
        producer: &str,
        language: &str,
        entries: Vec<(&str, &str, &str, &str)>,
    ) -> Vec<u8> {
        let shard = DxCodegenShard {
            producer: producer.to_owned(),
            language: language.to_owned(),
            entries: entries
                .into_iter()
                .map(
                    |(logical_path, import_root, namespace, exec_path)| DxCodegenEntry {
                        logical_path: logical_path.to_owned(),
                        import_root: import_root.to_owned(),
                        namespace: namespace.to_owned(),
                        read_only: true,
                        exec_path: exec_path.to_owned(),
                    },
                )
                .collect(),
        };
        codegen_shard::encode_validated(&shard).expect("valid shard")
    }

    fn output(label: &str, files: Vec<(&str, Vec<u8>)>) -> TargetOutput {
        TargetOutput {
            label: label.to_owned(),
            success: true,
            artifacts: files
                .into_iter()
                .map(|(path, bytes)| CollectedArtifact {
                    exec_path: PathBuf::from(path),
                    bytes,
                })
                .collect(),
        }
    }

    #[test]
    fn frozen_identities_match_starlark() {
        assert_eq!(OUTPUT_GROUP, "dx_codegen_plans");
        assert_eq!(SHARD_SUFFIX, ".dxcodegen.pb");
        assert_eq!(
            CODEGEN_ASPECT,
            "//generation:codegen.bzl%dx_codegen_plan_aspect"
        );
        assert_eq!(REPOSITORY_TARGET, "//dx:codegen");
    }

    #[test]
    fn empty_scope_selects_repository() {
        assert_eq!(resolve_scope(&[]), Ok(CodegenScope::Repository));
        assert_eq!(
            scope_targets(&CodegenScope::Repository),
            vec!["//dx:codegen"]
        );
    }

    #[test]
    fn root_plan_composes_baseline_invocation() {
        let plan = dx_roots::repository_plan();
        assert_eq!(targets_for_root_plan(&plan), vec!["//dx:codegen"]);
        assert_eq!(
            build_argv_for_plan(&plan),
            vec![
                "build".to_owned(),
                "//dx:codegen".to_owned(),
                format!("--aspects={CODEGEN_ASPECT}"),
                format!("--output_groups={OUTPUT_GROUP}"),
            ]
        );
    }

    #[test]
    fn root_plan_passes_aggregate_roots_through() {
        let plan = dx_roots::RepositoryRootPlan::monolithic_aggregate("//dx:codegen_roots");
        assert_eq!(
            targets_for_root_plan(&plan),
            vec!["//dx:codegen_roots".to_owned()]
        );
        assert_eq!(
            build_argv_for_plan(&plan)[1],
            "//dx:codegen_roots".to_owned()
        );
    }

    #[test]
    fn exact_labels_pass_through() {
        for label in ["//gen:consumer", "@rules_dx//generation:result_proto_rs"] {
            let scope = resolve_scope(&[label.to_owned()]).expect("exact label");
            assert_eq!(scope, CodegenScope::Exact(label.to_owned()));
            assert_eq!(scope_targets(&scope), vec![label.to_owned()]);
        }
    }

    #[test]
    fn scope_rejects_non_exact_inputs() {
        assert_eq!(
            resolve_scope(&["//a:x".to_owned(), "//b:y".to_owned()]),
            Err(ScopeError::MultipleTargets { count: 2 })
        );
        for pattern in ["//...", "//gen/...", "//gen:*", "@repo//pkg:all?"] {
            assert_eq!(
                resolve_scope(&[pattern.to_owned()]),
                Err(ScopeError::TargetPattern {
                    value: pattern.to_owned(),
                }),
                "pattern {pattern:?} must fail"
            );
        }
        for other in ["gen/alpha.rs", "gen", "rust", "--profile=fast", ":relative"] {
            assert_eq!(
                resolve_scope(&[other.to_owned()]),
                Err(ScopeError::NotTargetLabel {
                    value: other.to_owned(),
                }),
                "non-label {other:?} must fail"
            );
        }
    }

    #[test]
    fn scope_errors_display() {
        assert!(ScopeError::MultipleTargets { count: 2 }
            .to_string()
            .contains("MultipleTargets"));
    }

    #[test]
    fn shard_suffix_matches_on_path_bytes() {
        assert!(is_shard_artifact(Path::new("/out/a.dxcodegen.pb")));
        assert!(!is_shard_artifact(Path::new("/out/a.pb")));
        assert!(!is_shard_artifact(Path::new("/out/a.dxcodegen.pb.txt")));
    }

    #[test]
    fn exec_suffix_matches_on_component_boundaries() {
        assert!(exec_matches(
            Path::new("/out/result_proto.lib.rs"),
            "result_proto.lib.rs"
        ));
        assert!(exec_matches(
            Path::new("/bazel-out/k8-fastbuild/bin/gen/result_proto.lib.rs"),
            "result_proto.lib.rs"
        ));
        assert!(!exec_matches(
            Path::new("/out/other.lib.rs"),
            "result_proto.lib.rs"
        ));
        // Suffix matches only on "/" boundaries, never mid-segment.
        assert!(!exec_matches(
            Path::new("/out/xresult_proto.lib.rs"),
            "result_proto.lib.rs"
        ));
        assert!(!exec_matches(Path::new("/out/a.rs"), ""));
    }

    #[test]
    fn collect_shards_accepts_logical_only_without_artifacts() {
        let outputs = vec![output(
            "//gen:beta",
            vec![(
                "/out/beta.dxcodegen.pb",
                shard_bytes(
                    "//gen:beta",
                    "rust",
                    vec![("gen/beta.rs", "gen", "beta", "")],
                ),
            )],
        )];
        let records = collect_shards(&outputs).expect("collect");
        assert_eq!(
            records,
            vec![record(
                "//gen:beta",
                "rust",
                vec![entry("gen/beta.rs", "gen", "beta")]
            )]
        );
    }

    #[test]
    fn collect_shards_binds_exec_suffix_to_one_artifact() {
        let outputs = vec![output(
            "//gen:beta",
            vec![
                (
                    "/out/beta.dxcodegen.pb",
                    shard_bytes(
                        "//gen:beta",
                        "rust",
                        vec![("gen/beta.rs", "gen", "beta", "beta.lib.rs")],
                    ),
                ),
                ("/bazel-out/k8-fastbuild/bin/gen/beta.lib.rs", vec![1, 2, 3]),
            ],
        )];
        let records = collect_shards(&outputs).expect("collect");
        assert_eq!(
            records,
            vec![record(
                "//gen:beta",
                "rust",
                vec![entry_with_exec("gen/beta.rs", "gen", "beta", "beta.lib.rs")]
            )]
        );
    }

    #[test]
    fn collect_shards_rejects_missing_duplicate_and_unreported() {
        // Missing: exec suffix matches no reported artifact.
        let missing = vec![output(
            "//gen:beta",
            vec![(
                "/out/beta.dxcodegen.pb",
                shard_bytes(
                    "//gen:beta",
                    "rust",
                    vec![("gen/beta.rs", "gen", "beta", "beta.lib.rs")],
                ),
            )],
        )];
        assert_eq!(
            collect_shards(&missing),
            Err(CollectError::MissingArtifact {
                producer: "//gen:beta".to_owned(),
                logical_path: "gen/beta.rs".to_owned(),
                exec_path: "beta.lib.rs".to_owned(),
            })
        );
        // Ambiguous: one suffix matches two reported artifacts.
        let ambiguous = vec![output(
            "//gen:beta",
            vec![
                (
                    "/out/beta.dxcodegen.pb",
                    shard_bytes(
                        "//gen:beta",
                        "rust",
                        vec![("gen/beta.rs", "gen", "beta", "beta.lib.rs")],
                    ),
                ),
                ("/out/a/beta.lib.rs", vec![1]),
                ("/out/b/beta.lib.rs", vec![2]),
            ],
        )];
        assert!(matches!(
            collect_shards(&ambiguous),
            Err(CollectError::DuplicateArtifact { .. })
        ));
        // Unreported: a non-shard artifact no entry claims.
        let unreported = vec![output(
            "//gen:beta",
            vec![
                (
                    "/out/beta.dxcodegen.pb",
                    shard_bytes(
                        "//gen:beta",
                        "rust",
                        vec![("gen/beta.rs", "gen", "beta", "")],
                    ),
                ),
                ("/out/beta.lib.rs", vec![1, 2, 3]),
            ],
        )];
        assert_eq!(
            collect_shards(&unreported),
            Err(CollectError::UnreportedArtifact {
                path: "/out/beta.lib.rs".to_owned(),
            })
        );
        // Duplicate claim: two entries bind the same artifact.
        let duplicate = vec![output(
            "//gen:beta",
            vec![
                (
                    "/out/beta.dxcodegen.pb",
                    shard_bytes(
                        "//gen:beta",
                        "rust",
                        vec![
                            ("gen/a.rs", "gen", "a", "shared.lib.rs"),
                            ("gen/b.rs", "gen", "b", "shared.lib.rs"),
                        ],
                    ),
                ),
                ("/out/shared.lib.rs", vec![1]),
            ],
        )];
        assert!(matches!(
            collect_shards(&duplicate),
            Err(CollectError::DuplicateArtifact { .. })
        ));
    }

    #[test]
    fn collect_shards_rejects_invalid_shards_with_path() {
        let outputs = vec![output(
            "//gen:evil",
            vec![("/out/evil.dxcodegen.pb", vec![0xff, 0x00, 0x01])],
        )];
        let error = collect_shards(&outputs).expect_err("invalid shard");
        assert!(matches!(error, CollectError::Shard { .. }));
        assert!(error.to_string().contains("Shard"));
    }

    #[test]
    fn merge_groups_by_owner_and_sorts_entries() {
        let merged = merge_records(&[
            record(
                "//gen:b",
                "rust",
                vec![entry("z.rs", "s", ""), entry("a.rs", "s", "")],
            ),
            record_a(),
            record_a(),
            record("//gen:a", "python", vec![entry("m.py", "s", "")]),
        ]);
        let summary: Vec<(&str, &str, Vec<&str>)> = merged
            .iter()
            .map(|record| {
                (
                    record.producer.as_str(),
                    record.language.as_str(),
                    record
                        .entries
                        .iter()
                        .map(|entry| entry.logical_path.as_str())
                        .collect(),
                )
            })
            .collect();
        assert_eq!(
            summary,
            vec![
                ("//gen:a", "python", vec!["m.py"]),
                ("//gen:alpha", "rust", vec!["src/alpha.rs"]),
                ("//gen:b", "rust", vec!["a.rs", "z.rs"]),
            ]
        );
    }

    #[test]
    fn fingerprint_matches_starlark_rendering() {
        assert_eq!(
            fingerprint(&[record_b(), record_a(), record_a()]),
            "[{\"entries\":[{\"exec_path\":\"\",\"import_root\":\"src\",\"logical_path\":\"src/alpha.rs\",\"namespace\":\"alpha\",\"read_only\":true}],\"language\":\"rust\",\"producer\":\"//gen:alpha\"},\
             {\"entries\":[{\"exec_path\":\"\",\"import_root\":\"src\",\"logical_path\":\"src/beta.rs\",\"namespace\":\"beta\",\"read_only\":true}],\"language\":\"rust\",\"producer\":\"//gen:beta\"}]"
        );
        assert_eq!(
            fingerprint(&[record(
                "//gen:a",
                "rust",
                vec![entry_with_exec("a", "b", "", "out/a.rs")]
            )]),
            "[{\"entries\":[{\"exec_path\":\"out/a.rs\",\"import_root\":\"b\",\"logical_path\":\"a\",\"namespace\":\"\",\"read_only\":true}],\"language\":\"rust\",\"producer\":\"//gen:a\"}]"
        );
    }

    #[test]
    fn fingerprint_matches_chain_fixture() {
        let records = vec![
            record(
                "//generation:codegen_shard_beta",
                "rust",
                vec![entry("gen/beta.rs", "gen", "beta")],
            ),
            record(
                "//generation:codegen_shard_alpha",
                "rust",
                vec![entry("gen/alpha.rs", "gen", "alpha")],
            ),
        ];
        assert_eq!(
            fingerprint(&records),
            "[{\"entries\":[{\"exec_path\":\"\",\"import_root\":\"gen\",\"logical_path\":\"gen/alpha.rs\",\"namespace\":\"alpha\",\"read_only\":true}],\"language\":\"rust\",\"producer\":\"//generation:codegen_shard_alpha\"},\
             {\"entries\":[{\"exec_path\":\"\",\"import_root\":\"gen\",\"logical_path\":\"gen/beta.rs\",\"namespace\":\"beta\",\"read_only\":true}],\"language\":\"rust\",\"producer\":\"//generation:codegen_shard_beta\"}]"
        );
    }

    #[test]
    fn fingerprint_matches_prost_fixture() {
        let records = vec![record(
            "//generation:codegen_prost_fixture",
            "rust",
            vec![entry_with_exec(
                "gen/prost_result.rs",
                "gen",
                "result",
                "result_proto.lib.rs",
            )],
        )];
        assert_eq!(
            fingerprint(&records),
            "[{\"entries\":[{\"exec_path\":\"result_proto.lib.rs\",\"import_root\":\"gen\",\"logical_path\":\"gen/prost_result.rs\",\"namespace\":\"result\",\"read_only\":true}],\"language\":\"rust\",\"producer\":\"//generation:codegen_prost_fixture\"}]"
        );
    }

    #[test]
    fn conflict_accepts_disjoint_and_identical_records() {
        assert_eq!(conflict_error(&[record_a(), record_b()]), "");
        assert_eq!(conflict_error(&[record_a(), record_a()]), "");
        assert_eq!(conflict_error(&[]), "");
    }

    #[test]
    fn conflict_lists_every_claimant() {
        assert_eq!(
            conflict_error(&[
                record_a(),
                record(
                    "//gen:evil",
                    "rust",
                    vec![entry("src/alpha.rs", "src", "alpha")]
                ),
            ]),
            "codegen path conflict: logical path 'src/alpha.rs' claimed by //gen:alpha, //gen:evil"
        );
        assert_eq!(
            conflict_error(&[
                record_a(),
                record(
                    "//gen:alpha",
                    "rust",
                    vec![entry("src/alpha.rs", "other", "alpha")]
                ),
            ]),
            "codegen path conflict: logical path 'src/alpha.rs' claimed by //gen:alpha, //gen:alpha"
        );
        assert_eq!(
            conflict_error(&[
                record_a(),
                record(
                    "//gen:alpha",
                    "rust",
                    vec![entry_with_exec("src/alpha.rs", "src", "alpha", "out/a.rs")]
                ),
            ]),
            "codegen path conflict: logical path 'src/alpha.rs' claimed by //gen:alpha, //gen:alpha"
        );
    }

    #[test]
    fn collect_plan_merges_hashes_and_sorts_deterministically() {
        let beta = shard_bytes(
            "//gen:beta",
            "rust",
            vec![("src/beta.rs", "src", "beta", "")],
        );
        let alpha = shard_bytes(
            "//gen:alpha",
            "rust",
            vec![("src/alpha.rs", "src", "alpha", "")],
        );
        let forward = vec![
            output("//gen:alpha", vec![("/out/a.dxcodegen.pb", alpha.clone())]),
            output("//gen:beta", vec![("/out/b.dxcodegen.pb", beta.clone())]),
        ];
        let reverse = vec![
            output("//gen:beta", vec![("/out/b.dxcodegen.pb", beta)]),
            output("//gen:alpha", vec![("/out/a.dxcodegen.pb", alpha)]),
        ];
        let first = collect_plan(&forward).expect("plan");
        let second = collect_plan(&reverse).expect("plan");
        assert_eq!(first, second);
        assert_eq!(first.records.len(), 2);
        assert_eq!(first.records[0].producer, "//gen:alpha");
        assert_eq!(first.fingerprint, fingerprint(&first.records));
        assert_eq!(first.digest, plan_digest(&first.fingerprint));
        assert_eq!(first.hex(), plan_hex(&first.fingerprint));
        assert_eq!(first.hex().len(), 64);
    }

    #[test]
    fn collect_plan_rejects_conflicts() {
        let outputs = vec![
            output(
                "//gen:alpha",
                vec![(
                    "/out/a.dxcodegen.pb",
                    shard_bytes("//gen:alpha", "rust", vec![("src/same.rs", "src", "a", "")]),
                )],
            ),
            output(
                "//gen:evil",
                vec![(
                    "/out/e.dxcodegen.pb",
                    shard_bytes("//gen:evil", "rust", vec![("src/same.rs", "src", "e", "")]),
                )],
            ),
        ];
        assert_eq!(
            collect_plan(&outputs),
            Err(CollectError::Conflict(
                "codegen path conflict: logical path 'src/same.rs' claimed by //gen:alpha, //gen:evil"
                    .to_owned()
            ))
        );
    }

    #[test]
    fn empty_outputs_select_an_empty_plan() {
        let plan = collect_plan(&[]).expect("empty plan");
        assert!(plan.records.is_empty());
        assert_eq!(plan.fingerprint, "[]");
        assert_eq!(plan.digest, quality_result::digest(b"[]"));
    }

    #[test]
    fn projection_resolves_backed_entries_and_sorts() {
        let outputs = vec![
            output(
                "//gen:beta",
                vec![
                    (
                        "/out/beta.dxcodegen.pb",
                        shard_bytes(
                            "//gen:beta",
                            "rust",
                            vec![
                                ("gen/z.rs", "gen", "z", "z.lib.rs"),
                                ("gen/a.rs", "gen", "a", "a.lib.rs"),
                                ("gen/logical.rs", "gen", "logical", ""),
                            ],
                        ),
                    ),
                    ("/bazel-out/k8-fastbuild/bin/gen/z.lib.rs", vec![1]),
                    ("/bazel-out/k8-fastbuild/bin/gen/a.lib.rs", vec![2]),
                ],
            ),
            output(
                "//gen:alpha",
                vec![(
                    "/out/alpha.dxcodegen.pb",
                    shard_bytes("//gen:alpha", "rust", vec![("gen/m.rs", "gen", "m", "")]),
                )],
            ),
        ];
        let plan = collect_plan(&outputs).expect("plan");
        let projection = plan_projection(&plan.records, &outputs).expect("projection");
        let summary: Vec<(&str, &str)> = projection
            .iter()
            .map(|entry| (entry.logical_path.as_str(), entry.artifact.as_str()))
            .collect();
        // Sorted by logical path; logical-only entries hold no leaf.
        assert_eq!(
            summary,
            vec![
                ("gen/a.rs", "/bazel-out/k8-fastbuild/bin/gen/a.lib.rs"),
                ("gen/z.rs", "/bazel-out/k8-fastbuild/bin/gen/z.lib.rs"),
            ]
        );
        assert_eq!(projection[0].import_root, "gen");
        assert_eq!(projection[0].namespace, "a");
    }

    #[test]
    fn projection_rejects_bad_indexes() {
        // Missing: exec suffix matches no reported artifact.
        let missing_records = vec![record(
            "//gen:beta",
            "rust",
            vec![entry_with_exec("gen/beta.rs", "gen", "beta", "beta.lib.rs")],
        )];
        assert!(matches!(
            plan_projection(&missing_records, &[]),
            Err(CollectError::MissingArtifact { .. })
        ));
        // Unreported: an artifact no entry claims.
        let unreported_outputs = vec![output(
            "//gen:beta",
            vec![
                (
                    "/out/beta.dxcodegen.pb",
                    shard_bytes(
                        "//gen:beta",
                        "rust",
                        vec![("gen/beta.rs", "gen", "beta", "")],
                    ),
                ),
                ("/out/beta.lib.rs", vec![1, 2, 3]),
            ],
        )];
        let logical_records = vec![record(
            "//gen:beta",
            "rust",
            vec![entry("gen/beta.rs", "gen", "beta")],
        )];
        assert!(matches!(
            plan_projection(&logical_records, &unreported_outputs),
            Err(CollectError::UnreportedArtifact { .. })
        ));
    }
}
