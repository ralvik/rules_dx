//! Standard-report planning and SARIF/JUnit/LCOV projection (M07 WP2, M08 WP3).
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
//! Domain split (issue #236): report rendering lives in domain
//! submodules — SARIF in [`sarif`](self::sarif), JUnit in
//! [`junit`](self::junit), LCOV in [`lcov`](self::lcov). This facade
//! keeps planning plus the shared types; the public path stays
//! `crate::reports::{...}` via the re-exports below.

pub mod junit;
pub mod lcov;
pub mod sarif;

pub use junit::{junit_infrastructure_case, parse_test_xml, render_junit, JunitCase, JunitMessage};
pub use lcov::{coverage_line_rate, validate_lcov};
pub use sarif::{byte_to_line, render_sarif};

use std::collections::BTreeSet;

use crate::args::{Command, ReportRequest};
use crate::plan::spec;
use dx_output::{check_output_conflict, OutputError, OutputMode};

/// Standard-report format supported by quality and workflow commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StandardFormat {
    Sarif,
    Junit,
    Lcov,
}

impl StandardFormat {
    /// Stable format name used in requests, events, and documents.
    pub fn name(self) -> &'static str {
        match self {
            StandardFormat::Sarif => "sarif",
            StandardFormat::Junit => "junit",
            StandardFormat::Lcov => "lcov",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "sarif" => Some(StandardFormat::Sarif),
            "junit" => Some(StandardFormat::Junit),
            "lcov" => Some(StandardFormat::Lcov),
            _ => None,
        }
    }
}

/// Report destination: `-` streams the exclusive stdout document,
/// anything else is a workspace-relative or absolute file path whose
/// parent directory must already exist at emission time.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Destination {
    Stdout,
    File(String),
}

impl Destination {
    /// Normalized destination identity: `-` for stdout, else the path.
    /// File report events order bytewise by this identity.
    pub fn display(&self) -> &str {
        match self {
            Destination::Stdout => "-",
            Destination::File(path) => path,
        }
    }
}

/// One validated report request ready for emission after collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedReport {
    pub format: StandardFormat,
    pub destination: Destination,
}

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

fn stdout_conflict(error: OutputError) -> ReportError {
    match error {
        OutputError::SecondStdoutReport => ReportError::MultipleStdoutReports,
        OutputError::ConflictingStdoutReport { mode } => {
            ReportError::StdoutReportConflictsMode { mode }
        }
        unexpected => unreachable!("unexpected output conflict: {unexpected:?}"), // LCOV_EXCL_LINE - reason: defense-in-depth; check_output_conflict only yields the two stdout variants handled above (covered in dx_output), so this arm is unreachable.
    }
}

