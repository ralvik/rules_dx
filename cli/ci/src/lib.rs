// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

pub mod audit;
pub mod caller;
pub mod fork;
pub mod preset;
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
pub use preset::{
    dx_init_emits_preset_template, plan_preset_onboarding,
    preset_affecting_requires_minor_or_major, preset_bot_auto_merge_allowed,
    preset_change_allowed_in_patch, preset_change_needs_release_note, preset_regen_requires_review,
    validate_preset_label, PresetLabelError, PresetOnboardingError, PRESET_RUNBOOK_STEPS,
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
// Scheduling and isolation planning (WP1 slice 3).
// ---------------------------------------------------------------------------
// (moved to scheduling.rs; re-exported above)

// ---------------------------------------------------------------------------
// Supersession and queue-revision planning (WP1 slice 4).
// ---------------------------------------------------------------------------
// (moved to supersession.rs; re-exported above)

// ---------------------------------------------------------------------------
// Reporting and review-thread planning (WP1 slice 5).
// ---------------------------------------------------------------------------
// (moved to reporting.rs; re-exported above)

// ---------------------------------------------------------------------------
// Fork-security and aggregate-gating planning (WP1 slice 6).
// ---------------------------------------------------------------------------
// (moved to fork.rs; re-exported above)

// ---------------------------------------------------------------------------
// Rerun, scope, retry, and code-scanning planning (WP1 slice 7).
// ---------------------------------------------------------------------------
// (moved to rerun.rs; re-exported above)

// ---------------------------------------------------------------------------
// Starter triggers, platform qualification mechanics, no-op semantics, and
// latest-target revalidation (WP2 slice 8).
// ---------------------------------------------------------------------------
// (moved to triggers.rs; re-exported above)

// ---------------------------------------------------------------------------
// Caller-template composition and reviewed pin updates (WP2 slice 9).
// ---------------------------------------------------------------------------
// (moved to caller.rs; re-exported above)

// ---------------------------------------------------------------------------
// Audit rendering planning (WP4 slice 10).
// ---------------------------------------------------------------------------
// (moved to audit.rs; re-exported above)

// ---------------------------------------------------------------------------
// Untrusted-artifact/metadata validation and thread-accounting deltas
// (WP4 slice 11).
// ---------------------------------------------------------------------------
// (moved to untrusted.rs; re-exported above)

// ---------------------------------------------------------------------------
// Consumer `.bazelrc` preset onboarding planning (WP6 slice 12).
// ---------------------------------------------------------------------------
// (moved to preset.rs; re-exported above)
