//! Advisory snapshot acquisition for `dx audit`.
//!
//! Pure planning over injected snapshot records, per the audit contract
//! (`docs/cli/commands/audit-update-bazel.md#dx-audit`): dependency audits
//! automatically refresh applicable vulnerability advisory data through
//! supported upstream tooling when invoked; a separate manual refresh is
//! not the default workflow. An identified advisory snapshot is supplied
//! as an input to Bazel-owned analysis, so results and cache identity
//! reflect the data actually analyzed rather than an untracked live
//! database inside the audit action. Acquisition/cache updates never
//! change application dependency versions, manifests, lockfiles, or
//! selected environment/codegen projections. Auditors remain pinned
//! tools; advisory freshness never authorizes automatic tool-version
//! upgrades.
//!
//! If required advisory refresh fails, the audit fails and reports that
//! current data could not be obtained. It never falls back to a stale
//! snapshot for the affected dependency audit, and never reports that
//! dependency set as clean. This module plans over injected snapshot
//! identities and dates only, so freshness, identity, and
//! refresh-failure mapping stay deterministic and unit-testable without
//! network access, a Bazel server, or any auditor binary.
//!
//! Offline local matching (no lockfile/inventory upload) is enforced by
//! construction: matching in [`crate::vuln`] runs against the identified
//! snapshot bytes supplied here. Package-specific advisory requests that
//! disclose the inventory are never an alternative to local matching,
//! and a query-only upstream service never satisfies this contract.
//! Database-download and offline-matching routes are qualified here;
//! the snapshot bytes themselves arrive as Bazel inputs in aspect
//! execution and as cache files in CLI execution, both pinned by
//! fixtures.
//!
//! Supported upstream database-download sources (issue #628, no
//! inventory upload): per-set OSV GCS bucket zips fetched by HTTPS GET
//! with no query parameters, request body, or telemetry carrying package
//! names or versions. The OSV API query route (`https://api.osv.dev/v1/query`
//! with package/version in the body) discloses the inventory and never
//! satisfies this contract. V1 snapshots (OSV-format records in the
//! [`crate::vuln::Advisory`] subset) are derived from these databases via
//! upstream tooling (such as `osv-scanner --offline` with a local DB, which
//! sends no project information); the derived bytes plus their identity
//! (`url`, `sha256`, `retrieved_at`) are the audited inputs. A missing,
//! invalid, or stale snapshot fails with [`CODE_ADVISORY_REFRESH_FAILED`],
//! never clean and never a stale fallback.

use serde::{Deserialize, Serialize};

/// Advisory snapshot identity: where the bytes came from, what they
/// are, and when they were retrieved. Field shapes mirror the cache
/// record the CLI writes, so Bazel-owned analysis and CLI execution
/// agree on one identity without a second mechanism.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AdvisorySnapshot {
    /// Owning dependency set (selector spelling, verbatim).
    pub set: String,
    /// Immutable upstream source URL for the snapshot bytes.
    pub url: String,
    /// Lowercase hex SHA-256 of the exact snapshot bytes.
    pub sha256: String,
    /// Retrieval date, ISO-8601 UTC `YYYY-MM-DD`, evaluated at audit time.
    /// Day granularity keeps the 24h cache check deterministic without
    /// ambient clock state in unit tests.
    pub retrieved_at: String,
    /// Workspace-relative or absolute path to the snapshot bytes supplied
    /// as analysis input.
    pub path: String,
}

/// Snapshot identity failures. Every variant fails the audit for the
/// affected dependency set; none falls back to a stale snapshot or
/// reports the set as clean.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SnapshotProblem {
    /// Empty set, URL, digest, date, or path.
    #[error("advisory snapshot missing {field}")]
    MissingField { field: &'static str },
    /// URL is not an immutable `https://` reference.
    #[error("advisory snapshot has non-https URL {url:?}")]
    BadUrl { url: String },
    /// Digest is not 64 lowercase hex characters.
    #[error("advisory snapshot has invalid sha256 {value:?}; want 64 lowercase hex")]
    BadDigest { value: String },
    /// Retrieval date is not a calendar `YYYY-MM-DD` date.
    #[error("advisory snapshot has invalid retrieved_at {value:?}; want YYYY-MM-DD")]
    BadDate { value: String },
}

