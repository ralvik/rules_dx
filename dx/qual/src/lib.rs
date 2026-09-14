//! Pure release-qualification planning (M28 slices 1-3: WP1 blocker disposition,
//! WP2 API/support/registry freeze; WP5 artifact-identity/packaging boundary,
//! reproducibility and verification binding).
//!
//! This crate owns the qualification shape before any release candidate,
//! platform run, external-consumer run, signing, provenance, or publication
//! lands: blocker-vs-deferral disposition, the support-label evidence gate,
//! public-API change classification, command-registry exactness, artifact
//! identity over exact published bytes, the embedded-vs-detached packaging
//! boundary, manifest completeness, attestation subject binding,
//! reproducibility comparison, verification binding, and the self-attestation
//! level cap. It plans over injected booleans/strings only, so the rules stay
//! deterministic and unit-testable without platforms, consumers, builders,
//! or credentials.
//!
//! Out of scope here (M28 qualification + M29 publication): O6/O37/O38/O39
//! evidencing, trusted-builder/SBOM/provenance execution, reproducibility
//! measurement, coverage-instrumentation runs, consumer-CI matrix runs, and
//! any tag/registry/release publication (O45-gated). Those stay deferred;
//! this crate never claims `Supported`, never signs, and never publishes.

/// Which inventory cell a release blocker belongs to.
///
/// Per the M28 evidence rule, evidence-backed additional-foundation deferrals
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
/// `Supported` is promoted only by release evidence (M28); `Partial` cells
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

/// Candidate wire-profile predicate URIs (O39 provisional research, not frozen).
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
/// no build and measures no threshold (thresholds stay O39-gated).
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

/// Plan verification binding over injected attested-vs-expected strings.
///
/// All four bindings must match exactly (verbatim, non-empty attested value).
/// Any mismatch rejects; this executes no cryptography and trusts no log,
/// timestamp, or certificate on its own (those stay O38/O39-gated).
#[allow(clippy::too_many_arguments)]
pub fn verify_rejection(
    attested_digest: &str,
    expected_digest: &str,
    attested_predicate: &str,
    expected_predicate: &str,
    attested_signer: &str,
    expected_signer: &str,
    attested_builder: &str,
    expected_builder: &str,
) -> Option<VerifyReject> {
    if attested_digest.is_empty() || attested_digest != expected_digest {
        return Some(VerifyReject::DigestMismatch);
    }
    if attested_predicate.is_empty() || attested_predicate != expected_predicate {
        return Some(VerifyReject::PredicateMismatch);
    }
    if attested_signer.is_empty() || attested_signer != expected_signer {
        return Some(VerifyReject::SignerMismatch);
    }
    if attested_builder.is_empty() || attested_builder != expected_builder {
        return Some(VerifyReject::BuilderMismatch);
    }
    None
}

/// Whether verification accepts the attested values.
///
/// Accepts only when every binding matches its independent expectation.
#[allow(clippy::too_many_arguments)]
pub fn verification_accepts(
    attested_digest: &str,
    expected_digest: &str,
    attested_predicate: &str,
    expected_predicate: &str,
    attested_signer: &str,
    expected_signer: &str,
    attested_builder: &str,
    expected_builder: &str,
) -> bool {
    verify_rejection(
        attested_digest,
        expected_digest,
        attested_predicate,
        expected_predicate,
        attested_signer,
        expected_signer,
        attested_builder,
        expected_builder,
    )
    .is_none()
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
        let ok = |d: &str, p: &str, s: &str, b: &str| {
            verify_rejection(
                d,
                "sha256:abc",
                p,
                CANDIDATE_SLSA_PREDICATE_URI,
                s,
                "sig:alice",
                b,
                "builder:trusted",
            )
        };
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
        assert!(verification_accepts(
            "sha256:abc",
            "sha256:abc",
            CANDIDATE_SLSA_PREDICATE_URI,
            CANDIDATE_SLSA_PREDICATE_URI,
            "sig:alice",
            "sig:alice",
            "builder:trusted",
            "builder:trusted",
        ));
        assert!(!verification_accepts(
            "sha256:abc",
            "sha256:abc",
            CANDIDATE_SLSA_PREDICATE_URI,
            CANDIDATE_SLSA_PREDICATE_URI,
            "sig:alice",
            "sig:alice",
            "builder:self",
            "builder:trusted",
        ));
    }

    #[test]
    fn self_attestation_never_claims_l2_or_l3() {
        assert!(self_attested_level_admissible(1, true));
        assert!(self_attested_level_admissible(0, true));
        assert!(!self_attested_level_admissible(2, true));
        assert!(!self_attested_level_admissible(3, true));
        assert!(self_attested_level_admissible(3, false));
    }
}
