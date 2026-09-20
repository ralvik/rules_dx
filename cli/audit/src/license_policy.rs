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
//! [`crate::exception::check_expiry`], and obsolescence is identity
//! match over package plus license. An upgrade within an exception's
//! bounded version range retains acceptance while the exception still
//! matches the finding and remains otherwise valid; version-range
//! narrowing itself calls [`crate::exception::version_in_scope`] in the
//! resolver-owned slices, exactly like the vulnerability deferral
//! in [`crate::exception`].
//!
//! This module plans over injected table/root/exception records only,
//! so policy validation stays deterministic and unit-testable without
//! any lockfile or Bazel integration. [`load_licenses_toml`] parses the
//! committed `licenses.toml` root file into those records with `toml`
//! plus `serde`; per-ecosystem license-identity mappings, shared-lock
//! tier attribution, and proof evidence stay gated for later slices.

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
    /// Accepted versions or bounded range, upstream version semantics.
    /// Cargo-flavor scopes evaluate with
    /// [`crate::exception::version_in_scope`]; non-semver ecosystem
    /// scopes stay opaque for the resolver-owned ecosystem integration.
    pub versions: String,
    /// Explanatory reason. Empty reasons fail validation.
    pub reason: String,
    /// Expiration date, ISO-8601 UTC `YYYY-MM-DD`, evaluated at audit time.
    pub expires: String,
}

/// One assessed license finding an exception may apply to. The version
/// is carried for the future upstream-semantics range matcher;
/// applicability today is identity match over package plus license, and
/// an upgrade alone never invalidates an otherwise valid exception.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LicenseFinding {
    /// Affected package name.
    pub package: String,
    /// Found SPDX identity or `WITH` expression text.
    pub license: String,
    /// Found package version, reserved for range evaluation.
    pub version: String,
}

/// Validate one license exception against the injected audit date:
/// field presence plus the shared expiry lifecycle. Out-of-range
/// versions do not inherit acceptance; that narrowing arrives with the
/// upstream-semantics matcher.
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

/// True when no finding shares the exception's package and license
/// identity. Never infers obsolescence from failed/incomplete analysis.
pub fn is_obsolete(exception: &LicenseException, findings: &[LicenseFinding]) -> bool {
    !findings
        .iter()
        .any(|finding| finding.package == exception.package && finding.license == exception.license)
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
}

/// Load and validate one `licenses.toml` document into domain records.
///
/// Parses with `toml` plus `serde`, converts the file shape, then
/// validates the global table and every per-set adjustment, so a
/// loaded policy never carries multi-listed identities or set
/// conflicts. Unknown fields fail as [`PolicyProblem::InvalidLicensesToml`]:
/// a typo'd key (`alow`) must never silently become an empty list —
/// least of all an empty `blocked` list. Missing sections default to
/// empty, which stays fail-closed because unlisted identities and
/// distributables default to the strict side. Distribution roots and
/// exceptions convert verbatim; their call-site validation (known
/// labels, findings, audit date) is unchanged. The optional
/// `schema_version` must be [`LICENSE_POLICY_SCHEMA_VERSION`] when present
/// (absent means v1 for pre-versioned files); any other version fails
/// as invalid TOML so schema evolution is explicit, never silent drift.
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
    };
    policy.tables.validate()?;
    for adjustment in &policy.sets {
        policy.tables.validate_set(adjustment)?;
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
            set: "cargo-lock".to_owned(),
            license: "GPL-3.0-only".to_owned(),
            versions: ">=1.2.0, <2.0.0".to_owned(),
            reason: "Legal approved for internal fork; re-review on major bump.".to_owned(),
            expires: "2027-03-01".to_owned(),
        }
    }

    fn finding() -> LicenseFinding {
        LicenseFinding {
            package: "some-copyleft-lib".to_owned(),
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
    }

    #[test]
    fn upgrade_within_range_retains_acceptance_at_identity_match() {
        // Version-range narrowing calls version_in_scope in the
        // resolver-owned slices; an upgrade alone never invalidates: the
        // finding still carries the same package and license identity.
        let upgraded = LicenseFinding {
            version: "1.9.0".to_owned(),
            ..finding()
        };
        assert!(!is_obsolete(&exception(), &[upgraded.clone()]));
        check_applies(&exception(), &[upgraded]).expect("upgrade retains acceptance");
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
set = "cargo-lock"
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
                set: "cargo-lock".to_owned(),
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
}
