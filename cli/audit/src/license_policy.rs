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
mod tests {
    use super::*;
    use crate::license_expr::{evaluate, IdClass, LicenseExpr, TierOutcome};

    fn tables() -> PolicyTables {
        PolicyTables {
            blocked: ["AGPL-3.0-only"]
                .iter()
                .map(|item| (*item).to_owned())
                .collect(),
            allow: ["MIT"].iter().map(|item| (*item).to_owned()).collect(),
            review: ["MPL-2.0"].iter().map(|item| (*item).to_owned()).collect(),
            deny: ["GPL-3.0-only"]
                .iter()
                .map(|item| (*item).to_owned())
                .collect(),
        }
    }

    fn exception() -> LicenseException {
        LicenseException {
            package: "some-copyleft-lib".to_owned(),
            set: "cargo".to_owned(),
            license: "GPL-3.0-only".to_owned(),
            versions: ">=1.2.0, <2.0.0".to_owned(),
            reason: "Legal approved for internal fork; re-review on major bump.".to_owned(),
            expires: "2027-03-01".to_owned(),
        }
    }

    fn finding() -> LicenseFinding {
        LicenseFinding {
            package: "some-copyleft-lib".to_owned(),
            set: "cargo".to_owned(),
            license: "GPL-3.0-only".to_owned(),
            version: "1.2.0".to_owned(),
        }
    }

    #[test]
    fn single_listed_tables_validate() {
        tables().validate().expect("single listing passes");
    }

    #[test]
    fn identity_in_two_lists_fails() {
        let mut bad = tables();
        bad.deny.insert("MIT".to_owned());
        assert_eq!(
            bad.validate(),
            Err(PolicyProblem::MultiListed {
                identity: "MIT".to_owned()
            })
        );
        let mut bad = tables();
        bad.review.insert("AGPL-3.0-only".to_owned());
        assert!(matches!(
            bad.validate(),
            Err(PolicyProblem::MultiListed { .. })
        ));
    }

    #[test]
    fn set_adjustments_merge_additively_and_conflicts_fail() {
        let tables = tables();
        let additive = SetAdjustment {
            set: "npm-root".to_owned(),
            review: ["Unicode-3.0"]
                .iter()
                .map(|item| (*item).to_owned())
                .collect(),
        };
        tables.validate_set(&additive).expect("additive merges");
        let conflicting = SetAdjustment {
            set: "npm-root".to_owned(),
            review: ["MIT"].iter().map(|item| (*item).to_owned()).collect(),
        };
        assert_eq!(
            tables.validate_set(&conflicting),
            Err(PolicyProblem::SetConflict {
                set: "npm-root".to_owned(),
                identity: "MIT".to_owned(),
            })
        );
    }

    #[test]
    fn unlisted_distributables_default_distributed() {
        let distribution = Distribution {
            distributed: ["//services/payments:image"]
                .iter()
                .map(|item| (*item).to_owned())
                .collect(),
            internal: ["//tools/internal-admin:binary"]
                .iter()
                .map(|item| (*item).to_owned())
                .collect(),
        };
        assert_eq!(
            distribution.tier_of("//tools/internal-admin:binary"),
            Tier::Internal
        );
        assert_eq!(
            distribution.tier_of("//services/payments:image"),
            Tier::Distributed
        );
        // Fail closed: unlisted labels are distributed, never internal.
        assert_eq!(distribution.tier_of("//cli:dx"), Tier::Distributed);
    }

    #[test]
    fn unknown_distribution_labels_fail() {
        let distribution = Distribution {
            distributed: BTreeSet::new(),
            internal: ["//tools/gone:binary"]
                .iter()
                .map(|item| (*item).to_owned())
                .collect(),
        };
        let known: BTreeSet<String> = ["//tools/internal-admin:binary"]
            .iter()
            .map(|item| (*item).to_owned())
            .collect();
        assert_eq!(
            distribution.validate(&known),
            Err(PolicyProblem::UnknownDistributionRoot {
                label: "//tools/gone:binary".to_owned()
            })
        );
        let known: BTreeSet<String> = ["//tools/gone:binary"]
            .iter()
            .map(|item| (*item).to_owned())
            .collect();
        distribution.validate(&known).expect("known labels pass");
    }

