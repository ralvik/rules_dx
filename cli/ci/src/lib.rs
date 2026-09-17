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

/// Malformed untrusted artifact: artifacts and PR metadata are untrusted
/// inputs (`docs/github-ci.md#fork-security`) and privileged reporting must
/// validate them before use, never trusting by presence.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ArtifactError {
    /// Artifact snapshot identity missing or empty.
    #[error("artifact snapshot identity is required")]
    MissingArtifact,
    /// Artifact digest identity missing or empty.
    #[error("artifact digest identity is required")]
    MissingDigest,
    /// Artifact snapshot does not bind to the required validated snapshot.
    #[error("artifact does not bind to the required validated snapshot")]
    SnapshotMismatch,
}

/// Validate one untrusted artifact against the required validated snapshot.
///
/// `artifact_validated` is the snapshot the artifact claims; `required` is
/// the [`PlannedRevision::validated`] snapshot every selected check used;
/// `digest` is the opaque artifact identity. Empty identities fail closed and
/// a non-matching snapshot fails with [`ArtifactError::SnapshotMismatch`]:
/// stale or foreign artifacts never satisfy the current run, and presence
/// alone never establishes trust. Digest algorithms and transport stay
/// deferred; this plans only the exact-binding rule.
pub fn validate_artifact_snapshot(
    artifact_validated: &str,
    required: &str,
    digest: &str,
) -> Result<(), ArtifactError> {
    if artifact_validated.is_empty() {
        return Err(ArtifactError::MissingArtifact);
    }
    if digest.is_empty() {
        return Err(ArtifactError::MissingDigest);
    }
    if required.is_empty() || artifact_validated != required {
        return Err(ArtifactError::SnapshotMismatch);
    }
    Ok(())
}

/// Malformed untrusted PR metadata: privileged reporting must validate
/// metadata against the planned revision before creating, updating, or
/// cleaning up review threads or the summary.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum MetadataError {
    /// A metadata identity is missing or empty.
    #[error("PR metadata identities are required")]
    MissingField,
    /// Metadata does not match the planned revision (stale or foreign run).
    #[error("PR metadata does not match the planned revision snapshot")]
    StaleSnapshot,
}

/// Validate untrusted PR metadata against the planned revision.
///
/// `head`/`base`/`validated` are the opaque identities carried by the
/// metadata event; `planned` is the [`PlannedRevision`] the run validated.
/// Empty identities fail with [`MetadataError::MissingField`]; any mismatch
/// with the planned `(validated, head, base)` fails with
/// [`MetadataError::StaleSnapshot`] so stale reporting can neither create nor
/// modify current threads (see [`may_publish`]). Non-PR runs carry no PR
/// metadata: callers must not invent head/base there.
pub fn validate_pr_metadata(
    head: &str,
    base: &str,
    validated: &str,
    planned: &PlannedRevision,
) -> Result<(), MetadataError> {
    if head.is_empty() || base.is_empty() || validated.is_empty() {
        return Err(MetadataError::MissingField);
    }
    let planned_head = planned.head.as_deref().unwrap_or("");
    let planned_base = planned.base.as_deref().unwrap_or("");
    if validated != planned.validated || head != planned_head || base != planned_base {
        return Err(MetadataError::StaleSnapshot);
    }
    Ok(())
}

/// Untrusted inputs never authorize fork-code execution in privileged
/// reporting.
pub fn untrusted_inputs_execute_fork_code() -> bool {
    false
}

/// Untrusted inputs never grant secrets or write credentials to fork code.
pub fn untrusted_inputs_grant_secrets() -> bool {
    false
}

/// Slots occupied against the fixed per-PR review-thread limit.
///
/// Only still-present findings with retained threads ([`ThreadPlan::keep`])
/// occupy slots. Bot-only threads queued for deletion and replied threads
/// queued for resolution are confirmed gone, so they free their slots for new
/// findings; retained resolved discussions do not block new threads. Limit
/// accounting is per PR across checks and platforms with no fresh allowance
/// per job, rerun, retry, or concurrent completion (see [`may_publish`]: only
/// the current run publishes).
pub fn thread_slots_used(plan: &ThreadPlan) -> usize {
    plan.keep.len()
}

/// Retained resolved discussions never count against the new-thread budget.
pub fn resolved_discussions_count_against_limit() -> bool {
    false
}

