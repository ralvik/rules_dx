//! Normalized env plan collection for the `dx` CLI (M25 WP2 slice 3).
//!
//! Contract: `docs/environments/environment.md` (plan collection,
//! provider-selective aspects, private output group) and
//! `docs/cli/commands/environment-codegen-setup.md` (`dx env` scope:
//! no argument invokes canonical `//dx:env`, one exact target label,
//! paths/patterns/multiple labels/profiles/language selectors rejected).
//! Merge, conflict, and fingerprint semantics mirror the frozen Starlark
//! helpers in `//env:plan.bzl` (`env_plan_merge_records`,
//! `env_plan_conflict_error`, `env_plan_fingerprint`) over the binary
//! `DxEnvShard` wire schema (`//env:plan.proto`); shard decode and
//! per-shard validation stay in `env_shard`.
//!
//! The collector reads only BEP-reported artifacts in the requested output
//! group (via `dx_bep`), recognizes shards by the reserved suffix, and
//! never scans `bazel-out`. Byte-identical duplicate records (one record
//! reached through many transitive routes) merge silently; any other
//! second claim on one identity key fails before selection with every
//! claimant listed. The normalized fingerprint binds exec paths and is
//! hashed with BLAKE3-256 through `quality_result` (no algorithm
//! negotiation). Artifact-index validation (missing, ambiguous, or
//! unreported backing artifacts) and projection planning land in the next
//! slice; this crate only collects, merges, and hashes the plan.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use dx_bep::TargetOutput;

/// Private output group carrying collected shards plus every referenced
/// artifact. Frozen in `//env:plan.bzl`; matches
/// `DX_ENV_PLAN_OUTPUT_GROUP`.
pub const OUTPUT_GROUP: &str = "dx_env_plans";

/// Reserved controlled filename suffix recognizing shards among
/// BEP-reported files. Frozen in `//env:plan.bzl`; matches
/// `DX_ENV_SHARD_SUFFIX`.
pub const SHARD_SUFFIX: &str = ".dxenv.pb";

/// Collecting aspect applied to the selected roots. Matches
/// `dx_env_plan_aspect` in `//env:plan.bzl`.
pub const ENV_ASPECT: &str = "//env:plan.bzl%dx_env_plan_aspect";

/// Canonical repository-wide env selection invoked by bare `dx env`.
/// The bootstrap installer owns this label (`//dx/env:env`); this crate
/// only owns the plan-selection identity behind it.
pub const REPOSITORY_TARGET: &str = "//dx:env";

/// Selected env scope: the whole repository or one exact target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvScope {
    Repository,
    Exact(String),
}

/// Exact-target scope failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeError {
    /// More than one positional target: env selects at most one.
    MultipleTargets { count: usize },
    /// A target pattern (`...`, `*`, `?`): patterns never select env.
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

