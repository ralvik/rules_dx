//! License tier policy and distribution roots (M26 WP3 slice 2).
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
//! narrowing itself uses upstream ecosystem semantics and arrives with
//! the resolver-owned slices, exactly like the vulnerability deferral
//! in [`crate::exception`].
//!
//! This module plans over injected table/root/exception records only,
//! so policy validation stays deterministic and unit-testable without
//! any lockfile or Bazel integration. TOML loading, per-ecosystem
//! license-identity mappings, shared-lock tier attribution, and proof
//! evidence stay O58-gated for later slices.

use std::collections::{BTreeMap, BTreeSet};

use crate::exception::{check_expiry, ExceptionProblem};
use crate::license_expr::Tier;

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
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Distribution {
    /// Release roots that leave the company. Listing here is optional:
    /// unlisted labels are distributed anyway.
    pub distributed: BTreeSet<String>,
    /// Internal-only roots, inventoried but never gated on the
    /// allow/review/deny table (except `blocked`).
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
/// mirror the committed `licenses.toml` `[[exception]]` entries so a
/// future TOML loader cannot reinterpret them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LicenseException {
    /// Affected package name.
    pub package: String,
    /// Owning dependency set.
    pub set: String,
    /// Approved SPDX identity or complete `WITH` expression verbatim.
    pub license: String,
    /// Accepted versions or bounded range, upstream version semantics.
    /// Opaque in this slice; evaluated by the ecosystem integration.
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
        // Version-range narrowing uses upstream semantics in a later
        // slice; an upgrade alone never invalidates: the finding still
        // carries the same package and license identity.
        let upgraded = LicenseFinding {
            version: "1.9.0".to_owned(),
            ..finding()
        };
        assert!(!is_obsolete(&exception(), &[upgraded.clone()]));
        check_applies(&exception(), &[upgraded]).expect("upgrade retains acceptance");
    }
}
