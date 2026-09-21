//! Pure release-qualification planning (slices 1-4: WP1 blocker disposition,
//! WP2 API/support/registry freeze; WP5 artifact-identity/packaging boundary,
//! reproducibility and verification binding; WP4/WP6/WP7 evidence inventory,
//! coverage gate, consumer-CI requalification, publication handoff).
//! Contract: `docs/product/promotion-checklist.md`.
//!
//! Publication-input verification (destinations, credential
//! scopes, digest/policy match) and post-publication discipline (no silent
//! rebuild or substitution, public-artifact smoke tests, recorded incidents).
//!
//! This crate owns the qualification shape before any release candidate,
//! platform run, external-consumer run, signing, provenance, or publication
//! lands: blocker-vs-deferral disposition, the support-label evidence gate,
//! public-API change classification, command-registry exactness, artifact
//! identity over exact published bytes, the embedded-vs-detached packaging
//! boundary, manifest completeness, attestation subject binding,
//! reproducibility comparison, verification binding, the self-attestation
//! level cap, the release coverage gate, aggregate/matrix completeness, CI
//! identity requalification, and the -to- publication handoff. It plans
//! over injected booleans/strings/numbers only, so the rules stay
//! deterministic and unit-testable without platforms, consumers, builders,
//! or credentials.
//!
//! Out of scope here (qualification + publication): the issue tracker
//! evidencing, trusted-builder/SBOM/provenance execution, reproducibility
//! measurement, coverage-instrumentation runs, consumer-CI matrix runs, and
//! any tag/registry/release publication (gated). Those stay deferred;
//! this crate never claims `Supported`, never signs, and never publishes.

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

/// Which inventory cell a release blocker belongs to.
///
/// Per the evidence rule, evidence-backed additional-foundation deferrals
/// separate from required-core blockers and other unresolved required cells
/// under the first-release admission policy. A deferral waives neither
/// required-core obligations nor the unchanged quality-tool baseline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockerKind {
    /// Required-core cell: always blocks until qualified.
    RequiredCore,
    /// Admitted additional foundation: deferrable only with an
    /// evidence-backed admission decision.
    AdditionalFoundation,
    /// Any other unresolved required cell: blocks until qualified.
    OtherRequired,
}

/// Disposition of one blocker under the admission policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockerDisposition {
    /// Ships only after the cell qualifies; release is blocked meanwhile.
    BlocksRelease,
    /// Evidence-backed deferral with a recorded admission decision.
    DeferredWithDecision,
    /// Rejected deferral: it attempted to waive required-core obligations
    /// or the quality-tool baseline, which deferrals never waive.
    InvalidDeferral,
}

/// Plan one blocker disposition.
///
/// `attempts_waiver` is true when the deferral would waive required-core
/// obligations or quality-tool duties. Such waivers are always rejected.
/// Otherwise an additional-foundation blocker with a recorded admission
/// decision defers; everything else blocks the release.
pub fn plan_blocker_disposition(
    kind: BlockerKind,
    has_admission_decision: bool,
    attempts_waiver: bool,
) -> BlockerDisposition {
    if attempts_waiver {
        return BlockerDisposition::InvalidDeferral;
    }
    match kind {
        BlockerKind::AdditionalFoundation if has_admission_decision => {
            BlockerDisposition::DeferredWithDecision
        }
        _ => BlockerDisposition::BlocksRelease,
    }
}

/// Release support label for one inventory cell.
///
/// `Supported` is promoted only by release evidence; `Partial` cells
/// remain explicitly non-supported and never permit shipping an unresolved
/// required capability on any required platform.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupportLabel {
    Supported,
    Partial,
    NonSupported,
}

/// Whether `Supported` may be claimed for one cell.
///
/// Both required-platform and external-consumer evidence are required.
/// A partial or non-supported label never upgrades without that evidence.
pub fn may_claim_supported(has_platform_evidence: bool, has_consumer_evidence: bool) -> bool {
    has_platform_evidence && has_consumer_evidence
}

/// Whether a required cell may ship under its label and qualification.
///
/// Only a qualified `Supported` cell ships. A partial or non-supported
/// label never permits shipping an unresolved required capability.
pub fn may_ship_required_cell(label: SupportLabel, qualified: bool) -> bool {
    label == SupportLabel::Supported && qualified
}

