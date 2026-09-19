//! Caller-template composition and reviewed pin updates for consumer CI
//! (issue #236, M27 WP2 slice 9).
//!
//! Split from `super` (`lib.rs`): owns [`CALLER_OWNED_INPUTS`],
//! [`WORKFLOW_OWNED`], [`uses_generated_setup_command`], [`PlannedCaller`],
//! [`CallerError`], [`plan_caller`] (composes [`super::plan_selection`]
//! with [`super::validate_platforms`] over the injected supported set),
//! [`PinUpdate`], [`PinError`], [`plan_pin_update`], and
//! [`apply_pin_update`]. Re-exported through `super` so the public paths
//! stay `dx_ci::{CALLER_OWNED_INPUTS, WORKFLOW_OWNED,
//! uses_generated_setup_command, PlannedCaller, CallerError, plan_caller,
//! PinUpdate, PinError, plan_pin_update, apply_pin_update}`. Distinct from
//! the revision, selection, scheduling, supersession, reporting,
//! fork/aggregate, rerun, trigger, platform, audit, artifact, metadata,
//! and preset modules.

use super::{CiSelection, PlatformError, SchedulingMode, SelectionError};

/// Caller-owned inputs: the only surface the consumer-owned caller template
/// configures (`docs/github-ci.md#consumer-setup`, `docs/testing/github-ci.md#consumer-setup`).
///
/// Everything else — per-check execution, output parsing, review threads,
/// and comment management — stays in the maintained versioned reusable
/// workflow and its first-party reporting; callers must not copy that logic.
pub const CALLER_OWNED_INPUTS: [&str; 4] = [
    "disabled_checks",
    "platforms",
    "scheduling_mode",
    "code_scanning_opt_in",
];

/// Workflow-owned responsibilities the caller template must not duplicate.
pub const WORKFLOW_OWNED: [&str; 4] = [
    "execution",
    "parsing",
    "review_threads",
    "comment_management",
];

/// No generated setup command and no new `dx` command back the caller
/// template; do not repurpose `dx setup` or `dx generate`.
pub fn uses_generated_setup_command() -> bool {
    false
}

/// Planned caller configuration: the composed selection plus the overlap
/// mode and Code Scanning opt-in the caller chose.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedCaller {
    /// Enabled checks plus verbatim platform spellings.
    pub selection: CiSelection,
    /// Overlap mode (scheduling only; cells and isolation are fixed).
    pub mode: SchedulingMode,
    /// Explicit Code Scanning opt-in (starter default is off).
    pub code_scanning_opt_in: bool,
}

/// Malformed caller configuration: unknown opt-outs or platform-gate
/// failures fail closed instead of silently narrowing validation.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CallerError {
    /// Unknown check ID or missing platform list (see [`SelectionError`]).
    #[error(transparent)]
    Selection(#[from] SelectionError),
    /// Empty or unsupported platform selection (see [`PlatformError`]).
    #[error(transparent)]
    Platform(#[from] PlatformError),
}

/// Plan one caller configuration from its owned inputs.
///
/// Composes [`super::plan_selection`] with [`super::validate_platforms`]
/// over the injected supported set: unknown opt-outs fail, per-platform
/// selections must be explicit nonempty supported spellings passed through
/// verbatim, and Linux-once-only selections need no platforms. `supported`
/// is injected — the frozen runner/platform mapping arrives with workflow
/// qualification (`docs/github-ci.md#qualification`). Mode and opt-in pass
/// through unchanged; non-mutating consistency modes are fixed by
/// [`CiSelection`].
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

/// Planned pin change: pins are full-length commit SHAs here — the same
/// `[0-9a-f]{40}` shape the shell harnesses enforce
/// (`tools/ci/examples_pins_test.sh`, `tools/ci/consumer_pins_test.sh`).
/// Workflow/pin representation, compatibility, and release mechanics freeze
/// with workflow qualification, not in this crate. Cross-caller equality
/// (both example callers sharing one reviewed SHA) is enforced by
/// `examples_pins_test.sh`; this module enforces per-pin shape.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PinUpdate {
    /// Pin unchanged: idempotent, no review needed.
    NoChange,
    /// Explicit reviewed bump from one opaque pin to another.
    ReviewedUpdate {
        /// Previously pinned opaque identity.
        from: String,
        /// Newly pinned opaque identity.
        to: String,
    },
}

/// Malformed pin change: missing pins, malformed (non-SHA) pins, or silent
/// (unreviewed) upgrades fail closed — updates use reviewed version-pin
/// changes, never silent upgrades.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PinError {
    /// Either pin identity is missing or empty (no implicit latest/default).
    #[error("explicit workflow pin identities are required")]
    MissingPin,
    /// Either pin identity is not a full-length commit SHA: floating tags
    /// (`vN`, `main`, `master`, `latest`) and short SHAs are rejected, matching
    /// the shell pin harnesses.
    #[error("workflow pins must be full 40-hex commit SHAs, got {value:?}")]
    InvalidPin { value: String },
    /// Pin change without review.
    #[error("workflow pin changes require review, not silent upgrades")]
    UnreviewedChange,
}

/// True for a full-length lowercase hex commit SHA (`[0-9a-f]{40}`).
fn is_full_sha(pin: &str) -> bool {
    pin.len() == 40
        && pin
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// Plan a workflow pin change over full-SHA pin identities.
///
/// Same pin is [`PinUpdate::NoChange`]; a change requires `reviewed` and
/// yields [`PinUpdate::ReviewedUpdate`], otherwise [`PinError::UnreviewedChange`].
/// Empty identities fail with [`PinError::MissingPin`]; non-SHA identities
/// (floating tags, short SHAs) fail with [`PinError::InvalidPin`]: there is
/// no implicit latest or silently active example value.
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
/// customizations: pin bumps never alter the caller-owned selection, mode,
/// or opt-in.
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
    fn linux_only_caller_needs_no_supported_set() {
        let caller = plan_caller(
            &strings(&["test", "build", "coverage"]),
            &[],
            &[],
            SchedulingMode::Parallel,
            false,
        )
        .expect("linux-only caller plans");
        assert!(!caller.selection.requires_platforms());
        assert!(caller.selection.platforms.is_empty());
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