/// Advisory freshness verdict for one dependency set at audit time.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Freshness {
    /// Snapshot is current (retrieved today): analyze offline against it.
    Fresh,
    /// Snapshot is older than the 24h cache window: refresh before analysis.
    Stale,
}

/// Refresh outcome for one dependency set. A failed refresh fails the
/// audit for that set with `advisory_refresh_failed`; it never falls
/// back to the stale snapshot and never reports the set as clean.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RefreshOutcome {
    /// Snapshot was fresh: proceed with offline matching.
    Fresh,
    /// Stale snapshot was refreshed: proceed against the new identity.
    Refreshed { snapshot: AdvisorySnapshot },
    /// Refresh failed: fail the audit, report stale data could not be
    /// replaced, retain validated findings from other sets.
    Failed { detail: String },
}

/// Stable operational code for advisory refresh failures. The CLI maps
/// this to `audit_failed` with an `advisory_refresh_failed` diagnostic,
/// never to a clean result.
pub const CODE_ADVISORY_REFRESH_FAILED: &str = "advisory_refresh_failed";

/// Cache window in days: a snapshot retrieved today is fresh; any older
/// date is stale and must refresh before analysis. Day granularity is
/// the conservative 24h gate (a snapshot retrieved yesterday at any
/// hour is stale today), so an earlier cached snapshot never lets a
/// later audit pass on stale data.
pub const CACHE_DAYS: u32 = 0;

/// Supported upstream database-download source per dependency set
/// (issue #628): OSV GCS bucket zips fetched by HTTPS GET with no
/// inventory in the request. The URL carries no package names, versions,
/// query parameters, or body; matching runs offline after download, so
/// no lockfile or inventory ever leaves the workspace. The OSV query API
/// (package/version in the request) never satisfies this contract.
pub fn advisory_source(set: &str) -> Option<&'static str> {
    match set {
        "cargo" => Some("https://osv-vulnerabilities.storage.googleapis.com/crates.io/all.zip"),
        "npm" => Some("https://osv-vulnerabilities.storage.googleapis.com/npm/all.zip"),
        "maven" => Some("https://osv-vulnerabilities.storage.googleapis.com/Maven/all.zip"),
        "nuget" => Some("https://osv-vulnerabilities.storage.googleapis.com/NuGet/all.zip"),
        "go" => Some("https://osv-vulnerabilities.storage.googleapis.com/Go/all.zip"),
        _ => None,
    }
}

/// Workspace-relative snapshot bytes for one set: the identified advisory
/// snapshot supplied as analysis input (Bazel input in aspect execution,
/// cache file in CLI execution). A missing file means current data could
/// not be obtained and fails with [`CODE_ADVISORY_REFRESH_FAILED`],
/// never clean.
pub fn snapshot_rel(set: &str) -> String {
    format!(".dx/advisory/{set}.json")
}

/// Workspace-relative snapshot identity for one set: the
/// [`AdvisorySnapshot`] record (`url`, `sha256`, `retrieved_at`) for the
/// bytes at [`snapshot_rel`]. A missing or invalid identity fails like a
/// missing snapshot; a `retrieved_at` older than today is stale and fails
/// without analyzing the stale bytes.
pub fn identity_rel(set: &str) -> String {
    format!(".dx/advisory/{set}.meta.json")
}

/// Parse one snapshot identity document (JSON [`AdvisorySnapshot`]).
/// Malformed JSON fails closed with the document error, never a default
/// identity that could pass as fresh.
pub fn parse_identity(text: &str) -> Result<AdvisorySnapshot, String> {
    serde_json::from_str(text).map_err(|error| format!("invalid advisory identity: {error}"))
}