/// Public-API change class under distribution SemVer 2.0.0
/// (`docs/environments/environment.md#distribution`).
///
/// Incompatible documented public-API changes (Starlark symbols/providers;
/// CLI commands, flags, behavior) require major; backward-compatible public
/// additions/deprecations require minor; backward-compatible fixes use patch.
/// Internal details sit outside the promise; native tool diagnostics and
/// formatting evolve under the tool-update policy without becoming API breaks.
/// Protocol schemas keep their own compatibility rules.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApiChange {
    /// Incompatible documented public-API change.
    IncompatiblePublic,
    /// Backward-compatible public addition or deprecation.
    CompatibleAddition,
    /// Backward-compatible public bug fix.
    CompatibleFix,
    /// Internal detail outside the compatibility promise.
    InternalOnly,
    /// Native tool diagnostic/formatting evolution (tool-update policy).
    NativeToolOutput,
}

/// Required release bump for one change class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseBump {
    Major,
    Minor,
    Patch,
    None,
}

/// Classify one change into its required release bump.
///
/// Internal-only and native-tool-output changes require no public bump on
/// their own; any accompanying public change classifies separately.
pub fn classify_api_change(change: ApiChange) -> ReleaseBump {
    match change {
        ApiChange::IncompatiblePublic => ReleaseBump::Major,
        ApiChange::CompatibleAddition => ReleaseBump::Minor,
        ApiChange::CompatibleFix => ReleaseBump::Patch,
        ApiChange::InternalOnly | ApiChange::NativeToolOutput => ReleaseBump::None,
    }
}

/// Whether the release command registry is exact.
///
/// The final registry contains exactly the accepted commands: no missing
/// accepted command and no extra unaccepted command. Order-insensitive;
/// spellings compare verbatim.
pub fn registry_is_exact(registry: &[String], accepted: &[String]) -> bool {
    let mut got = registry.to_vec();
    let mut want = accepted.to_vec();
    got.sort();
    want.sort();
    got == want
}

/// Candidate wire-profile predicate URIs (provisional research, not frozen).
///
/// From `docs/tools/tool-acquisition.md#provenance-profile-research`: SPDX 2.3
/// JSON and SLSA Build Provenance v1, each in an in-toto Statement v1. These
/// name the research candidates the qualification must test; pinning the
/// strings here does not freeze the profiles, select a builder, or claim an
/// assurance level.
pub const CANDIDATE_SPDX_PREDICATE_URI: &str = "https://spdx.dev/Document/v2.3";
pub const CANDIDATE_SLSA_PREDICATE_URI: &str = "https://slsa.dev/provenance/v1";
pub const CANDIDATE_STATEMENT_TYPE_URI: &str = "https://in-toto.io/Statement/v1";

/// Whether an attestation subject binds the exact published bytes.
///
/// The subject digest must equal the digest of the exact published archive
/// bytes, matched purely by digest. Empty digests never bind.
pub fn attestation_binds_exact_bytes(subject_digest: &str, published_digest: &str) -> bool {
    !subject_digest.is_empty() && subject_digest == published_digest
}

/// Whether the packaging uses the single correct path.
///
/// Per `docs/tools/tool-acquisition.md#artifact-identity-and-metadata`:
/// payload-only embedded manifest for constituent inputs plus detached
/// final-archive attestations over the exact published bytes. A manifest
/// self-entry carrying the final digest (self-referential digest/size) is
/// rejected, as are null or deferred digests.
pub fn packaging_uses_single_correct_path(
    embedded_for_constituents: bool,
    detached_for_final: bool,
    has_self_digest_entry: bool,
) -> bool {
    embedded_for_constituents && detached_for_final && !has_self_digest_entry
}

/// Whether the embedded manifest covers every payload file.
///
/// No payload file may be silently omitted: every verbatim payload path must
/// have a matching inventoried entry. Extra inventory entries do not excuse a
/// missing payload file. An empty payload is vacuously complete.
pub fn manifest_covers_payload(payload: &[String], inventoried: &[String]) -> bool {
    payload.iter().all(|file| inventoried.contains(file))
}

/// Whether changing embedded metadata requires new final-archive attestations.
///
/// Adding or changing embedded metadata changes the artifact identity (the
/// digest of the exact published bytes), so detached attestations bound to
/// the old digest no longer apply.
pub fn embedded_change_requires_new_attestation(embedded_changed: bool) -> bool {
    embedded_changed
}

