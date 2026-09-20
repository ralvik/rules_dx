//! SPDX license-expression evaluation (WP3 slice 1).
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
//! policy file. [`parse_license`] builds the expression shape from SPDX
//! text via the upstream `spdx` parser; per-ecosystem license-identity
//! mappings, policy-table loading, tier attribution for shared locks,
//! and proof evidence stay gated for later slices.
//!
//! Dependency evaluation (adopted): SPDX text parses via the
//! upstream `spdx` crate in strict mode (fail-closed to [`LicenseExpr::Unknown`]);
//! the allow/review/deny lattice plus `WITH` verbatim approval stays hand-rolled
//! because it is the repo's license-policy contract, not an upstream type
//! (`license-exprs` would duplicate the lattice at dependency cost).

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

/// Injected SPDX expression tree. [`parse_license`] produces this shape
/// from lock metadata via the upstream `spdx` parser; per-ecosystem
/// text-to-identity mappings are qualification, so text the parser
/// rejects arrives as [`LicenseExpr::Unknown`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LicenseExpr {
    /// One SPDX identity (or `UNKNOWN` spelled as unknown text).
    Ident(String),
    /// `A OR B` over two or more disjuncts.
    Or(Vec<LicenseExpr>),
    /// `A AND B` over two or more conjuncts.
    And(Vec<LicenseExpr>),
    /// `base WITH exception`: `text` is the canonical `{base} WITH
    /// {exception}` rendering, the only string verbatim approval may
    /// name (the upstream parser's requirement span covers the base
    /// license only, so the full requirement cannot be sliced back out
    /// of the operator input).
    With {
        base: Box<LicenseExpr>,
        exception: String,
        text: String,
    },
    /// Unparseable license text. Never silently allowed.
    Unknown,
}

/// Parse SPDX license-expression text into the [`LicenseExpr`] shape.
///
/// Uses the upstream `spdx` parser in strict mode: syntactically or
/// semantically invalid text — including unknown SPDX identities — maps
/// to [`LicenseExpr::Unknown`], denied in `distributed` and inventoried
/// in `internal` by [`evaluate`]. Unknown-to-SPDX text therefore no
/// longer reaches per-identity approval as an [`LicenseExpr::Ident`];
/// failing closed is deliberate (an unrecognized license must never be
/// silently allowed, and a bare-identity approval must not bless text
/// the parser cannot attribute).
///
/// Valid `WITH` requirements render `text` canonically as `{base}
/// WITH {exception}`, so verbatim approval matches canonically spelled
/// policy input; the base identity and exception strings are the
/// parser's canonical spellings for stable policy-table lookup. Same-operator nesting
/// flattens (`A OR (B OR C)` parses like `A OR B OR C`), so parsed trees
/// compare equal to hand-built flat combinations. The allow/review/deny
/// lattice itself stays custom in [`evaluate`].
pub fn parse_license(text: &str) -> LicenseExpr {
    let expr = match spdx::Expression::parse(text) {
        Ok(expr) => expr,
        Err(_) => return LicenseExpr::Unknown,
    };
    let mut stack: Vec<LicenseExpr> = Vec::new();
    for node in expr.iter() {
        match node {
            spdx::expression::ExprNode::Req(req) => stack.push(leaf_expr(req)),
            spdx::expression::ExprNode::Op(op) => {
                let rhs = match stack.pop() {
                    Some(expr) => expr,
                    None => return LicenseExpr::Unknown,
                };
                let lhs = match stack.pop() {
                    Some(expr) => expr,
                    None => return LicenseExpr::Unknown,
                };
                stack.push(match op {
                    spdx::expression::Operator::Or => merge_or(lhs, rhs),
                    spdx::expression::Operator::And => merge_and(lhs, rhs),
                });
            }
        }
    }
    if stack.len() == 1 {
        stack.pop().unwrap_or(LicenseExpr::Unknown)
    } else {
        LicenseExpr::Unknown
    }
}

/// Map one parsed license requirement to a [`LicenseExpr`] leaf.
///
/// `WITH` requirements become [`LicenseExpr::With`] with the canonical
/// `{base} WITH {exception}` rendering as `text`; anything else becomes
/// a canonical-spelling [`LicenseExpr::Ident`].
fn leaf_expr(req: &spdx::expression::ExpressionReq) -> LicenseExpr {
    match &req.req.addition {
        Some(addition) => {
            let base = req.req.license.to_string();
            let exception = addition.to_string();
            LicenseExpr::With {
                text: format!("{base} WITH {exception}"),
                base: Box::new(LicenseExpr::Ident(base)),
                exception,
            }
        }
        None => LicenseExpr::Ident(req.req.license.to_string()),
    }
}

