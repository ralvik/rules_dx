//! Normalized codegen plan collection for the `dx` CLI.
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
//! normalized fingerprint binds exec paths and the replacement contract
//! and is hashed with BLAKE3-256 through `dx_digest` (no algorithm
//! negotiation). [`plan_projection`] resolves the merged plan to
//! deterministic mirror leaves (`logical_path` to full BEP artifact path
//! plus the replacement contract) for setup to commit.

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(
    not(test),
    deny(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::unreachable,
        clippy::todo
    )
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use codegen_shard::{decode_validated, Error};
use dx_bep::TargetOutput;
use dx_digest::blake3 as digest;
use dx_roots::{
    build_argv, invocation_targets, repository_plan, resolve_exact_target, ExactScopeError,
    RepositoryRootPlan,
};
use serde::Serialize;

/// Private output group carrying collected shards plus every generated
/// artifact referenced by them. Frozen; matches
/// `DX_CODEGEN_PLAN_OUTPUT_GROUP` in `//generation:codegen.bzl`.
pub const OUTPUT_GROUP: &str = "dx_codegen_plans";

/// Reserved controlled filename suffix recognizing shards among
/// BEP-reported files. Frozen; matches
/// `DX_CODEGEN_SHARD_SUFFIX` in `//generation:codegen.bzl`.
pub const SHARD_SUFFIX: &str = ".dxcodegen.pb";

/// Collecting aspect applied to the selected roots. Matches
/// `dx_codegen_plan_aspect` in `//generation:codegen.bzl`.
pub const CODEGEN_ASPECT: &str = "//generation:codegen.bzl%dx_codegen_plan_aspect";

/// Canonical repository-wide codegen selection invoked by bare
/// `dx codegen`. The effective Bazel roots behind this label stay
/// frozen to the //... baseline by fiat per ADR 0022 (no standing benchmarks); this crate only
/// owns the selection identity.
pub const REPOSITORY_TARGET: &str = "//dx:codegen";

/// Kind filter for the bare-schema reverse-dependent expansion query:
/// every registered generated-language projection is a `*_codegen_shard`
/// rule (`dx_codegen_shard`, `prost_codegen_shard`). Matches the
/// `kind('... rule', ...)` vocabulary the scope resolvers use for
/// `_test`/`_binary` mapping.
/// See: `docs/environments/codegen.md` (bare-schema expansion).
pub const EXPANSION_KIND_FILTER: &str = ".*codegen_shard rule";

/// Builds the unconfigured `bazel query` expression expanding one exact
/// target to its registered projection reverse dependents: every
/// `*_codegen_shard` rule transitively depending on `schema` in `//...`.
/// Transitive (no depth bound) so a bare `proto_library` reaches its
/// shards through intermediate `rust_prost_library` edges; an aspect
/// cannot traverse reverse deps, so this query feeds back to analysis.
/// See: `docs/environments/codegen.md` (bare-schema expansion).
pub fn expansion_expression(schema: &str) -> String {
    format!(
        "kind('{EXPANSION_KIND_FILTER}', rdeps(//..., set({})))",
        quote_label(schema)
    )
}

/// Quotes one label as a double-quoted query string literal, escaping
/// backslashes and quotes. Single owner for Bazel query string literals;
/// the scope resolver delegates here so expansion and ownership
/// expressions stay stable and inspectable.
///
/// Labels never contain control characters (alphabet `[A-Za-z0-9/_:.\-@]`
/// plus `"`/`\` in pathological tests): for that alphabet the hand
/// escaper matches `serde_json::to_string` byte-for-byte (pinned by
/// `quote_matches_json_for_label_alphabet`).
/// See: `docs/cli/target-resolution.md` (query safety).
pub fn quote_label(label: &str) -> String {
    let mut quoted = String::with_capacity(label.len() + 2);
    quoted.push('"');
    for ch in label.chars() {
        match ch {
            '\\' => quoted.push_str("\\\\"),
            '"' => quoted.push_str("\\\""),
            _ => quoted.push(ch),
        }
    }
    quoted.push('"');
    quoted
}

/// Merges one exact target with its queried projections into the
/// deterministic Bazel roots to analyze: the sorted deduplicated union
/// of `schema` plus `projections`. An empty projection set keeps the
/// single label so a bare schema with no consumers still selects its
/// own (empty) exact closure instead of failing; a consumer or shard
/// keeps its own closure plus any downstream shards (deduped, so the
/// merged plan is unchanged). Callers pass the `run_label_query` output
/// for [`expansion_expression`].
/// See: `docs/environments/codegen.md` (bare-schema expansion).
pub fn expand_roots(schema: &str, projections: &[String]) -> Vec<String> {
    let mut roots: Vec<String> = Vec::with_capacity(1 + projections.len());
    roots.push(schema.to_owned());
    roots.extend(projections.iter().cloned());
    roots.sort();
    roots.dedup();
    roots
}

/// Selected codegen scope: the whole repository or one exact target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodegenScope {
    Repository,
    Exact(String),
}

