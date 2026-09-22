// LCOV_EXCL_START - policy: docs/testing/README.md#coverage
//! `dx_cli`: quality command planning for the `dx` CLI.
//!
//! Contract: `docs/cli/cli-contract.md`.
//!
//! WP1 owns invocation parsing ([`args`]) and Bazel workflow planning
//! with canonical workspace policy selection ([`plan`]). Result projection
//! (WP2) and repository dogfood wiring (WP3) build on these plans. WP1
//! adds scope resolution ([`resolve`]): labels pass through while files
//! resolve to owning targets through Bazel query.

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(
    not(test),
    deny(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::unreachable,
        clippy::todo
    )
)]

pub mod adopt;
pub mod args;
pub mod exec;
pub mod finalize;
pub mod generate;
pub mod plan;
pub mod platform;
pub mod reports;
pub mod resolve;
pub mod skew;

pub use args::{ArgsError, Command, Invocation, ReportRequest};
pub use exec::{execute, Env};
pub use finalize::{
    finalize, FinalizeError, FinalizeInput, FAILURE_MISSING_FILE, FAILURE_UNREADABLE_FILE,
    FAILURE_WRITE_MISMATCH,
};
pub use generate::{
    ensure_mode, manifest_is_check, project, render_diff, text_lines, ProjectedFile,
    ProjectedManifest, IGNORED_IMPORT_CODE, IGNORED_IMPORT_LEVEL, IGNORED_IMPORT_MESSAGE,
};
pub use plan::{BuildPlan, CommandSpec};
pub use reports::{Destination, PlannedReport, ReportError, StandardFormat};
pub use resolve::{
    expand_codegen_roots, map_owners_to_tests, resolve, resolve_for_test, resolve_run,
    ProcessQueryRunner, QueryResult, QueryRunner, ResolveError, ResolvedScope,
};
// LCOV_EXCL_STOP - policy: docs/testing/README.md#coverage