    #[test]
    fn promotion_to_distributed_requalifies_under_strict_table() {
        // The same finding inventoried for an internal root fails once
        // the root is promoted: tier lookup, not the finding, decides.
        let internal = Distribution {
            distributed: BTreeSet::new(),
            internal: ["//cli:dx"].iter().map(|item| (*item).to_owned()).collect(),
        };
        let promoted = Distribution::default();
        assert_eq!(internal.tier_of("//cli:dx"), Tier::Internal);
        assert_eq!(promoted.tier_of("//cli:dx"), Tier::Distributed);
        let lookup = |id: &str| match id {
            "MPL-2.0" => IdClass::Review,
            _ => IdClass::Unlisted,
        };
        let denied: fn(&str) -> bool = |_| false;
        let expr = LicenseExpr::Ident("MPL-2.0".to_owned());
        assert_eq!(
            evaluate(&expr, Tier::Internal, &lookup, &denied),
            TierOutcome::Allow
        );
        assert_eq!(
            evaluate(&expr, Tier::Distributed, &lookup, &denied),
            TierOutcome::Review
        );
    }

    #[test]
    fn valid_license_exception_passes_and_applies() {
        validate_license_exception(&exception(), "2026-09-14").expect("valid");
        check_applies(&exception(), &[finding()]).expect("applies");
    }

    #[test]
    fn license_exception_rejects_empty_fields_bad_dates_and_expiry() {
        let mut bad = exception();
        bad.reason = String::new();
        assert_eq!(
            validate_license_exception(&bad, "2026-09-14"),
            Err(PolicyProblem::ExceptionMissingField { field: "reason" })
        );
        let mut bad = exception();
        bad.expires = "2027-02-29".to_owned();
        assert!(validate_license_exception(&bad, "2026-09-14").is_err());
        assert!(validate_license_exception(&exception(), "2027-03-01").is_err());
    }

    #[test]
    fn license_exception_without_finding_is_obsolete() {
        assert!(is_obsolete(&exception(), &[]));
        assert!(check_applies(&exception(), &[]).is_err());
        assert!(!is_obsolete(&exception(), &[finding()]));
        // Wrong owning set shares no finding: obsolete, never silently
        // applied across sets.
        let other_set = LicenseFinding {
            set: "npm".to_owned(),
            ..finding()
        };
        assert!(is_obsolete(&exception(), &[other_set]));
        // Wrong license shares no finding either.
        let other_license = LicenseFinding {
            license: "MIT".to_owned(),
            ..finding()
        };
        assert!(is_obsolete(&exception(), &[other_license]));
    }

    #[test]
    fn upgrade_within_range_retains_acceptance_at_identity_match() {
        // Version-range narrowing calls license_exception_covers in the
        // resolver-owned slices; an upgrade alone never invalidates: the
        // finding still carries the same package, set, and license
        // identity, and an in-range upgrade stays covered.
        let upgraded = LicenseFinding {
            version: "1.9.0".to_owned(),
            ..finding()
        };
        assert!(!is_obsolete(&exception(), &[upgraded.clone()]));
        check_applies(&exception(), &[upgraded.clone()]).expect("upgrade retains acceptance");
        assert!(license_exception_covers(&exception(), &upgraded));
    }

