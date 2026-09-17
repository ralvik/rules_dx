//! Consumer `.bazelrc` preset onboarding planning (issue #236, M27 WP6 slice 12).
//!
//! Split from `super` (`lib.rs`): owns [`PRESET_RUNBOOK_STEPS`],
//! [`PresetLabelError`], [`validate_preset_label`],
//! [`preset_affecting_requires_minor_or_major`],
//! [`preset_change_allowed_in_patch`],
//! [`preset_change_needs_release_note`],
//! [`dx_init_emits_preset_template`],
//! [`preset_bot_auto_merge_allowed`], [`preset_regen_requires_review`],
//! [`PresetOnboardingError`], and [`plan_preset_onboarding`].
//! Re-exported through `super` so the public paths stay
//! `dx_ci::{PRESET_RUNBOOK_STEPS, validate_preset_label, ...}`. Distinct
//! from the selection, revision, scheduling, supersession, reporting,
//! fork/aggregate, rerun, trigger, caller/pin, audit, and untrusted
//! modules.

/// Setup runbook steps in canonical order (`M27 WP6`): the consumer setup
/// path is the dependency snippet, the generation target, the `.bazelrc`
/// import block, the regen-and-review update loop, and the bot configuration
/// sample. `dx init` template emission stays M30 scope (see
/// [`dx_init_emits_preset_template`]).
pub const PRESET_RUNBOOK_STEPS: [&str; 5] = [
    "dependency_snippet",
    "generation_target",
    "import_block",
    "update_loop",
    "bot_config_sample",
];

/// Malformed preset load label: the public `extra_presets` load label stays
/// opaque here — the frozen public string freezes with workflow qualification
/// (`docs/github-ci.md#qualification`), not in this crate. This plans only the
/// shape: an explicit nonempty label preserved verbatim, never an implicit
/// default or silent substitution.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PresetLabelError {
    /// Label missing or empty (no implicit default).
    #[error("explicit preset load label is required")]
    MissingLabel,
}

/// Validate a consumer preset load label: nonempty opaque spellings pass
/// through verbatim; missing/empty labels fail closed.
pub fn validate_preset_label(label: &str) -> Result<String, PresetLabelError> {
    if label.is_empty() {
        return Err(PresetLabelError::MissingLabel);
    }
    Ok(label.to_owned())
}

/// Stability discipline (`M27 WP6`): preset-affecting changes ship only in
/// minor or major releases, never in a patch. Non-preset-affecting changes
/// carry no such restriction.
pub fn preset_affecting_requires_minor_or_major(preset_affecting: bool) -> bool {
    preset_affecting
}

/// Whether a release level may carry the change: patch releases accept only
/// non-preset-affecting changes.
pub fn preset_change_allowed_in_patch(preset_affecting: bool) -> bool {
    !preset_affecting
}

/// Preset-affecting changes are called out in the release notes; other
/// changes need no such callout.
pub fn preset_change_needs_release_note(preset_affecting: bool) -> bool {
    preset_affecting
}

/// `dx init` does not emit the preset caller template: template emission
/// stays M30 scope.
pub fn dx_init_emits_preset_template() -> bool {
    false
}

/// The preset regen-and-review loop stays manual with review enforcement
/// (O53): no auto-merge, whether bot-owned or human-driven.
pub fn preset_bot_auto_merge_allowed() -> bool {
    false
}

/// Regen without a reviewed flag diff is not an update.
pub fn preset_regen_requires_review() -> bool {
    true
}

/// Incomplete clean-external-consumer preset proof: generation, import, and
/// regen-and-review update must all be exercised through a clean external
/// consumer; repository dogfood alone is insufficient (the caller proves the
/// three legs, this crate only plans the gate).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PresetOnboardingError {
    /// Generation leg missing (no clean-consumer generation proof).
    #[error("clean-consumer preset generation proof is required")]
    MissingGeneration,
    /// Import leg missing (no clean-consumer import proof).
    #[error("clean-consumer preset import proof is required")]
    MissingImport,
    /// Regen leg missing or unreviewed (no reviewed flag-diff update proof).
    #[error("preset regen-and-review update requires a reviewed flag diff")]
    UnreviewedRegen,
}

/// Plan the clean-external-consumer preset proof.
///
/// All three legs must hold: `generation_ok` (the consumer generates from the
/// documented generation target), `import_ok` (the consumer imports the preset
/// fragment through the documented import block), and `regen_reviewed` (a
/// version-bump regen passed with a reviewed flag diff). Preset bumps never
/// alter consumer customizations — callers preserve the caller-owned
/// selection/mode/opt-in exactly as workflow pin bumps do (see
/// [`crate::apply_pin_update`]).
pub fn plan_preset_onboarding(
    generation_ok: bool,
    import_ok: bool,
    regen_reviewed: bool,
) -> Result<(), PresetOnboardingError> {
    if !generation_ok {
        return Err(PresetOnboardingError::MissingGeneration);
    }
    if !import_ok {
        return Err(PresetOnboardingError::MissingImport);
    }
    if !regen_reviewed {
        return Err(PresetOnboardingError::UnreviewedRegen);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_runbook_covers_setup_without_dx_init_template() {
        assert_eq!(
            PRESET_RUNBOOK_STEPS,
            [
                "dependency_snippet",
                "generation_target",
                "import_block",
                "update_loop",
                "bot_config_sample"
            ]
        );
        assert!(!dx_init_emits_preset_template());
    }

    #[test]
    fn preset_labels_pass_through_verbatim_without_implicit_default() {
        assert_eq!(
            validate_preset_label("@rules_dx//tools/bazelrc:preset"),
            Ok("@rules_dx//tools/bazelrc:preset".to_owned())
        );
        assert_eq!(
            validate_preset_label(""),
            Err(PresetLabelError::MissingLabel)
        );
    }

    #[test]
    fn preset_affecting_changes_require_minor_or_major_and_notes() {
        assert!(preset_affecting_requires_minor_or_major(true));
        assert!(!preset_affecting_requires_minor_or_major(false));
        assert!(!preset_change_allowed_in_patch(true));
        assert!(preset_change_allowed_in_patch(false));
        assert!(preset_change_needs_release_note(true));
        assert!(!preset_change_needs_release_note(false));
    }

    #[test]
    fn preset_regen_stays_manual_without_auto_merge() {
        assert!(!preset_bot_auto_merge_allowed());
        assert!(preset_regen_requires_review());
    }

    #[test]
    fn clean_consumer_must_prove_generation_import_and_reviewed_regen() {
        assert_eq!(
            plan_preset_onboarding(false, true, true),
            Err(PresetOnboardingError::MissingGeneration)
        );
        assert_eq!(
            plan_preset_onboarding(true, false, true),
            Err(PresetOnboardingError::MissingImport)
        );
        assert_eq!(
            plan_preset_onboarding(true, true, false),
            Err(PresetOnboardingError::UnreviewedRegen)
        );
        assert_eq!(plan_preset_onboarding(true, true, true), Ok(()));
    }
}