/// Concurrent runs never receive a fresh per-run thread allowance.
pub fn concurrent_runs_share_limit() -> bool {
    true
}

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

    fn finding(id: &str, failure: bool, located: bool) -> Finding {
        Finding {
            id: id.to_owned(),
            contributes_to_failure: failure,
            location: if located {
                Some(format!("file.rs:{id}"))
            } else {
                None
            },
        }
    }

    #[test]
    fn artifacts_bind_exactly_to_the_validated_snapshot() {
        assert_eq!(
            validate_artifact_snapshot("", "merge-a", "digest-1"),
            Err(ArtifactError::MissingArtifact)
        );
        assert_eq!(
            validate_artifact_snapshot("merge-a", "merge-a", ""),
            Err(ArtifactError::MissingDigest)
        );
        assert_eq!(
            validate_artifact_snapshot("merge-a", "merge-b", "digest-1"),
            Err(ArtifactError::SnapshotMismatch)
        );
        // Presence alone never establishes trust: empty required snapshot
        // cannot be satisfied.
        assert_eq!(
            validate_artifact_snapshot("merge-a", "", "digest-1"),
            Err(ArtifactError::SnapshotMismatch)
        );
        assert_eq!(
            validate_artifact_snapshot("merge-a", "merge-a", "digest-1"),
            Ok(())
        );
    }

    #[test]
    fn stale_or_foreign_artifacts_never_satisfy_the_current_run() {
        let planned = plan_revision(RevisionRequest::PullRequest {
            head: "head-sha",
            base: "base-sha",
            merged: "merge-b",
            draft: false,
        })
        .expect("PR plans");
        assert!(validate_artifact_snapshot("merge-a", &planned.validated, "digest-1").is_err());
        assert!(
            validate_artifact_snapshot(&planned.validated, &planned.validated, "digest-1").is_ok()
        );
    }

    #[test]
    fn pr_metadata_must_match_the_planned_revision_exactly() {
        let planned = plan_revision(RevisionRequest::PullRequest {
            head: "head-sha",
            base: "base-sha",
            merged: "merge-sha",
            draft: false,
        })
        .expect("PR plans");
        assert_eq!(
            validate_pr_metadata("head-sha", "base-sha", "merge-sha", &planned),
            Ok(())
        );
        assert_eq!(
            validate_pr_metadata("", "base-sha", "merge-sha", &planned),
            Err(MetadataError::MissingField)
        );
        assert_eq!(
            validate_pr_metadata("head-sha", "base-sha", "merge-stale", &planned),
            Err(MetadataError::StaleSnapshot)
        );
        assert_eq!(
            validate_pr_metadata("head-other", "base-sha", "merge-sha", &planned),
            Err(MetadataError::StaleSnapshot)
        );
        assert_eq!(
            validate_pr_metadata("head-sha", "base-other", "merge-sha", &planned),
            Err(MetadataError::StaleSnapshot)
        );
    }

    #[test]
    fn untrusted_inputs_grant_no_execution_or_secrets() {
        assert!(!untrusted_inputs_execute_fork_code());
        assert!(!untrusted_inputs_grant_secrets());
        assert!(!privileged_reporting_executes_fork_code());
        assert_eq!(fork_credentials(), ForkCredentials::ReadOnly);
    }

    #[test]
    fn cleared_findings_free_thread_slots_for_new_findings() {
        // Limit 1: `old` occupies the only slot, `new` is omitted.
        let first = vec![finding("old", true, true), finding("new", true, true)];
        let existing = vec![OwnedThread {
            finding: "old".to_owned(),
            has_human_replies: false,
        }];
        let full = plan_threads(&first, &existing, &[], 1);
        assert_eq!(full.keep, vec!["old".to_owned()]);
        assert!(full.create.is_empty());
        assert_eq!(thread_slots_used(&full), 1);
        // `old` clears (bot-only, confirmed gone): its slot frees and the
        // next assessment creates `new`.
        let second = vec![finding("new", true, true)];
        let next = plan_threads(&second, &existing, &["old".to_owned()], 1);
        assert_eq!(next.delete, vec!["old".to_owned()]);
        assert_eq!(thread_slots_used(&next), 0);
        let after = plan_threads(
            &second,
            &[OwnedThread {
                finding: "new".to_owned(),
                has_human_replies: false,
            }],
            &["old".to_owned()],
            1,
        );
        assert_eq!(after.keep, vec!["new".to_owned()]);
    }

    #[test]
    fn retained_resolved_discussions_do_not_block_new_threads() {
        assert!(!resolved_discussions_count_against_limit());
        assert!(concurrent_runs_share_limit());
        // Replied thread for a gone finding resolves (discussion retained)
        // and frees its slot: only still-present threads count.
        let existing = vec![OwnedThread {
            finding: "gone-discussed".to_owned(),
            has_human_replies: true,
        }];
        let cleared = plan_threads(&[], &existing, &["gone-discussed".to_owned()], 1);
        assert_eq!(cleared.resolve, vec!["gone-discussed".to_owned()]);
        assert_eq!(thread_slots_used(&cleared), 0);
        let newcomer = vec![finding("fresh", true, true)];
        let next = plan_threads(&newcomer, &[], &["gone-discussed".to_owned()], 1);
        assert_eq!(next.create, vec!["fresh".to_owned()]);
        // Concurrent completions share one budget: no fresh allowance per run.
        let shared = plan_threads(&newcomer, &[], &[], 1);
        assert_eq!(shared.create, vec!["fresh".to_owned()]);
        assert_eq!(shared.create.len(), 1);
    }

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