/// Whether an independent rebuild reproduces the candidate.
///
/// A qualified artifact reproduces from a clean release environment: the
/// rebuild digest must equal the candidate digest. Empty digests never count
/// as reproduction. This compares injected digest strings only; it performs
/// no build and measures no threshold (thresholds stay gated).
pub fn rebuild_reproduces(candidate_digest: &str, rebuild_digest: &str) -> bool {
    !candidate_digest.is_empty() && candidate_digest == rebuild_digest
}

/// First verification mismatch between attested and independently expected values.
///
/// Verification binds artifact bytes, predicate type, signer, issuer/source,
/// and builder policy to independently trusted expectations, never to values
/// trusted merely because they arrived in the download. The first mismatch in
/// that order is reported; `None` means every bound value matched.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerifyReject {
    DigestMismatch,
    PredicateMismatch,
    SignerMismatch,
    BuilderMismatch,
}

/// One attested-vs-expected verification binding: the claimed value and
/// the independent expectation it must match exactly (verbatim, non-empty
/// attested value).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Binding<'a> {
    /// Value claimed by the attestation under review.
    pub attested: &'a str,
    /// Independently trusted expectation.
    pub expected: &'a str,
}

impl Binding<'_> {
    /// True when the binding rejects: empty attested value or any mismatch.
    fn rejects(&self) -> bool {
        self.attested.is_empty() || self.attested != self.expected
    }
}

/// The four verification bindings: digest, predicate, signer, builder.
/// Grouped so verification entry points take one argument instead of
/// eight positional strings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerificationBindings<'a> {
    /// Artifact digest binding.
    pub digest: Binding<'a>,
    /// Predicate-type binding.
    pub predicate: Binding<'a>,
    /// Signer binding.
    pub signer: Binding<'a>,
    /// Builder binding.
    pub builder: Binding<'a>,
}

/// Plan verification binding over injected attested-vs-expected strings.
///
/// All four bindings must match exactly (verbatim, non-empty attested value).
/// Any mismatch rejects; this executes no cryptography and trusts no log,
/// timestamp, or certificate on its own (those stay the issue tracker-gated).
pub fn verify_rejection(bindings: VerificationBindings<'_>) -> Option<VerifyReject> {
    if bindings.digest.rejects() {
        return Some(VerifyReject::DigestMismatch);
    }
    if bindings.predicate.rejects() {
        return Some(VerifyReject::PredicateMismatch);
    }
    if bindings.signer.rejects() {
        return Some(VerifyReject::SignerMismatch);
    }
    if bindings.builder.rejects() {
        return Some(VerifyReject::BuilderMismatch);
    }
    None
}

/// Whether verification accepts the attested values.
///
/// Accepts only when every binding matches its independent expectation.
pub fn verification_accepts(bindings: VerificationBindings<'_>) -> bool {
    verify_rejection(bindings).is_none()
}

/// Whether a self-attested provenance level claim is admissible.
///
/// Self-attested provenance is L0/L1 at best and must not self-assert L2/L3:
/// a self-attested claim above level 1 is rejected outright.
pub fn self_attested_level_admissible(claimed_level: u8, self_attested: bool) -> bool {
    if self_attested && claimed_level > 1 {
        return false;
    }
    true
}

/// Whether the release coverage gate passes for one source class.
///
/// The central gate passes only with a present report, valid ignore/reason
/// directives, and zero uncovered non-ignored executable lines. Missing
/// reports, invalid or unreasoned ignores, and any uncovered non-ignored line
/// fail rather than grant a release waiver. This plans the gate shape over
/// injected inputs; it instruments nothing and counts no lines.
pub fn coverage_gate_accepts(
    has_report: bool,
    ignores_valid: bool,
    uncovered_nonignored: u32,
) -> bool {
    has_report && ignores_valid && uncovered_nonignored == 0
}

/// Whether every cell in a gate set is green.
///
/// Shared by the aggregate CI check and the per-platform coverage rollup: the
/// set must be non-empty and every cell must hold. An empty set never counts
/// as success.
fn all_cells_ok(cells: &[bool]) -> bool {
    !cells.is_empty() && cells.iter().all(|cell| *cell)
}

