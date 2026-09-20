//! Normalized env plan collection for the `dx` CLI (issue #506 WP2 slice 4).
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
//! never scans `bazel-out`. Each non-empty entry `exec_path` suffix must
//! resolve to exactly one BEP-reported non-shard artifact (suffix match
//! on "/" boundaries so output bases differ); missing or ambiguous
//! suffixes fail, and every non-shard BEP artifact must be claimed by at
//! least one entry, otherwise collection fails with unreported before
//! projection planning. Unlike codegen, two identity keys may share one
//! backing artifact (selective identity dimensions, not a closed file
//! manifest), so no duplicate-claim rejection applies. Empty exec paths
//! are logical-only identity inputs requiring no artifact.
//! Byte-identical duplicate records (one record reached through many
//! transitive routes) merge silently; any other second claim on one
//! identity key fails before selection with every claimant listed. The
//! normalized fingerprint binds exec paths and is hashed with BLAKE3-256
//! through `dx_digest` (no algorithm negotiation).
//! [`plan_projection`] resolves the merged plan to deterministic
//! key-sorted backing leaves (`key`/`value` to full BEP artifact path)
//! for setup to commit.

// Issue #238: infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use dx_bep::TargetOutput;
use dx_digest::blake3 as digest;
use dx_roots::{build_argv, invocation_targets, repository_plan, RepositoryRootPlan};
use env_shard::{decode_validated, Error};
use serde::Serialize;

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
/// The bootstrap installer owns this label (`//cli/env:env`); this crate
/// only owns the plan-selection identity behind it.
pub const REPOSITORY_TARGET: &str = "//dx:env";

/// Selected env scope: the whole repository or one exact target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvScope {
    Repository,
    Exact(String),
}

/// Exact-target scope failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ScopeError {
    /// More than one positional target: env selects at most one.
    #[error("expected at most one target, found {count}")]
    MultipleTargets { count: usize },
    /// A target pattern (`...`, `*`, `?`): patterns never select env.
    #[error("invalid target {value:?}: patterns never select env")]
    TargetPattern { value: String },
    /// Anything that is not an exact target label: paths, directories,
    /// profiles/flags, and language selectors.
    #[error("invalid target {value:?}: want an exact // or @ label")]
    NotTargetLabel { value: String },
}

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
/// the one exact label. The repository arm composes the WP4 (issue #506)
/// [`dx_roots::repository_plan`] (still the `//...` baseline) behind the
/// `//dx:env` selection identity; exact scopes bypass root selection.
pub fn scope_targets(scope: &EnvScope) -> Vec<String> {
    match scope {
        EnvScope::Repository => targets_for_root_plan(&repository_plan()),
        EnvScope::Exact(label) => vec![label.clone()],
    }
}

