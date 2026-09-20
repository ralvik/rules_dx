//! Pure consumer-CI check-selection planning (WP1 slices 1-7, WP2 slices 8-9,
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
//! the supported-identity set arrive in later slices; this crate
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
//! Out of scope here (qualification): workflow APIs/pins beyond the opaque
//! identities below, event/ref bindings beyond the revision-planning identities
//! below, and any YAML or reporter implementation. Those arrive in later
//! slices (workflow/reporter YAML plus GitHub execution stay blocked on
//! qualification). Audit rendering below plans only the
//! accepted presentation/safety boundary; numeric thread limits and frozen
//! runner mappings stay deferred. Untrusted artifact/metadata validation and
//! thread-accounting deltas below plan only the pure binding/accounting rules;
//! transport, API limits, and execution stay deferred. Preset onboarding below
//! plans only the stability discipline and runbook/update shape over opaque
//! labels; the frozen public load-label string, workflow/pin representation,
//! and `dx init` template emission stay deferred.

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
