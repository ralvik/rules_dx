//! Normalized env plan collection for the `dx` CLI.
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

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use dx_bep::TargetOutput;
use dx_digest::blake3 as digest;
use dx_roots::{
    build_argv, invocation_targets, repository_plan, resolve_exact_target, ExactScopeError,
    RepositoryRootPlan,
};
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
///
/// Single-source scope validation for `#651`: the `...`/`*`/`?` and
/// `//`/`@` checks live in [`dx_roots::resolve_exact_target`]; this keeps
/// the noun-specific `EnvScope`/`ScopeError` while sharing the logic with
/// `dx_setup` and `dx_codegen` (intentional divergence: distinct scope
/// types and canonical targets per command).
pub fn resolve_scope(targets: &[String]) -> Result<EnvScope, ScopeError> {
    match resolve_exact_target(targets) {
        Ok(None) => Ok(EnvScope::Repository),
        Ok(Some(label)) => Ok(EnvScope::Exact(label)),
        Err(ExactScopeError::MultipleTargets { count }) => {
            Err(ScopeError::MultipleTargets { count })
        }
        Err(ExactScopeError::TargetPattern { value }) => Err(ScopeError::TargetPattern { value }),
        Err(ExactScopeError::NotTargetLabel { value }) => Err(ScopeError::NotTargetLabel { value }),
    }
}

/// Bazel labels to build for `scope`: the canonical repository target or
/// the one exact label. The repository arm composes the WP4
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
///
/// Single-source plan helper for `#651`: shares the baseline/pattern-file
/// policy with `dx_codegen` and `dx_setup` via [`dx_roots::invocation_targets`].
pub fn targets_for_root_plan(plan: &RepositoryRootPlan) -> Vec<String> {
    invocation_targets(plan, REPOSITORY_TARGET)
}

/// Full `bazel build` command line for a WP4 root plan: `build` plus the
/// plan roots, the collecting aspect, and the private output group (plus
/// `--target_pattern_file` when the plan carries a pattern file).
///
/// Single-source argv helper for `#651`: shares assembly with `dx_codegen`
/// and `dx_setup` via [`dx_roots::build_argv`].
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
/// Shared index plumbing; only the suffix is plan-specific.
/// See: `cli/bep/src/lib.rs` (`dx_bep::is_shard_artifact`).
pub fn is_shard_artifact(path: &Path) -> bool {
    dx_bep::is_shard_artifact(path, SHARD_SUFFIX)
}

/// Reports whether a BEP-reported artifact path satisfies an entry's
/// exec suffix: exact equality or a "/"-boundary suffix match, so
/// different output bases still resolve without scanning `bazel-out`.
/// Mirrors `_exec_matches` in `//env:plan.bzl`.
/// Shared index plumbing. See: `cli/bep/src/lib.rs` (`dx_bep::exec_matches`).
pub fn exec_matches(artifact_path: &Path, exec_path: &str) -> bool {
    dx_bep::exec_matches(artifact_path, exec_path)
}

/// Decodes every shard among the BEP-reported artifacts and validates
/// the artifact index against entry declarations: each non-empty entry
/// exec suffix must resolve to exactly one non-shard BEP artifact, and
/// every non-shard BEP artifact must be claimed by at least one entry
/// (sharing is allowed). Logical-only entries (empty exec) require no
/// artifact. Inputs arrive sorted by label from `dx_bep`; determinism
/// comes from [`merge_records`], never arrival order.
///
/// Keep: the codegen collector shares the shard/index plumbing via
/// `dx_bep` but keeps its own decode plus duplicate-claim rejection
/// (env allows sharing; codegen is a closed manifest).
/// See: `cli/codegen/src/lib.rs` (`collect_shards`).
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
/// Shared index plumbing. See: `cli/bep/src/lib.rs`.
fn backing_artifact_paths(outputs: &[TargetOutput]) -> Vec<String> {
    dx_bep::non_shard_artifact_paths(outputs, SHARD_SUFFIX)
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
            .filter(|path| dx_bep::suffix_matches(path, &entry.exec_path))
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
            .any(|(_, entry)| dx_bep::suffix_matches(path, &entry.exec_path))
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
///
/// Keep: the codegen fingerprint shares the JSON owner
/// (`dx_fingerprint::to_json`) but keeps its own view shape (env
/// keys on integration plus key/value; codegen on language plus
/// logical path, import root, namespace).
/// See: `cli/codegen/src/lib.rs` (`fingerprint`).
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
#[path = "lib_tests.rs"]
mod lib_tests;
