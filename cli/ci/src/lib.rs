//! Pure consumer-CI check-selection planning (M27 WP1 slices 1-7, WP2 slices 8-9,
//! WP4 slices 10-11, WP6 slice 12).
//!
//! This crate owns the check-selection surface before any reusable
//! workflow, caller template, or reporter lands: the nine accepted CI
//! checks, starter defaults, explicit opt-outs, the explicit
//! platform-selection gate for per-platform checks, revision planning,
//! and scheduling/isolation planning. It plans over
//! injected argument strings only, so selection stays deterministic and
//! unit-testable without GitHub Actions, a Bazel server, or any runner.
//!
//! Frozen check table (`docs/github-ci.md#check-selection`): six checks run
//! once on Linux (`lint`, `typecheck`, `format`, `generate`,
//! `security-audit`, `license-audit`); three run on every
//! consumer-selected platform (`test`, `build`, `coverage`). The starter
//! enables all nine; callers disable individual checks by ID without
//! affecting unrelated checks, and disabled checks are omitted, never
//! reported as passed.
//!
//! Platform identities stay opaque here: when any per-platform check is
//! enabled, the caller must supply an explicit nonempty platform list,
//! and missing/empty selections fail closed. Runner/OS/arch mapping and
//! the supported-identity set arrive in later M27 slices; this crate
//! preserves spellings verbatim and never substitutes an implicit
//! Linux/current-runner/all-platforms default.
//!
//! Scheduling (`docs/github-ci.md#execution`): independent cells run in
//! parallel by default; explicit sequential mode changes overlap only.
//! Failures preserve completed results, fail the overall run, and never
//! cancel independent cells. Starter cells are independent (no
//! prerequisites), so a sibling failure blocks nothing. Every cell owns
//! isolated checkout, report-destination, and Bazel output-base identities;
//! no two cells share an output-base.
//!
//! Out of scope here (M27 qualification): workflow APIs/pins beyond the opaque
//! identities below, event/ref bindings beyond the revision-planning identities
//! below, and any YAML or reporter implementation. Those arrive in later M27
//! slices (workflow/reporter YAML plus GitHub execution stay blocked on
//! qualification). Audit rendering below plans only the
//! accepted presentation/safety boundary; numeric thread limits and frozen
//! runner mappings stay deferred. Untrusted artifact/metadata validation and
//! thread-accounting deltas below plan only the pure binding/accounting rules;
//! transport, API limits, and execution stay deferred. Preset onboarding below
//! plans only the stability discipline and runbook/update shape over opaque
//! labels; the frozen public load-label string, workflow/pin representation,
//! and `dx init` template emission stay deferred (M30).

// Issue #238: infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

pub mod audit;
pub mod caller;
pub mod fork;
pub mod reporting;
pub mod rerun;
pub mod revision;
pub mod scheduling;
pub mod selection;
pub mod supersession;
pub mod triggers;
pub mod untrusted;

pub use audit::{
    audit_assumes_private, audit_body_publishable, audit_claims_generic_redaction,
    audit_has_disclosure_toggle, audit_uses_counts_only_mode, plan_audit_placement, AuditFinding,
    AuditPlacement,
};
pub use caller::{
    apply_pin_update, plan_caller, plan_pin_update, uses_generated_setup_command, CallerError,
    PinError, PinUpdate, PlannedCaller, CALLER_OWNED_INPUTS, WORKFLOW_OWNED,
};
pub use fork::{
    fork_credentials, plan_aggregate, plan_approval, privileged_reporting_executes_fork_code,
    AggregateState, ApproverRole, ForkCredentials, AGGREGATE_CHECK,
};
pub use reporting::{
    plan_summary, plan_threads, reporting_gate, Finding, OwnedThread, Summary, ThreadPlan,
};
pub use rerun::{
    plan_code_scanning, plan_coverage_aggregate, plan_report_retry, plan_rerun,
    scope_runs_all_selected, uses_path_filters, CodeScanningPlan, ReportRetry,
    CODE_SCANNING_DEFAULT,
};
pub use revision::{plan_revision, PlannedRevision, Reporting, RevisionError, RevisionRequest};
pub use scheduling::{
    aggregate_outcome, plan_schedule, CellIsolation, CellOutcome, ExecutionCell, PlannedCell,
    PlannedSchedule, SchedulingMode,
};
pub use selection::{
    find_check, plan_selection, Check, CiSelection, ExecutionScope, SelectionError, ALL_CHECKS,
    BUILD_CHECK, CHECK_COUNT, COVERAGE_CHECK, FORMAT_CHECK, GENERATE_CHECK, LICENSE_AUDIT_CHECK,
    LINT_CHECK, SECURITY_AUDIT_CHECK, TEST_CHECK, TYPECHECK_CHECK,
};
pub use supersession::{may_publish, result_satisfies, supersedes, RunScope, TrackedRun};
pub use triggers::{
    base_advanced_requires_rerun, is_collection_failure, starter_triggers, validate_platforms,
    PlatformError,
};
pub use untrusted::{
    concurrent_runs_share_limit, resolved_discussions_count_against_limit, thread_slots_used,
    untrusted_inputs_execute_fork_code, untrusted_inputs_grant_secrets, validate_artifact_snapshot,
    validate_pr_metadata, ArtifactError, MetadataError,
};

// ---------------------------------------------------------------------------
// Scheduling and isolation planning (M27 WP1 slice 3).
// ---------------------------------------------------------------------------
// (moved to scheduling.rs; re-exported above)

// ---------------------------------------------------------------------------
// Supersession and queue-revision planning (M27 WP1 slice 4).
// ---------------------------------------------------------------------------
// (moved to supersession.rs; re-exported above)

// ---------------------------------------------------------------------------
// Reporting and review-thread planning (M27 WP1 slice 5).
// ---------------------------------------------------------------------------
// (moved to reporting.rs; re-exported above)

// ---------------------------------------------------------------------------
// Fork-security and aggregate-gating planning (M27 WP1 slice 6).
// ---------------------------------------------------------------------------
// (moved to fork.rs; re-exported above)

// ---------------------------------------------------------------------------
// Rerun, scope, retry, and code-scanning planning (M27 WP1 slice 7).
// ---------------------------------------------------------------------------
// (moved to rerun.rs; re-exported above)

// ---------------------------------------------------------------------------
// Starter triggers, platform qualification mechanics, no-op semantics, and
// latest-target revalidation (M27 WP2 slice 8).
// ---------------------------------------------------------------------------
// (moved to triggers.rs; re-exported above)

// ---------------------------------------------------------------------------
// Caller-template composition and reviewed pin updates (M27 WP2 slice 9).
// ---------------------------------------------------------------------------
// (moved to caller.rs; re-exported above)

// ---------------------------------------------------------------------------
// Audit rendering planning (M27 WP4 slice 10).
// ---------------------------------------------------------------------------
// (moved to audit.rs; re-exported above)

// ---------------------------------------------------------------------------
// Untrusted-artifact/metadata validation and thread-accounting deltas
// (M27 WP4 slice 11).
// ---------------------------------------------------------------------------
// (moved to untrusted.rs; re-exported above)

// ---------------------------------------------------------------------------
// Consumer `.bazelrc` preset onboarding planning (M27 WP6 slice 12).
// ---------------------------------------------------------------------------

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
/// [`apply_pin_update`]).
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
