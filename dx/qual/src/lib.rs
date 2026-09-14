//! Pure release-qualification planning (M28 slice 1: WP1 blocker disposition,
//! WP2 API/support/registry freeze).
//!
//! This crate owns the qualification shape before any release candidate,
//! platform run, external-consumer run, signing, provenance, or publication
//! lands: blocker-vs-deferral disposition, the support-label evidence gate,
//! public-API change classification, and command-registry exactness. It plans
//! over injected booleans/strings only, so the rules stay deterministic and
//! unit-testable without platforms, consumers, builders, or credentials.
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
}
