//! SPDX license-expression evaluation (M26 WP3 slice 1).
//!
//! Pure boolean-math evaluation over the allow/review/deny lattice from
//! the license contract
//! (`docs/cli/commands/audit-update-bazel.md#license-family-dx-audit-license`):
//!
//! * `A OR B` passes if any disjunct is allowed (the distributor chooses
//!   the license); otherwise review if any disjunct needs review;
//!   otherwise denied. `MIT OR AGPL-3.0-only` therefore passes by
//!   choosing MIT.
//! * `A AND B` is denied if any conjunct is denied (all terms must be
//!   satisfied); otherwise review if any conjunct needs review;
//!   otherwise allowed.
//! * `WITH <exception>` expressions require approval of the complete
//!   expression verbatim wherever license-policy approval is required.
//!   Allowing the base license alone does not approve the expression.
//! * `UNKNOWN` or unparseable license text is denied in `distributed`
//!   and inventoried in `internal`.
//! * `blocked` acts as deny in both tiers; only a matching, reasoned,
//!   version-scoped, unexpired exception (see [`crate::exception`])
//!   approves a finding, without hiding it.
//!
//! This module evaluates over injected expression trees and injected
//! identity/approval predicates only, so the lattice stays deterministic
//! and unit-testable without any lockfile, advisory snapshot, or TOML
//! policy file. SPDX text parsing, per-ecosystem license-identity
//! mappings, policy-table loading, tier attribution for shared locks,
//! and proof evidence stay O58-gated for later slices; the expression
//! shape here is the injected AST a future parser must produce.

/// Distribution tier under evaluation. `distributed` release roots
/// leave the company and face the strict table; `internal` roots are
/// inventoried in the SBOM and never fail on the allow/review/deny
/// table (except `blocked`, which fails in both tiers).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tier {
    Distributed,
    Internal,
}

/// Policy-table classification of one SPDX identity: exactly one list
/// (`allow`, `review`, `deny`, `blocked`) or none (`Unlisted`).
/// Membership in more than one list fails validation in a later slice;
/// evaluation assumes the validated single listing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdClass {
    Allow,
    Review,
    Deny,
    Blocked,
    Unlisted,
}

/// Lattice outcome of one evaluated expression. Ordered Allow < Review
/// < Deny by strictness: `OR` takes the most permissive disjunct, `AND`
/// takes the strictest conjunct.
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum TierOutcome {
    Allow,
    Review,
    Deny,
}

/// Injected SPDX expression tree. A future parser produces this shape
/// from lock metadata; per-ecosystem text-to-identity mappings are O58
/// qualification, so unparseable text arrives as [`LicenseExpr::Unknown`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LicenseExpr {
    /// One SPDX identity (or `UNKNOWN` spelled as unknown text).
    Ident(String),
    /// `A OR B` over two or more disjuncts.
    Or(Vec<LicenseExpr>),
    /// `A AND B` over two or more conjuncts.
    And(Vec<LicenseExpr>),
    /// `base WITH exception`: `text` is the complete expression
    /// verbatim, the only string verbatim approval may name.
    With {
        base: Box<LicenseExpr>,
        exception: String,
        text: String,
    },
    /// Unparseable license text. Never silently allowed.
    Unknown,
}

