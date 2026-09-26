use super::{CiSelection, PlatformError, SchedulingMode, SelectionError};

pub const CALLER_OWNED_INPUTS: [&str; 4] = [
    "disabled_checks",
    "platforms",
    "scheduling_mode",
    "code_scanning_opt_in",
];

pub const WORKFLOW_OWNED: [&str; 4] = [
    "execution",
    "parsing",
    "review_threads",
    "comment_management",
];

pub fn uses_generated_setup_command() -> bool {
    false
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedCaller {
    pub selection: CiSelection,
    pub mode: SchedulingMode,
    pub code_scanning_opt_in: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CallerError {
    #[error(transparent)]
    Selection(#[from] SelectionError),
    #[error(transparent)]
    Platform(#[from] PlatformError),
}

pub fn plan_caller(
    disabled: &[String],
    platforms: &[String],
    supported: &[String],
    mode: SchedulingMode,
    code_scanning_opt_in: bool,
) -> Result<PlannedCaller, CallerError> {
    let selection = super::plan_selection(disabled, platforms).map_err(CallerError::Selection)?;
    if selection.requires_platforms() {
        let validated = super::validate_platforms(&selection.platforms, supported)
            .map_err(CallerError::Platform)?;
        Ok(PlannedCaller {
            selection: CiSelection {
                enabled: selection.enabled,
                platforms: validated,
            },
            mode,
            code_scanning_opt_in,
        })
    } else {
        Ok(PlannedCaller {
            selection,
            mode,
            code_scanning_opt_in,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PinUpdate {
    NoChange,
    ReviewedUpdate {
        from: String,
        to: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PinError {
    #[error("explicit workflow pin identities are required")]
    MissingPin,
    #[error("workflow pins must be full 40-hex commit SHAs, got {value:?}")]
    InvalidPin { value: String },
    #[error("workflow pin changes require review, not silent upgrades")]
    UnreviewedChange,
}

fn is_full_sha(pin: &str) -> bool {
    dx_digest::is_pin_sha(pin)
}

pub fn plan_pin_update(
    old_pin: &str,
    new_pin: &str,
    reviewed: bool,
) -> Result<PinUpdate, PinError> {
    if old_pin.is_empty() || new_pin.is_empty() {
        return Err(PinError::MissingPin);
    }
    if !is_full_sha(old_pin) {
        return Err(PinError::InvalidPin {
            value: old_pin.to_owned(),
        });
    }
    if !is_full_sha(new_pin) {
        return Err(PinError::InvalidPin {
            value: new_pin.to_owned(),
        });
    }
    if old_pin == new_pin {
        return Ok(PinUpdate::NoChange);
    }
    if !reviewed {
        return Err(PinError::UnreviewedChange);
    }
    Ok(PinUpdate::ReviewedUpdate {
        from: old_pin.to_owned(),
        to: new_pin.to_owned(),
    })
}

/// Apply a planned pin change to a caller without touching consumer
pub fn apply_pin_update(caller: &PlannedCaller, update: &PinUpdate) -> PlannedCaller {
    let _ = update;
    caller.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SelectionError, CHECK_COUNT};

    fn strings(words: &[&str]) -> Vec<String> {
        words.iter().copied().map(str::to_owned).collect()
    }

    #[test]
    fn caller_template_owns_selection_only_not_workflow_logic() {
        assert_eq!(
            CALLER_OWNED_INPUTS,
            [
                "disabled_checks",
                "platforms",
                "scheduling_mode",
                "code_scanning_opt_in"
            ]
        );
        assert_eq!(
            WORKFLOW_OWNED,
            [
                "execution",
                "parsing",
                "review_threads",
                "comment_management"
            ]
        );
        assert!(!uses_generated_setup_command());
    }

    #[test]
    fn starter_caller_composes_selection_mode_and_opt_in() {
        let supported = strings(&["linux_x86_64", "macos_arm64"]);
        let caller = plan_caller(
            &[],
            &strings(&["linux_x86_64", "macos_arm64"]),
            &supported,
            SchedulingMode::Parallel,
            false,
        )
        .expect("starter caller plans");
        assert_eq!(caller.selection.enabled.len(), CHECK_COUNT);
        assert_eq!(
            caller.selection.platforms,
            vec!["linux_x86_64".to_owned(), "macos_arm64".to_owned()]
        );
        assert_eq!(caller.mode, SchedulingMode::Parallel);
        assert!(!caller.code_scanning_opt_in);
    }

    #[test]
    fn caller_opt_out_and_sequential_mode_pass_through() {
        let supported = strings(&["linux_x86_64"]);
        let caller = plan_caller(
            &strings(&["lint"]),
            &strings(&["linux_x86_64"]),
            &supported,
            SchedulingMode::Sequential,
            true,
        )
        .expect("caller plans");
        assert_eq!(caller.selection.enabled.len(), CHECK_COUNT - 1);
        assert!(!caller.selection.enabled_ids().contains(&"lint"));
        assert_eq!(caller.mode, SchedulingMode::Sequential);
        assert!(caller.code_scanning_opt_in);
    }

    #[test]
    fn empty_platforms_plan_only_when_all_checks_disabled() {
        let disabled = strings(&[
            "lint",
            "typecheck",
            "format",
            "generate",
            "security-audit",
            "license-audit",
            "test",
            "build",
            "coverage",
        ]);
        let caller = plan_caller(&disabled, &[], &[], SchedulingMode::Parallel, false)
            .expect("empty selection plans");
        assert!(!caller.selection.requires_platforms());
        assert!(caller.selection.platforms.is_empty());
        assert!(caller.selection.enabled.is_empty());
    }

    #[test]
    fn caller_rejects_unknown_checks_and_bad_platforms_closed() {
        let supported = strings(&["linux_x86_64"]);
        assert_eq!(
            plan_caller(
                &strings(&["audit"]),
                &strings(&["linux_x86_64"]),
                &supported,
                SchedulingMode::Parallel,
                false,
            ),
            Err(CallerError::Selection(SelectionError::UnknownCheck {
                value: "audit".to_owned()
            }))
        );
        assert_eq!(
            plan_caller(&[], &[], &supported, SchedulingMode::Parallel, false),
            Err(CallerError::Selection(SelectionError::MissingPlatforms))
        );
        assert_eq!(
            plan_caller(
                &[],
                &strings(&["windows_x86_64"]),
                &supported,
                SchedulingMode::Parallel,
                false,
            ),
            Err(CallerError::Platform(PlatformError::Unsupported {
                value: "windows_x86_64".to_owned()
            }))
        );
    }

    #[test]
    fn pin_changes_require_review_and_explicit_identities() {
        let sha_a = "3d3c42e5aac5ba805825da76410c181273ba90b1";
        let sha_b = "55cc8345863c7cc4c66a329aec7e433d2d1c52a9";
        assert_eq!(
            plan_pin_update(sha_a, sha_a, false),
            Ok(PinUpdate::NoChange)
        );
        assert_eq!(
            plan_pin_update(sha_a, sha_b, false),
            Err(PinError::UnreviewedChange)
        );
        assert_eq!(
            plan_pin_update(sha_a, sha_b, true),
            Ok(PinUpdate::ReviewedUpdate {
                from: sha_a.to_owned(),
                to: sha_b.to_owned(),
            })
        );
        assert_eq!(plan_pin_update("", sha_b, true), Err(PinError::MissingPin));
        assert_eq!(plan_pin_update(sha_a, "", true), Err(PinError::MissingPin));
    }

    #[test]
    fn pin_changes_reject_floating_tags_and_short_shas() {
        let sha = "3d3c42e5aac5ba805825da76410c181273ba90b1";
        for floating in ["v7", "main", "master", "latest", "3d3c42e5", ""] {
            assert!(
                matches!(
                    plan_pin_update(floating, sha, true),
                    Err(PinError::MissingPin) | Err(PinError::InvalidPin { .. })
                ),
                "floating {floating:?} must fail closed"
            );
            assert!(
                matches!(
                    plan_pin_update(sha, floating, true),
                    Err(PinError::MissingPin) | Err(PinError::InvalidPin { .. })
                ),
                "floating {floating:?} must fail closed"
            );
        }
        // Uppercase hex is not the canonical `[0-9a-f]{40}` shape.
        assert!(matches!(
            plan_pin_update("3D3C42E5AAC5BA805825DA76410C181273BA90B1", sha, true),
            Err(PinError::InvalidPin { .. })
        ));
    }

    #[test]
    fn pin_updates_preserve_consumer_customizations() {
        let supported = strings(&["linux_x86_64"]);
        let caller = plan_caller(
            &strings(&["lint"]),
            &strings(&["linux_x86_64"]),
            &supported,
            SchedulingMode::Sequential,
            true,
        )
        .expect("caller plans");
        let update = plan_pin_update(
            "3d3c42e5aac5ba805825da76410c181273ba90b1",
            "55cc8345863c7cc4c66a329aec7e433d2d1c52a9",
            true,
        )
        .expect("reviewed update plans");
        assert_eq!(apply_pin_update(&caller, &update), caller);
    }
}
