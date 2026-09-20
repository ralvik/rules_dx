//! License tier policy and distribution roots (WP3 slice 2).
//!
//! Pure planning for the license-family policy shape in the license
//! contract
//! (`docs/cli/commands/audit-update-bazel.md#license-family-dx-audit-license`):
//! the committed root file is TOML (`licenses.toml`, dedicated config —
//! no per-directory policy files and no local overlay that relaxes
//! license policy); a distributable is marked internal by listing its
//! label under `[distribution.internal]` while unlisted distributables
//! default to `distributed` (fail closed); per-set adjustments merge
//! additively with conflicts against the global table failing
//! validation.
//!
//! License exceptions reuse the vulnerability risk-acceptance lifecycle
//! (version-scoped with upstream version semantics, reasoned, expiring
//! with ISO-8601 UTC dates evaluated at audit time, obsolete only when
//! no applicable finding remains, always visible): expiry shares
//! [`crate::exception::check_expiry`], obsolescence is identity
//! match over package plus set plus license, and version-range
//! narrowing calls [`crate::vuln::version_affected`] in the
//! resolver-owned slices, exactly like the vulnerability deferral
//! in [`crate::exception`]. An upgrade within an exception's
//! bounded version range retains acceptance while the exception still
//! matches the finding and remains otherwise valid; out-of-range
//! versions do not inherit acceptance.
//!
//! Per-ecosystem license identities plus per-package notice texts ride
//! the committed `[[inventory]]` table: each entry names its owning set,
//! package, SPDX license text, upstream version scope, and whether the
//! package archive delivered `LICENSE*`/`NOTICE*` words. Missing entries
//! stay `UNKNOWN` with no words (fail closed in `distributed`).
//!
//! This module plans over injected table/root/exception/inventory records
//! only, so policy validation stays deterministic and unit-testable
//! without any lockfile or Bazel integration. [`load_licenses_toml`]
//! parses the committed `licenses.toml` root file into those records with
//! `toml` plus `serde`; shared-lock tier attribution and proof evidence
//! stay gated for later slices.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use crate::exception::{check_expiry, ExceptionProblem};
use crate::license_expr::Tier;

/// Versioned license-policy schema.
///
/// Consumers query the policy via `load_licenses_toml` plus
/// `PolicyTables::validate` / `Distribution::validate` /
/// `validate_license_exception` instead of duplicating license inventories,
/// so adding a license identity or exception edits `licenses.toml` data
/// only, never a parallel allowlist or struct. The TOML document carries an
/// optional `schema_version` (default 1 for pre-versioned files); loaders
/// reject any other version fail-closed.
pub const LICENSE_POLICY_SCHEMA_VERSION: u32 = 1;

/// Global license-policy table: each listed SPDX identity belongs in
/// exactly one list. `blocked` is evaluated in both tiers
/// (network-trigger and non-open licenses that are never silently
/// acceptable); the rest only gates `distributed`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PolicyTables {
    /// Never silently acceptable in any tier; needs a versioned
    /// exception with reason and expiry to pass anywhere.
    pub blocked: BTreeSet<String>,
    /// Passes in `distributed`.
    pub allow: BTreeSet<String>,
    /// Fails in `distributed` unless explicitly approved; inventoried
    /// in `internal`. Merely listing here is not approval.
    pub review: BTreeSet<String>,
    /// Fails in `distributed` unless explicitly approved; inventoried
    /// in `internal`.
    pub deny: BTreeSet<String>,
}

/// Additive per-set policy adjustment: extra identities the set reviews
/// beyond the global table. Adjustments only add; anything conflicting
/// with the global table fails validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetAdjustment {
    /// Owning dependency set.
    pub set: String,
    /// Additional identities reviewed for this set.
    pub review: BTreeSet<String>,
}

/// Distribution roots: which distributables are internal. Unlisted
/// distributables default to `distributed` (fail closed); promoting an
/// internal root to distributed re-qualifies it under the strict table
/// on the next audit. The name `distributed` is deliberate:
/// distribution is the legal trigger, while `ship` is slang.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Distribution {
    /// Release roots that leave the company. Listing here is optional:
    /// unlisted labels are distributed anyway.
    #[serde(default)]
    pub distributed: BTreeSet<String>,
    /// Internal-only roots, inventoried but never gated on the
    /// allow/review/deny table (except `blocked`).
    #[serde(default)]
    pub internal: BTreeSet<String>,
}