/// Exact-target scope failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ScopeError {
    /// More than one positional target: codegen selects at most one.
    #[error("expected at most one target, found {count}")]
    MultipleTargets { count: usize },
    /// A target pattern (`...`, `*`, `?`): patterns never select codegen.
    #[error("invalid target {value:?}: patterns never select codegen")]
    TargetPattern { value: String },
    /// Anything that is not an exact target label: paths, directories,
    /// profiles/flags, and language selectors.
    #[error("invalid target {value:?}: want an exact // or @ label")]
    NotTargetLabel { value: String },
}

/// Resolves the `dx codegen` positional scope: empty selects the
/// repository (`//dx:codegen`), one exact `//` or `@` label selects its
/// configured closure, and anything else fails before execution. A
/// compatible target whose closure contributes no shards selects an
/// empty exact projection downstream, never a failure here.
///
/// Single-source scope validation for `#651`: the `...`/`*`/`?` and
/// `//`/`@` checks live in [`dx_roots::resolve_exact_target`]; this keeps
/// the noun-specific `CodegenScope`/`ScopeError` while sharing the logic
/// with `dx_setup` and `dx_env_plan` (intentional divergence: distinct
/// scope types and canonical targets per command).
pub fn resolve_scope(targets: &[String]) -> Result<CodegenScope, ScopeError> {
    match resolve_exact_target(targets) {
        Ok(None) => Ok(CodegenScope::Repository),
        Ok(Some(label)) => Ok(CodegenScope::Exact(label)),
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
/// `//dx:codegen` selection identity; exact scopes bypass root selection.
pub fn scope_targets(scope: &CodegenScope) -> Vec<String> {
    match scope {
        CodegenScope::Repository => targets_for_root_plan(&repository_plan()),
        CodegenScope::Exact(label) => vec![label.clone()],
    }
}

/// Bazel labels to build for a WP4 root plan behind `//dx:codegen`: the
/// baseline plan keeps the canonical selection identity while the
/// fiat selection stands; every other candidate passes its own roots through.
/// The query-pattern-file candidate carries no command-line patterns
/// (Bazel reads them from `--target_pattern_file`).
///
/// Single-source plan helper for `#651`: shares the baseline/pattern-file
/// policy with `dx_env_plan` and `dx_setup` via [`dx_roots::invocation_targets`].
pub fn targets_for_root_plan(plan: &RepositoryRootPlan) -> Vec<String> {
    invocation_targets(plan, REPOSITORY_TARGET)
}

/// Full `bazel build` command line for a WP4 root plan: `build` plus the
/// plan roots, the collecting aspect, and the private output group (plus
/// `--target_pattern_file` when the plan carries a pattern file).
///
/// Single-source argv helper for `#651`: shares assembly with `dx_env_plan`
/// and `dx_setup` via [`dx_roots::build_argv`].
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
/// `replaces` is the replacement contract: empty for fail-closed
/// entries, otherwise equal to `logical_path` with non-empty
/// `exec_path`, identifying the replaced checked-in source and the
/// replacing generated artifact.
/// See: `docs/environments/codegen.md` (provider contract).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenEntry {
    pub logical_path: String,
    pub import_root: String,
    pub namespace: String,
    pub exec_path: String,
    pub replaces: String,
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
pub enum CollectError {
    /// A BEP-reported shard file fails to decode or validate.
    #[error("invalid shard {path:?}: {error}")]
    Shard { path: String, error: Error },
    /// Two records claim one logical path incompatibly. The message
    /// matches `codegen_conflict_error` rendering, listing every
    /// claimant: no traversal-order winner is accepted.
    #[error("{0}")]
    Conflict(String),
    /// An entry's non-empty exec suffix matches no BEP-reported
    /// non-shard artifact.
    #[error(
        "missing artifact for {producer} {logical_path:?}: no BEP artifact matches {exec_path:?}"
    )]
    MissingArtifact {
        producer: String,
        logical_path: String,
        exec_path: String,
    },
    /// An exec suffix matches more than one artifact, or one artifact
    /// is claimed by more than one entry. `claimants` holds the
    /// colliding logical paths (ambiguous case) or the artifact path
    /// rendered once per claimant context; see message construction.
    #[error("duplicate artifact {exec_path:?}: claimed by {claimants:?}")]
    DuplicateArtifact {
        exec_path: String,
        claimants: Vec<String>,
    },
    /// A BEP-reported non-shard artifact is claimed by no entry.
    #[error("unreported artifact {path:?}: claimed by no entry")]
    UnreportedArtifact { path: String },
    /// The normalized fingerprint view failed to serialize as JSON.
    #[error("{0}")]
    Fingerprint(#[from] dx_fingerprint::FingerprintError),
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
/// Mirrors `_exec_matches` in `//generation:codegen.bzl`.
/// Shared index plumbing. See: `cli/bep/src/lib.rs` (`dx_bep::exec_matches`).
pub fn exec_matches(artifact_path: &Path, exec_path: &str) -> bool {
    dx_bep::exec_matches(artifact_path, exec_path)
}

/// Decodes every shard among the BEP-reported artifacts and validates
/// the artifact index against shard declarations: each non-empty entry
/// exec suffix must resolve to exactly one non-shard BEP artifact, and
/// every non-shard BEP artifact must be claimed by exactly one entry.
/// Logical-only entries (empty exec) require no artifact. Inputs arrive
/// sorted by label from `dx_bep`; determinism comes from
/// [`merge_records`], never arrival order.
///
/// Keep: the env collector shares the shard/index plumbing via
/// `dx_bep` but keeps its own decode plus sharing-tolerant index
/// (codegen rejects duplicate claims as a closed manifest).
/// See: `cli/env_plan/src/lib.rs` (`collect_shards`).
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
                        replaces: entry.replaces,
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
/// Shared index plumbing. See: `cli/bep/src/lib.rs`.
fn generated_artifact_paths(outputs: &[TargetOutput]) -> Vec<String> {
    dx_bep::non_shard_artifact_paths(outputs, SHARD_SUFFIX)
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
            .filter(|path| dx_bep::suffix_matches(path, &entry.exec_path))
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
            .filter(|(_, entry)| dx_bep::suffix_matches(path, &entry.exec_path))
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
/// `(logical_path, import_root, namespace, exec_path, replaces)` to entry.
type MergeIndex<'a> = BTreeMap<
    (&'a str, &'a str),
    BTreeMap<(&'a str, &'a str, &'a str, &'a str, &'a str), &'a CodegenEntry>,
>;

/// Merges records into deterministic normalized order, mirroring
/// `codegen_merge_records`: byte-identical duplicate entries collapse,
/// surviving records sort by `(producer, language)` with entries sorted
/// by `(logical_path, import_root, namespace, exec_path, replaces)`.
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
                    entry.replaces.as_str(),
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
/// language, root, namespace, exec path, and replaces) merge silently,
/// any other second claim fails, listing every claimant. Returns `""`
/// when conflict-free.
pub fn conflict_error(records: &[CodegenRecord]) -> String {
    /// One logical-path claim: owner `(producer, language)` plus the
    /// full entry identity `(import_root, namespace, exec_path, replaces)`.
    type Claim<'a> = (&'a str, &'a str, &'a str, &'a str, &'a str, &'a str);
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
                    entry.replaces.as_str(),
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

/// Serializable fingerprint view: field order matches the frozen
/// Starlark `codegen_plan_fingerprint` (`entries`, `language`,
/// `producer`; entry `exec_path`, `import_root`, `logical_path`,
/// `namespace`, `read_only`, `replaces`) so `serde_json::to_string` stays
/// byte-identical for the pinned fixtures while gaining correct
/// escaping for quotes, backslashes, and control characters.
#[derive(Serialize)]
struct FingerprintEntry<'a> {
    exec_path: &'a str,
    import_root: &'a str,
    logical_path: &'a str,
    namespace: &'a str,
    read_only: bool,
    replaces: &'a str,
}

#[derive(Serialize)]
struct FingerprintRecord<'a> {
    entries: Vec<FingerprintEntry<'a>>,
    language: &'a str,
    producer: &'a str,
}

/// Renders the normalized complete-plan hash input, mirroring
/// `codegen_plan_fingerprint`: deterministic JSON over the merged
/// records, one object per record with producer, language, and entries
/// sorted by (logical path, import root, namespace, exec path,
/// replaces), each entry binding its exec suffix and replacement
/// contract. Byte-identical to the Starlark rendering for the same
/// records (pinned by the chain/prost fixture fingerprints).
///
/// Keep: the env fingerprint shares the JSON owner
/// (`dx_fingerprint::to_json`) but keeps its own view shape (codegen
/// keys on language plus logical path, import root, namespace; env on
/// integration plus key/value).
/// See: `cli/env_plan/src/lib.rs` (`fingerprint`).
pub fn fingerprint(records: &[CodegenRecord]) -> Result<String, dx_fingerprint::FingerprintError> {
    let merged = merge_records(records);
    let view: Vec<FingerprintRecord<'_>> = merged
        .iter()
        .map(|record| FingerprintRecord {
            entries: record
                .entries
                .iter()
                .map(|entry| FingerprintEntry {
                    exec_path: entry.exec_path.as_str(),
                    import_root: entry.import_root.as_str(),
                    logical_path: entry.logical_path.as_str(),
                    namespace: entry.namespace.as_str(),
                    read_only: true,
                    replaces: entry.replaces.as_str(),
                })
                .collect(),
            language: record.language.as_str(),
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
    pub records: Vec<CodegenRecord>,
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
/// (`"[]"`) rather than failing, so compatible targets with no
/// generated sources clear stale selections.
pub fn collect_plan(outputs: &[TargetOutput]) -> Result<CollectedPlan, CollectError> {
    let records = collect_shards(outputs)?;
    let conflict = conflict_error(&records);
    if !conflict.is_empty() {
        return Err(CollectError::Conflict(conflict));
    }
    let merged = merge_records(&records);
    let rendered = fingerprint(&merged)?;
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
/// no projection entry: the mirror links backed files only. `replaces`
/// carries the replacement contract (empty for fail-closed entries,
/// otherwise equal to `logical_path`); staging allows the workspace
/// collision only for contracted entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionEntry {
    pub logical_path: String,
    pub artifact: String,
    pub import_root: String,
    pub namespace: String,
    pub replaces: String,
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
                replaces: entry.replaces.clone(),
            });
        }
    }
    projection.sort_by(|left, right| left.logical_path.cmp(&right.logical_path));
    Ok(projection)
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod lib_tests;