/// Validates report requests against the command registry and the live
/// output mode. Fails before Bazel execution on dry-run conflicts,
/// unsupported formats, duplicate pairs, and stdout ownership
/// conflicts. Returns plans ordered bytewise by format and normalized
/// destination, independent of option order.
pub fn plan_reports(
    command: Command,
    requests: &[ReportRequest],
    mode: &OutputMode,
    dry_run: bool,
) -> Result<Vec<PlannedReport>, ReportError> {
    if dry_run && !requests.is_empty() {
        return Err(ReportError::DryRunConflict);
    }
    let entry = spec(command);
    let mut planned = Vec::with_capacity(requests.len());
    let mut seen = BTreeSet::new();
    for request in requests {
        let format = StandardFormat::parse(&request.format).ok_or_else(|| {
            ReportError::UnsupportedFormat {
                command: command.name(),
                format: request.format.clone(),
                supported: entry.reports.to_vec(),
            }
        })?;
        if !entry.reports.contains(&format.name()) {
            return Err(ReportError::UnsupportedFormat {
                command: command.name(),
                format: request.format.clone(),
                supported: entry.reports.to_vec(),
            });
        }
        let destination = if request.destination == "-" {
            Destination::Stdout
        } else {
            Destination::File(request.destination.clone())
        };
        if !seen.insert((format, destination.clone())) {
            return Err(ReportError::DuplicateReport {
                format: format.name().to_owned(),
                destination: destination.display().to_owned(),
            });
        }
        planned.push(PlannedReport {
            format,
            destination,
        });
    }
    let stdout_reports = planned
        .iter()
        .filter(|report| report.destination == Destination::Stdout)
        .count();
    check_output_conflict(mode, stdout_reports).map_err(stdout_conflict)?;
    planned.sort_by(|a, b| {
        (a.format, a.destination.display()).cmp(&(b.format, b.destination.display()))
    });
    Ok(planned)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn requests(pairs: &[(&str, &str)]) -> Vec<ReportRequest> {
        pairs
            .iter()
            .map(|(format, destination)| ReportRequest {
                format: (*format).to_owned(),
                destination: (*destination).to_owned(),
            })
            .collect()
    }

    fn text_mode() -> OutputMode {
        OutputMode::Text { quiet: false }
    }

    #[test]
    fn file_reports_plan_in_destination_order() {
        let got = plan_reports(
            Command::Lint,
            &requests(&[("sarif", "b.sarif"), ("sarif", "a.sarif")]),
            &text_mode(),
            false,
        )
        .expect("plan");
        assert_eq!(
            got.iter()
                .map(|report| report.destination.display().to_owned())
                .collect::<Vec<_>>(),
            vec!["a.sarif".to_owned(), "b.sarif".to_owned()]
        );
    }

    #[test]
    fn stdout_report_plans_with_text_mode() {
        let got = plan_reports(
            Command::Typecheck,
            &requests(&[("sarif", "-")]),
            &text_mode(),
            false,
        )
        .expect("plan");
        assert_eq!(
            got,
            vec![PlannedReport {
                format: StandardFormat::Sarif,
                destination: Destination::Stdout,
            }]
        );
    }

    #[test]
    fn stdout_report_conflicts_with_diff_and_json_modes() {
        for mode in [OutputMode::Diff, OutputMode::Json] {
            assert_eq!(
                plan_reports(Command::Lint, &requests(&[("sarif", "-")]), &mode, false),
                Err(ReportError::StdoutReportConflictsMode { mode: mode.name() })
            );
        }
        assert_eq!(
            plan_reports(
                Command::Lint,
                &requests(&[("sarif", "-"), ("sarif", "second.sarif".into())]),
                &text_mode(),
                false,
            ),
            Ok(vec![
                PlannedReport {
                    format: StandardFormat::Sarif,
                    destination: Destination::Stdout,
                },
                PlannedReport {
                    format: StandardFormat::Sarif,
                    destination: Destination::File("second.sarif".to_owned()),
                },
            ])
        );
    }

    #[test]
    fn planning_rejects_duplicates_and_second_stdout() {
        assert_eq!(
            plan_reports(
                Command::Lint,
                &requests(&[("sarif", "a.sarif"), ("sarif", "a.sarif")]),
                &text_mode(),
                false,
            ),
            Err(ReportError::DuplicateReport {
                format: "sarif".to_owned(),
                destination: "a.sarif".to_owned(),
            })
        );
        assert_eq!(
            plan_reports(
                Command::Lint,
                &requests(&[("sarif", "-"), ("sarif", "-")]),
                &text_mode(),
                false,
            ),
            Err(ReportError::DuplicateReport {
                format: "sarif".to_owned(),
                destination: "-".to_owned(),
            })
        );
    }

    #[test]
    fn planning_rejects_dry_run_and_unsupported_formats() {
        assert_eq!(
            plan_reports(
                Command::Lint,
                &requests(&[("sarif", "a.sarif")]),
                &text_mode(),
                true,
            ),
            Err(ReportError::DryRunConflict)
        );
        assert_eq!(
            plan_reports(
                Command::Lint,
                &requests(&[("junit", "a.xml")]),
                &text_mode(),
                false,
            ),
            Err(ReportError::UnsupportedFormat {
                command: "lint",
                format: "junit".to_owned(),
                supported: vec!["sarif"],
            })
        );
        assert_eq!(
            plan_reports(
                Command::Format,
                &requests(&[("sarif", "a.sarif")]),
                &text_mode(),
                false,
            ),
            Err(ReportError::UnsupportedFormat {
                command: "format",
                format: "sarif".to_owned(),
                supported: vec![],
            })
        );
        assert!(plan_reports(Command::Format, &[], &text_mode(), false)
            .expect("plan")
            .is_empty());
    }

    #[test]
    fn error_display_reports_variant() {
        assert_eq!(
            stdout_conflict(OutputError::SecondStdoutReport),
            ReportError::MultipleStdoutReports
        );
        assert!(format!("{}", ReportError::DryRunConflict).contains("dry-run"));
        assert!(format!(
            "{}",
            ReportError::UnsupportedFormat {
                command: "lint",
                format: "junit".to_owned(),
                supported: vec![],
            }
        )
        .contains("no standard report exists"));
        assert!(format!(
            "{}",
            ReportError::UnsupportedFormat {
                command: "lint",
                format: "junit".to_owned(),
                supported: vec!["sarif"],
            }
        )
        .contains("sarif"));
        assert!(format!(
            "{}",
            ReportError::DuplicateReport {
                format: "sarif".to_owned(),
                destination: "out.sarif".to_owned(),
            }
        )
        .contains("duplicate"));
        assert!(format!("{}", ReportError::MultipleStdoutReports).contains("stdout"));
        assert!(format!(
            "{}",
            ReportError::StdoutReportConflictsMode { mode: "diff" }
        )
        .contains("diff"));
        assert!(format!(
            "{}",
            ReportError::InvalidFinding {
                detail: "empty tool",
            }
        )
        .contains("empty tool"));
        assert!(format!("{}", ReportError::RangeWithoutPath).contains("byte range"));
        assert!(format!("{}", ReportError::InvertedRange).contains("starts after"));
        assert!(format!(
            "{}",
            ReportError::MissingSnapshot {
                path: "src/a.py".to_owned(),
            }
        )
        .contains("src/a.py"));
        assert!(format!(
            "{}",
            ReportError::UnknownTool {
                tool: "other".to_owned(),
            }
        )
        .contains("other"));
        assert!(format!(
            "{}",
            ReportError::BadOffset {
                path: "src/a.py".to_owned(),
                offset: 1,
            }
        )
        .contains("character boundary"));
        assert!(format!(
            "{}",
            ReportError::InvalidJunit {
                detail: "boom".to_owned(),
            }
        )
        .contains("boom"));
        assert!(format!(
            "{}",
            ReportError::InvalidLcov {
                detail: "boom".to_owned(),
            }
        )
        .contains("boom"));
    }

    #[test]
    fn unknown_format_is_unsupported_before_registry() {
        let err = plan_reports(
            Command::Lint,
            &requests(&[("bogus", "a.xml")]),
            &text_mode(),
            false,
        )
        .expect_err("bogus");
        assert_eq!(
            err,
            ReportError::UnsupportedFormat {
                command: "lint",
                format: "bogus".to_owned(),
                supported: vec!["sarif"],
            }
        );
    }
}