/// Bazel labels to build for a WP4 root plan behind `//dx:env`: the
/// baseline plan keeps the canonical selection identity while the
/// fiat selection stands; every other candidate passes its own roots through.
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
        &[ENV_ASPECT.to_owned()],
        &[OUTPUT_GROUP.to_owned()],
    )
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
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CollectError {
    /// A BEP-reported shard file fails to decode or validate.
    #[error("invalid shard {path:?}: {error}")]
    Shard { path: String, error: Error },
    /// Two records claim one identity key incompatibly. The message
    /// matches `env_plan_conflict_error` rendering, listing every
    /// claimant: no traversal-order winner is accepted.
    #[error("{0}")]
    Conflict(String),
    /// An entry's non-empty exec suffix matches no BEP-reported
    /// non-shard artifact.
    #[error("missing artifact for {producer} {key:?}: no BEP artifact matches {exec_path:?}")]
    MissingArtifact {
        producer: String,
        key: String,
        exec_path: String,
    },
    /// An exec suffix matches more than one BEP-reported non-shard
    /// artifact. `artifacts` holds the colliding full BEP paths.
    /// Sharing (several keys bound to one artifact) is allowed and
    /// never reports this variant.
    #[error("ambiguous artifact {exec_path:?}: matches {artifacts:?}")]
    AmbiguousArtifact {
        exec_path: String,
        artifacts: Vec<String>,
    },
    /// A BEP-reported non-shard artifact is claimed by no entry.
    #[error("unreported artifact {path:?}: claimed by no entry")]
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
/// Mirrors `_exec_matches` in `//env:plan.bzl`.
pub fn exec_matches(artifact_path: &Path, exec_path: &str) -> bool {
    if exec_path.is_empty() {
        return false;
    }
    let rendered = artifact_path.as_os_str().to_string_lossy();
    rendered.as_ref() == exec_path || rendered.ends_with(&format!("/{exec_path}"))
}

/// Decodes every shard among the BEP-reported artifacts and validates
/// the artifact index against entry declarations: each non-empty entry
/// exec suffix must resolve to exactly one non-shard BEP artifact, and
/// every non-shard BEP artifact must be claimed by at least one entry
/// (sharing is allowed). Logical-only entries (empty exec) require no
/// artifact. Inputs arrive sorted by label from `dx_bep`; determinism
/// comes from [`merge_records`], never arrival order.
pub fn collect_shards(outputs: &[TargetOutput]) -> Result<Vec<EnvRecord>, CollectError> {
    let backing_paths = backing_artifact_paths(outputs);
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
    let _ = index_artifacts(&records, &backing_paths)?;
    Ok(records)
}

/// Lists every BEP-reported non-shard artifact path: the backing files
/// the entry `exec_path` suffixes index into. Shard files are
/// recognized by the reserved suffix and excluded.
fn backing_artifact_paths(outputs: &[TargetOutput]) -> Vec<String> {
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

/// Validates the artifact index and resolves every exec-bound entry to
/// its backing artifact: each non-empty entry exec suffix must resolve
/// to exactly one non-shard BEP artifact, and every non-shard BEP
/// artifact must be claimed by at least one entry. Sharing (several
/// keys bound to one artifact) is allowed: selective identity
/// dimensions are not a closed file manifest, so no duplicate-claim
/// rejection applies. Returns the resolution map from
/// `(producer, key)` to the full BEP-reported artifact path;
/// logical-only entries (empty exec) resolve to no artifact and hold no
/// map entry.
fn index_artifacts<'a>(
    records: &'a [EnvRecord],
    backing_paths: &[String],
) -> Result<BTreeMap<(&'a str, &'a str), String>, CollectError> {
    let bound: Vec<(&EnvRecord, &EnvEntry)> = records
        .iter()
        .flat_map(|record| record.entries.iter().map(move |entry| (record, entry)))
        .filter(|(_, entry)| !entry.exec_path.is_empty())
        .collect();
    let mut resolved: BTreeMap<(&str, &str), String> = BTreeMap::new();
    for (record, entry) in &bound {
        let matches: Vec<&String> = backing_paths
            .iter()
            .filter(|path| suffix_matches(path, &entry.exec_path))
            .collect();
        if matches.is_empty() {
            return Err(CollectError::MissingArtifact {
                producer: record.producer.clone(),
                key: entry.key.clone(),
                exec_path: entry.exec_path.clone(),
            });
        }
        if matches.len() > 1 {
            return Err(CollectError::AmbiguousArtifact {
                exec_path: entry.exec_path.clone(),
                artifacts: matches.into_iter().map(|path| (*path).clone()).collect(),
            });
        }
        resolved.insert(
            (record.producer.as_str(), entry.key.as_str()),
            (*matches[0]).clone(),
        );
    }
    for path in backing_paths {
        if !bound
            .iter()
            .any(|(_, entry)| suffix_matches(path, &entry.exec_path))
        {
            return Err(CollectError::UnreportedArtifact { path: path.clone() });
        }
    }
    Ok(resolved)
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

/// Serializable fingerprint view: field order matches the frozen
/// Starlark `env_plan_fingerprint` (`entries`, `integration`,
/// `producer`; entry `exec_path`, `key`, `value`) so
/// `serde_json::to_string` stays byte-identical for the pinned
/// fixtures while gaining correct escaping.
#[derive(Serialize)]
struct FingerprintEntry<'a> {
    exec_path: &'a str,
    key: &'a str,
    value: &'a str,
}

#[derive(Serialize)]
struct FingerprintRecord<'a> {
    entries: Vec<FingerprintEntry<'a>>,
    integration: &'a str,
    producer: &'a str,
}

/// Renders the normalized complete-plan hash input, mirroring
/// `env_plan_fingerprint`: deterministic JSON over the merged records,
/// one object per record with producer, integration, and entries sorted
/// by (key, value, exec path), each entry binding its exec-path suffix.
/// Byte-identical to the Starlark rendering for the same records.
pub fn fingerprint(records: &[EnvRecord]) -> String {
    let merged = merge_records(records);
    let view: Vec<FingerprintRecord<'_>> = merged
        .iter()
        .map(|record| FingerprintRecord {
            entries: record
                .entries
                .iter()
                .map(|entry| FingerprintEntry {
                    exec_path: entry.exec_path.as_str(),
                    key: entry.key.as_str(),
                    value: entry.value.as_str(),
                })
                .collect(),
            integration: record.integration.as_str(),
            producer: record.producer.as_str(),
        })
        .collect();
    dx_fingerprint::to_json(&view)
}

