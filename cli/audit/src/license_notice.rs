//! License notice-text inputs and SPDX report shape (WP3 slice 3).
//!
//! Pure planning for the license-evidence tail of the license contract
//! (`docs/cli/commands/audit-update-bazel.md#license-family-dx-audit-license`):
//! lock metadata says *which* license a package claims, while the
//! words (copyright notice plus text) come from each package archive's
//! `LICENSE*`/`NOTICE*` files, delivered as declared Bazel inputs per
//! package so future NOTICE aggregation stays hermetic and cached. A
//! package whose license requires reproduction but ships no text
//! reports `missing-notice-text`, which fails in `distributed` unless
//! excepted and is inventoried in `internal`.
//!
//! The report is one SPDX 2.3 JSON document per invocation, with
//! package IDs as package URLs, `DESCRIBES` relations from each audited
//! root, and `CONTAINS` relations where the lock graph is known.
//! Assembling and bundling an aggregated NOTICE artifact into releases
//! is out of scope until the deferred packaging/publishing pipeline
//! exists; collecting the texts now keeps the data ready.
//!
//! This module plans over injected notice records only. The shared
//! `--report` format identifier and event mapping for SPDX remain
//! pending under per the output protocol; no identifier string or
//! event schema is invented here. Full per-ecosystem
//! license-identity mappings and proof evidence stay gated.

use crate::license_expr::{Tier, TierOutcome};

/// SPDX document version pinned by the license contract.
pub const SPDX_VERSION: &str = "2.3";

/// Package identifier scheme: package URLs.
pub const PACKAGE_ID_SCHEME: &str = "package-url";

/// Relationship from each audited root to the document it describes.
pub const DESCRIBES_RELATIONSHIP: &str = "DESCRIBES";

/// Relationship recording lock-graph containment where the graph is
/// known. Unknown graphs omit it rather than guessing containment.
pub const CONTAINS_RELATIONSHIP: &str = "CONTAINS";

/// Report granularity: exactly one SPDX document per audit invocation,
/// never one per package, set, or root.
pub fn documents_per_invocation() -> usize {
    1
}

/// Aggregated NOTICE assembly stays out of scope until the deferred
/// packaging/publishing pipeline exists. Pinned here so a future
/// release integration cannot assume the artifact already ships.
pub fn aggregates_notice_artifact() -> bool {
    false
}

/// SPDX identities the contract names as legally requiring notice-text
/// reproduction (copyright notice plus text). Full per-ecosystem
/// license-identity mappings stay gated; this seed covers exactly
/// the contract-named MIT/BSD/Apache-2.0 families and nothing else.
pub const NOTICE_REQUIRED_IDS: &[&str] = &["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause"];

/// Whether the named SPDX identity requires reproduction of the
/// license words. Unknown identities are handled by the expression
/// lattice (denied in `distributed`); notice evaluation only asks
/// whether *known listed* identities need their words collected.
pub fn requires_notice_text(identity: &str) -> bool {
    NOTICE_REQUIRED_IDS.contains(&identity)
}

/// One package's declared notice-text input: the words delivered as a
/// declared Bazel input per package, present or not.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NoticeInput {
    /// Affected package name.
    pub package: String,
    /// Claimed SPDX identity.
    pub license: String,
    /// Whether the package archive delivered `LICENSE*`/`NOTICE*`
    /// words as a declared input.
    pub text_present: bool,
}

/// Evaluate one notice input under a tier: `missing-notice-text` fails
/// in `distributed` unless a matching exception approves it, and is
/// inventoried in `internal`. `approved` names licenses covered by a
/// matching, reasoned, version-scoped, unexpired exception, mirroring
/// the expression-lattice approval hook.
pub fn evaluate_notice(
    input: &NoticeInput,
    tier: Tier,
    approved: &dyn Fn(&str) -> bool,
) -> TierOutcome {
    if !requires_notice_text(&input.license) || input.text_present || approved(&input.license) {
        return TierOutcome::Allow;
    }
    match tier {
        Tier::Distributed => TierOutcome::Deny,
        Tier::Internal => TierOutcome::Allow,
    }
}

/// Whether a notice outcome fails the audit in a tier. Missing text
/// that survives evaluation is a deny, so it fails everywhere it can
/// appear; internal inventory already folds to allow in
/// [`evaluate_notice`].
pub fn notice_fails(outcome: TierOutcome, tier: Tier) -> bool {
    crate::license_expr::fails_in_tier(outcome, tier)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none_approved() -> impl Fn(&str) -> bool {
        |_| false
    }

    fn input(license: &str, text_present: bool) -> NoticeInput {
        NoticeInput {
            package: "some-lib".to_owned(),
            license: license.to_owned(),
            text_present,
        }
    }

    #[test]
    fn report_shape_pins_are_frozen() {
        assert_eq!(SPDX_VERSION, "2.3");
        assert_eq!(PACKAGE_ID_SCHEME, "package-url");
        assert_eq!(DESCRIBES_RELATIONSHIP, "DESCRIBES");
        assert_eq!(CONTAINS_RELATIONSHIP, "CONTAINS");
        assert_eq!(documents_per_invocation(), 1);
        assert!(!aggregates_notice_artifact());
    }

    #[test]
    fn contract_named_families_require_notice_text() {
        for identity in ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause"] {
            assert!(requires_notice_text(identity), "{identity} needs words");
        }
        // The seed covers exactly the contract-named families; anything
        // else waits on identity qualification.
        for identity in ["ISC", "MPL-2.0", "GPL-3.0-only", "Unicode-3.0"] {
            assert!(!requires_notice_text(identity), "{identity} unqualified");
        }
    }

    #[test]
    fn present_text_passes_everywhere() {
        for tier in [Tier::Distributed, Tier::Internal] {
            assert_eq!(
                evaluate_notice(&input("MIT", true), tier, &none_approved()),
                TierOutcome::Allow
            );
        }
    }

    #[test]
    fn missing_text_fails_distributed_and_inventories_internal() {
        assert_eq!(
            evaluate_notice(&input("MIT", false), Tier::Distributed, &none_approved()),
            TierOutcome::Deny
        );
        assert_eq!(
            evaluate_notice(&input("MIT", false), Tier::Internal, &none_approved()),
            TierOutcome::Allow
        );
        assert!(notice_fails(TierOutcome::Deny, Tier::Distributed));
        assert!(!notice_fails(TierOutcome::Allow, Tier::Internal));
    }

    #[test]
    fn matching_exception_approves_missing_text_without_hiding() {
        let approved = |name: &str| name == "MIT";
        assert_eq!(
            evaluate_notice(&input("MIT", false), Tier::Distributed, &approved),
            TierOutcome::Allow
        );
    }

    #[test]
    fn non_reproduction_licenses_need_no_text() {
        for tier in [Tier::Distributed, Tier::Internal] {
            assert_eq!(
                evaluate_notice(&input("MPL-2.0", false), tier, &none_approved()),
                TierOutcome::Allow
            );
        }
    }
}