    #[test]
    fn out_of_range_versions_do_not_inherit_acceptance() {
        // Identity match keeps the exception applicable (not obsolete),
        // but coverage narrows by version: an out-of-range finding is
        // not covered, exactly like vulnerability exceptions.
        let out_of_range = LicenseFinding {
            version: "2.0.0".to_owned(),
            ..finding()
        };
        assert!(!is_obsolete(&exception(), &[out_of_range.clone()]));
        check_applies(&exception(), &[out_of_range.clone()]).expect("identity still applies");
        assert!(!license_exception_covers(&exception(), &out_of_range));
        // Malformed scopes and versions fail closed to uncovered.
        let bad_scope = LicenseException {
            versions: "not a range".to_owned(),
            ..exception()
        };
        assert!(!license_exception_covers(&bad_scope, &finding()));
        let bad_version = LicenseFinding {
            version: "banana".to_owned(),
            ..finding()
        };
        assert!(!license_exception_covers(&exception(), &bad_version));
    }

    #[test]
    fn license_exceptions_narrow_per_set_like_vuln() {
        // Cargo: ranges, carets, tildes, star; bare versions are caret
        // shorthand, `||` and hyphen stay invalid and fail closed.
        let cargo_exception = |versions: &str| LicenseException {
            package: "serde".to_owned(),
            set: "cargo".to_owned(),
            license: "GPL-3.0-only".to_owned(),
            versions: versions.to_owned(),
            reason: "Legal approved.".to_owned(),
            expires: "2027-03-01".to_owned(),
        };
        let cargo_finding = |version: &str| LicenseFinding {
            package: "serde".to_owned(),
            set: "cargo".to_owned(),
            license: "GPL-3.0-only".to_owned(),
            version: version.to_owned(),
        };
        assert!(license_exception_covers(
            &cargo_exception(">=1.2.0, <2.0.0"),
            &cargo_finding("1.9.0")
        ));
        assert!(!license_exception_covers(
            &cargo_exception(">=1.2.0, <2.0.0"),
            &cargo_finding("2.0.0")
        ));
        assert!(license_exception_covers(
            &cargo_exception("*"),
            &cargo_finding("9.9.9")
        ));
        assert!(!license_exception_covers(
            &cargo_exception("1.0.0 || 2.0.0"),
            &cargo_finding("1.0.0")
        ));
        // npm: `||` unions, hyphen ranges, carets, tildes, bare exact.
        let npm_exception = |versions: &str| LicenseException {
            package: "react".to_owned(),
            set: "npm".to_owned(),
            license: "GPL-3.0-only".to_owned(),
            versions: versions.to_owned(),
            reason: "Legal approved.".to_owned(),
            expires: "2027-03-01".to_owned(),
        };
        let npm_finding = |version: &str| LicenseFinding {
            package: "react".to_owned(),
            set: "npm".to_owned(),
            license: "GPL-3.0-only".to_owned(),
            version: version.to_owned(),
        };
        assert!(license_exception_covers(
            &npm_exception("1.2.7 || >=1.2.9 <2.0.0"),
            &npm_finding("1.2.9")
        ));
        assert!(!license_exception_covers(
            &npm_exception("1.2.7 || >=1.2.9 <2.0.0"),
            &npm_finding("1.2.8")
        ));
        assert!(license_exception_covers(
            &npm_exception("1.2.3 - 2.3.4"),
            &npm_finding("2.0.0")
        ));
        assert!(!license_exception_covers(
            &npm_exception("1.2.3 - 2.3.4"),
            &npm_finding("2.3.5")
        ));
        // Go: `v`-prefix normalization on scope and version.
        let go_exception = LicenseException {
            package: "example.com/mod".to_owned(),
            set: "go".to_owned(),
            license: "GPL-3.0-only".to_owned(),
            versions: ">=v1.0.0, <v2.0.0".to_owned(),
            reason: "Legal approved.".to_owned(),
            expires: "2027-03-01".to_owned(),
        };
        let go_covered = LicenseFinding {
            package: "example.com/mod".to_owned(),
            set: "go".to_owned(),
            license: "GPL-3.0-only".to_owned(),
            version: "v1.5.0".to_owned(),
        };
        let go_missed = LicenseFinding {
            version: "v2.0.0".to_owned(),
            ..go_covered.clone()
        };
        assert!(license_exception_covers(&go_exception, &go_covered));
        assert!(!license_exception_covers(&go_exception, &go_missed));
        // Maven: bare versions use Maven equality, intervals use Maven
        // ordering with inclusive/exclusive bounds.
        let maven_exception = |versions: &str| LicenseException {
            package: "junit:junit".to_owned(),
            set: "maven".to_owned(),
            license: "GPL-3.0-only".to_owned(),
            versions: versions.to_owned(),
            reason: "Legal approved.".to_owned(),
            expires: "2027-03-01".to_owned(),
        };
        let maven_finding = |version: &str| LicenseFinding {
            package: "junit:junit".to_owned(),
            set: "maven".to_owned(),
            license: "GPL-3.0-only".to_owned(),
            version: version.to_owned(),
        };
        assert!(license_exception_covers(
            &maven_exception("[4.0,5.0)"),
            &maven_finding("4.13.2")
        ));
        assert!(!license_exception_covers(
            &maven_exception("[5.0,6.0)"),
            &maven_finding("4.13.2")
        ));
        assert!(license_exception_covers(
            &maven_exception("1.0"),
            &maven_finding("1.0.0")
        ));
        assert!(!license_exception_covers(
            &maven_exception(">=1.0.0"),
            &maven_finding("1.2.0")
        ));
        // NuGet: bare versions use NuGet equality, intervals use NuGet
        // ordering; floating `*` stays invalid and fails closed.
        let nuget_exception = |versions: &str| LicenseException {
            package: "Newtonsoft.Json".to_owned(),
            set: "nuget".to_owned(),
            license: "GPL-3.0-only".to_owned(),
            versions: versions.to_owned(),
            reason: "Legal approved.".to_owned(),
            expires: "2027-03-01".to_owned(),
        };
        let nuget_finding = |version: &str| LicenseFinding {
            package: "Newtonsoft.Json".to_owned(),
            set: "nuget".to_owned(),
            license: "GPL-3.0-only".to_owned(),
            version: version.to_owned(),
        };
        assert!(license_exception_covers(
            &nuget_exception("[12.0,13.0.2)"),
            &nuget_finding("13.0.1")
        ));
        assert!(!license_exception_covers(
            &nuget_exception("[13.0.2,14.0)"),
            &nuget_finding("13.0.1")
        ));
        assert!(license_exception_covers(
            &nuget_exception("1.0"),
            &nuget_finding("1.0.0")
        ));
        assert!(!license_exception_covers(
            &nuget_exception("1.*"),
            &nuget_finding("1.5.0")
        ));
        // Identity mismatch never covers, even in range.
        assert!(!license_exception_covers(
            &cargo_exception(">=1.0.0, <2.0.0"),
            &npm_finding("1.5.0")
        ));
    }

