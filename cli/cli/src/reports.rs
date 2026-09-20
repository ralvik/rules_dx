//! Standard-report planning and SARIF/JUnit/LCOV projection (WP2, WP3).
//!
//! Contract: `docs/cli/standard-reports.md` and
//! `docs/cli/commands/quality.md`. Lint and typecheck export normalized
//! findings as SARIF 2.1.0 through the shared `--report` contract; format
//! has no initial standard report. `test` normalizes Bazel-reported
//! `test.xml` artifacts into one JUnit document and `coverage`
//! normalizes the BEP-reported combined tracefile into one LCOV document;
//! both derive from Bazel-owned artifacts rather than normalized
//! findings.
//!
//! Human text, diff, and NDJSON v1 projection reuses the `dx_output`
//! emitters directly; this module owns report planning (which fails
//! before Bazel execution) and the SARIF/JUnit/LCOV document rendering.
//!
//! Domain split: report rendering lives in domain
//! submodules — SARIF in [`sarif`](self::sarif), JUnit facade in
//! [`junit`](self::junit) (types in `junit_types`, parsing in
//! `junit_parse`, rendering in `junit_render`), LCOV in
//! [`lcov`](self::lcov), and planning
//! (`StandardFormat`, `Destination`, `PlannedReport`, `plan_reports`)
//! in [`planning`](self::planning). This facade keeps the shared error
//! (used by all four domains); the public path stays
//! `crate::reports::{...}` via the re-exports below.

pub mod junit;
mod junit_parse;
mod junit_render;
mod junit_types;
pub mod lcov;
pub mod planning;
pub mod sarif;

pub use junit::{junit_infrastructure_case, parse_test_xml, render_junit, JunitCase, JunitMessage};
pub use lcov::{coverage_line_rate, validate_lcov};
pub use planning::{plan_reports, Destination, PlannedReport, StandardFormat};
pub use sarif::{byte_to_line, render_sarif};

/// Report planning or rendering failure. Planning failures are
/// CLI-detected pre-execution usage errors (exit code 2); rendering
/// failures fail an otherwise successful invocation without altering
/// the underlying findings or mutation plan.
fn unsupported_format_message(command: &str, format: &str, supported: &[&str]) -> String {
    if supported.is_empty() {
        format!("unsupported report format {format:?} for {command}: no standard report exists")
    } else {
        format!(
            "unsupported report format {format:?} for {command}: want {}",
            supported.join("|")
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReportError {
    /// A report was requested together with `--dry-run`.
    #[error("--dry-run conflicts with every --report request")]
    DryRunConflict,
    /// The format is not supported by the command.
    #[error(
        "{msg}",
        msg = unsupported_format_message(command, format, supported)
    )]
    UnsupportedFormat {
        command: &'static str,
        format: String,
        supported: Vec<&'static str>,
    },
    /// The same format/destination pair was requested twice.
    #[error("duplicate report {format:?} for destination {destination:?}")]
    DuplicateReport { format: String, destination: String },
    /// More than one standard report targets stdout.
    #[error("more than one standard report targets stdout")]
    MultipleStdoutReports,
    /// A stdout report combined with `--output diff` or `--output json`.
    #[error(
        "a stdout report conflicts with --output {mode}: use --output text or a file destination"
    )]
    StdoutReportConflictsMode { mode: &'static str },
    /// A finding without the stable identity SARIF requires.
    #[error("invalid finding for SARIF export: {detail}")]
    InvalidFinding { detail: &'static str },
    /// A byte range without a path.
    #[error("a byte range without a path cannot be located")]
    RangeWithoutPath,
    /// A byte range whose start exceeds its end.
    #[error("a byte range starts after its end")]
    InvertedRange,
    /// A ranged finding without its source snapshot for line conversion.
    #[error("missing source snapshot for ranged finding in {path:?}")]
    MissingSnapshot { path: String },
    /// A finding for a tool with no planned run.
    #[error("finding references unknown tool {tool:?}")]
    UnknownTool { tool: String },
    /// A byte offset outside the snapshot or inside a character.
    #[error("byte offset {offset} is not a character boundary in {path:?}")]
    BadOffset { path: String, offset: u64 },
    /// A Bazel-reported test XML artifact that cannot be parsed.
    #[error("invalid Bazel test XML artifact: {detail}")]
    InvalidJunit { detail: String },
    /// A Bazel-reported combined tracefile that is not a syntactically
    /// complete LCOV document.
    #[error("invalid Bazel combined LCOV tracefile: {detail}")]
    InvalidLcov { detail: String },
}
