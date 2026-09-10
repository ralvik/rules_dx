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

use std::collections::{BTreeMap, BTreeSet};

use crate::args::{Command, ReportRequest};
use crate::plan::spec;
use dx_output::{
    check_output_conflict, sort_diagnostics, DiagnosticEvent, OutputError, OutputMode, Severity,
};
use serde_json::{json, Value};

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
    /// A Bazel-reported test XML artifact that cannot be parsed.
    InvalidJunit { detail: String },
    /// A Bazel-reported combined tracefile that is not a syntactically
    /// complete LCOV document.
    InvalidLcov { detail: String },
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
            ReportError::InvalidJunit { detail } => {
                write!(f, "invalid Bazel test XML artifact: {detail}")
            }
            ReportError::InvalidLcov { detail } => {
                write!(f, "invalid Bazel combined LCOV tracefile: {detail}")
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

/// One Bazel-reported test case normalized for JUnit rendering.
/// `shard` and `attempt` are zero-based indices derived from the 1-based
/// BEP `testResult` identity (`shard = bep_shard - 1`).
#[derive(Debug, Clone, PartialEq)]
pub struct JunitCase {
    pub name: String,
    pub classname: Option<String>,
    pub time: f64,
    pub failure: Option<JunitMessage>,
    pub error: Option<JunitMessage>,
    pub skipped: Option<JunitMessage>,
    pub system_out: Option<String>,
    pub system_err: Option<String>,
    pub shard: u32,
    pub attempt: u32,
}

/// Message plus body preserved from a Bazel-reported
/// `<failure>`, `<error>`, or `<skipped>` element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JunitMessage {
    pub message: Option<String>,
    pub text: String,
}

fn junit_error(detail: impl Into<String>) -> ReportError {
    ReportError::InvalidJunit {
        detail: detail.into(),
    }
}

fn xml_escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn format_junit_time(time: f64) -> String {
    if !time.is_finite() || time < 0.0 {
        return "0".to_owned();
    }
    format!("{time:.3}")
}

/// Display name for one case: the Bazel-provided name, plus a
/// zero-based `[shard=…,attempt=…]` suffix when either index is
/// nonzero. Ordering uses the original name plus indices, so an
/// existing identical display name stays disambiguated by indices
/// rather than encounter order.
fn junit_display_name(name: &str, shard: u32, attempt: u32) -> String {
    if shard == 0 && attempt == 0 {
        name.to_owned()
    } else {
        format!("{name} [shard={shard},attempt={attempt}]")
    }
}

fn junit_attr(
    element: &quick_xml::events::BytesStart<'_>,
    name: &[u8],
) -> Result<Option<String>, ReportError> {
    for attr in element.attributes() {
        let attr = attr.map_err(|e| junit_error(format!("malformed testcase attribute: {e}")))?;
        if attr.key.as_ref() == name {
            let value = attr
                .unescape_value()
                .map_err(|e| junit_error(format!("malformed testcase attribute: {e}")))?;
            return Ok(Some(value.into_owned()));
        }
    }
    Ok(None)
}