    fn doc_example() -> &'static str {
        r#"
[policy]
blocked = ["AGPL-3.0-only", "AGPL-3.0-or-later", "SSPL-1.0"]

[policy.distributed]
allow = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC"]
review = ["MPL-2.0", "EPL-2.0"]
deny = ["GPL-3.0-only", "GPL-3.0-or-later"]

[policy.sets.npm-root]
review = ["Unicode-3.0"]

[distribution]
distributed = ["//services/payments:image", "//cli:dx"]
internal = ["//tools/internal-admin:binary"]

[[exception]]
package = "some-copyleft-lib"
set = "cargo"
license = "GPL-3.0-only"
versions = ">=1.2.0, <2.0.0"
reason = "Legal approved for internal fork."
expires = "2027-03-01"
"#
    }

    #[test]
    fn loader_converts_doc_shape_to_domain_records() {
        let policy = load_licenses_toml(doc_example()).expect("doc example loads");
        assert_eq!(
            policy.tables.blocked,
            ["AGPL-3.0-only", "AGPL-3.0-or-later", "SSPL-1.0"]
                .iter()
                .map(|item| (*item).to_owned())
                .collect()
        );
        assert!(policy.tables.allow.contains("MIT"));
        assert!(policy.tables.review.contains("MPL-2.0"));
        assert!(policy.tables.deny.contains("GPL-3.0-only"));
        assert_eq!(
            policy.sets,
            vec![SetAdjustment {
                set: "npm-root".to_owned(),
                review: ["Unicode-3.0"]
                    .iter()
                    .map(|item| (*item).to_owned())
                    .collect(),
            }]
        );
        assert_eq!(
            policy.distribution.tier_of("//tools/internal-admin:binary"),
            Tier::Internal
        );
        assert_eq!(policy.distribution.tier_of("//cli:dx"), Tier::Distributed);
        assert_eq!(
            policy.distribution.tier_of("//unlisted:thing"),
            Tier::Distributed
        );
        assert_eq!(
            policy.exceptions,
            vec![LicenseException {
                package: "some-copyleft-lib".to_owned(),
                set: "cargo".to_owned(),
                license: "GPL-3.0-only".to_owned(),
                versions: ">=1.2.0, <2.0.0".to_owned(),
                reason: "Legal approved for internal fork.".to_owned(),
                expires: "2027-03-01".to_owned(),
            }]
        );
    }

    #[test]
    fn loader_defaults_missing_sections_to_empty_and_stays_closed() {
        let policy = load_licenses_toml("").expect("empty document loads");
        assert_eq!(
            policy,
            LicensePolicy {
                tables: PolicyTables::default(),
                sets: Vec::new(),
                distribution: Distribution::default(),
                exceptions: Vec::new(),
                inventory: Vec::new(),
            }
        );
        // Fail closed: unlisted identities and labels land on the strict side.
        assert_eq!(policy.distribution.tier_of("//cli:dx"), Tier::Distributed);
    }

    #[test]
    fn loader_rejects_unknown_fields_typos_and_malformed_toml() {
        for bad in [
            // Typo'd list key must not silently become an empty list.
            "[policy.distributed]\nalow = [\"MIT\"]\n",
            // Unknown top-level section.
            "[bogus]\nkey = 1\n",
            // Unknown exception field.
            "[[exception]]\npackage = \"p\"\nset = \"s\"\nlicense = \"MIT\"\n\
             versions = \"*\"\nreason = \"r\"\nexpires = \"2027-03-01\"\nnote = \"x\"\n",
            // Malformed TOML.
            "[[exception]\n",
            // Duplicate keys.
            "[policy.distributed]\nallow = [\"MIT\"]\nallow = [\"ISC\"]\n",
        ] {
            assert!(
                matches!(
                    load_licenses_toml(bad),
                    Err(PolicyProblem::InvalidLicensesToml { .. })
                ),
                "{bad:?} must fail as invalid TOML"
            );
        }
    }

    #[test]
    fn loader_validates_tables_and_set_adjustments() {
        let multi = "[policy.distributed]\nallow = [\"MIT\"]\ndeny = [\"MIT\"]\n";
        assert_eq!(
            load_licenses_toml(multi),
            Err(PolicyProblem::MultiListed {
                identity: "MIT".to_owned()
            })
        );
        let conflict = "[policy.distributed]\nallow = [\"MIT\"]\n\
                        [policy.sets.npm-root]\nreview = [\"MIT\"]\n";
        assert_eq!(
            load_licenses_toml(conflict),
            Err(PolicyProblem::SetConflict {
                set: "npm-root".to_owned(),
                identity: "MIT".to_owned(),
            })
        );
    }

    #[test]
    fn loaded_exceptions_validate_against_audit_date() {
        let policy = load_licenses_toml(doc_example()).expect("doc example loads");
        validate_license_exception(&policy.exceptions[0], "2026-09-14").expect("valid");
        assert!(validate_license_exception(&policy.exceptions[0], "2027-03-01").is_err());
    }

    #[test]
    fn schema_version_accepts_v1_and_rejects_other_versions() {
        assert_eq!(LICENSE_POLICY_SCHEMA_VERSION, 1);
        // Absent version means v1 for pre-versioned files.
        load_licenses_toml("").expect("empty document loads as v1");
        load_licenses_toml("schema_version = 1\n").expect("explicit v1 loads");
        for bad in [
            "schema_version = 0\n",
            "schema_version = 2\n",
            "schema_version = 999\n",
        ] {
            assert!(
                matches!(
                    load_licenses_toml(bad),
                    Err(PolicyProblem::InvalidLicensesToml { .. })
                ),
                "{bad:?} must fail as unsupported schema version"
            );
        }
    }

    #[test]
    fn license_additions_need_no_struct_edits() {
        // New identities are data in the versioned TOML lists, never struct
        // edits: any string loads and validates as single-listed.
        let policy =
            load_licenses_toml("[policy.distributed]\nallow = [\"MIT\", \"New-Permissive-1.0\"]\n")
                .expect("new allow identity loads");
        assert!(policy.tables.allow.contains("New-Permissive-1.0"));
        policy.tables.validate().expect("single listing passes");
    }

    #[test]
    fn loader_converts_inventory_entries_per_ecosystem() {
        let text = r#"
[[inventory]]
package = "react"
set = "npm"
license = "MIT"
versions = "18.2.0"
text_present = true

[[inventory]]
package = "junit:junit"
set = "maven"
license = "EPL-1.0"
versions = "4.13.2"

[[inventory]]
package = "FSharp.Core"
set = "nuget"
license = "MIT"
versions = "10.1.201"
text_present = true

[[inventory]]
package = "github.com/google/go-cmp"
set = "go"
license = "BSD-3-Clause"
versions = "v0.6.0"
text_present = false

[[inventory]]
package = "serde"
set = "cargo"
license = "MIT OR Apache-2.0"
versions = "1.0.100"
text_present = true
"#;
        let policy = load_licenses_toml(text).expect("inventory loads");
        assert_eq!(policy.inventory.len(), 5);
        let npm = policy
            .inventory
            .iter()
            .find(|entry| entry.set == "npm")
            .expect("npm entry");
        assert_eq!(npm.package, "react");
        assert_eq!(npm.license, "MIT");
        assert!(npm.text_present);
        let maven = policy
            .inventory
            .iter()
            .find(|entry| entry.set == "maven")
            .expect("maven entry");
        assert_eq!(maven.package, "junit:junit");
        // Absent `text_present` defaults to false (fail closed).
        assert!(!maven.text_present);
        validate_license_inventory(npm).expect("valid entry passes");
    }

    #[test]
    fn loader_rejects_empty_inventory_fields_and_unknown_keys() {
        for bad in [
            "[[inventory]]\npackage = \"\"\nset = \"npm\"\nlicense = \"MIT\"\nversions = \"1.0.0\"\n",
            "[[inventory]]\npackage = \"react\"\nset = \"\"\nlicense = \"MIT\"\nversions = \"1.0.0\"\n",
            "[[inventory]]\npackage = \"react\"\nset = \"npm\"\nlicense = \"\"\nversions = \"1.0.0\"\n",
            "[[inventory]]\npackage = \"react\"\nset = \"npm\"\nlicense = \"MIT\"\nversions = \"\"\n",
        ] {
            assert!(
                matches!(
                    load_licenses_toml(bad),
                    Err(PolicyProblem::ExceptionMissingField { .. })
                ),
                "{bad:?} must fail on empty inventory field"
            );
        }
        // Unknown inventory keys fail as invalid TOML, never silent drift.
        let typo = "[[inventory]]\npackage = \"react\"\nset = \"npm\"\nlicense = \"MIT\"\nversions = \"1.0.0\"\nlicence = \"x\"\n";
        assert!(
            matches!(
                load_licenses_toml(typo),
                Err(PolicyProblem::InvalidLicensesToml { .. })
            ),
            "typo'd inventory key must fail as invalid TOML"
        );
    }
}