/// Resolves the `dx env` positional scope: empty selects the repository
/// (`//dx:env`), one exact `//` or `@` label selects its configured
/// closure, and anything else fails before execution. A compatible
/// target whose closure contributes no shards selects an empty exact
/// plan downstream, never a failure here.
pub fn resolve_scope(targets: &[String]) -> Result<EnvScope, ScopeError> {
    match targets {
        [] => Ok(EnvScope::Repository),
        [single] => {
            if single.contains("...") || single.contains('*') || single.contains('?') {
                Err(ScopeError::TargetPattern {
                    value: single.clone(),
                })
            } else if single.starts_with("//") || single.starts_with('@') {
                Ok(EnvScope::Exact(single.clone()))
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
pub fn scope_targets(scope: &EnvScope) -> Vec<String> {
    match scope {
        EnvScope::Repository => vec![REPOSITORY_TARGET.to_owned()],
        EnvScope::Exact(label) => vec![label.clone()],
    }
}

/// One normalized identity entry. Empty `exec_path` means a
/// logical-only identity input requiring no materialized artifact;
/// non-empty is the BEP-matching suffix for the backing artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvEntry {
    pub key: String,
    pub value: String,
    pub exec_path: String,
}

/// One contributor record: all entries from one producer in one
/// language integration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvRecord {
    pub producer: String,
    pub integration: String,
    pub entries: Vec<EnvEntry>,
}

/// Shard collection failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectError {
    /// A BEP-reported shard file fails to decode or validate.
    Shard {
        path: String,
        error: env_shard::Error,
    },
    /// Two records claim one identity key incompatibly. The message
    /// matches `env_plan_conflict_error` rendering, listing every
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
/// non-shard backing artifacts (indexed against entry declarations in a
/// later slice). Inputs arrive sorted by label from `dx_bep`;
/// determinism comes from [`merge_records`], never arrival order.
pub fn collect_shards(outputs: &[TargetOutput]) -> Result<Vec<EnvRecord>, CollectError> {
    let mut records = Vec::new();
    for output in outputs {
        for artifact in &output.artifacts {
            if !is_shard_artifact(&artifact.exec_path) {
                continue;
            }
            let shard = env_shard::decode_validated(&artifact.bytes).map_err(|error| {
                CollectError::Shard {
                    path: artifact.exec_path.display().to_string(),
                    error,
                }
            })?;
            records.push(EnvRecord {
                producer: shard.producer,
                integration: shard.integration,
                entries: shard
                    .entries
                    .into_iter()
                    .map(|entry| EnvEntry {
                        key: entry.key,
                        value: entry.value,
                        exec_path: entry.exec_path,
                    })
                    .collect(),
            });
        }
    }
    Ok(records)
}

/// Merge index: owner `(producer, integration)` to entry key
/// `(key, value, exec_path)` to entry.
type MergeIndex<'a> =
    BTreeMap<(&'a str, &'a str), BTreeMap<(&'a str, &'a str, &'a str), &'a EnvEntry>>;

/// Merges records into deterministic normalized order, mirroring
/// `env_plan_merge_records`: byte-identical duplicate entries collapse,
/// surviving records sort by `(producer, integration)` with entries
/// sorted by `(key, value, exec_path)`.
pub fn merge_records(records: &[EnvRecord]) -> Vec<EnvRecord> {
    let mut owners: MergeIndex<'_> = BTreeMap::new();
    for record in records {
        let owned = owners
            .entry((record.producer.as_str(), record.integration.as_str()))
            .or_default();
        for entry in &record.entries {
            owned
                .entry((
                    entry.key.as_str(),
                    entry.value.as_str(),
                    entry.exec_path.as_str(),
                ))
                .or_insert(entry);
        }
    }
    owners
        .into_iter()
        .map(|((producer, integration), entries)| EnvRecord {
            producer: producer.to_owned(),
            integration: integration.to_owned(),
            entries: entries.into_values().cloned().collect(),
        })
        .collect()
}

/// One identity-key claim: owner `(producer, integration)` plus the full
/// entry identity `(key, value, exec_path)`. The alias keeps Clippy's
/// `type_complexity` lint quiet once the tuple grows to five elements.
type Claim<'a> = (&'a str, &'a str, &'a str, &'a str, &'a str);

/// Detects incompatible identity-key claims across records, mirroring
/// `env_plan_conflict_error`: byte-identical duplicates (same producer,
/// integration, key, value, and exec path) merge silently, any other
/// second claim on one key fails, listing every claimant. Returns `""`
/// when conflict-free.
pub fn conflict_error(records: &[EnvRecord]) -> String {
    let mut by_key: BTreeMap<&str, BTreeSet<Claim<'_>>> = BTreeMap::new();
    for record in records {
        for entry in &record.entries {
            by_key.entry(entry.key.as_str()).or_default().insert((
                record.producer.as_str(),
                record.integration.as_str(),
                entry.key.as_str(),
                entry.value.as_str(),
                entry.exec_path.as_str(),
            ));
        }
    }
    let mut conflicts = Vec::new();
    for (key, claims) in &by_key {
        if claims.len() > 1 {
            let claimants: Vec<&str> = claims.iter().map(|claim| claim.0).collect();
            conflicts.push(format!(
                "identity key '{key}' claimed by {}",
                claimants.join(", ")
            ));
        }
    }
    if conflicts.is_empty() {
        String::new()
    } else {
        format!("env plan conflict: {}", conflicts.join("; "))
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
/// `env_plan_fingerprint`: deterministic JSON over the merged records,
/// one object per record with producer, integration, and entries sorted
/// by (key, value, exec path), each entry binding its exec-path suffix.
/// Byte-identical to the Starlark rendering for the same records.
pub fn fingerprint(records: &[EnvRecord]) -> String {
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
            out.push_str("\",\"key\":\"");
            escape_into(&mut out, &entry.key);
            out.push_str("\",\"value\":\"");
            escape_into(&mut out, &entry.value);
            out.push_str("\"}");
        }
        out.push_str("],\"integration\":\"");
        escape_into(&mut out, &record.integration);
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
    pub records: Vec<EnvRecord>,
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
/// (`"[]"`) rather than failing, so compatible targets with no env
/// contribution clear stale selections.
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

    use dx_bep::CollectedArtifact;
    use env_shard::proto::{DxEnvEntry, DxEnvShard};

    fn entry(key: &str, value: &str) -> EnvEntry {
        EnvEntry {
            key: key.to_owned(),
            value: value.to_owned(),
            exec_path: String::new(),
        }
    }

    fn entry_with_exec(key: &str, value: &str, exec_path: &str) -> EnvEntry {
        EnvEntry {
            key: key.to_owned(),
            value: value.to_owned(),
            exec_path: exec_path.to_owned(),
        }
    }

    fn record(producer: &str, integration: &str, entries: Vec<EnvEntry>) -> EnvRecord {
        EnvRecord {
            producer: producer.to_owned(),
            integration: integration.to_owned(),
            entries,
        }
    }

    fn record_a() -> EnvRecord {
        record(
            "//env:alpha",
            "rust",
            vec![entry("runtime", "stable-x86_64")],
        )
    }

    fn record_b() -> EnvRecord {
        record("//env:beta", "rust", vec![entry("abi", "gnu")])
    }

    fn shard_bytes(producer: &str, integration: &str, entries: Vec<(&str, &str, &str)>) -> Vec<u8> {
        let shard = DxEnvShard {
            producer: producer.to_owned(),
            integration: integration.to_owned(),
            entries: entries
                .into_iter()
                .map(|(key, value, exec_path)| DxEnvEntry {
                    key: key.to_owned(),
                    value: value.to_owned(),
                    exec_path: exec_path.to_owned(),
                })
                .collect(),
        };
        env_shard::encode_validated(&shard).expect("valid shard")
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
        assert_eq!(OUTPUT_GROUP, "dx_env_plans");
        assert_eq!(SHARD_SUFFIX, ".dxenv.pb");
        assert_eq!(ENV_ASPECT, "//env:plan.bzl%dx_env_plan_aspect");
        assert_eq!(REPOSITORY_TARGET, "//dx:env");
    }

    #[test]
    fn empty_scope_selects_repository() {
        assert_eq!(resolve_scope(&[]), Ok(EnvScope::Repository));
        assert_eq!(scope_targets(&EnvScope::Repository), vec!["//dx:env"]);
    }

    #[test]
    fn exact_labels_pass_through() {
        for label in ["//app:server", "@rules_dx//env:plan_proto"] {
            let scope = resolve_scope(&[label.to_owned()]).expect("exact label");
            assert_eq!(scope, EnvScope::Exact(label.to_owned()));
            assert_eq!(scope_targets(&scope), vec![label.to_owned()]);
        }
    }

    #[test]
    fn scope_rejects_non_exact_inputs() {
        assert_eq!(
            resolve_scope(&["//a:x".to_owned(), "//b:y".to_owned()]),
            Err(ScopeError::MultipleTargets { count: 2 })
        );
        for pattern in ["//...", "//env/...", "//env:*", "@repo//pkg:all?"] {
            assert_eq!(
                resolve_scope(&[pattern.to_owned()]),
                Err(ScopeError::TargetPattern {
                    value: pattern.to_owned(),
                }),
                "pattern {pattern:?} must fail"
            );
        }
        for other in ["env/alpha", "env", "rust", "--profile=fast", ":relative"] {
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
        assert!(is_shard_artifact(Path::new("/out/a.dxenv.pb")));
        assert!(!is_shard_artifact(Path::new("/out/a.pb")));
        assert!(!is_shard_artifact(Path::new("/out/a.dxenv.pb.txt")));
    }

    #[test]
    fn collect_shards_decodes_and_ignores_non_shards() {
        let outputs = vec![output(
            "//env:beta",
            vec![
                (
                    "/out/beta.dxenv.pb",
                    shard_bytes("//env:beta", "rust", vec![("abi", "gnu", "")]),
                ),
                ("/out/backing.lib.rs", vec![1, 2, 3]),
            ],
        )];
        let records = collect_shards(&outputs).expect("collect");
        assert_eq!(
            records,
            vec![record("//env:beta", "rust", vec![entry("abi", "gnu")])]
        );
    }

    #[test]
    fn collect_shards_preserves_exec_paths() {
        let outputs = vec![output(
            "//env:rust",
            vec![(
                "/out/rust.dxenv.pb",
                shard_bytes(
                    "//env:rust",
                    "rust",
                    vec![("runtime", "stable-x86_64", "env/env_shard/src/lib.rs")],
                ),
            )],
        )];
        let records = collect_shards(&outputs).expect("collect");
        assert_eq!(
            records,
            vec![record(
                "//env:rust",
                "rust",
                vec![entry_with_exec(
                    "runtime",
                    "stable-x86_64",
                    "env/env_shard/src/lib.rs"
                )]
            )]
        );
    }

    #[test]
    fn collect_shards_rejects_invalid_shards_with_path() {
        let outputs = vec![output(
            "//env:evil",
            vec![("/out/evil.dxenv.pb", vec![0xff, 0x00, 0x01])],
        )];
        let error = collect_shards(&outputs).expect_err("invalid shard");
        assert!(matches!(error, CollectError::Shard { .. }));
        assert!(error.to_string().contains("Shard"));
    }

    #[test]
    fn merge_groups_by_owner_and_sorts_entries() {
        let merged = merge_records(&[
            record(
                "//env:b",
                "rust",
                vec![entry("runtime", "stable"), entry("abi", "gnu")],
            ),
            record_a(),
            record_a(),
            record("//env:a", "rust", vec![entry("abi", "gnu")]),
        ]);
        let summary: Vec<(&str, &str, Vec<&str>)> = merged
            .iter()
            .map(|record| {
                (
                    record.producer.as_str(),
                    record.integration.as_str(),
                    record
                        .entries
                        .iter()
                        .map(|entry| entry.key.as_str())
                        .collect(),
                )
            })
            .collect();
        assert_eq!(
            summary,
            vec![
                ("//env:a", "rust", vec!["abi"]),
                ("//env:alpha", "rust", vec!["runtime"]),
                ("//env:b", "rust", vec!["abi", "runtime"]),
            ]
        );
    }

    #[test]
    fn fingerprint_matches_starlark_rendering() {
        assert_eq!(
            fingerprint(&[record_b(), record_a(), record_a()]),
            "[{\"entries\":[{\"exec_path\":\"\",\"key\":\"runtime\",\"value\":\"stable-x86_64\"}],\"integration\":\"rust\",\"producer\":\"//env:alpha\"},\
             {\"entries\":[{\"exec_path\":\"\",\"key\":\"abi\",\"value\":\"gnu\"}],\"integration\":\"rust\",\"producer\":\"//env:beta\"}]"
        );
        assert_eq!(
            fingerprint(&[record(
                "//env:a",
                "rust",
                vec![entry_with_exec("runtime", "stable", "out/rustc")]
            )]),
            "[{\"entries\":[{\"exec_path\":\"out/rustc\",\"key\":\"runtime\",\"value\":\"stable\"}],\"integration\":\"rust\",\"producer\":\"//env:a\"}]"
        );
    }

    #[test]
    fn fingerprint_matches_chain_fixture() {
        let records = vec![
            record("//env:env_shard_beta", "rust", vec![entry("abi", "gnu")]),
            record(
                "//env:env_shard_alpha",
                "rust",
                vec![entry("runtime", "stable-x86_64")],
            ),
        ];
        assert_eq!(
            fingerprint(&records),
            "[{\"entries\":[{\"exec_path\":\"\",\"key\":\"runtime\",\"value\":\"stable-x86_64\"}],\"integration\":\"rust\",\"producer\":\"//env:env_shard_alpha\"},\
             {\"entries\":[{\"exec_path\":\"\",\"key\":\"abi\",\"value\":\"gnu\"}],\"integration\":\"rust\",\"producer\":\"//env:env_shard_beta\"}]"
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
                record("//env:evil", "rust", vec![entry("runtime", "nightly")]),
            ]),
            "env plan conflict: identity key 'runtime' claimed by //env:alpha, //env:evil"
        );
        // Same key with a different exec path is still a conflict: the
        // identity binds inputs to artifacts.
        assert_eq!(
            conflict_error(&[
                record_a(),
                record(
                    "//env:alpha",
                    "rust",
                    vec![entry_with_exec("runtime", "stable-x86_64", "out/rustc")]
                ),
            ]),
            "env plan conflict: identity key 'runtime' claimed by //env:alpha, //env:alpha"
        );
    }

    #[test]
    fn collect_plan_merges_hashes_and_sorts_deterministically() {
        let beta = shard_bytes("//env:beta", "rust", vec![("abi", "gnu", "")]);
        let alpha = shard_bytes(
            "//env:alpha",
            "rust",
            vec![("runtime", "stable-x86_64", "")],
        );
        let forward = vec![
            output("//env:alpha", vec![("/out/a.dxenv.pb", alpha.clone())]),
            output("//env:beta", vec![("/out/b.dxenv.pb", beta.clone())]),
        ];
        let reverse = vec![
            output("//env:beta", vec![("/out/b.dxenv.pb", beta)]),
            output("//env:alpha", vec![("/out/a.dxenv.pb", alpha)]),
        ];
        let first = collect_plan(&forward).expect("plan");
        let second = collect_plan(&reverse).expect("plan");
        assert_eq!(first, second);
        assert_eq!(first.records.len(), 2);
        assert_eq!(first.records[0].producer, "//env:alpha");
        assert_eq!(first.fingerprint, fingerprint(&first.records));
        assert_eq!(first.digest, plan_digest(&first.fingerprint));
        assert_eq!(first.hex(), plan_hex(&first.fingerprint));
        assert_eq!(first.hex().len(), 64);
    }

    #[test]
    fn collect_plan_rejects_conflicts() {
        let outputs = vec![
            output(
                "//env:alpha",
                vec![(
                    "/out/a.dxenv.pb",
                    shard_bytes("//env:alpha", "rust", vec![("runtime", "stable", "")]),
                )],
            ),
            output(
                "//env:evil",
                vec![(
                    "/out/e.dxenv.pb",
                    shard_bytes("//env:evil", "rust", vec![("runtime", "nightly", "")]),
                )],
            ),
        ];
        assert_eq!(
            collect_plan(&outputs),
            Err(CollectError::Conflict(
                "env plan conflict: identity key 'runtime' claimed by //env:alpha, //env:evil"
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
