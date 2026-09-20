//! Implementation-coverage gate.
//!
//! Bazel-owned enforcement for the resolved coverage policy over the
//! eligible scope. The gate parses the combined LCOV report from
//! `bazel coverage --combined_report=lcov`, validates source-level exclusion
//! markers carrying nearby `reason:` comments, reconciles the
//! repository-owned inventory against Bazel-declared sources, and requires
//! exact 100% covered-over-eligible executable lines. Only `DA` records
//! define executable lines; blank and comment-only lines are not executable.
//! Missing reports, unowned or absent sources, and any uncovered
//! non-excluded executable line fail the gate. Percentages are informational
//! only and never decide the verdict.
//!
//! Marker recognition is textual per-extension comment syntax: `//` line
//! comments for C-like sources, `#` line comments for Python, Starlark,
//! TOML, shell, and YAML, and `<!-- ... -->` segments for Markdown and
//! HTML. A marker is honored only inside its language's comments and
//! outside string or char literals (byte-level scan honoring `"`/`'`
//! and backslash escapes). Markers inside block comments or raw strings
//! stay wont-fix out of scope (gate-owned: line-comment
//! textual scan only; no eligible source uses those shapes). The `reason:`
//! lookup itself is a textual per-line match on the marker line or the
//! line directly above it, and the reason text after the colon must be
//! non-empty.
//!
//! Domain split: combined-LCOV parsing (`FileHits`,
//! `parse_lcov`, `validate_lcov_report`) lives in the `parse` module,
//! source-level exclusion markers (`Ignores`, `is_ignored`, `find_ignores`)
//! live in the `ignores` module, gate evaluation (`FileVerdict`,
//! `GateVerdict`, `is_covered_language`, `evaluate`, `render`) lives in the
//! `verdict` module, repo inventory (`ELIGIBLE`, `SUPPORT`,
//! `parse_inventory`) lives in the `inventory` module, and the gate CLI
//! (`run`) lives in the `run` module. This facade keeps the shared error;
//! the public paths stay `dx_lcov::{FileHits, parse_lcov,
//! validate_lcov_report, Ignores, is_ignored, find_ignores, FileVerdict,
//! GateVerdict, is_covered_language, evaluate, render, ELIGIBLE, SUPPORT,
//! parse_inventory, run}` via the re-exports below.
//!
//! Dependency evaluation (parser adopted):
//! the gate needs the combined-LCOV `SF`/`DA` union via the `lcov` crate
//! plus per-extension comment-syntax marker scanning outside string literals
//! with the nearby `reason:` gate plus the repo inventory and the exact 100%
//! eligible verdict. `cargo-llvm-cov` is a coverage-tool binary rather than
//! a parser library, and the `lcov` crate provides none of the marker,
//! inventory, or verdict semantics, so each stays hand-rolled with owned
//! reasons; `parse_lcov`/`validate_lcov_report` keep the maximum-hits union,
//! ignore non-`DA` summaries, and preserve empty-report semantics over `lcov`
//! records.

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

pub mod ignores;
pub mod inventory;
pub mod parse;
pub mod run;
pub mod verdict;

pub use ignores::{find_ignores, is_ignored, Ignores};
pub use inventory::{parse_inventory, ELIGIBLE, SUPPORT};
pub use parse::{parse_lcov, validate_lcov_report, FileHits};
pub use run::run;
pub use verdict::{evaluate, is_covered_language, render, FileVerdict, GateVerdict};

/// Typed LCOV gate failure.
///
/// Every variant renders byte-identical to the historical `String` error
/// it replaces, so CLI operational diagnostics stay stable while callers
/// gain matchable structure instead of `format!` string plumbing.
/// Binary edges keep rendering via `Display` (`to_string()`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LcovError {
    /// `SF:` record carries no path.
    #[error("LCOV record with empty SF path")]
    EmptySfPath,
    /// `DA:` record appears before any `SF:` record.
    #[error("LCOV DA record outside any SF record: {line}")]
    DaOutsideSf { line: String },
    /// `DA:` line number is not a positive integer.
    #[error("malformed LCOV DA line number in {path}: {line}")]
    MalformedLineNumber { path: String, line: String },
    /// `DA:` hit count is not an integer.
    #[error("malformed LCOV DA hit count in {path}: {line}")]
    MalformedHitCount { path: String, line: String },
    /// `SF:` appears before the open section closes with `end_of_record`.
    #[error("LCOV SF record before end_of_record")]
    SfBeforeEndOfRecord,
    /// `end_of_record` appears outside any `SF:` section.
    #[error("LCOV end_of_record outside any SF record")]
    EndOfRecordOutsideSf,
    /// Report carries no `SF:` records.
    #[error("LCOV has no SF records")]
    NoSfRecords,
    /// Open `SF:` section never closes with `end_of_record`.
    #[error("LCOV SF record without end_of_record")]
    SfWithoutEndOfRecord,
    /// Exclusion directive lacks a nearby non-empty `reason:`.
    #[error("coverage ignore without nearby reason at {path}:{lineno}: {directive} requires a reason: comment on the same or previous line")]
    MissingReason {
        path: String,
        lineno: usize,
        directive: String,
    },
    /// `START` opens while another range is open.
    #[error("nested range START at {path}:{lineno}")]
    NestedStart { path: String, lineno: usize },
    /// `STOP` closes with no open range.
    #[error("range STOP without START at {path}:{lineno}")]
    StopWithoutStart { path: String, lineno: usize },
    /// Marker prefix spells no known directive.
    #[error("unrecognized coverage ignore directive at {path}:{lineno}")]
    UnrecognizedDirective { path: String, lineno: usize },
    /// `START` never closes.
    #[error("unclosed range START at {path}:{start}")]
    UnclosedStart { path: String, start: usize },
    /// Inventory line is not `<disposition> <path>`.
    #[error("malformed inventory line {lineno}: {raw:?}")]
    MalformedInventory { lineno: usize, raw: String },
    /// Injected file read failed; carries the reader's message verbatim
    /// so gate output stays byte-identical.
    #[error("{message}")]
    Io { message: String },
}

impl From<String> for LcovError {
    fn from(message: String) -> Self {
        Self::Io { message }
    }
}

#[cfg(test)]
mod facade_tests {
    use super::LcovError;

    #[test]
    fn string_conversion_preserves_message() {
        let error = LcovError::from("fixture io failure".to_string());
        assert_eq!(
            error,
            LcovError::Io {
                message: "fixture io failure".to_string()
            }
        );
        assert!(error.to_string().contains("fixture io failure"));
    }
}