/// Policy validation failures. Every variant fails the audit; none
/// auto-repairs.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PolicyProblem {
    /// One SPDX identity is listed in more than one policy list.
    #[error("license {identity:?} is listed in more than one policy list")]
    MultiListed { identity: String },
    /// A per-set adjustment conflicts with the global table.
    #[error("license {identity:?} for set {set:?} conflicts with the global policy table")]
    SetConflict { set: String, identity: String },
    /// A label under `[distribution]` names no known distributable.
    #[error("unknown_distribution_root: {label:?} names no known distributable")]
    UnknownDistributionRoot { label: String },
    /// A `licenses.toml` document fails to parse or match the file shape.
    #[error("invalid licenses.toml: {message}")]
    InvalidLicensesToml { message: String },
    /// A license exception is invalid, expired, or obsolete.
    #[error("license exception invalid: {0}")]
    Exception(#[from] ExceptionProblem),
    /// A license exception names no package, set, license, versions, or
    /// reason.
    #[error("license exception missing {field}")]
    ExceptionMissingField { field: &'static str },
}

impl PolicyTables {
    /// Validate the global table: each identity in exactly one list.
    pub fn validate(&self) -> Result<(), PolicyProblem> {
        let mut seen: BTreeMap<&str, ()> = BTreeMap::new();
        for identity in self
            .blocked
            .iter()
            .chain(self.allow.iter())
            .chain(self.review.iter())
            .chain(self.deny.iter())
        {
            if seen.insert(identity.as_str(), ()).is_some() {
                return Err(PolicyProblem::MultiListed {
                    identity: identity.clone(),
                });
            }
        }
        Ok(())
    }

    /// Validate one per-set adjustment against the global table: set
    /// additions merge additively, so any identity already classified
    /// globally is a conflict, never a silent override.
    pub fn validate_set(&self, adjustment: &SetAdjustment) -> Result<(), PolicyProblem> {
        for identity in &adjustment.review {
            if self.blocked.contains(identity)
                || self.allow.contains(identity)
                || self.review.contains(identity)
                || self.deny.contains(identity)
            {
                return Err(PolicyProblem::SetConflict {
                    set: adjustment.set.clone(),
                    identity: identity.clone(),
                });
            }
        }
        Ok(())
    }
}

impl Distribution {
    /// Tier of one distributable label: internal only when listed under
    /// `[distribution.internal]`; every other label (listed or not) is
    /// distributed. Fail-closed by construction.
    pub fn tier_of(&self, label: &str) -> Tier {
        if self.internal.contains(label) {
            Tier::Internal
        } else {
            Tier::Distributed
        }
    }

    /// Validate distribution roots against the injected known
    /// distributable labels (Bazel-owned): any listed label naming
    /// nothing known fails as `unknown_distribution_root`.
    pub fn validate(&self, known: &BTreeSet<String>) -> Result<(), PolicyProblem> {
        for label in self.distributed.iter().chain(self.internal.iter()) {
            if !known.contains(label) {
                return Err(PolicyProblem::UnknownDistributionRoot {
                    label: label.clone(),
                });
            }
        }
        Ok(())
    }
}

/// One license-policy exception: a matching, reasoned, version-scoped,
/// expiring approval of a finding, without hiding it. Field shapes
/// mirror the committed `licenses.toml` `[[exception]]` entries so the
/// TOML loader cannot reinterpret them.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LicenseException {
    /// Affected package name.
    pub package: String,
    /// Owning dependency set.
    pub set: String,
    /// Approved SPDX identity or complete `WITH` expression verbatim.
    pub license: String,
    /// Accepted versions or bounded range, upstream version semantics
    /// per owning set via [`crate::vuln::version_affected`] (Cargo
    /// semver, npm ranges, Go `v`-prefix normalization, Maven and NuGet
    /// intervals, exact-match elsewhere); malformed scopes fail closed
    /// to no-match, never acceptance.
    pub versions: String,
    /// Explanatory reason. Empty reasons fail validation.
    pub reason: String,
    /// Expiration date, ISO-8601 UTC `YYYY-MM-DD`, evaluated at audit time.
    pub expires: String,
}

/// One assessed license finding an exception may apply to. Identity
/// is package plus set plus license; the version narrows coverage via
/// [`license_exception_covers`] using the owning set's upstream
/// semantics, exactly like vulnerability exception narrowing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LicenseFinding {
    /// Affected package name.
    pub package: String,
    /// Owning dependency set.
    pub set: String,
    /// Found SPDX identity or `WITH` expression text.
    pub license: String,
    /// Found package version, evaluated against the exception scope.
    pub version: String,
}