/// Whether the stable aggregate CI check may report success.
///
/// Success requires every selected check/platform cell to complete
/// successfully under its command contract with all required reporting
/// finished. Missing, blocked, unexpectedly skipped, cancelled, or incomplete
/// cells (each passed here as `false`) never produce success.
pub fn aggregate_may_succeed(cell_ok: &[bool]) -> bool {
    all_cells_ok(cell_ok)
}

/// Whether per-platform coverage aggregation is complete.
///
/// Combining reports must not hide a missing platform or coverage gap: every
/// required selected platform/configuration cell must be covered. One
/// uncovered cell fails the rollup even when the rest pass.
pub fn coverage_aggregation_complete(per_platform_covered: &[bool]) -> bool {
    all_cells_ok(per_platform_covered)
}

/// Whether a used release identity matches the qualified identity.
///
/// Publication and CI requalification never substitute bytes or revisions:
/// the used digest/workflow/reporter/caller/module identity must equal the
/// qualified identity verbatim. Empty identities never match.
pub fn handoff_identity_matches(qualified: &str, used: &str) -> bool {
    !qualified.is_empty() && qualified == used
}

/// Whether the consumer-CI requalification may accept one run.
///
/// The candidate workflow, reporter, caller template, and module-matched CLI
/// must each equal the handoff identity. Any substitution fails the run;
/// this checks the four equalities, it does not execute CI.
pub fn ci_requalification_accepts(
    workflow_matches: bool,
    reporter_matches: bool,
    caller_matches: bool,
    module_cli_matches: bool,
) -> bool {
    workflow_matches && reporter_matches && caller_matches && module_cli_matches
}

/// Whether may hand off to publication.
///
/// Handoff needs a qualified candidate whose digests still match the
/// verified bytes and whose CI identities requalified without substitution.
/// Publication itself (credentials, registry, release host) stays gated
/// and never rebuilds or swaps bytes silently.
pub fn may_hand_off_to_publication(
    qualified: bool,
    digests_match: bool,
    ci_identities_match: bool,
) -> bool {
    qualified && digests_match && ci_identities_match
}

/// Whether a publication destination is approved.
///
/// Destinations are approved by policy (`docs/environments/environment.md#distribution`
/// selects the module registry and release host only); the used destination
/// must be a verbatim member of the injected approved set. This checks
/// membership over injected strings; it approves no destination and performs
/// no submission.
pub fn publish_destination_approved(destination: &str, approved: &[String]) -> bool {
    !destination.is_empty() && approved.contains(&destination.to_owned())
}

/// Whether granted credential scopes cover every required operation scope.
///
/// Every required scope must be present in the granted set. Scope selection
/// and issuance stay gated; this compares injected scope strings only
/// and grants nothing.
pub fn credential_scopes_cover(granted: &[String], required: &[String]) -> bool {
    required.iter().all(|scope| granted.contains(scope))
}

/// Whether publication inputs verify before any credentialed operation.
///
/// Destinations, credential scopes, artifact digests, and the publication
/// policy must each match the qualified inputs. Any mismatch blocks
/// publication; verification performs no credentialed step.
pub fn publication_inputs_verified(
    destinations_ok: bool,
    scopes_ok: bool,
    digests_match: bool,
    policy_ok: bool,
) -> bool {
    destinations_ok && scopes_ok && digests_match && policy_ok
}

/// Whether a post-publication smoke test accepts one install path.
///
/// The smoke test must run against the public artifact obtained through an
/// approved destination (never a local substitute), its digest must still
/// match the qualified digest, and the host run must pass. Any failure in
/// the triple fails the path.
pub fn public_install_accepts(
    uses_public_artifact: bool,
    digests_match: bool,
    host_passed: bool,
) -> bool {
    uses_public_artifact && digests_match && host_passed
}