/// Parses one Bazel-reported `test.xml` artifact into normalized cases.
///
/// `shard` and `attempt` are the zero-based indices for every case in
/// `bytes` (derived from the BEP identity). Names, durations,
/// `<failure>`, `<error>`, `<skipped>`, `<system-out>`, and
/// `<system-err>` content are preserved after safe XML parsing; suite
/// structure is ignored because the caller groups by Bazel target
/// label. Malformed XML fails the whole artifact so the caller can
/// mark collection partial.
pub fn parse_test_xml(
    bytes: &[u8],
    shard: u32,
    attempt: u32,
) -> Result<Vec<JunitCase>, ReportError> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let text = std::str::from_utf8(bytes)
        .map_err(|e| junit_error(format!("test XML is not UTF-8: {e}")))?;
    // Reject documents with no element structure early so empty or
    // whitespace-only artifacts fail closed instead of yielding zero
    // cases that look like a passing suite.
    if !text.contains('<') {
        return Err(junit_error("test XML has no elements"));
    }
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(true);
    reader.config_mut().check_end_names = true;

    #[derive(Debug)]
    struct ActiveCase {
        name: String,
        classname: Option<String>,
        time: f64,
        failure: Option<JunitMessage>,
        error: Option<JunitMessage>,
        skipped: Option<JunitMessage>,
        system_out: Option<String>,
        system_err: Option<String>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ChildKind {
        Failure,
        Error,
        Skipped,
        SystemOut,
        SystemErr,
    }

    struct ActiveChild {
        kind: ChildKind,
        message: Option<String>,
        text: String,
    }

    let mut cases: Vec<JunitCase> = Vec::new();
    let mut active: Option<ActiveCase> = None;
    let mut child: Option<ActiveChild> = None;
    let mut depth: usize = 0;
    let mut buf = Vec::new();
    loop {
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| junit_error(format!("malformed test XML: {e}")))?;
        match event {
            Event::Eof => break,
            Event::Start(element) => {
                let tag = element.name();
                let tag = tag.as_ref();
                if active.is_none() && tag == b"testcase" {
                    let name = junit_attr(&element, b"name")?
                        .ok_or_else(|| junit_error("testcase without name"))?;
                    if name.is_empty() {
                        return Err(junit_error("testcase without name"));
                    }
                    let classname = junit_attr(&element, b"classname")?;
                    let time = match junit_attr(&element, b"time")? {
                        None => 0.0,
                        Some(raw) => raw.parse::<f64>().map_err(|_| {
                            junit_error(format!("malformed testcase time: {raw:?}"))
                        })?,
                    };
                    if !time.is_finite() || time < 0.0 {
                        return Err(junit_error("malformed testcase time"));
                    }
                    active = Some(ActiveCase {
                        name,
                        classname,
                        time,
                        failure: None,
                        error: None,
                        skipped: None,
                        system_out: None,
                        system_err: None,
                    });
                    depth = 1;
                } else if let Some(current) = active.as_mut() {
                    depth += 1;
                    if depth == 2 && child.is_none() {
                        let kind = if tag == b"failure" {
                            Some(ChildKind::Failure)
                        } else if tag == b"error" {
                            Some(ChildKind::Error)
                        } else if tag == b"skipped" {
                            Some(ChildKind::Skipped)
                        } else if tag == b"system-out" {
                            Some(ChildKind::SystemOut)
                        } else if tag == b"system-err" {
                            Some(ChildKind::SystemErr)
                        } else {
                            None
                        };
                        if let Some(kind) = kind {
                            let message = junit_attr(&element, b"message")?;
                            child = Some(ActiveChild {
                                kind,
                                message,
                                text: String::new(),
                            });
                            let _ = current;
                        }
                    }
                }
                buf.clear();
            }
            Event::Empty(element) => {
                let tag = element.name();
                let tag = tag.as_ref();
                if active.is_none() && tag == b"testcase" {
                    let name = junit_attr(&element, b"name")?
                        .ok_or_else(|| junit_error("testcase without name"))?;
                    if name.is_empty() {
                        return Err(junit_error("testcase without name"));
                    }
                    let classname = junit_attr(&element, b"classname")?;
                    let time = match junit_attr(&element, b"time")? {
                        None => 0.0,
                        Some(raw) => raw.parse::<f64>().map_err(|_| {
                            junit_error(format!("malformed testcase time: {raw:?}"))
                        })?,
                    };
                    if !time.is_finite() || time < 0.0 {
                        return Err(junit_error("malformed testcase time"));
                    }
                    cases.push(JunitCase {
                        name,
                        classname,
                        time,
                        failure: None,
                        error: None,
                        skipped: None,
                        system_out: None,
                        system_err: None,
                        shard,
                        attempt,
                    });
                } else if let Some(current) = active.as_mut() {
                    if depth == 1 {
                        if tag == b"failure" {
                            if current.failure.is_some() {
                                return Err(junit_error("duplicate failure element"));
                            }
                            current.failure = Some(JunitMessage {
                                message: junit_attr(&element, b"message")?,
                                text: String::new(),
                            });
                        } else if tag == b"error" {
                            if current.error.is_some() {
                                return Err(junit_error("duplicate error element"));
                            }
                            current.error = Some(JunitMessage {
                                message: junit_attr(&element, b"message")?,
                                text: String::new(),
                            });
                        } else if tag == b"skipped" {
                            if current.skipped.is_some() {
                                return Err(junit_error("duplicate skipped element"));
                            }
                            current.skipped = Some(JunitMessage {
                                message: junit_attr(&element, b"message")?,
                                text: String::new(),
                            });
                        } else if tag == b"system-out" {
                            if current.system_out.is_some() {
                                return Err(junit_error("duplicate system-out element"));
                            }
                            current.system_out = Some(String::new());
                        } else if tag == b"system-err" {
                            if current.system_err.is_some() {
                                return Err(junit_error("duplicate system-err element"));
                            }
                            current.system_err = Some(String::new());
                        }
                    } else if let Some(open) = child.as_mut() {
                        // Nested empty elements inside failure/error text
                        // contribute no text; the outer child still
                        // closes with its accumulated content.
                        let _ = open;
                    }
                } // LCOV_EXCL_LINE - reason: closing brace of a fully covered nesting level carries no executable region of its own
                buf.clear();
            }
            Event::Text(text) => {
                if let Some(open) = child.as_mut() {
                    let decoded = text
                        .unescape()
                        .map_err(|e| junit_error(format!("malformed test XML text: {e}")))?;
                    open.text.push_str(&decoded);
                }
                buf.clear();
            }
            Event::CData(text) => {
                if let Some(open) = child.as_mut() {
                    // defense-in-depth; parse_test_xml rejects non-UTF-8 documents up front.
                    let decoded = std::str::from_utf8(text.as_ref())
                        .map_err(|_| junit_error("test XML CDATA is not UTF-8"))?; // LCOV_EXCL_LINE - reason: CDATA slices of a valid UTF-8 document are always UTF-8, so this error never fires
                    open.text.push_str(decoded);
                }
                buf.clear();
            }
            Event::End(element) => {
                let tag = element.name();
                let tag = tag.as_ref();
                if let Some(open) = child.take() {
                    if depth == 2 {
                        let current = active
                            .as_mut()
                            .ok_or_else(|| junit_error("test XML child outside testcase"))?;
                        match open.kind {
                            ChildKind::Failure => {
                                if current.failure.is_some() {
                                    return Err(junit_error("duplicate failure element"));
                                }
                                current.failure = Some(JunitMessage {
                                    message: open.message,
                                    text: open.text,
                                });
                            }
                            ChildKind::Error => {
                                if current.error.is_some() {
                                    return Err(junit_error("duplicate error element"));
                                }
                                current.error = Some(JunitMessage {
                                    message: open.message,
                                    text: open.text,
                                });
                            }
                            ChildKind::Skipped => {
                                if current.skipped.is_some() {
                                    return Err(junit_error("duplicate skipped element"));
                                }
                                current.skipped = Some(JunitMessage {
                                    message: open.message,
                                    text: open.text,
                                });
                            }
                            ChildKind::SystemOut => {
                                if current.system_out.is_some() {
                                    return Err(junit_error("duplicate system-out element"));
                                }
                                current.system_out = Some(open.text);
                            }
                            ChildKind::SystemErr => {
                                if current.system_err.is_some() {
                                    return Err(junit_error("duplicate system-err element"));
                                }
                                current.system_err = Some(open.text);
                            }
                        }
                    } else {
                        // Closing a nested element inside child text:
                        // restore the child so the outer end closes it.
                        child = Some(open);
                    }
                    depth = depth.saturating_sub(1);
                    let _ = tag;
                } else if active.is_some() {
                    if depth == 0 {
                        return Err(junit_error("unbalanced test XML")); // LCOV_EXCL_LINE - reason: defense-in-depth; active testcase always sets depth to 1 so depth 0 with active is unreachable
                    }
                    depth -= 1;
                    if depth == 0 {
                        if tag != b"testcase" {
                            return Err(junit_error("unbalanced test XML")); // LCOV_EXCL_LINE - reason: defense-in-depth; quick-xml check_end_names rejects mismatched closes before this guard
                        }
                        let finished = active.take().expect("active case");
                        cases.push(JunitCase {
                            name: finished.name,
                            classname: finished.classname,
                            time: finished.time,
                            failure: finished.failure,
                            error: finished.error,
                            skipped: finished.skipped,
                            system_out: finished.system_out,
                            system_err: finished.system_err,
                            shard,
                            attempt,
                        });
                    }
                }
                buf.clear();
            }
            _ => {
                buf.clear();
            }
        }
    }
    if active.is_some() || child.is_some() {
        return Err(junit_error("truncated test XML"));
    }
    Ok(cases)
}