/// Validate one license exception against the injected audit date:
/// field presence plus the shared expiry lifecycle. Out-of-range
/// versions do not inherit acceptance; that narrowing is
/// [`license_exception_covers`] with the owning set's upstream
/// semantics.
pub fn validate_license_exception(
    exception: &LicenseException,
    today: &str,
) -> Result<(), PolicyProblem> {
    for (field, value) in [
        ("package", exception.package.as_str()),
        ("set", exception.set.as_str()),
        ("license", exception.license.as_str()),
        ("versions", exception.versions.as_str()),
        ("reason", exception.reason.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(PolicyProblem::ExceptionMissingField { field });
        }
    }
    check_expiry(&exception.expires, today)?;
    Ok(())
}

/// True when one license exception covers one finding: package plus
/// set plus license identity match, and the finding version falls
/// inside the exception's accepted scope under the owning set's
/// upstream semantics via [`crate::vuln::version_affected`]. Malformed
/// scopes or versions fail closed to `false`: an exception never
/// covers a version the matcher cannot attribute. Out-of-range
/// findings do not inherit acceptance even when identity matches.
pub fn license_exception_covers(exception: &LicenseException, finding: &LicenseFinding) -> bool {
    if exception.package != finding.package
        || exception.set != finding.set
        || exception.license != finding.license
    {
        return false;
    }
    crate::vuln::version_affected(&exception.set, &exception.versions, &finding.version)
}

/// True when no finding shares the exception's package, set, and
/// license identity. Version-range narrowing calls
/// [`license_exception_covers`] in the resolver-owned slices; until
/// then identity match is the conservative applicability gate (never
/// infers obsolescence from failed analysis).
pub fn is_obsolete(exception: &LicenseException, findings: &[LicenseFinding]) -> bool {
    !findings.iter().any(|finding| {
        finding.package == exception.package
            && finding.set == exception.set
            && finding.license == exception.license
    })
}

/// Check an exception for obsolescence after validation: an exception
/// with no applicable finding fails as obsolete (reported for explicit
/// removal, never auto-deleted).
pub fn check_applies(
    exception: &LicenseException,
    findings: &[LicenseFinding],
) -> Result<(), PolicyProblem> {
    if is_obsolete(exception, findings) {
        return Err(PolicyProblem::Exception(ExceptionProblem::Obsolete {
            advisory: exception.license.clone(),
            package: exception.package.clone(),
        }));
    }
    Ok(())
}

/// One per-package license identity plus notice-text presence: the
/// factual inventory the auditor joins against locked packages. Entries
/// are version-scoped with upstream version semantics (see
/// [`crate::vuln::version_affected`]); out-of-range versions never
/// inherit the entry. Missing entries stay `UNKNOWN` with no words (fail
/// closed in `distributed`).
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LicenseInventory {
    /// Affected package name (npm `@scope/name` keeps its spelling;
    /// Maven `group:artifact` keeps its verbatim key).
    pub package: String,
    /// Owning dependency set (`cargo`, `npm`, `maven`, `nuget`, `go`).
    pub set: String,
    /// Claimed SPDX expression text, or `UNKNOWN` when unidentified.
    pub license: String,
    /// Accepted versions or bounded range, upstream version semantics.
    pub versions: String,
    /// Whether the package archive delivered `LICENSE*`/`NOTICE*` words
    /// as a declared input. Defaults to false (fail closed) when absent.
    #[serde(default)]
    pub text_present: bool,
}

/// License policy loaded from one `licenses.toml` document: the
/// converted domain records, ready for the injected-label validation
/// the call sites own (distribution roots against Bazel-known labels,
/// exceptions against findings and the audit date).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LicensePolicy {
    /// Validated global table.
    pub tables: PolicyTables,
    /// Validated per-set adjustments, sorted by set name.
    pub sets: Vec<SetAdjustment>,
    /// Distribution roots. Validated against known labels by the caller.
    pub distribution: Distribution,
    /// License exceptions. Validated against findings and the audit date
    /// by the caller.
    pub exceptions: Vec<LicenseException>,
    /// Per-package license identities plus notice-text presence.
    pub inventory: Vec<LicenseInventory>,
}

/// Validate one inventory entry's required fields: package, set,
/// license, and versions must be non-empty. Empty fields fail so a
/// typo'd empty entry never becomes a silent `UNKNOWN`.
pub fn validate_license_inventory(entry: &LicenseInventory) -> Result<(), PolicyProblem> {
    if entry.package.trim().is_empty() {
        return Err(PolicyProblem::ExceptionMissingField {
            field: "inventory.package",
        });
    }
    if entry.set.trim().is_empty() {
        return Err(PolicyProblem::ExceptionMissingField {
            field: "inventory.set",
        });
    }
    if entry.license.trim().is_empty() {
        return Err(PolicyProblem::ExceptionMissingField {
            field: "inventory.license",
        });
    }
    if entry.versions.trim().is_empty() {
        return Err(PolicyProblem::ExceptionMissingField {
            field: "inventory.versions",
        });
    }
    Ok(())
}

