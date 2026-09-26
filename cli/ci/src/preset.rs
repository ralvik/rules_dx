pub const PRESET_RUNBOOK_STEPS: [&str; 5] = [
    "dependency_snippet",
    "generation_target",
    "import_block",
    "update_loop",
    "bot_config_sample",
];

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PresetLabelError {
    #[error("explicit preset load label is required")]
    MissingLabel,
}

pub fn validate_preset_label(label: &str) -> Result<String, PresetLabelError> {
    if label.is_empty() {
        return Err(PresetLabelError::MissingLabel);
    }
    Ok(label.to_owned())
}

pub fn preset_affecting_requires_minor_or_major(preset_affecting: bool) -> bool {
    preset_affecting
}

pub fn preset_change_allowed_in_patch(preset_affecting: bool) -> bool {
    !preset_affecting
}

pub fn preset_change_needs_release_note(preset_affecting: bool) -> bool {
    preset_affecting
}

pub fn dx_init_emits_preset_template() -> bool {
    false
}

pub fn preset_bot_auto_merge_allowed() -> bool {
    false
}

pub fn preset_regen_requires_review() -> bool {
    true
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PresetOnboardingError {
    #[error("clean-consumer preset generation proof is required")]
    MissingGeneration,
    #[error("clean-consumer preset import proof is required")]
    MissingImport,
    #[error("preset regen-and-review update requires a reviewed flag diff")]
    UnreviewedRegen,
}

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