/// Combine two disjuncts, flattening nested `OR` so parsed trees match
/// hand-built flat combinations.
fn merge_or(lhs: LicenseExpr, rhs: LicenseExpr) -> LicenseExpr {
    let mut disjuncts = match lhs {
        LicenseExpr::Or(items) => items,
        other => vec![other],
    };
    match rhs {
        LicenseExpr::Or(mut items) => disjuncts.append(&mut items),
        other => disjuncts.push(other),
    }
    LicenseExpr::Or(disjuncts)
}

/// Combine two conjuncts, flattening nested `AND` so parsed trees match
/// hand-built flat combinations.
fn merge_and(lhs: LicenseExpr, rhs: LicenseExpr) -> LicenseExpr {
    let mut conjuncts = match lhs {
        LicenseExpr::And(items) => items,
        other => vec![other],
    };
    match rhs {
        LicenseExpr::And(mut items) => conjuncts.append(&mut items),
        other => conjuncts.push(other),
    }
    LicenseExpr::And(conjuncts)
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

    #[test]
    fn parse_single_identity_uses_canonical_spelling() {
        assert_eq!(parse_license("MIT"), ident("MIT"));
        assert_eq!(parse_license("Apache-2.0"), ident("Apache-2.0"));
    }

    #[test]
    fn parse_or_and_match_flat_combinations() {
        assert_eq!(
            parse_license("MIT OR Apache-2.0"),
            LicenseExpr::Or(vec![ident("MIT"), ident("Apache-2.0")])
        );
        assert_eq!(
            parse_license("MIT AND Apache-2.0"),
            LicenseExpr::And(vec![ident("MIT"), ident("Apache-2.0")])
        );
    }

    #[test]
    fn parse_flattens_same_operator_nesting() {
        assert_eq!(
            parse_license("MIT OR Apache-2.0 OR MPL-2.0"),
            LicenseExpr::Or(vec![ident("MIT"), ident("Apache-2.0"), ident("MPL-2.0")])
        );
        assert_eq!(
            parse_license("(MIT OR Apache-2.0) AND MPL-2.0"),
            LicenseExpr::And(vec![
                LicenseExpr::Or(vec![ident("MIT"), ident("Apache-2.0")]),
                ident("MPL-2.0"),
            ])
        );
    }

    #[test]
    fn parse_with_renders_canonical_text_for_approval() {
        assert_eq!(
            parse_license("Apache-2.0 WITH LLVM-exception"),
            LicenseExpr::With {
                base: Box::new(ident("Apache-2.0")),
                exception: "LLVM-exception".to_owned(),
                text: "Apache-2.0 WITH LLVM-exception".to_owned(),
            }
        );
    }

    #[test]
    fn parse_unknown_identities_fail_closed() {
        assert_eq!(parse_license("MIT OR NOPE"), LicenseExpr::Unknown);
        assert_eq!(parse_license("Made-Up-1.0"), LicenseExpr::Unknown);
        assert_eq!(parse_license(""), LicenseExpr::Unknown);
        assert_eq!(parse_license("not a license !!!"), LicenseExpr::Unknown);
    }

    #[test]
    fn parsed_expressions_evaluate_through_the_lattice() {
        let table = contract_table();
        let lookup = lookup(&table);
        let expr = parse_license("MIT OR GPL-3.0-only");
        assert_eq!(
            evaluate(&expr, Tier::Distributed, &lookup, &none_approved()),
            TierOutcome::Allow
        );
        let denied = parse_license("GPL-3.0-only");
        assert_eq!(
            evaluate(&denied, Tier::Distributed, &lookup, &none_approved()),
            TierOutcome::Deny
        );
        let unknown = parse_license("MIT OR NOPE");
        assert_eq!(
            evaluate(&unknown, Tier::Distributed, &lookup, &none_approved()),
            TierOutcome::Deny
        );
        assert_eq!(
            evaluate(&unknown, Tier::Internal, &lookup, &none_approved()),
            TierOutcome::Allow
        );
    }
}
