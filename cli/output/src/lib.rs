//! Public output modes and versioned NDJSON protocol for the `dx` CLI
//! (WP2).
//!
//! Contract: `docs/cli/output-protocol.md`. This crate owns live-mode
//! selection (`text`/`diff`/`json`), stdout-ownership policy, the v1 NDJSON
//! event shapes with exact field presence and omission rules, deterministic
//! diagnostic ordering, and protocol-shape validation (workspace paths,
//! digest spellings, edit ordering). It never renders subprocess argv,
//! option values, credentials, or original source bytes: replacement text
//! appears only inside validated `change` events built by the caller.
//!
//! JSON objects serialize with `serde_json`'s default key ordering; NDJSON
//! validity never depends on key order. Consumers parse values, not bytes.
//!
//! Domain split: structured diagnostics and human status
//! emission live in the `diagnostics` module, live output modes and stdout
//! ownership live in the `modes` module, normalized severity and fail-on
//! threshold live in the `severity` module, protocol-shape validation
//! (paths, digests, edits, `OutputError`) lives in the `validation`
//! module, command lifecycle NDJSON events live in the `lifecycle` module,
//! change and mutation NDJSON events live in the `changes` module,
//! diagnostic and notice NDJSON events live in the `findings` module. This
//! facade only re-exports; the public path stays `dx_output::{...}`.

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

pub mod changes;
pub mod diagnostics;
pub mod findings;
pub mod lifecycle;
pub mod modes;
pub mod severity;
pub mod validation;

pub use changes::{change_event, mutation_event, ChangeEvent, ChangeKind, MutationOutcome};
pub use diagnostics::{
    color_enabled, colors_allowed, emit_status, format_status, init_diagnostics, styled_status,
    styled_status_for, DEFAULT_LOG_FILTER, VERBOSE_LOG_FILTER,
};
pub use findings::{
    diagnostic_event, notice_event, sort_diagnostics, DiagnosticEvent, NoticeEvent, Resolution,
    Snapshot,
};
pub use lifecycle::{
    command_finished, command_started, error_event, operation_event, report_event, schema,
    selection_event, status_event, write_event, FinishedCounts, StatusEvent, SCHEMA_MAJOR,
    SCHEMA_MINOR,
};
pub use modes::{
    check_output_conflict, dx_text_visible, stdout_owner, OutputMode, OutputModeName, StdoutOwner,
};
pub use severity::{meets_threshold, Severity, Threshold};
pub use validation::{check_edits, check_path, parse_digest, Edit, OutputError};
