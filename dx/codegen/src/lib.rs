//! Normalized codegen plan collection for the `dx` CLI (M25 WP1 slice 3).
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
//! never scans `bazel-out`: missing, duplicate, or unreported referenced
//! artifacts fail collection. Byte-identical duplicate records (one record
//! reached through many transitive routes) merge silently; any other
//! second claim on a logical path fails before selection with every
//! claimant listed. The normalized fingerprint is hashed with BLAKE3-256
//! through `quality_result` (no algorithm negotiation).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use dx_bep::TargetOutput;

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeError {
    /// More than one positional target: codegen selects at most one.
    MultipleTargets { count: usize },
    /// A target pattern (`...`, `*`, `?`): patterns never select codegen.
    TargetPattern { value: String },
    /// Anything that is not an exact target label: paths, directories,
    /// profiles/flags, and language selectors.
    NotTargetLabel { value: String },
}

impl std::fmt::Display for ScopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ScopeError {}

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
/// the one exact label.
pub fn scope_targets(scope: &CodegenScope) -> Vec<String> {
    match scope {
        CodegenScope::Repository => vec![REPOSITORY_TARGET.to_owned()],
        CodegenScope::Exact(label) => vec![label.clone()],
    }
}

/// One normalized projection entry. Projections are always read-only
/// context, so no `read_only` bit is carried: the fingerprint renders
/// `read_only: true` unconditionally, matching `codegen_entry`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenEntry {
    pub logical_path: String,
    pub import_root: String,
    pub namespace: String,
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectError {
    /// A BEP-reported shard file fails to decode or validate.
    Shard {
        path: String,
        error: codegen_shard::Error,
    },
    /// Two records claim one logical path incompatibly. The message
    /// matches `codegen_conflict_error` rendering, listing every
    /// claimant: no traversal-order winner is accepted.
    Conflict(String),
}

impl std::fmt::Display for CollectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for CollectError {}

/// Reports whether a BEP-reported artifact path is a plan shard, by
/// reserved filename suffix on the raw path bytes.
pub fn is_shard_artifact(path: &Path) -> bool {
    path.as_os_str()
        .as_encoded_bytes()
        .ends_with(SHARD_SUFFIX.as_bytes())
}

/// Decodes every shard among the BEP-reported artifacts, ignoring
/// non-shard generated artifacts (indexed against shard declarations in
/// a later slice). Inputs arrive sorted by label from `dx_bep`;
/// determinism comes from [`merge_records`], never arrival order.
pub fn collect_shards(outputs: &[TargetOutput]) -> Result<Vec<CodegenRecord>, CollectError> {
    let mut records = Vec::new();
    for output in outputs {
        for artifact in &output.artifacts {
            if !is_shard_artifact(&artifact.exec_path) {
                continue;
            }
            let shard = codegen_shard::decode_validated(&artifact.bytes).map_err(|error| {
                CollectError::Shard {
                    path: artifact.exec_path.display().to_string(),
                    error,
                }
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
                    })
                    .collect(),
            });
        }
    }
    Ok(records)
}

/// Merge index: owner `(producer, language)` to entry key
/// `(logical_path, import_root, namespace)` to entry.
type MergeIndex<'a> =
    BTreeMap<(&'a str, &'a str), BTreeMap<(&'a str, &'a str, &'a str), &'a CodegenEntry>>;

/// Merges records into deterministic normalized order, mirroring
/// `codegen_merge_records`: byte-identical duplicate entries collapse,
/// surviving records sort by `(producer, language)` with entries sorted
/// by `(logical_path, import_root, namespace)`.
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
/// `codegen_conflict_error`: byte-identical duplicates merge silently,
/// any other second claim fails, listing every claimant. Returns `""`
/// when conflict-free.
pub fn conflict_error(records: &[CodegenRecord]) -> String {
    let mut by_path: BTreeMap<&str, BTreeSet<(&str, &str, &str, &str)>> = BTreeMap::new();
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
/// sorted by logical path. Byte-identical to the Starlark rendering for
/// the same records (pinned by the chain/prost fixture fingerprints).
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
            out.push_str("{\"import_root\":\"");
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
    quality_result::digest(fingerprint.as_bytes())
}

/// Lowercase hex of the plan digest, for operator messaging.
pub fn plan_hex(fingerprint: &str) -> String {
    plan_digest(fingerprint)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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
        self.digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
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
    let digest = quality_result::digest(rendered.as_bytes());
    Ok(CollectedPlan {
        records: merged,
        fingerprint: rendered,
        digest,
    })
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

    fn shard_bytes(producer: &str, language: &str, entries: Vec<(&str, &str, &str)>) -> Vec<u8> {
        let shard = DxCodegenShard {
            producer: producer.to_owned(),
            language: language.to_owned(),
            entries: entries
                .into_iter()
                .map(|(logical_path, import_root, namespace)| DxCodegenEntry {
                    logical_path: logical_path.to_owned(),
                    import_root: import_root.to_owned(),
                    namespace: namespace.to_owned(),
                    read_only: true,
                })
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
    fn collect_shards_ignores_generated_artifacts() {
        let outputs = vec![output(
            "//gen:beta",
            vec![
                (
                    "/out/beta.dxcodegen.pb",
                    shard_bytes("//gen:beta", "rust", vec![("gen/beta.rs", "gen", "beta")]),
                ),
                ("/out/beta.lib.rs", vec![1, 2, 3]),
            ],
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
            "[{\"entries\":[{\"import_root\":\"src\",\"logical_path\":\"src/alpha.rs\",\"namespace\":\"alpha\",\"read_only\":true}],\"language\":\"rust\",\"producer\":\"//gen:alpha\"},\
             {\"entries\":[{\"import_root\":\"src\",\"logical_path\":\"src/beta.rs\",\"namespace\":\"beta\",\"read_only\":true}],\"language\":\"rust\",\"producer\":\"//gen:beta\"}]"
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
            "[{\"entries\":[{\"import_root\":\"gen\",\"logical_path\":\"gen/alpha.rs\",\"namespace\":\"alpha\",\"read_only\":true}],\"language\":\"rust\",\"producer\":\"//generation:codegen_shard_alpha\"},\
             {\"entries\":[{\"import_root\":\"gen\",\"logical_path\":\"gen/beta.rs\",\"namespace\":\"beta\",\"read_only\":true}],\"language\":\"rust\",\"producer\":\"//generation:codegen_shard_beta\"}]"
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
    }

    #[test]
    fn collect_plan_merges_hashes_and_sorts_deterministically() {
        let beta = shard_bytes("//gen:beta", "rust", vec![("src/beta.rs", "src", "beta")]);
        let alpha = shard_bytes(
            "//gen:alpha",
            "rust",
            vec![("src/alpha.rs", "src", "alpha")],
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
                    shard_bytes("//gen:alpha", "rust", vec![("src/same.rs", "src", "a")]),
                )],
            ),
            output(
                "//gen:evil",
                vec![(
                    "/out/e.dxcodegen.pb",
                    shard_bytes("//gen:evil", "rust", vec![("src/same.rs", "src", "e")]),
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
}