/// Whether a publication incident is disposed without silent byte changes.
///
/// Incidents are recorded with the published bytes left unchanged: a
/// rebuild or substitution without a new qualified release is rejected even
/// when the incident itself is recorded. Recording alone never authorizes
/// new bytes.
pub fn incident_disposition_ok(incident_recorded: bool, bytes_unchanged: bool) -> bool {
    incident_recorded && bytes_unchanged
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(words: &[&str]) -> Vec<String> {
        words.iter().copied().map(str::to_owned).collect()
    }

    #[test]
    fn required_core_always_blocks_even_with_a_decision() {
        assert_eq!(
            plan_blocker_disposition(BlockerKind::RequiredCore, true, false),
            BlockerDisposition::BlocksRelease
        );
        assert_eq!(
            plan_blocker_disposition(BlockerKind::RequiredCore, false, false),
            BlockerDisposition::BlocksRelease
        );
    }

    #[test]
    fn additional_foundation_defers_only_with_a_decision() {
        assert_eq!(
            plan_blocker_disposition(BlockerKind::AdditionalFoundation, true, false),
            BlockerDisposition::DeferredWithDecision
        );
        assert_eq!(
            plan_blocker_disposition(BlockerKind::AdditionalFoundation, false, false),
            BlockerDisposition::BlocksRelease
        );
    }

    #[test]
    fn other_required_cells_block() {
        assert_eq!(
            plan_blocker_disposition(BlockerKind::OtherRequired, true, false),
            BlockerDisposition::BlocksRelease
        );
    }

    #[test]
    fn waivers_never_succeed() {
        assert_eq!(
            plan_blocker_disposition(BlockerKind::AdditionalFoundation, true, true),
            BlockerDisposition::InvalidDeferral
        );
        assert_eq!(
            plan_blocker_disposition(BlockerKind::RequiredCore, false, true),
            BlockerDisposition::InvalidDeferral
        );
    }

    #[test]
    fn supported_needs_both_platform_and_consumer_evidence() {
        assert!(may_claim_supported(true, true));
        assert!(!may_claim_supported(true, false));
        assert!(!may_claim_supported(false, true));
        assert!(!may_claim_supported(false, false));
    }

    #[test]
    fn only_qualified_supported_cells_ship_required_scope() {
        assert!(may_ship_required_cell(SupportLabel::Supported, true));
        assert!(!may_ship_required_cell(SupportLabel::Supported, false));
        assert!(!may_ship_required_cell(SupportLabel::Partial, true));
        assert!(!may_ship_required_cell(SupportLabel::NonSupported, true));
        assert!(!may_ship_required_cell(SupportLabel::Partial, false));
    }

    #[test]
    fn api_changes_classify_per_distribution_semver() {
        assert_eq!(
            classify_api_change(ApiChange::IncompatiblePublic),
            ReleaseBump::Major
        );
        assert_eq!(
            classify_api_change(ApiChange::CompatibleAddition),
            ReleaseBump::Minor
        );
        assert_eq!(
            classify_api_change(ApiChange::CompatibleFix),
            ReleaseBump::Patch
        );
        assert_eq!(
            classify_api_change(ApiChange::InternalOnly),
            ReleaseBump::None
        );
        assert_eq!(
            classify_api_change(ApiChange::NativeToolOutput),
            ReleaseBump::None
        );
    }

    #[test]
    fn registry_exactness_rejects_missing_and_extra() {
        let accepted = strings(&["lint", "test", "build"]);
        assert!(registry_is_exact(
            &strings(&["lint", "test", "build"]),
            &accepted
        ));
        assert!(registry_is_exact(
            &strings(&["build", "lint", "test"]),
            &accepted
        ));
        assert!(!registry_is_exact(&strings(&["lint", "test"]), &accepted));
        assert!(!registry_is_exact(
            &strings(&["lint", "test", "build", "watch"]),
            &accepted
        ));
        assert!(!registry_is_exact(
            &strings(&["Lint", "test", "build"]),
            &accepted
        ));
    }

    #[test]
    fn attestation_binds_only_the_exact_published_digest() {
        assert!(attestation_binds_exact_bytes("sha256:abc", "sha256:abc"));
        assert!(!attestation_binds_exact_bytes("sha256:abc", "sha256:def"));
        assert!(!attestation_binds_exact_bytes("", ""));
        assert!(!attestation_binds_exact_bytes("", "sha256:abc"));
    }

    #[test]
    fn candidate_wire_profiles_name_the_o39_research_uris() {
        assert_eq!(
            CANDIDATE_SPDX_PREDICATE_URI,
            "https://spdx.dev/Document/v2.3"
        );
        assert_eq!(
            CANDIDATE_SLSA_PREDICATE_URI,
            "https://slsa.dev/provenance/v1"
        );
        assert_eq!(
            CANDIDATE_STATEMENT_TYPE_URI,
            "https://in-toto.io/Statement/v1"
        );
    }

    #[test]
    fn single_correct_packaging_path_rejects_self_reference() {
        assert!(packaging_uses_single_correct_path(true, true, false));
        assert!(!packaging_uses_single_correct_path(true, true, true));
        assert!(!packaging_uses_single_correct_path(false, true, false));
        assert!(!packaging_uses_single_correct_path(true, false, false));
    }

    #[test]
    fn manifest_completeness_rejects_silently_omitted_files() {
        let payload = strings(&["dx", "NOTICE", "manifest.pb"]);
        assert!(manifest_covers_payload(
            &payload,
            &strings(&["dx", "NOTICE", "manifest.pb"])
        ));
        assert!(manifest_covers_payload(
            &payload,
            &strings(&["dx", "NOTICE", "manifest.pb", "extra"])
        ));
        assert!(!manifest_covers_payload(
            &payload,
            &strings(&["dx", "NOTICE"])
        ));
        assert!(manifest_covers_payload(&[], &[]));
    }

    #[test]
    fn embedded_changes_invalidate_detached_attestations() {
        assert!(embedded_change_requires_new_attestation(true));
        assert!(!embedded_change_requires_new_attestation(false));
    }

    #[test]
    fn reproduction_needs_equal_nonempty_digests() {
        assert!(rebuild_reproduces("sha256:abc", "sha256:abc"));
        assert!(!rebuild_reproduces("sha256:abc", "sha256:def"));
        assert!(!rebuild_reproduces("", ""));
    }

    #[test]
    fn verification_binds_digest_predicate_signer_builder_in_order() {
        fn ok<'a>(d: &'a str, p: &'a str, s: &'a str, b: &'a str) -> Option<VerifyReject> {
            verify_rejection(VerificationBindings {
                digest: Binding {
                    attested: d,
                    expected: "sha256:abc",
                },
                predicate: Binding {
                    attested: p,
                    expected: CANDIDATE_SLSA_PREDICATE_URI,
                },
                signer: Binding {
                    attested: s,
                    expected: "sig:alice",
                },
                builder: Binding {
                    attested: b,
                    expected: "builder:trusted",
                },
            })
        }
        assert_eq!(
            ok(
                "sha256:abc",
                CANDIDATE_SLSA_PREDICATE_URI,
                "sig:alice",
                "builder:trusted"
            ),
            None
        );
        assert_eq!(
            ok(
                "sha256:zzz",
                CANDIDATE_SLSA_PREDICATE_URI,
                "sig:alice",
                "builder:trusted"
            ),
            Some(VerifyReject::DigestMismatch)
        );
        assert_eq!(
            ok(
                "sha256:abc",
                "https://example.invalid/other",
                "sig:alice",
                "builder:trusted"
            ),
            Some(VerifyReject::PredicateMismatch)
        );
        assert_eq!(
            ok(
                "sha256:abc",
                CANDIDATE_SLSA_PREDICATE_URI,
                "sig:mallory",
                "builder:trusted"
            ),
            Some(VerifyReject::SignerMismatch)
        );
        assert_eq!(
            ok(
                "sha256:abc",
                CANDIDATE_SLSA_PREDICATE_URI,
                "sig:alice",
                "builder:self"
            ),
            Some(VerifyReject::BuilderMismatch)
        );
        assert_eq!(
            ok(
                "",
                CANDIDATE_SLSA_PREDICATE_URI,
                "sig:alice",
                "builder:trusted"
            ),
            Some(VerifyReject::DigestMismatch)
        );
    }

    #[test]
    fn verification_accepts_only_fully_bound_attestations() {
        fn bindings<'a>(
            digest: &'a str,
            predicate: &'a str,
            signer: &'a str,
            builder: &'a str,
        ) -> VerificationBindings<'a> {
            VerificationBindings {
                digest: Binding {
                    attested: digest,
                    expected: "sha256:abc",
                },
                predicate: Binding {
                    attested: predicate,
                    expected: CANDIDATE_SLSA_PREDICATE_URI,
                },
                signer: Binding {
                    attested: signer,
                    expected: "sig:alice",
                },
                builder: Binding {
                    attested: builder,
                    expected: "builder:trusted",
                },
            }
        }
        assert!(verification_accepts(bindings(
            "sha256:abc",
            CANDIDATE_SLSA_PREDICATE_URI,
            "sig:alice",
            "builder:trusted",
        )));
        assert!(!verification_accepts(bindings(
            "sha256:abc",
            CANDIDATE_SLSA_PREDICATE_URI,
            "sig:alice",
            "builder:self",
        )));
    }

    #[test]
    fn self_attestation_never_claims_l2_or_l3() {
        assert!(self_attested_level_admissible(1, true));
        assert!(self_attested_level_admissible(0, true));
        assert!(!self_attested_level_admissible(2, true));
        assert!(!self_attested_level_admissible(3, true));
        assert!(self_attested_level_admissible(3, false));
    }

    #[test]
    fn coverage_gate_rejects_missing_invalid_and_uncovered() {
        assert!(coverage_gate_accepts(true, true, 0));
        assert!(!coverage_gate_accepts(false, true, 0));
        assert!(!coverage_gate_accepts(true, false, 0));
        assert!(!coverage_gate_accepts(true, true, 1));
        assert!(!coverage_gate_accepts(false, false, 7));
    }

    #[test]
    fn aggregate_success_needs_every_cell_green() {
        assert!(aggregate_may_succeed(&[true, true, true]));
        assert!(!aggregate_may_succeed(&[true, false, true]));
        assert!(!aggregate_may_succeed(&[false]));
        assert!(!aggregate_may_succeed(&[]));
    }

    #[test]
    fn coverage_rollup_hides_no_platform_gap() {
        assert!(coverage_aggregation_complete(&[true, true]));
        assert!(!coverage_aggregation_complete(&[true, false]));
        assert!(!coverage_aggregation_complete(&[]));
    }

    #[test]
    fn handoff_identities_match_exactly_or_not_at_all() {
        assert!(handoff_identity_matches("sha256:abc", "sha256:abc"));
        assert!(!handoff_identity_matches("sha256:abc", "sha256:def"));
        assert!(!handoff_identity_matches("", ""));
        assert!(!handoff_identity_matches("wf@42", ""));
    }

    #[test]
    fn ci_requalification_rejects_any_substitution() {
        assert!(ci_requalification_accepts(true, true, true, true));
        assert!(!ci_requalification_accepts(false, true, true, true));
        assert!(!ci_requalification_accepts(true, false, true, true));
        assert!(!ci_requalification_accepts(true, true, false, true));
        assert!(!ci_requalification_accepts(true, true, true, false));
    }

    #[test]
    fn publication_handoff_needs_qualified_matching_requalified() {
        assert!(may_hand_off_to_publication(true, true, true));
        assert!(!may_hand_off_to_publication(false, true, true));
        assert!(!may_hand_off_to_publication(true, false, true));
        assert!(!may_hand_off_to_publication(true, true, false));
    }

    #[test]
    fn publication_destinations_need_verbatim_approval() {
        let approved = strings(&["bazel-central-registry", "github-releases"]);
        assert!(publish_destination_approved("github-releases", &approved));
        assert!(!publish_destination_approved("personal-blog", &approved));
        assert!(!publish_destination_approved("", &approved));
        assert!(!publish_destination_approved("GitHub-Releases", &approved));
    }

    #[test]
    fn credential_scopes_must_cover_every_required_scope() {
        let granted = strings(&["registry:write", "release:write"]);
        assert!(credential_scopes_cover(
            &granted,
            &strings(&["registry:write"])
        ));
        assert!(credential_scopes_cover(&granted, &[]));
        assert!(!credential_scopes_cover(
            &granted,
            &strings(&["registry:write", "admin:all"])
        ));
    }

    #[test]
    fn publication_starts_only_on_fully_verified_inputs() {
        assert!(publication_inputs_verified(true, true, true, true));
        assert!(!publication_inputs_verified(false, true, true, true));
        assert!(!publication_inputs_verified(true, false, true, true));
        assert!(!publication_inputs_verified(true, true, false, true));
        assert!(!publication_inputs_verified(true, true, true, false));
    }

    #[test]
    fn public_install_needs_public_bytes_matching_passing() {
        assert!(public_install_accepts(true, true, true));
        assert!(!public_install_accepts(false, true, true));
        assert!(!public_install_accepts(true, false, true));
        assert!(!public_install_accepts(true, true, false));
    }

    #[test]
    fn incidents_record_without_silent_rebuilds() {
        assert!(incident_disposition_ok(true, true));
        assert!(!incident_disposition_ok(false, true));
        assert!(!incident_disposition_ok(true, false));
        assert!(!incident_disposition_ok(false, false));
    }
}