/// Whether one identity matches its snapshot bytes: the recorded
/// `sha256` equals the lowercase hex SHA-256 of the exact bytes.
/// Mismatches fail closed (stale or tampered snapshot, never analyzed).
pub fn identity_matches_bytes(snapshot: &AdvisorySnapshot, bytes: &[u8]) -> bool {
    dx_digest::sha256_hex(bytes) == snapshot.sha256.trim()
}

/// Validate one snapshot identity without fetching anything: set, URL,
/// digest, date, and path must be present; the URL must be `https://`;
/// the digest must be 64 lowercase hex; the date must be calendar
/// `YYYY-MM-DD`. Byte identity against upstream is proven by the
/// acquisition command that wrote the snapshot, not here.
pub fn validate_snapshot(snapshot: &AdvisorySnapshot) -> Result<(), SnapshotProblem> {
    for (field, value) in [
        ("set", snapshot.set.as_str()),
        ("url", snapshot.url.as_str()),
        ("sha256", snapshot.sha256.as_str()),
        ("retrieved_at", snapshot.retrieved_at.as_str()),
        ("path", snapshot.path.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(SnapshotProblem::MissingField { field });
        }
    }
    if !(snapshot.url.starts_with("https://") && url::Url::parse(&snapshot.url).is_ok()) {
        return Err(SnapshotProblem::BadUrl {
            url: snapshot.url.clone(),
        });
    }
    if !dx_digest::is_hex(&snapshot.sha256) {
        return Err(SnapshotProblem::BadDigest {
            value: snapshot.sha256.clone(),
        });
    }
    if crate::exception::check_expiry("2099-01-01", &snapshot.retrieved_at).is_err()
        && snapshot.retrieved_at != "2099-01-01"
    {
        // Reuse the strict YYYY-MM-DD shape gate from the exception
        // lifecycle without importing its error type: any date that
        // parses as an audit date is a valid retrieval date. The
        // sentinel above only exercises the parser; expiry itself is
        // irrelevant here.
    }
    if !is_audit_date(&snapshot.retrieved_at) {
        return Err(SnapshotProblem::BadDate {
            value: snapshot.retrieved_at.clone(),
        });
    }
    Ok(())
}

/// Check freshness of one validated snapshot against the injected audit
/// date (`YYYY-MM-DD` UTC). Fresh means retrieved today; any older date
/// is stale. Unparseable dates fail closed to stale (refresh, never
/// analyze against an undated snapshot).
pub fn freshness(snapshot: &AdvisorySnapshot, today: &str) -> Freshness {
    if snapshot.retrieved_at.trim() == today.trim() && is_audit_date(today) {
        Freshness::Fresh
    } else {
        Freshness::Stale
    }
}

/// Map one refresh attempt to its audit outcome: fresh snapshots
/// proceed; stale snapshots proceed only when the refresh supplies a
/// new validated identity; a failed refresh fails the audit for that
/// set without falling back to stale data.
pub fn map_refresh(
    snapshot: &AdvisorySnapshot,
    today: &str,
    refreshed: Option<AdvisorySnapshot>,
    refresh_error: Option<String>,
) -> RefreshOutcome {
    if freshness(snapshot, today) == Freshness::Fresh {
        return RefreshOutcome::Fresh;
    }
    if let Some(next) = refreshed {
        if validate_snapshot(&next).is_ok() {
            return RefreshOutcome::Refreshed { snapshot: next };
        }
    }
    RefreshOutcome::Failed {
        detail: refresh_error.unwrap_or_else(|| {
            format!(
                "could not obtain current advisory data for {} (stale snapshot {})",
                snapshot.set, snapshot.retrieved_at
            )
        }),
    }
}

/// Whether offline matching may proceed for one set: only fresh or
/// successfully refreshed snapshots analyze; failed refreshes never
/// analyze against stale data.
pub fn may_analyze(outcome: &RefreshOutcome) -> bool {
    matches!(
        outcome,
        RefreshOutcome::Fresh | RefreshOutcome::Refreshed { .. }
    )
}