/// Evaluate one expression under a tier.
///
/// * `lookup` classifies each SPDX identity against the (validated)
///   policy table; unlisted identities and [`LicenseExpr::Unknown`]
///   are denied in `distributed` and inventoried (`Allow`) in
///   `internal`.
/// * `approved` reports whether its argument names a finding covered by
///   a matching, reasoned, version-scoped, unexpired exception: a bare
///   identity for [`LicenseExpr::Ident`], or the complete expression
///   verbatim for [`LicenseExpr::With`]. Approval passes the finding
///   without hiding it; a `review` listing or an allowed base license
///   alone is never approval.
///
/// `blocked` (via [`IdClass::Blocked`] or a denied `WITH` base) acts as
/// deny in both tiers unless `approved` names it.
pub fn evaluate(
    expr: &LicenseExpr,
    tier: Tier,
    lookup: &dyn Fn(&str) -> IdClass,
    approved: &dyn Fn(&str) -> bool,
) -> TierOutcome {
    match expr {
        LicenseExpr::Ident(id) => evaluate_ident(id, tier, lookup, approved),
        LicenseExpr::Unknown => match tier {
            Tier::Distributed => TierOutcome::Deny,
            Tier::Internal => TierOutcome::Allow,
        },
        LicenseExpr::Or(disjuncts) => {
            let mut outcome = TierOutcome::Deny;
            for disjunct in disjuncts {
                let next = evaluate(disjunct, tier, lookup, approved);
                if next < outcome {
                    outcome = next;
                }
                if outcome == TierOutcome::Allow {
                    break;
                }
            }
            outcome
        }
        LicenseExpr::And(conjuncts) => {
            let mut outcome = TierOutcome::Allow;
            for conjunct in conjuncts {
                let next = evaluate(conjunct, tier, lookup, approved);
                if next > outcome {
                    outcome = next;
                }
                if outcome == TierOutcome::Deny {
                    break;
                }
            }
            outcome
        }
        LicenseExpr::With { base, text, .. } => {
            if approved(text) {
                return TierOutcome::Allow;
            }
            match evaluate(base, tier, lookup, approved) {
                TierOutcome::Deny => TierOutcome::Deny,
                TierOutcome::Allow | TierOutcome::Review => TierOutcome::Review,
            }
        }
    }
}

/// Whether a lattice outcome fails the audit in a tier. `Review` fails
/// in `distributed` unless explicitly approved (approval already folds
/// into [`evaluate`]); `internal` inventories everything except deny.
pub fn fails_in_tier(outcome: TierOutcome, tier: Tier) -> bool {
    match (outcome, tier) {
        (TierOutcome::Deny, _) => true,
        (TierOutcome::Review, Tier::Distributed) => true,
        (TierOutcome::Review, Tier::Internal) | (TierOutcome::Allow, _) => false,
    }
}