/// Load and validate one `licenses.toml` document into domain records.
///
/// Parses with `toml` plus `serde`, converts the file shape, then
/// validates the global table, every per-set adjustment, and every
/// inventory entry, so a loaded policy never carries multi-listed
/// identities, set conflicts, or empty inventory fields. Unknown fields
/// fail as [`PolicyProblem::InvalidLicensesToml`]: a typo'd key (`alow`)
/// must never silently become an empty list — least of all an empty
/// `blocked` list. Missing sections default to empty, which stays
/// fail-closed because unlisted identities and distributables default to
/// the strict side. Distribution roots, exceptions, and inventory convert
/// verbatim; call-site validation (known labels, findings, audit date,
/// version-scope matching) is unchanged. The optional `schema_version`
/// must be [`LICENSE_POLICY_SCHEMA_VERSION`] when present (absent means
/// v1 for pre-versioned files); any other version fails as invalid TOML
/// so schema evolution is explicit, never silent drift.
pub fn load_licenses_toml(text: &str) -> Result<LicensePolicy, PolicyProblem> {
    let file: LicensesFile =
        toml::from_str(text).map_err(|error| PolicyProblem::InvalidLicensesToml {
            message: error.to_string(),
        })?;
    let version = file.schema_version.unwrap_or(LICENSE_POLICY_SCHEMA_VERSION);
    if version != LICENSE_POLICY_SCHEMA_VERSION {
        return Err(PolicyProblem::InvalidLicensesToml {
            message: format!(
                "unsupported licenses.toml schema_version {version} (want {LICENSE_POLICY_SCHEMA_VERSION})"
            ),
        });
    }
    let policy = LicensePolicy {
        tables: PolicyTables {
            blocked: file.policy.blocked.into_iter().collect(),
            allow: file.policy.distributed.allow.into_iter().collect(),
            review: file.policy.distributed.review.into_iter().collect(),
            deny: file.policy.distributed.deny.into_iter().collect(),
        },
        sets: file
            .policy
            .sets
            .into_iter()
            .map(|(set, adjustment)| SetAdjustment {
                set,
                review: adjustment.review.into_iter().collect(),
            })
            .collect(),
        distribution: file.distribution,
        exceptions: file.exception,
        inventory: file.inventory,
    };
    policy.tables.validate()?;
    for adjustment in &policy.sets {
        policy.tables.validate_set(adjustment)?;
    }
    for entry in &policy.inventory {
        validate_license_inventory(entry)?;
    }
    Ok(policy)
}

/// Committed `licenses.toml` root-file shape
/// (`docs/cli/commands/audit-update-bazel.md#license-family-dx-audit-license`):
/// dedicated config, no per-directory files, no relaxing overlay.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LicensesFile {
    /// Versioned schema marker. Optional for backward compat:
    /// absent means v1; any other value fails in `load_licenses_toml`.
    #[serde(default)]
    schema_version: Option<u32>,
    /// Global table plus per-set adjustments.
    #[serde(default)]
    policy: PolicyFile,
    /// Distribution roots.
    #[serde(default)]
    distribution: Distribution,
    /// License exceptions.
    #[serde(default)]
    exception: Vec<LicenseException>,
    /// Per-package license identities plus notice-text presence.
    #[serde(default)]
    inventory: Vec<LicenseInventory>,
}

/// `[policy]` shape: the global `blocked` list plus the
/// `[policy.distributed]` allow/review/deny lists and the
/// `[policy.sets.<name>]` adjustments.
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyFile {
    /// Never silently acceptable in any tier.
    #[serde(default)]
    blocked: Vec<String>,
    /// Tier-gating lists.
    #[serde(default)]
    distributed: PolicyLists,
    /// Per-set adjustments keyed by set name.
    #[serde(default)]
    sets: BTreeMap<String, SetReview>,
}

/// `[policy.distributed]` allow/review/deny lists.
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyLists {
    /// Passes in `distributed`.
    #[serde(default)]
    allow: Vec<String>,
    /// Needs explicit approval in `distributed`.
    #[serde(default)]
    review: Vec<String>,
    /// Fails in `distributed` unless explicitly approved.
    #[serde(default)]
    deny: Vec<String>,
}

/// One `[policy.sets.<name>]` adjustment: extra reviewed identities.
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct SetReview {
    /// Additional identities reviewed for this set.
    #[serde(default)]
    review: Vec<String>,
}

#[cfg(test)]
#[path = "license_policy_tests.rs"]
mod license_policy_tests;