/// BLAKE3-256 over the normalized fingerprint bytes: the complete-plan
/// identity, with no algorithm negotiation. Routed through `dx_digest` so
/// the digest algorithm has one owner.
pub fn plan_digest(fingerprint: &str) -> [u8; 32] {
    digest(fingerprint.as_bytes())
}

/// Lowercase hex of the plan digest, for operator messaging.
pub fn plan_hex(fingerprint: &str) -> String {
    dx_digest::to_hex(&plan_digest(fingerprint))
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
        dx_digest::to_hex(&self.digest)
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
    let digest = digest(rendered.as_bytes());
    Ok(CollectedPlan {
        records: merged,
        fingerprint: rendered,
        digest,
    })
}

/// One planned backing leaf: the deterministic identity key and value
/// plus the full BEP-reported artifact path backing the exec-bound
/// entry. Logical-only entries (empty exec) carry no backing artifact
/// and hold no projection entry: the projection links backed identity
/// inputs only. Producer and integration stay in
/// [`CollectedPlan::records`] for adapter partitioning; the leaf keeps
/// the address (`key`), its value, and the artifact, mirroring the
/// codegen mirror leaf shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionEntry {
    pub key: String,
    pub value: String,
    pub artifact: String,
}

/// Plans the backing leaves from merged records and BEP-reported
/// outputs: resolves every exec-bound entry to its unique backing
/// artifact through the same index [`collect_shards`] validates
/// (missing, ambiguous, or unreported artifacts fail here too), skips
/// logical-only entries, and sorts by key so setup commits one
/// deterministic leaf set. Sharing (several keys bound to one
/// artifact) resolves each key to the shared path. Callers pass the
/// merged [`CollectedPlan::records`]; conflicts must already be
/// rejected by [`conflict_error`] before selection.
pub fn plan_projection(
    records: &[EnvRecord],
    outputs: &[TargetOutput],
) -> Result<Vec<ProjectionEntry>, CollectError> {
    let backing_paths = backing_artifact_paths(outputs);
    let resolved = index_artifacts(records, &backing_paths)?;
    let mut projection: Vec<ProjectionEntry> = Vec::new();
    for record in records {
        for entry in &record.entries {
            if entry.exec_path.is_empty() {
                continue;
            }
            let artifact = resolved
                .get(&(record.producer.as_str(), entry.key.as_str()))
                .cloned()
                .unwrap_or_else(|| entry.exec_path.clone());
            projection.push(ProjectionEntry {
                key: entry.key.clone(),
                value: entry.value.clone(),
                artifact,
            });
        }
    }
    projection.sort_by(|left, right| left.key.cmp(&right.key));
    Ok(projection)
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
    fn root_plan_composes_baseline_invocation() {
        let plan = dx_roots::repository_plan();
        assert_eq!(targets_for_root_plan(&plan), vec!["//dx:env"]);
        assert_eq!(
            build_argv_for_plan(&plan),
            vec![
                "build".to_owned(),
                "//dx:env".to_owned(),
                format!("--aspects={ENV_ASPECT}"),
                format!("--output_groups={OUTPUT_GROUP}"),
            ]
        );
    }

    #[test]
    fn root_plan_passes_aggregate_roots_through() {
        let plan = dx_roots::RepositoryRootPlan::monolithic_aggregate("//dx:env_roots");
        assert_eq!(
            targets_for_root_plan(&plan),
            vec!["//dx:env_roots".to_owned()]
        );
        assert_eq!(build_argv_for_plan(&plan)[1], "//dx:env_roots".to_owned());
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
            .contains("at most one target"));
    }

    #[test]
    fn shard_suffix_matches_on_path_bytes() {
        assert!(is_shard_artifact(Path::new("/out/a.dxenv.pb")));
        assert!(!is_shard_artifact(Path::new("/out/a.pb")));
        assert!(!is_shard_artifact(Path::new("/out/a.dxenv.pb.txt")));
    }

    #[test]
    fn exec_suffix_matches_on_component_boundaries() {
        assert!(exec_matches(Path::new("/out/rustc"), "rustc"));
        assert!(exec_matches(
            Path::new("/bazel-out/k8-fastbuild/bin/env/rustc"),
            "env/rustc"
        ));
        assert!(!exec_matches(Path::new("/out/other"), "rustc"));
        // Suffix matches only on "/" boundaries, never mid-segment.
        assert!(!exec_matches(Path::new("/out/xrustc"), "rustc"));
        assert!(!exec_matches(Path::new("/out/rustc"), ""));
    }

    #[test]
    fn collect_shards_accepts_logical_only_without_artifacts() {
        let outputs = vec![output(
            "//env:beta",
            vec![(
                "/out/beta.dxenv.pb",
                shard_bytes("//env:beta", "rust", vec![("abi", "gnu", "")]),
            )],
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
            vec![
                (
                    "/out/rust.dxenv.pb",
                    shard_bytes(
                        "//env:rust",
                        "rust",
                        vec![("runtime", "stable-x86_64", "env/env_shard/src/lib.rs")],
                    ),
                ),
                (
                    "/bazel-out/k8-fastbuild/bin/env/env_shard/src/lib.rs",
                    vec![1, 2, 3],
                ),
            ],
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
    fn collect_shards_binds_exec_suffix_to_one_artifact() {
        let outputs = vec![output(
            "//env:rust",
            vec![
                (
                    "/out/rust.dxenv.pb",
                    shard_bytes(
                        "//env:rust",
                        "rust",
                        vec![("toolchain", "1.89.0", "toolchain/rustc")],
                    ),
                ),
                ("/bazel-out/k8-fastbuild/bin/env/toolchain/rustc", vec![7]),
            ],
        )];
        let records = collect_shards(&outputs).expect("collect");
        assert_eq!(
            records,
            vec![record(
                "//env:rust",
                "rust",
                vec![entry_with_exec("toolchain", "1.89.0", "toolchain/rustc")]
            )]
        );
    }

    #[test]
    fn collect_shards_rejects_missing_ambiguous_and_unreported() {
        // Missing: exec suffix matches no reported artifact.
        let missing = vec![output(
            "//env:rust",
            vec![(
                "/out/rust.dxenv.pb",
                shard_bytes(
                    "//env:rust",
                    "rust",
                    vec![("toolchain", "1.89.0", "toolchain/rustc")],
                ),
            )],
        )];
        assert_eq!(
            collect_shards(&missing),
            Err(CollectError::MissingArtifact {
                producer: "//env:rust".to_owned(),
                key: "toolchain".to_owned(),
                exec_path: "toolchain/rustc".to_owned(),
            })
        );
        // Ambiguous: one suffix matches two reported artifacts.
        let ambiguous = vec![output(
            "//env:rust",
            vec![
                (
                    "/out/rust.dxenv.pb",
                    shard_bytes(
                        "//env:rust",
                        "rust",
                        vec![("toolchain", "1.89.0", "toolchain/rustc")],
                    ),
                ),
                ("/out/a/toolchain/rustc", vec![1]),
                ("/out/b/toolchain/rustc", vec![2]),
            ],
        )];
        assert!(matches!(
            collect_shards(&ambiguous),
            Err(CollectError::AmbiguousArtifact { .. })
        ));
        // Unreported: a non-shard artifact no entry claims.
        let unreported = vec![output(
            "//env:beta",
            vec![
                (
                    "/out/beta.dxenv.pb",
                    shard_bytes("//env:beta", "rust", vec![("abi", "gnu", "")]),
                ),
                ("/out/backing.lib.rs", vec![1, 2, 3]),
            ],
        )];
        assert_eq!(
            collect_shards(&unreported),
            Err(CollectError::UnreportedArtifact {
                path: "/out/backing.lib.rs".to_owned(),
            })
        );
    }

    #[test]
    fn collect_shards_allows_shared_backing_artifact() {
        // Selective identity dimensions may share one backing artifact:
        // no duplicate-claim rejection, unlike the codegen file mirror.
        let outputs = vec![output(
            "//env:rust",
            vec![
                (
                    "/out/rust.dxenv.pb",
                    shard_bytes(
                        "//env:rust",
                        "rust",
                        vec![
                            ("toolchain", "1.89.0", "shared/rustc"),
                            ("runtime", "stable", "shared/rustc"),
                        ],
                    ),
                ),
                ("/out/shared/rustc", vec![1]),
            ],
        )];
        let records = collect_shards(&outputs).expect("shared backing");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].entries.len(), 2);
    }

    #[test]
    fn collect_shards_rejects_invalid_shards_with_path() {
        let outputs = vec![output(
            "//env:evil",
            vec![("/out/evil.dxenv.pb", vec![0xff, 0x00, 0x01])],
        )];
        let error = collect_shards(&outputs).expect_err("invalid shard");
        assert!(matches!(error, CollectError::Shard { .. }));
        assert!(error.to_string().contains("invalid shard"));
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
    fn fingerprint_escapes_quotes_newlines_and_controls() {
        let records = vec![EnvRecord {
            producer: "//env:\"a\"\n".to_owned(),
            integration: "rust".to_owned(),
            entries: vec![EnvEntry {
                key: "k\"ey".to_owned(),
                value: "v\nalue\u{1}".to_owned(),
                exec_path: "out\\bin".to_owned(),
            }],
        }];
        let rendered = fingerprint(&records);
        assert!(rendered.contains("\\\""), "{rendered}");
        assert!(rendered.contains("\\n"), "{rendered}");
        assert!(rendered.contains("\\\\"), "{rendered}");
        assert!(rendered.contains("\\u0001"), "{rendered}");
        let parsed: serde_json::Value =
            serde_json::from_str(&rendered).expect("fingerprint is valid JSON");
        assert_eq!(
            parsed[0]["entries"][0]["value"],
            serde_json::Value::String("v\nalue\u{1}".to_owned())
        );
        assert_eq!(
            serde_json::to_string(&parsed).expect("reserialize"),
            rendered
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
        assert_eq!(plan.digest, dx_digest::blake3(b"[]"));
    }

    #[test]
    fn projection_resolves_backed_entries_and_sorts() {
        let outputs = vec![
            output(
                "//env:rust",
                vec![
                    (
                        "/out/rust.dxenv.pb",
                        shard_bytes(
                            "//env:rust",
                            "rust",
                            vec![
                                ("toolchain", "1.89.0", "toolchain/rustc"),
                                ("abi", "gnu", ""),
                                ("runtime", "stable", "env/runtime.json"),
                            ],
                        ),
                    ),
                    ("/bazel-out/k8-fastbuild/bin/env/toolchain/rustc", vec![1]),
                    ("/bazel-out/k8-fastbuild/bin/env/env/runtime.json", vec![2]),
                ],
            ),
            output(
                "//env:beta",
                vec![(
                    "/out/beta.dxenv.pb",
                    shard_bytes("//env:beta", "rust", vec![("extra", "1", "")]),
                )],
            ),
        ];
        let plan = collect_plan(&outputs).expect("plan");
        let projection = plan_projection(&plan.records, &outputs).expect("projection");
        let summary: Vec<(&str, &str)> = projection
            .iter()
            .map(|leaf| (leaf.key.as_str(), leaf.artifact.as_str()))
            .collect();
        // Sorted by key; logical-only entries hold no leaf.
        assert_eq!(
            summary,
            vec![
                (
                    "runtime",
                    "/bazel-out/k8-fastbuild/bin/env/env/runtime.json"
                ),
                (
                    "toolchain",
                    "/bazel-out/k8-fastbuild/bin/env/toolchain/rustc"
                ),
            ]
        );
        assert_eq!(projection[0].value, "stable");
        assert_eq!(projection[1].value, "1.89.0");
    }

    #[test]
    fn projection_shares_one_artifact_across_keys() {
        let outputs = vec![output(
            "//env:rust",
            vec![
                (
                    "/out/rust.dxenv.pb",
                    shard_bytes(
                        "//env:rust",
                        "rust",
                        vec![
                            ("b-key", "2", "shared/rustc"),
                            ("a-key", "1", "shared/rustc"),
                        ],
                    ),
                ),
                ("/out/shared/rustc", vec![1]),
            ],
        )];
        let plan = collect_plan(&outputs).expect("plan");
        let projection = plan_projection(&plan.records, &outputs).expect("projection");
        assert_eq!(projection.len(), 2);
        assert_eq!(projection[0].key, "a-key");
        assert_eq!(projection[1].key, "b-key");
        assert_eq!(projection[0].artifact, "/out/shared/rustc");
        assert_eq!(projection[1].artifact, "/out/shared/rustc");
    }

    #[test]
    fn projection_rejects_bad_indexes() {
        // Missing: exec suffix matches no reported artifact.
        let missing_records = vec![record(
            "//env:rust",
            "rust",
            vec![entry_with_exec("toolchain", "1.89.0", "toolchain/rustc")],
        )];
        assert!(matches!(
            plan_projection(&missing_records, &[]),
            Err(CollectError::MissingArtifact { .. })
        ));
        // Unreported: an artifact no entry claims.
        let unreported_outputs = vec![output(
            "//env:beta",
            vec![
                (
                    "/out/beta.dxenv.pb",
                    shard_bytes("//env:beta", "rust", vec![("abi", "gnu", "")]),
                ),
                ("/out/backing.lib.rs", vec![1, 2, 3]),
            ],
        )];
        let logical_records = vec![record("//env:beta", "rust", vec![entry("abi", "gnu")])];
        assert!(matches!(
            plan_projection(&logical_records, &unreported_outputs),
            Err(CollectError::UnreportedArtifact { .. })
        ));
    }
}