/// Classify one bare identity under a tier, before boolean combination.
fn evaluate_ident(
    id: &str,
    tier: Tier,
    lookup: &dyn Fn(&str) -> IdClass,
    approved: &dyn Fn(&str) -> bool,
) -> TierOutcome {
    if approved(id) {
        return TierOutcome::Allow;
    }
    match (lookup(id), tier) {
        (IdClass::Allow, _) => TierOutcome::Allow,
        (IdClass::Blocked, _) => TierOutcome::Deny,
        (IdClass::Review, Tier::Distributed) => TierOutcome::Review,
        (IdClass::Review, Tier::Internal) => TierOutcome::Allow,
        (IdClass::Deny, Tier::Distributed) => TierOutcome::Deny,
        (IdClass::Deny, Tier::Internal) => TierOutcome::Allow,
        (IdClass::Unlisted, Tier::Distributed) => TierOutcome::Deny,
        (IdClass::Unlisted, Tier::Internal) => TierOutcome::Allow,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn table(entries: &[(&str, IdClass)]) -> HashMap<String, IdClass> {
        entries
            .iter()
            .map(|(id, class)| ((*id).to_owned(), *class))
            .collect()
    }

    fn contract_table() -> HashMap<String, IdClass> {
        table(&[
            ("MIT", IdClass::Allow),
            ("Apache-2.0", IdClass::Allow),
            ("LGPL-2.1-only", IdClass::Review),
            ("MPL-2.0", IdClass::Review),
            ("GPL-3.0-only", IdClass::Deny),
            ("AGPL-3.0-only", IdClass::Blocked),
        ])
    }

    fn lookup(table: &HashMap<String, IdClass>) -> impl Fn(&str) -> IdClass + '_ {
        move |id| table.get(id).copied().unwrap_or(IdClass::Unlisted)
    }

    fn none_approved() -> impl Fn(&str) -> bool {
        |_| false
    }

    fn approved(names: &[&str]) -> impl Fn(&str) -> bool {
        let owned: Vec<String> = names.iter().copied().map(str::to_owned).collect();
        move |name| owned.iter().any(|known| known == name)
    }

    fn ident(id: &str) -> LicenseExpr {
        LicenseExpr::Ident(id.to_owned())
    }

    #[test]
    fn allowed_identity_passes_both_tiers() {
        let table = contract_table();
        let lookup = lookup(&table);
        for tier in [Tier::Distributed, Tier::Internal] {
            assert_eq!(
                evaluate(&ident("MIT"), tier, &lookup, &none_approved()),
                TierOutcome::Allow
            );
        }
    }

    #[test]
    fn or_passes_by_choosing_the_allowed_disjunct() {
        let table = contract_table();
        let lookup = lookup(&table);
        let expr = LicenseExpr::Or(vec![ident("MIT"), ident("AGPL-3.0-only")]);
        assert_eq!(
            evaluate(&expr, Tier::Distributed, &lookup, &none_approved()),
            TierOutcome::Allow
        );
    }

    #[test]
    fn or_falls_back_to_review_then_deny() {
        let table = contract_table();
        let lookup = lookup(&table);
        let review = LicenseExpr::Or(vec![ident("MPL-2.0"), ident("GPL-3.0-only")]);
        assert_eq!(
            evaluate(&review, Tier::Distributed, &lookup, &none_approved()),
            TierOutcome::Review
        );
        let denied = LicenseExpr::Or(vec![ident("GPL-3.0-only"), ident("Unicode-3.0")]);
        assert_eq!(
            evaluate(&denied, Tier::Distributed, &lookup, &none_approved()),
            TierOutcome::Deny
        );
    }

    #[test]
    fn and_is_denied_if_any_conjunct_is_denied() {
        let table = contract_table();
        let lookup = lookup(&table);
        let expr = LicenseExpr::And(vec![ident("MIT"), ident("GPL-3.0-only")]);
        assert_eq!(
            evaluate(&expr, Tier::Distributed, &lookup, &none_approved()),
            TierOutcome::Deny
        );
        let review = LicenseExpr::And(vec![ident("MIT"), ident("MPL-2.0")]);
        assert_eq!(
            evaluate(&review, Tier::Distributed, &lookup, &none_approved()),
            TierOutcome::Review
        );
        let allowed = LicenseExpr::And(vec![ident("MIT"), ident("Apache-2.0")]);
        assert_eq!(
            evaluate(&allowed, Tier::Distributed, &lookup, &none_approved()),
            TierOutcome::Allow
        );
    }

    #[test]
    fn with_requires_verbatim_approval_despite_allowed_base() {
        let table = contract_table();
        let lookup = lookup(&table);
        let expr = LicenseExpr::With {
            base: Box::new(ident("Apache-2.0")),
            exception: "LLVM-exception".to_owned(),
            text: "Apache-2.0 WITH LLVM-exception".to_owned(),
        };
        assert_eq!(
            evaluate(&expr, Tier::Distributed, &lookup, &none_approved()),
            TierOutcome::Review
        );
        let verbatim = approved(&["Apache-2.0 WITH LLVM-exception"]);
        assert_eq!(
            evaluate(&expr, Tier::Distributed, &lookup, &verbatim),
            TierOutcome::Allow
        );
        // Approving the base alone never approves the expression.
        let base_only = approved(&["Apache-2.0"]);
        assert_eq!(
            evaluate(&expr, Tier::Distributed, &lookup, &base_only),
            TierOutcome::Review
        );
    }

    #[test]
    fn with_over_denied_base_stays_denied_without_verbatim_approval() {
        let table = contract_table();
        let lookup = lookup(&table);
        let expr = LicenseExpr::With {
            base: Box::new(ident("GPL-3.0-only")),
            exception: "Classpath-exception-2.0".to_owned(),
            text: "GPL-3.0-only WITH Classpath-exception-2.0".to_owned(),
        };
        assert_eq!(
            evaluate(&expr, Tier::Distributed, &lookup, &none_approved()),
            TierOutcome::Deny
        );
    }

    #[test]
    fn unknown_is_denied_distributed_and_inventoried_internal() {
        let table = contract_table();
        let lookup = lookup(&table);
        assert_eq!(
            evaluate(
                &LicenseExpr::Unknown,
                Tier::Distributed,
                &lookup,
                &none_approved()
            ),
            TierOutcome::Deny
        );
        assert_eq!(
            evaluate(
                &LicenseExpr::Unknown,
                Tier::Internal,
                &lookup,
                &none_approved()
            ),
            TierOutcome::Allow
        );
        assert_eq!(
            evaluate(
                &ident("Made-Up-1.0"),
                Tier::Distributed,
                &lookup,
                &none_approved()
            ),
            TierOutcome::Deny
        );
        assert_eq!(
            evaluate(
                &ident("Made-Up-1.0"),
                Tier::Internal,
                &lookup,
                &none_approved()
            ),
            TierOutcome::Allow
        );
    }

    #[test]
    fn blocked_is_deny_in_both_tiers_until_excepted() {
        let table = contract_table();
        let lookup = lookup(&table);
        for tier in [Tier::Distributed, Tier::Internal] {
            assert_eq!(
                evaluate(&ident("AGPL-3.0-only"), tier, &lookup, &none_approved()),
                TierOutcome::Deny
            );
        }
        let excepted = approved(&["AGPL-3.0-only"]);
        for tier in [Tier::Distributed, Tier::Internal] {
            assert_eq!(
                evaluate(&ident("AGPL-3.0-only"), tier, &lookup, &excepted),
                TierOutcome::Allow
            );
        }
    }

    #[test]
    fn internal_inventories_review_and_deny_without_hiding_blocked() {
        let table = contract_table();
        let lookup = lookup(&table);
        assert_eq!(
            evaluate(&ident("MPL-2.0"), Tier::Internal, &lookup, &none_approved()),
            TierOutcome::Allow
        );
        assert_eq!(
            evaluate(
                &ident("GPL-3.0-only"),
                Tier::Internal,
                &lookup,
                &none_approved()
            ),
            TierOutcome::Allow
        );
        assert_eq!(
            evaluate(
                &ident("AGPL-3.0-only"),
                Tier::Internal,
                &lookup,
                &none_approved()
            ),
            TierOutcome::Deny
        );
    }

    #[test]
    fn review_listing_is_not_approval() {
        let table = contract_table();
        let lookup = lookup(&table);
        // Merely placing an identity in `review` still fails distributed.
        assert_eq!(
            evaluate(
                &ident("MPL-2.0"),
                Tier::Distributed,
                &lookup,
                &none_approved()
            ),
            TierOutcome::Review
        );
        assert!(fails_in_tier(TierOutcome::Review, Tier::Distributed));
        assert!(!fails_in_tier(TierOutcome::Review, Tier::Internal));
        assert!(fails_in_tier(TierOutcome::Deny, Tier::Internal));
        assert!(!fails_in_tier(TierOutcome::Allow, Tier::Distributed));
    }

    #[test]
    fn empty_combinations_fail_closed() {
        let table = contract_table();
        let lookup = lookup(&table);
        assert_eq!(
            evaluate(
                &LicenseExpr::Or(vec![]),
                Tier::Distributed,
                &lookup,
                &none_approved()
            ),
            TierOutcome::Deny
        );
        assert_eq!(
            evaluate(
                &LicenseExpr::And(vec![]),
                Tier::Distributed,
                &lookup,
                &none_approved()
            ),
            TierOutcome::Allow
        );
    }

    #[test]
    fn lattice_ordering_pins_or_and_extremes() {
        assert!(TierOutcome::Allow < TierOutcome::Review);
        assert!(TierOutcome::Review < TierOutcome::Deny);
    }
}
