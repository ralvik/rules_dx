//! Standard-report planning and SARIF 2.1.0 projection (M07 WP2).
//!
//! Contract: `docs/cli/standard-reports.md` and
//! `docs/cli/commands/quality.md`. Lint and typecheck export normalized
//! findings as SARIF 2.1.0 through the shared `--report` contract; format
//! has no initial standard report. JUnit and LCOV belong to the future
//! `test` and `coverage` commands whose contract profiles derive from
//! Bazel-owned artifacts rather than normalized findings, so they are
//! out of scope here.
//!
//! Human text, diff, and NDJSON v1 projection reuses the `dx_output`
//! emitters directly; this module owns report planning (which fails
//! before Bazel execution) and the SARIF document rendering.

use std::collections::{BTreeMap, BTreeSet};

use crate::args::{Command, ReportRequest};
use crate::plan::spec;
use dx_output::{
    check_output_conflict, sort_diagnostics, DiagnosticEvent, OutputError, OutputMode, Severity,
};
use serde_json::{json, Value};

/// Standard-report format supported by the M07 quality commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StandardFormat {
    Sarif,
}

impl StandardFormat {
    /// Stable format name used in requests, events, and documents.
    pub fn name(self) -> &'static str {
        match self {
            StandardFormat::Sarif => "sarif",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "sarif" => Some(StandardFormat::Sarif),
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportError {
    /// A report was requested together with `--dry-run`.
    DryRunConflict,
    /// The format is not supported by the command.
    UnsupportedFormat {
        command: &'static str,
        format: String,
        supported: Vec<&'static str>,
    },
    /// The same format/destination pair was requested twice.
    DuplicateReport { format: String, destination: String },
    /// More than one standard report targets stdout.
    MultipleStdoutReports,
    /// A stdout report combined with `--output diff` or `--output json`.
    StdoutReportConflictsMode { mode: &'static str },
    /// A finding without the stable identity SARIF requires.
    InvalidFinding { detail: &'static str },
    /// A byte range without a path.
    RangeWithoutPath,
    /// A byte range whose start exceeds its end.
    InvertedRange,
    /// A ranged finding without its source snapshot for line conversion.
    MissingSnapshot { path: String },
    /// A finding for a tool with no planned run.
    UnknownTool { tool: String },
    /// A byte offset outside the snapshot or inside a character.
    BadOffset { path: String, offset: u64 },
}

impl std::fmt::Display for ReportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReportError::DryRunConflict => {
                write!(f, "--dry-run conflicts with every --report request")
            }
            ReportError::UnsupportedFormat {
                command,
                format,
                supported,
            } => {
                if supported.is_empty() {
                    write!(
                        f,
                        "unsupported report format {format:?} for {command}: no standard report exists"
                    )
                } else {
                    write!(
                        f,
                        "unsupported report format {format:?} for {command}: want {}",
                        supported.join("|")
                    )
                }
            }
            ReportError::DuplicateReport {
                format,
                destination,
            } => {
                write!(
                    f,
                    "duplicate report {format:?} for destination {destination:?}"
                )
            }
            ReportError::MultipleStdoutReports => {
                write!(f, "more than one standard report targets stdout")
            }
            ReportError::StdoutReportConflictsMode { mode } => {
                write!(
                    f,
                    "a stdout report conflicts with --output {mode}: use --output text or a file destination"
                )
            }
            ReportError::InvalidFinding { detail } => {
                write!(f, "invalid finding for SARIF export: {detail}")
            }
            ReportError::RangeWithoutPath => {
                write!(f, "a byte range without a path cannot be located")
            }
            ReportError::InvertedRange => {
                write!(f, "a byte range starts after its end")
            }
            ReportError::MissingSnapshot { path } => {
                write!(f, "missing source snapshot for ranged finding in {path:?}")
            }
            ReportError::UnknownTool { tool } => {
                write!(f, "finding references unknown tool {tool:?}")
            }
            ReportError::BadOffset { path, offset } => {
                write!(
                    f,
                    "byte offset {offset} is not a character boundary in {path:?}"
                )
            }
        }
    }
}

impl std::error::Error for ReportError {}

fn stdout_conflict(error: OutputError) -> ReportError {
    match error {
        OutputError::SecondStdoutReport => ReportError::MultipleStdoutReports,
        OutputError::ConflictingStdoutReport { mode } => {
            ReportError::StdoutReportConflictsMode { mode }
        }
        unexpected => panic!("unexpected output conflict: {unexpected:?}"), // LCOV_EXCL_LINE - reason: defense-in-depth; check_output_conflict only yields the two stdout variants handled above (covered in dx_output), so this arm is unreachable.
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

/// Converts a canonical UTF-8 byte offset into a 1-based
/// `(line, column)` pair over the same source snapshot. Columns count
/// Unicode scalar values from the line start. Offsets past the end of
/// the snapshot or inside a character fail rather than misreport a
/// scanner location.
pub fn byte_to_line(path: &str, text: &str, offset: u64) -> Result<(u64, u64), ReportError> {
    let offset = offset as usize;
    if offset > text.len() || !text.is_char_boundary(offset) {
        return Err(ReportError::BadOffset {
            path: path.to_owned(),
            offset: offset as u64,
        });
    }
    let prefix = &text[..offset];
    let line = prefix.as_bytes().iter().filter(|&&b| b == b'\n').count() as u64 + 1;
    let column = prefix.rsplit('\n').next().unwrap_or("").chars().count() as u64 + 1;
    Ok((line, column))
}

/// Validates the finding shape SARIF export requires, independent of
/// the tool registry: stable tool identity, a message, and a
/// well-formed located range.
fn check_shape(finding: &DiagnosticEvent) -> Result<(), ReportError> {
    if finding.tool.is_empty() {
        return Err(ReportError::InvalidFinding {
            detail: "empty tool",
        });
    }
    if finding.message.is_empty() {
        return Err(ReportError::InvalidFinding {
            detail: "empty message",
        });
    }
    if finding.path.is_none() && finding.range.is_some() {
        return Err(ReportError::RangeWithoutPath);
    }
    if let Some((start, end)) = finding.range {
        if start > end {
            return Err(ReportError::InvertedRange);
        }
    }
    Ok(())
}

fn sarif_level(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "note",
    }
}

fn location(
    finding: &DiagnosticEvent,
    snapshots: &BTreeMap<String, String>,
) -> Result<Option<Value>, ReportError> {
    let Some(path) = &finding.path else {
        if finding.range.is_some() {
            return Err(ReportError::RangeWithoutPath);
        }
        return Ok(None);
    };
    let mut physical = BTreeMap::from([("artifactLocation".to_owned(), json!({"uri": path}))]);
    if let Some((start, end)) = finding.range {
        if start > end {
            return Err(ReportError::InvertedRange);
        }
        let text = snapshots
            .get(path)
            .ok_or_else(|| ReportError::MissingSnapshot { path: path.clone() })?;
        let (start_line, start_column) = byte_to_line(path, text, start)?;
        let (end_line, end_column) = byte_to_line(path, text, end)?;
        physical.insert(
            "region".to_owned(),
            json!({
                "startLine": start_line,
                "startColumn": start_column,
                "endLine": end_line,
                "endColumn": end_column,
            }),
        );
    }
    Ok(Some(Value::Object(
        [(
            "physicalLocation".to_owned(),
            Value::Object(physical.into_iter().collect()),
        )]
        .into_iter()
        .collect(),
    )))
}

fn result(
    finding: &DiagnosticEvent,
    snapshots: &BTreeMap<String, String>,
) -> Result<Value, ReportError> {
    if finding.tool.is_empty() {
        return Err(ReportError::InvalidFinding {
            detail: "empty tool",
        });
    }
    if finding.message.is_empty() {
        return Err(ReportError::InvalidFinding {
            detail: "empty message",
        });
    }
    let mut map = BTreeMap::new();
    if let Some(rule) = &finding.rule {
        map.insert("ruleId".to_owned(), Value::String(rule.clone()));
    }
    map.insert(
        "level".to_owned(),
        Value::String(sarif_level(finding.severity).to_owned()),
    );
    map.insert("message".to_owned(), json!({"text": finding.message}));
    if let Some(location) = location(finding, snapshots)? {
        map.insert("locations".to_owned(), Value::Array(vec![location]));
    }
    Ok(Value::Object(map.into_iter().collect()))
}

/// Renders normalized current findings as a SARIF 2.1.0 document.
///
/// One deterministically ordered run per tool adapter carries stable
/// tool and rule IDs with workspace-relative artifact URIs; severity
/// maps to SARIF `note`, `warning`, and `error`, and line/column
/// regions derive from the same source snapshots as canonical byte
/// ranges. `findings` are the current results under the command policy
/// (check mode passes every initial diagnostic; default mutating mode
/// omits successfully fixed findings and retains remaining or
/// not-applied ones), so the renderer applies no fixability filter
/// itself. `tools` lists every executed adapter so runs with no
/// remaining findings stay present with an empty `results` array. A
/// partial collection (`complete=false`) records an unsuccessful
/// invocation in every run while retaining validated findings. V1
/// emits no `baselineState` and no `fixes`.
pub fn render_sarif(
    tools: &[String],
    findings: &[DiagnosticEvent],
    snapshots: &BTreeMap<String, String>,
    complete: bool,
) -> Result<String, ReportError> {
    let ordered: BTreeSet<&str> = tools.iter().map(String::as_str).collect();
    let mut working = findings.to_vec();
    sort_diagnostics(&mut working);
    let mut by_tool: BTreeMap<&str, Vec<&DiagnosticEvent>> = BTreeMap::new();
    for finding in &working {
        check_shape(finding)?;
        if !ordered.contains(finding.tool.as_str()) {
            return Err(ReportError::UnknownTool {
                tool: finding.tool.clone(),
            });
        }
        by_tool
            .entry(finding.tool.as_str())
            .or_default()
            .push(finding);
    }
    let mut runs = Vec::with_capacity(ordered.len());
    for tool in ordered {
        let tool_findings = by_tool.get(tool).cloned().unwrap_or_default();
        let rules: BTreeSet<&str> = tool_findings
            .iter()
            .filter_map(|finding| finding.rule.as_deref())
            .collect();
        let mut results = Vec::with_capacity(tool_findings.len());
        for finding in tool_findings {
            results.push(result(finding, snapshots)?);
        }
        let mut run = BTreeMap::from([
            (
                "tool".to_owned(),
                json!({
                    "driver": {
                        "name": tool,
                        "rules": rules.into_iter().map(|rule| json!({"id": rule})).collect::<Vec<_>>(),
                    },
                }),
            ),
            ("results".to_owned(), Value::Array(results)),
        ]);
        if !complete {
            run.insert(
                "invocations".to_owned(),
                json!([{"executionSuccessful": false}]),
            );
        }
        runs.push(Value::Object(run.into_iter().collect()));
    }
    let document = json!({
        "version": "2.1.0",
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "runs": runs,
    });
    Ok(document.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dx_output::Snapshot;

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

    fn finding(tool: &str, severity: Severity, path: Option<&str>) -> DiagnosticEvent {
        DiagnosticEvent {
            severity,
            tool: tool.to_owned(),
            message: format!("{tool} finding"),
            rule: Some(format!("{tool}/rule")),
            path: path.map(str::to_owned),
            range: None,
            snapshot: Snapshot::Terminal,
            fixable: false,
            resolution: None,
        }
    }

    fn ranged(path: &str, start: u64, end: u64) -> DiagnosticEvent {
        DiagnosticEvent {
            severity: Severity::Warning,
            tool: "lint-tool".to_owned(),
            message: "ranged finding".to_owned(),
            rule: Some("lint-tool/rule".to_owned()),
            path: Some(path.to_owned()),
            range: Some((start, end)),
            snapshot: Snapshot::Terminal,
            fixable: false,
            resolution: None,
        }
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
    fn byte_to_line_counts_lines_and_characters() {
        let text = "ab\nc\u{e9}d\nef";
        assert_eq!(byte_to_line("f", text, 0), Ok((1, 1)));
        assert_eq!(byte_to_line("f", text, 2), Ok((1, 3)));
        assert_eq!(byte_to_line("f", text, 3), Ok((2, 1)));
        assert_eq!(byte_to_line("f", text, 4), Ok((2, 2)));
        assert_eq!(byte_to_line("f", text, 7), Ok((2, 4)));
        assert_eq!(byte_to_line("f", text, 8), Ok((3, 1)));
        assert_eq!(byte_to_line("f", text, text.len() as u64), Ok((3, 3)));
    }

    #[test]
    fn byte_to_line_rejects_bad_offsets() {
        let text = "\u{e9}x";
        assert_eq!(
            byte_to_line("f", text, 1),
            Err(ReportError::BadOffset {
                path: "f".to_owned(),
                offset: 1,
            })
        );
        assert_eq!(
            byte_to_line("f", text, 99),
            Err(ReportError::BadOffset {
                path: "f".to_owned(),
                offset: 99,
            })
        );
    }

    #[test]
    fn sarif_projects_runs_rules_results_and_regions() {
        let snapshots = BTreeMap::from([("src/a.py".to_owned(), "x = 1\nxx = 2\n".to_owned())]);
        let tools = ["lint-tool".to_owned(), "other-tool".to_owned()];
        let findings = vec![
            finding("other-tool", Severity::Error, None),
            ranged("src/a.py", 0, 5),
            DiagnosticEvent {
                severity: Severity::Info,
                tool: "lint-tool".to_owned(),
                message: "note without rule".to_owned(),
                rule: None,
                path: Some("src/a.py".to_owned()),
                range: None,
                snapshot: Snapshot::Terminal,
                fixable: false,
                resolution: None,
            },
        ];
        let document: Value = serde_json::from_str(
            &render_sarif(&tools, &findings, &snapshots, true).expect("render"),
        )
        .expect("valid JSON");
        assert_eq!(document["version"], json!("2.1.0"));
        let runs = document["runs"].as_array().expect("runs");
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0]["tool"]["driver"]["name"], json!("lint-tool"));
        assert_eq!(runs[1]["tool"]["driver"]["name"], json!("other-tool"));
        let lint_results = runs[0]["results"].as_array().expect("results");
        assert_eq!(lint_results.len(), 2);
        assert_eq!(lint_results[0]["level"], json!("warning"));
        assert_eq!(
            lint_results[0]["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
            json!("src/a.py")
        );
        assert_eq!(
            lint_results[0]["locations"][0]["physicalLocation"]["region"],
            json!({"startLine": 1, "startColumn": 1, "endLine": 1, "endColumn": 6})
        );
        assert_eq!(lint_results[1]["level"], json!("note"));
        assert!(lint_results[1].get("ruleId").is_none());
        assert_eq!(
            lint_results[1]["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
            json!("src/a.py")
        );
        assert!(lint_results[1]["locations"][0]["physicalLocation"]
            .get("region")
            .is_none());
        let rules = runs[0]["tool"]["driver"]["rules"]
            .as_array()
            .expect("rules");
        assert_eq!(rules, &vec![json!({"id": "lint-tool/rule"})]);
        assert_eq!(runs[1]["results"][0]["level"], json!("error"));
        assert!(runs[1]["results"][0].get("locations").is_none());
        assert!(document.get("invocations").is_none());
        assert!(runs[0].get("invocations").is_none());
    }

    #[test]
    fn sarif_keeps_empty_runs_and_marks_partial_invocations() {
        let document: Value = serde_json::from_str(
            &render_sarif(&["lint-tool".to_owned()], &[], &BTreeMap::new(), false).expect("render"),
        )
        .expect("valid JSON");
        let runs = document["runs"].as_array().expect("runs");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0]["results"], json!([]));
        assert_eq!(
            runs[0]["invocations"],
            json!([{"executionSuccessful": false}])
        );
        assert_eq!(runs[0]["tool"]["driver"]["rules"], json!([]));
    }

    #[test]
    fn sarif_is_deterministic_over_finding_order() {
        let snapshots = BTreeMap::new();
        let tools = ["b-tool".to_owned(), "a-tool".to_owned()];
        let first = vec![
            finding("b-tool", Severity::Error, None),
            finding("a-tool", Severity::Info, None),
        ];
        let mut second = first.clone();
        second.reverse();
        assert_eq!(
            render_sarif(&tools, &first, &snapshots, true),
            render_sarif(&tools, &second, &snapshots, true)
        );
    }

    #[test]
    fn sarif_rejects_inconsistent_inputs() {
        let snapshots = BTreeMap::new();
        let tools = ["lint-tool".to_owned()];
        assert_eq!(
            render_sarif(
                &tools,
                &[finding("other-tool", Severity::Error, None)],
                &snapshots,
                true
            ),
            Err(ReportError::UnknownTool {
                tool: "other-tool".to_owned(),
            })
        );
        assert_eq!(
            render_sarif(&tools, &[ranged("src/a.py", 0, 1)], &snapshots, true),
            Err(ReportError::MissingSnapshot {
                path: "src/a.py".to_owned(),
            })
        );
        assert_eq!(
            render_sarif(
                &tools,
                &[DiagnosticEvent {
                    range: Some((1, 0)),
                    ..ranged("src/a.py", 1, 0)
                }],
                &BTreeMap::from([("src/a.py".to_owned(), "ab".to_owned())]),
                true,
            ),
            Err(ReportError::InvertedRange)
        );
        assert_eq!(
            render_sarif(
                &tools,
                &[DiagnosticEvent {
                    range: Some((0, 1)),
                    path: None,
                    ..finding("lint-tool", Severity::Error, None)
                }],
                &snapshots,
                true,
            ),
            Err(ReportError::RangeWithoutPath)
        );
        assert_eq!(
            render_sarif(
                &tools,
                &[DiagnosticEvent {
                    tool: String::new(),
                    ..finding("lint-tool", Severity::Error, None)
                }],
                &snapshots,
                true,
            ),
            Err(ReportError::InvalidFinding {
                detail: "empty tool"
            })
        );
        assert_eq!(
            render_sarif(
                &tools,
                &[DiagnosticEvent {
                    message: String::new(),
                    ..finding("lint-tool", Severity::Error, None)
                }],
                &snapshots,
                true,
            ),
            Err(ReportError::InvalidFinding {
                detail: "empty message"
            })
        );
    }

    #[test]
    fn shape_helpers_reject_locally() {
        let snapshots = BTreeMap::new();
        assert_eq!(
            location(
                &DiagnosticEvent {
                    range: Some((0, 1)),
                    path: None,
                    ..finding("lint-tool", Severity::Error, None)
                },
                &snapshots,
            ),
            Err(ReportError::RangeWithoutPath)
        );
        assert_eq!(
            location(&finding("lint-tool", Severity::Error, None), &snapshots),
            Ok(None)
        );
        assert_eq!(
            location(
                &DiagnosticEvent {
                    range: Some((1, 0)),
                    ..ranged("src/a.py", 1, 0)
                },
                &snapshots,
            ),
            Err(ReportError::InvertedRange)
        );
        assert_eq!(
            result(
                &DiagnosticEvent {
                    tool: String::new(),
                    ..finding("lint-tool", Severity::Error, None)
                },
                &snapshots,
            ),
            Err(ReportError::InvalidFinding {
                detail: "empty tool"
            })
        );
        assert_eq!(
            result(
                &DiagnosticEvent {
                    message: String::new(),
                    ..finding("lint-tool", Severity::Error, None)
                },
                &snapshots,
            ),
            Err(ReportError::InvalidFinding {
                detail: "empty message"
            })
        );
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
    }
}