fn junit_message_element(kind: &str, message: &JunitMessage) -> String {
    let mut element = String::from("<");
    element.push_str(kind);
    if let Some(note) = &message.message {
        element.push_str(r#" message=""#);
        element.push_str(&xml_escape(note));
        element.push('"');
    }
    if message.text.is_empty() {
        element.push_str("/>");
    } else {
        element.push('>');
        element.push_str(&xml_escape(&message.text));
        element.push_str("</");
        element.push_str(kind);
        element.push('>');
    }
    element
}

/// Renders normalized Bazel test cases as one JUnit XML document.
///
/// `suites` groups parsed cases by Bazel target label; every case in
/// one group shares that label. Suites order bytewise by label; cases
/// order bytewise by original name, then shard, then attempt. Retries
/// and shards stay separate cases with zero-based suffixes. Root and
/// suite `tests`, `failures`, `errors`, `skipped`, and `time` counts
/// are recomputed from the normalized cases.
pub fn render_junit(suites: &[(String, Vec<JunitCase>)]) -> String {
    let mut ordered: Vec<(String, Vec<JunitCase>)> = suites.to_vec();
    for (_, cases) in &mut ordered {
        cases.sort_by(|a, b| {
            a.name
                .as_bytes()
                .cmp(b.name.as_bytes())
                .then(a.shard.cmp(&b.shard))
                .then(a.attempt.cmp(&b.attempt))
        });
    }
    ordered.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));

    let mut total_tests = 0u64;
    let mut total_failures = 0u64;
    let mut total_errors = 0u64;
    let mut total_skipped = 0u64;
    let mut total_time = 0.0f64;
    let mut body = String::new();
    for (label, cases) in &ordered {
        let mut failures = 0u64;
        let mut errors = 0u64;
        let mut skipped = 0u64;
        let mut time = 0.0f64;
        let mut rendered_cases = String::new();
        for case in cases {
            total_tests += 1;
            time += case.time;
            let mut element = String::from("    <testcase name=\"");
            element.push_str(&xml_escape(&junit_display_name(
                &case.name,
                case.shard,
                case.attempt,
            )));
            element.push('"');
            if let Some(classname) = &case.classname {
                element.push_str(" classname=\"");
                element.push_str(&xml_escape(classname));
                element.push('"');
            }
            element.push_str(" time=\"");
            element.push_str(&format_junit_time(case.time));
            element.push('"');
            let has_children = case.failure.is_some()
                || case.error.is_some()
                || case.skipped.is_some()
                || case.system_out.is_some()
                || case.system_err.is_some();
            if !has_children {
                element.push_str("/>\n");
                rendered_cases.push_str(&element);
                continue;
            }
            element.push_str(">\n");
            if let Some(failure) = &case.failure {
                failures += 1;
                total_failures += 1;
                element.push_str("      ");
                element.push_str(&junit_message_element("failure", failure));
                element.push('\n');
            }
            if let Some(error) = &case.error {
                errors += 1;
                total_errors += 1;
                element.push_str("      ");
                element.push_str(&junit_message_element("error", error));
                element.push('\n');
            }
            if let Some(skipped_case) = &case.skipped {
                skipped += 1;
                total_skipped += 1;
                element.push_str("      ");
                element.push_str(&junit_message_element("skipped", skipped_case));
                element.push('\n');
            }
            if let Some(out) = &case.system_out {
                element.push_str("      <system-out>");
                element.push_str(&xml_escape(out));
                element.push_str("</system-out>\n");
            }
            if let Some(err_text) = &case.system_err {
                element.push_str("      <system-err>");
                element.push_str(&xml_escape(err_text));
                element.push_str("</system-err>\n");
            }
            element.push_str("    </testcase>\n");
            rendered_cases.push_str(&element);
        }
        total_time += time;
        body.push_str(&format!(
            "  <testsuite name=\"{}\" tests=\"{}\" failures=\"{}\" errors=\"{}\" skipped=\"{}\" time=\"{}\">\n{}  </testsuite>\n",
            xml_escape(label),
            cases.len(),
            failures,
            errors,
            skipped,
            format_junit_time(time),
            rendered_cases,
        ));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<testsuites name=\"dx\" tests=\"{total_tests}\" failures=\"{total_failures}\" errors=\"{total_errors}\" skipped=\"{total_skipped}\" time=\"{}\">\n{body}</testsuites>\n",
        format_junit_time(total_time),
    )
}