/// Strict `YYYY-MM-DD` calendar gate, mirroring the exception lifecycle
/// shape plus upstream calendar validation. Kept local so advisory
/// validation never depends on exception error variants.
///
/// Dependency evaluation (keep): stays on `chrono`
/// (`NaiveDate::parse_from_str`) per the exception-gate `jiff` rejection —
/// day-granularity retrieval dates need no `tzdb`, same mechanical-churn
/// cost; re-evaluate on `jiff 1.0`.
fn is_audit_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    if !bytes
        .iter()
        .enumerate()
        .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
    {
        return false;
    }
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> AdvisorySnapshot {
        AdvisorySnapshot {
            set: "cargo".to_owned(),
            url: "https://osv.dev/snapshots/cargo-2026-09-18.json".to_owned(),
            sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_owned(),
            retrieved_at: "2026-09-18".to_owned(),
            path: ".dx/advisory/cargo-2026-09-18.json".to_owned(),
        }
    }

    #[test]
    fn valid_snapshot_passes() {
        validate_snapshot(&snapshot()).expect("valid");
    }

    #[test]
    fn empty_fields_fail() {
        let mut bad = snapshot();
        bad.set = "  ".to_owned();
        assert_eq!(
            validate_snapshot(&bad),
            Err(SnapshotProblem::MissingField { field: "set" })
        );
        let mut bad = snapshot();
        bad.path = String::new();
        assert!(validate_snapshot(&bad).is_err());
    }

    #[test]
    fn non_https_url_fails() {
        let mut bad = snapshot();
        bad.url = "http://osv.dev/snapshot.json".to_owned();
        assert_eq!(
            validate_snapshot(&bad),
            Err(SnapshotProblem::BadUrl {
                url: "http://osv.dev/snapshot.json".to_owned()
            })
        );
    }

    #[test]
    fn malformed_digests_fail() {
        for digest in [
            "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855",
            "not-a-digest",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b85",
        ] {
            let mut bad = snapshot();
            bad.sha256 = digest.to_owned();
            assert!(
                matches!(
                    validate_snapshot(&bad),
                    Err(SnapshotProblem::BadDigest { .. })
                ),
                "{digest} must fail"
            );
        }
    }

    #[test]
    fn malformed_dates_fail() {
        for bad_date in ["2026-9-18", "2026/09/18", "2026-13-01", "not-a-date"] {
            let mut bad = snapshot();
            bad.retrieved_at = bad_date.to_owned();
            assert_eq!(
                validate_snapshot(&bad),
                Err(SnapshotProblem::BadDate {
                    value: bad_date.to_owned()
                }),
                "{bad_date} must fail"
            );
        }
    }

    #[test]
    fn freshness_is_same_day_only() {
        let snap = snapshot();
        assert_eq!(freshness(&snap, "2026-09-18"), Freshness::Fresh);
        assert_eq!(freshness(&snap, "2026-09-19"), Freshness::Stale);
        assert_eq!(freshness(&snap, "2026-09-17"), Freshness::Stale);
        assert_eq!(freshness(&snap, "not-a-date"), Freshness::Stale);
    }

    #[test]
    fn fresh_snapshot_proceeds_without_refresh() {
        let snap = snapshot();
        assert_eq!(
            map_refresh(&snap, "2026-09-18", None, None),
            RefreshOutcome::Fresh
        );
        assert!(may_analyze(&RefreshOutcome::Fresh));
    }

    #[test]
    fn stale_refresh_success_proceeds_against_new_identity() {
        let snap = snapshot();
        let next = AdvisorySnapshot {
            retrieved_at: "2026-09-19".to_owned(),
            ..snapshot()
        };
        let outcome = map_refresh(&snap, "2026-09-19", Some(next.clone()), None);
        assert_eq!(outcome, RefreshOutcome::Refreshed { snapshot: next });
        assert!(may_analyze(&outcome));
    }

    #[test]
    fn stale_refresh_failure_fails_without_stale_fallback() {
        let snap = snapshot();
        let outcome = map_refresh(
            &snap,
            "2026-09-19",
            None,
            Some("network unreachable".to_owned()),
        );
        assert_eq!(
            outcome,
            RefreshOutcome::Failed {
                detail: "network unreachable".to_owned()
            }
        );
        assert!(!may_analyze(&outcome));
        // Default detail names the set and stale date, never claims clean.
        let defaulted = map_refresh(&snap, "2026-09-19", None, None);
        match &defaulted {
            RefreshOutcome::Failed { detail } => {
                assert!(detail.contains("cargo"));
                assert!(!detail.contains("clean"));
            }
            _ => panic!("must fail"),
        }
        assert!(!may_analyze(&defaulted));
    }

    #[test]
    fn stale_refresh_with_invalid_identity_fails() {
        let snap = snapshot();
        let mut bad = snapshot();
        bad.sha256 = "bad".to_owned();
        let outcome = map_refresh(&snap, "2026-09-19", Some(bad), None);
        assert!(matches!(outcome, RefreshOutcome::Failed { .. }));
        assert!(!may_analyze(&outcome));
    }

    #[test]
    fn cache_window_and_code_are_pinned() {
        assert_eq!(CACHE_DAYS, 0);
        assert_eq!(CODE_ADVISORY_REFRESH_FAILED, "advisory_refresh_failed");
    }

    #[test]
    fn advisory_sources_are_https_database_downloads_without_inventory() {
        // Every registry set owns an OSV GCS database-download source;
        // unknown sets have none (fail closed, never empty clean).
        for (set, ecosystem) in [
            ("cargo", "crates.io"),
            ("npm", "npm"),
            ("maven", "Maven"),
            ("nuget", "NuGet"),
            ("go", "Go"),
        ] {
            let url = advisory_source(set).unwrap_or_else(|| panic!("{set} needs a source"));
            assert!(url.starts_with("https://"), "{set} must be https");
            assert!(
                url.contains(ecosystem) && url.ends_with("/all.zip"),
                "{set} must name its OSV ecosystem bucket"
            );
            // No inventory in the request: the URL carries no package
            // names, versions, query parameters, or body. Matching runs
            // offline after download.
            assert!(!url.contains('?'), "{set} must carry no query");
            assert!(
                !url.contains("api.osv.dev"),
                "{set} must not use the query API"
            );
        }
        assert_eq!(advisory_source("unknown-set"), None);
        assert_eq!(advisory_source(""), None);
    }

    #[test]
    fn snapshot_and_identity_rels_are_pinned() {
        assert_eq!(snapshot_rel("cargo"), ".dx/advisory/cargo.json");
        assert_eq!(identity_rel("cargo"), ".dx/advisory/cargo.meta.json");
        assert_eq!(snapshot_rel("go"), ".dx/advisory/go.json");
        assert_eq!(identity_rel("go"), ".dx/advisory/go.meta.json");
    }

    #[test]
    fn identity_round_trips_and_rejects_malformed() {
        let snap = snapshot();
        let text = serde_json::to_string(&snap).expect("serialize");
        assert_eq!(parse_identity(&text).expect("parse"), snap);
        assert!(parse_identity("not json").is_err());
        // Missing required identity fields fail closed at parse time,
        // never a default identity that could pass as fresh.
        assert!(parse_identity("{}").is_err());
    }

    #[test]
    fn identity_must_match_bytes() {
        let bytes = b"[]";
        let digest = dx_digest::sha256_hex(bytes);
        let mut snap = snapshot();
        snap.sha256 = digest.clone();
        assert!(identity_matches_bytes(&snap, bytes));
        snap.sha256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_owned();
        assert!(!identity_matches_bytes(&snap, b"tampered"));
    }
}
