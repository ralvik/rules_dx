// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockerKind {
    RequiredCore,
    AdditionalFoundation,
    OtherRequired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockerDisposition {
    BlocksRelease,
    DeferredWithDecision,
    InvalidDeferral,
}

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupportLabel {
    Supported,
    Partial,
    NonSupported,
}

pub fn may_claim_supported(has_platform_evidence: bool, has_consumer_evidence: bool) -> bool {
    has_platform_evidence && has_consumer_evidence
}

pub fn may_ship_required_cell(label: SupportLabel, qualified: bool) -> bool {
    label == SupportLabel::Supported && qualified
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApiChange {
    IncompatiblePublic,
    CompatibleAddition,
    CompatibleFix,
    InternalOnly,
    NativeToolOutput,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseBump {
    Major,
    Minor,
    Patch,
    None,
}

pub fn classify_api_change(change: ApiChange) -> ReleaseBump {
    match change {
        ApiChange::IncompatiblePublic => ReleaseBump::Major,
        ApiChange::CompatibleAddition => ReleaseBump::Minor,
        ApiChange::CompatibleFix => ReleaseBump::Patch,
        ApiChange::InternalOnly | ApiChange::NativeToolOutput => ReleaseBump::None,
    }
}

pub fn registry_is_exact(registry: &[String], accepted: &[String]) -> bool {
    let mut got = registry.to_vec();
    let mut want = accepted.to_vec();
    got.sort();
    want.sort();
    got == want
}

pub const CANDIDATE_SPDX_PREDICATE_URI: &str = "https://spdx.dev/Document/v2.3";
pub const CANDIDATE_SLSA_PREDICATE_URI: &str = "https://slsa.dev/provenance/v1";
pub const CANDIDATE_STATEMENT_TYPE_URI: &str = "https://in-toto.io/Statement/v1";

pub fn attestation_binds_exact_bytes(subject_digest: &str, published_digest: &str) -> bool {
    !subject_digest.is_empty() && subject_digest == published_digest
}

pub fn packaging_uses_single_correct_path(
    embedded_for_constituents: bool,
    detached_for_final: bool,
    has_self_digest_entry: bool,
) -> bool {
    embedded_for_constituents && detached_for_final && !has_self_digest_entry
}

pub fn manifest_covers_payload(payload: &[String], inventoried: &[String]) -> bool {
    payload.iter().all(|file| inventoried.contains(file))
}

pub fn embedded_change_requires_new_attestation(embedded_changed: bool) -> bool {
    embedded_changed
}

pub fn rebuild_reproduces(candidate_digest: &str, rebuild_digest: &str) -> bool {
    !candidate_digest.is_empty() && candidate_digest == rebuild_digest
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerifyReject {
    DigestMismatch,
    PredicateMismatch,
    SignerMismatch,
    BuilderMismatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Binding<'a> {
    pub attested: &'a str,
    pub expected: &'a str,
}

impl Binding<'_> {
    fn rejects(&self) -> bool {
        self.attested.is_empty() || self.attested != self.expected
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerificationBindings<'a> {
    pub digest: Binding<'a>,
    pub predicate: Binding<'a>,
    pub signer: Binding<'a>,
    pub builder: Binding<'a>,
}

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

pub fn verification_accepts(bindings: VerificationBindings<'_>) -> bool {
    verify_rejection(bindings).is_none()
}

pub fn self_attested_level_admissible(claimed_level: u8, self_attested: bool) -> bool {
    if self_attested && claimed_level > 1 {
        return false;
    }
    true
}

pub fn coverage_gate_accepts(
    has_report: bool,
    ignores_valid: bool,
    uncovered_nonignored: u32,
) -> bool {
    has_report && ignores_valid && uncovered_nonignored == 0
}

fn all_cells_ok(cells: &[bool]) -> bool {
    !cells.is_empty() && cells.iter().all(|cell| *cell)
}

pub fn aggregate_may_succeed(cell_ok: &[bool]) -> bool {
    all_cells_ok(cell_ok)
}

pub fn coverage_aggregation_complete(per_platform_covered: &[bool]) -> bool {
    all_cells_ok(per_platform_covered)
}

pub fn handoff_identity_matches(qualified: &str, used: &str) -> bool {
    !qualified.is_empty() && qualified == used
}

pub fn ci_requalification_accepts(
    workflow_matches: bool,
    reporter_matches: bool,
    caller_matches: bool,
    module_cli_matches: bool,
) -> bool {
    workflow_matches && reporter_matches && caller_matches && module_cli_matches
}

pub fn may_hand_off_to_publication(
    qualified: bool,
    digests_match: bool,
    ci_identities_match: bool,
) -> bool {
    qualified && digests_match && ci_identities_match
}

pub fn publish_destination_approved(destination: &str, approved: &[String]) -> bool {
    !destination.is_empty() && approved.contains(&destination.to_owned())
}

pub fn credential_scopes_cover(granted: &[String], required: &[String]) -> bool {
    required.iter().all(|scope| granted.contains(scope))
}

pub fn publication_inputs_verified(
    destinations_ok: bool,
    scopes_ok: bool,
    digests_match: bool,
    policy_ok: bool,
) -> bool {
    destinations_ok && scopes_ok && digests_match && policy_ok
}

pub fn public_install_accepts(
    uses_public_artifact: bool,
    digests_match: bool,
    host_passed: bool,
) -> bool {
    uses_public_artifact && digests_match && host_passed
}

/// Whether a publication incident is disposed without silent byte changes.
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