/// Renders one partial-infrastructure suite for JUnit collection that
/// lost at least one `test.xml` artifact. The suite is named
/// `dx.infrastructure` with one error case named `incomplete_results`;
/// callers add it to the normalized suite list before
/// [`render_junit`] so aggregate counts include the error.
pub fn junit_infrastructure_case(detail: &str) -> (String, Vec<JunitCase>) {
    (
        "dx.infrastructure".to_owned(),
        vec![JunitCase {
            name: "incomplete_results".to_owned(),
            classname: Some("dx.infrastructure".to_owned()),
            time: 0.0,
            failure: None,
            error: Some(JunitMessage {
                message: Some("incomplete_results".to_owned()),
                text: detail.to_owned(),
            }),
            skipped: None,
            system_out: None,
            system_err: None,
            shard: 0,
            attempt: 0,
        }],
    )
}

/// Validates that `bytes` are a syntactically complete LCOV tracefile.
///
/// The exact bytes are preserved for the report; validation only
/// checks UTF-8, at least one `SF:` record with a non-empty path, no
/// `DA:` outside an `SF:` section, well-formed `DA:<line>,<hits>`
/// counters with `line >= 1`, and that every `SF:` section closes with
/// `end_of_record`. Unknown `FN`/`BRDA`/summary lines are ignored like
/// the M00 gate parser. Failures return [`ReportError::InvalidLcov`]
/// so callers emit no LCOV report.
pub fn validate_lcov(bytes: &[u8]) -> Result<(), ReportError> {
    let invalid = |detail: String| ReportError::InvalidLcov { detail };
    let text =
        std::str::from_utf8(bytes).map_err(|e| invalid(format!("LCOV is not UTF-8: {e}")))?;
    let mut sections = 0u64;
    let mut open = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(path) = line.strip_prefix("SF:") {
            if path.is_empty() {
                return Err(invalid("LCOV record with empty SF path".to_owned()));
            }
            if open {
                return Err(invalid("LCOV SF record before end_of_record".to_owned()));
            }
            open = true;
            sections += 1;
        } else if let Some(rest) = line.strip_prefix("DA:") {
            if !open {
                return Err(invalid(format!(
                    "LCOV DA record outside any SF record: {line}"
                )));
            }
            let mut parts = rest.split(',');
            let lineno: u32 = parts
                .next()
                .unwrap_or_default()
                .parse()
                .map_err(|_| invalid(format!("malformed LCOV DA line number: {line}")))?;
            if lineno == 0 {
                return Err(invalid(format!("malformed LCOV DA line number: {line}")));
            }
            parts
                .next()
                .unwrap_or_default()
                .parse::<u64>()
                .map_err(|_| invalid(format!("malformed LCOV DA hit count: {line}")))?;
        } else if line == "end_of_record" {
            if !open {
                return Err(invalid(
                    "LCOV end_of_record outside any SF record".to_owned(),
                ));
            }
            open = false;
        }
    }
    if sections == 0 {
        return Err(invalid("LCOV has no SF records".to_owned()));
    }
    if open {
        return Err(invalid("LCOV SF record without end_of_record".to_owned()));
    }
    Ok(())
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

    fn junit_case(name: &str) -> JunitCase {
        JunitCase {
            name: name.to_owned(),
            classname: None,
            time: 0.0,
            failure: None,
            error: None,
            skipped: None,
            system_out: None,
            system_err: None,
            shard: 0,
            attempt: 0,
        }
    }

    #[test]
    fn junit_helpers_cover_escapes_times_and_names() {
        assert_eq!(format_junit_time(f64::NAN), "0");
        assert_eq!(format_junit_time(-1.0), "0");
        assert_eq!(format_junit_time(f64::INFINITY), "0");
        assert_eq!(format_junit_time(1.23456), "1.235");
        assert_eq!(junit_display_name("a", 0, 0), "a");
        assert_eq!(junit_display_name("a", 1, 2), "a [shard=1,attempt=2]");
        let doc = render_junit(&[(
            "a&<b>\"'".to_owned(),
            vec![JunitCase {
                name: "n&<m>\"'".to_owned(),
                classname: Some("c&<>".to_owned()),
                time: 1.0,
                failure: Some(JunitMessage {
                    message: Some("msg&<>".to_owned()),
                    text: "body&<>".to_owned(),
                }),
                error: None,
                skipped: None,
                system_out: None,
                system_err: None,
                shard: 0,
                attempt: 0,
            }],
        )]);
        assert!(doc.contains("&amp;"), "{doc}");
        assert!(doc.contains("&lt;"), "{doc}");
        let empty_msg = junit_message_element(
            "failure",
            &JunitMessage {
                message: None,
                text: String::new(),
            },
        );
        assert_eq!(empty_msg, "<failure/>");
        let with_text = junit_message_element(
            "error",
            &JunitMessage {
                message: Some("m".to_owned()),
                text: "t".to_owned(),
            },
        );
        assert!(with_text.contains("message=\"m\""), "{with_text}");
        let (_label, cases) = junit_infrastructure_case("detail");
        assert_eq!(cases[0].name, "incomplete_results");
    }

    #[test]
    fn junit_render_covers_all_child_kinds() {
        let suites = vec![
            ("//b:t".to_owned(), vec![junit_case("b")]),
            (
                "//a:t".to_owned(),
                vec![
                    JunitCase {
                        failure: Some(JunitMessage {
                            message: Some("f".to_owned()),
                            text: "ft".to_owned(),
                        }),
                        ..junit_case("a1")
                    },
                    JunitCase {
                        error: Some(JunitMessage {
                            message: None,
                            text: String::new(),
                        }),
                        skipped: Some(JunitMessage {
                            message: None,
                            text: String::new(),
                        }),
                        system_out: Some("out".to_owned()),
                        system_err: Some("err".to_owned()),
                        shard: 1,
                        attempt: 2,
                        ..junit_case("a2")
                    },
                ],
            ),
        ];
        let doc = render_junit(&suites);
        assert!(doc.contains("name=\"//a:t\""), "{doc}");
        assert!(doc.contains("a2 [shard=1,attempt=2]"), "{doc}");
        assert!(doc.contains("<failure"), "{doc}");
        assert!(doc.contains("<error"), "{doc}");
        assert!(doc.contains("<skipped"), "{doc}");
        assert!(doc.contains("<system-out>out</system-out>"), "{doc}");
        assert!(doc.contains("<system-err>err</system-err>"), "{doc}");
        assert!(doc.starts_with("<?xml"), "{doc}");
    }

    #[test]
    fn junit_parse_covers_happy_and_error_paths() {
        // Happy: start/end testcase with children, empty testcase, decl/comment.
        let good = r#"<?xml version="1.0"?><!-- c --><testsuite><testcase name="a" classname="c" time="1.5"><failure message="m">text</failure></testcase><testcase name="b"/><testcase name="c" time="0"><error/><skipped/><system-out/><system-err/></testcase></testsuite>"#;
        let cases = parse_test_xml(good.as_bytes(), 0, 0).expect("good");
        assert_eq!(cases.len(), 3);
        // Start/end with system-out text and nested markup.
        let nested = r#"<testsuite><testcase name="a"><failure>text <b>bold</b> more<br/>tail</failure></testcase><testcase name="b"><error><![CDATA[blob]]></error></testcase></testsuite>"#;
        let cases = parse_test_xml(nested.as_bytes(), 0, 0).expect("nested");
        assert_eq!(cases.len(), 2);
        assert!(cases[0]
            .failure
            .as_ref()
            .expect("failure")
            .text
            .contains("text"));
        // Errors: non-utf8, no elements, malformed, missing/empty name, bad time.
        assert!(parse_test_xml(&[0xff], 0, 0).is_err());
        assert!(parse_test_xml(b"hello", 0, 0).is_err());
        assert!(parse_test_xml(b"<testcase", 0, 0).is_err());
        assert!(parse_test_xml(b"<testsuite><testcase/></testsuite>", 0, 0).is_err());
        assert!(parse_test_xml(b"<testsuite><testcase name=\"\"/></testsuite>", 0, 0).is_err());
        assert!(parse_test_xml(
            b"<testsuite><testcase name=\"a\" time=\"bogus\"/></testsuite>",
            0,
            0
        )
        .is_err());
        assert!(parse_test_xml(
            b"<testsuite><testcase name=\"a\" time=\"-1\"/></testsuite>",
            0,
            0
        )
        .is_err());
        assert!(parse_test_xml(
            b"<testsuite><testcase name=\"a\" time=\"inf\"/></testsuite>",
            0,
            0
        )
        .is_err());
        // Duplicate children fail.
        assert!(parse_test_xml(
            b"<testsuite><testcase name=\"a\"><failure/><failure/></testcase></testsuite>",
            0,
            0
        )
        .is_err());
        assert!(parse_test_xml(
            b"<testsuite><testcase name=\"a\"><error/><error/></testcase></testsuite>",
            0,
            0
        )
        .is_err());
        // Unbalanced and truncated fail.
        assert!(parse_test_xml(
            b"<testsuite><testcase name=\"a\"></foo></testcase></testsuite>",
            0,
            0
        )
        .is_err());
        assert!(parse_test_xml(b"<testsuite><testcase name=\"a\">", 0, 0).is_err());
        // Malformed attribute fails; well-formed text succeeds.
        assert!(parse_test_xml(
            b"<testsuite><testcase name=\"a\"><failure message=\"\xff\"/></testcase></testsuite>",
            0,
            0
        )
        .is_err());
        assert!(
            parse_test_xml(
                b"<testsuite><testcase name=\"a\"><failure message=\"m\">ok</failure></testcase></testsuite>",
                0, 0
            )
            .is_ok()
        );
    }

    #[test]
    fn junit_parse_covers_start_and_end_branches() {
        // Start testcase empty name and bad times (Empty variants are in the
        // happy/error test; these hit the Start arms).
        assert!(parse_test_xml(
            b"<testsuite><testcase name=\"\"></testcase></testsuite>",
            0,
            0
        )
        .is_err());
        for bad in ["bogus", "-1", "inf", "NaN"] {
            let xml =
                format!("<testsuite><testcase name=\"a\" time=\"{bad}\"></testcase></testsuite>");
            assert!(parse_test_xml(xml.as_bytes(), 0, 0).is_err(), "{bad}");
        }
        // Start children for every kind plus unknown tags (unknown yields no
        // child but still closes cleanly).
        for child in [
            "<failure></failure>",
            "<error></error>",
            "<skipped></skipped>",
            "<system-out>out</system-out>",
            "<system-err>err</system-err>",
            "<foo></foo>",
            "<foo/>",
        ] {
            let xml = format!("<testsuite><testcase name=\"a\">{child}</testcase></testsuite>");
            assert!(parse_test_xml(xml.as_bytes(), 0, 0).is_ok(), "{child}");
        }
        // Empty duplicates for skipped/system-out/system-err.
        for dup in [
            "<skipped/><skipped/>",
            "<system-out/><system-out/>",
            "<system-err/><system-err/>",
        ] {
            let xml = format!("<testsuite><testcase name=\"a\">{dup}</testcase></testsuite>");
            assert!(parse_test_xml(xml.as_bytes(), 0, 0).is_err(), "{dup}");
        }
        // Text and CDATA outside any child cover the no-child arms.
        assert!(parse_test_xml(
            b"<testsuite><testcase name=\"a\">hello</testcase></testsuite>",
            0,
            0
        )
        .is_ok());
        assert!(parse_test_xml(
            b"<testsuite><testcase name=\"a\"><![CDATA[hello]]></testcase></testsuite>",
            0,
            0
        )
        .is_ok());
        // Bad text entity fails unescape.
        assert!(parse_test_xml(
            b"<testsuite><testcase name=\"a\"><failure>&notanentity</failure></testcase></testsuite>",
            0,
            0
        )
        .is_err());
        // End duplicates for every kind via Start/End pairs.
        for dup in [
            "<failure></failure><failure></failure>",
            "<error></error><error></error>",
            "<skipped></skipped><skipped></skipped>",
            "<system-out>a</system-out><system-out>b</system-out>",
            "<system-err>a</system-err><system-err>b</system-err>",
        ] {
            let xml = format!("<testsuite><testcase name=\"a\">{dup}</testcase></testsuite>");
            assert!(parse_test_xml(xml.as_bytes(), 0, 0).is_err(), "{dup}");
        }
        // Children outside testcase are ignored; unbalanced closes fail.
        assert!(parse_test_xml(
            b"<testsuite><failure></failure><testcase name=\"a\"/></testsuite>",
            0,
            0
        )
        .is_ok());
        assert!(parse_test_xml(
            b"<testsuite><testcase name=\"a\"></testcase></testsuite>",
            0,
            0
        )
        .is_ok());
        assert!(parse_test_xml(b"<testsuite><testcase name=\"a\"></testsuite>", 0, 0).is_err());
    }

    #[test]
    fn lcov_validation_covers_all_branches() {
        let good = "SF:/a.rs\nDA:1,1\nend_of_record\n";
        assert_eq!(validate_lcov(good.as_bytes()), Ok(()));
        assert!(validate_lcov(&[0xff]).is_err());
        assert!(validate_lcov(b"DA:1,1\nend_of_record\n").is_err());
        assert!(validate_lcov(b"SF:\nDA:1,1\nend_of_record\n").is_err());
        assert!(validate_lcov(b"SF:/a.rs\nSF:/b.rs\n").is_err());
        assert!(validate_lcov(b"SF:/a.rs\nDA:bad,1\nend_of_record\n").is_err());
        assert!(validate_lcov(b"SF:/a.rs\nDA:0,1\nend_of_record\n").is_err());
        assert!(validate_lcov(b"SF:/a.rs\nDA:1,bad\nend_of_record\n").is_err());
        assert!(validate_lcov(b"end_of_record\n").is_err());
        assert!(validate_lcov(b"SF:/a.rs\nDA:1,1\n").is_err());
        assert!(validate_lcov(b"").is_err());
        // Blank lines and unknown FN/BRDA lines are ignored.
        let with_noise = "\nSF:/a.rs\nFN:1,fn\nBRDA:1,0,0,1\nDA:1,1\nend_of_record\n";
        assert_eq!(validate_lcov(with_noise.as_bytes()), Ok(()));
    }
}
