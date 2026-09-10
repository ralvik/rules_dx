// LCOV_EXCL_START - reason: module root holds only mod declarations and re-exports with no executable statements; every item is covered in its own module.
//! `dx_cli`: quality command planning for the `dx` CLI.
//!
//! M07 WP1 owns invocation parsing ([`args`]) and Bazel workflow planning
//! with canonical workspace policy selection ([`plan`]). Result projection
//! (WP2) and repository dogfood wiring (WP3) build on these plans. M08 WP1
//! adds scope resolution ([`resolve`]): labels pass through while files
//! resolve to owning targets through Bazel query.

pub mod args;
pub mod exec;
pub mod plan;
pub mod reports;
pub mod resolve;

pub use args::{ArgsError, Command, Invocation, ReportRequest};
pub use exec::{execute, Env};
pub use plan::{BuildPlan, CommandSpec};
pub use reports::{Destination, PlannedReport, ReportError, StandardFormat};
pub use resolve::{
    map_owners_to_tests, resolve, resolve_for_test, resolve_run, ProcessQueryRunner, QueryResult,
    QueryRunner, ResolveError, ResolvedScope,
};
// LCOV_EXCL_STOP - reason: end of re-export-only module root exclusion.
