//! Per-tool output grammars for the pinned M04 binaries.
//!
//! Each parser maps one tool's check output onto [`FileFinding`] values
//! addressed by the scratch-absolute path the tool reported; the caller
//! re-roots that path onto the workspace path before placement. Parsers
//! never spawn processes and never invent positions: anything outside the
//! pinned grammar is a [`ParseError`], which the runner surfaces as an
//! action failure.
//!
//! Pinned shapes (probed against the M04 binaries):
//!
//! * Buildifier `--mode=check --format=json --lint=warn`: stdout JSON
//!   `{success, files:[{filename, formatted, valid, warnings:[...]}]}`.
//!   `success` is false whenever findings exist and the exit code is
//!   always 0, so both are ignored: the file entries decide. A
//!   `valid:false` entry is a syntax error positioned from the
//!   `<path>:<line>:<col>` stderr line; `formatted:false` is one format
//!   finding; each warning is one finding under its category.
//! * rustfmt `--check`: stdout `Diff in <path>:<line>:` headers (one per
//!   file, first-differing line) plus stderr `error` / ` --> <path>:...`
//!   blocks for syntax errors. A nonzero exit with neither is a grammar
//!   mismatch, never a clean result.
//! * Taplo `lint`: stderr `error:` / `  ┌─ <path>:<line>:<col>` blocks
//!   with `^` caret detail lines. A nonzero exit with no parsed block is
//!   a grammar mismatch (version-qualified fail-closed text parsing).
//! * Taplo `format --check`: the same syntax blocks plus
//!   `the file is not properly formatted path="<path>"` lines, one
//!   format finding each.
//! * Vale `--output=JSON`: stdout object mapping each checked path to its
//!   alert list (`Span:[start,end]` columns are inclusive on both ends,
//!   so placement uses `end + 1`). An envelope object with a `Code` key
//!   (`E100`/`E201`, exit 2) is [`ParseError::ValeConfig`], never a
//!   finding.
//! * Markdown checker (`//quality/markdown:quality_markdown`): stdout is
//!   newline-delimited JSON, one `{"path","line","kind","message"}` object
//!   per finding and nothing at all when clean. `path` is the workspace
//!   key from the `--source` mapping (the caller re-roots it onto the
//!   scratch-absolute path before placement), `line` is one-based, `kind`
//!   is one of the frozen kebab-case finding kinds. Findings exist only on
//!   exit 0: any other exit is a [`ParseError::Shape`], never a partial
//!   result.
//! * Clippy `--error-format=json --emit=metadata`: stderr JSON
//!   diagnostics, one per line. Span-less summaries (`N warnings
//!   emitted`, `aborting due to ...`) are skipped; every other
//!   diagnostic needs a primary span in a checked file. `level`
//!   maps warning/error; other levels with placed spans are a grammar
//!   mismatch. `MachineApplicable` child spans become byte [`Suggestion`]
//!   values; Clippy columns are half-open `[start, end)`, Vale-adjusted
//!   spans aside, every tool column here counts Unicode scalar values.

use serde::Deserialize;

use crate::{Finding, Suggestion, TextPosition, ToolSeverity};

/// One parsed finding, still addressed by the scratch-absolute path the
/// tool reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFinding {
    pub file: String,
    pub finding: Finding,
}

/// Grammar or attribution failure. Every variant is an action failure,
/// never a skipped finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// Tool stdout is not the pinned JSON grammar.
    Json { tool: &'static str, detail: String },
    /// Check output matches neither findings nor clean for the pinned
    /// tool, or a placed span/level is outside the pinned grammar.
    Shape { tool: &'static str, detail: String },
    /// A reported path is not one of the checked scratch files.
    UnknownFile { tool: &'static str, path: String },
    /// Vale refused without a usable config (E100/E201 envelope).
    ValeConfig { detail: String },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Json { tool, detail } => {
                write!(f, "{tool} output is not the pinned JSON grammar: {detail}")
            }
            ParseError::Shape { tool, detail } => {
                write!(f, "{tool} output is outside the pinned grammar: {detail}")
            }
            ParseError::UnknownFile { tool, path } => {
                write!(f, "{tool} reported an unchecked file: {path}")
            }
            ParseError::ValeConfig { detail } => {
                write!(f, "vale needs a usable config: {detail}")
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// Resolves a tool-reported path against the checked scratch files.
fn known<'a>(tool: &'static str, files: &[&'a str], path: &str) -> Result<&'a str, ParseError> {
    files
        .iter()
        .find(|file| **file == path)
        .copied()
        .ok_or_else(|| ParseError::UnknownFile {
            tool,
            path: path.to_owned(),
        })
}

fn point(line: u64, column: u64) -> (TextPosition, Option<TextPosition>) {
    (TextPosition { line, column }, None)
}

// ---------------------------------------------------------------------------
// Buildifier
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct BuildifierReport {
    files: Vec<BuildifierFile>,
}

#[derive(Debug, Deserialize)]
struct BuildifierFile {
    filename: String,
    formatted: bool,
    valid: bool,
    warnings: Vec<BuildifierWarning>,
}

#[derive(Debug, Deserialize)]
struct BuildifierWarning {
    start: BuildifierPosition,
    end: BuildifierPosition,
    category: String,
    message: String,
}

#[derive(Debug, Deserialize)]
struct BuildifierPosition {
    line: u64,
    column: u64,
}

/// Parses Buildifier check stdout. The exit code is always 0 and
/// `success` is false whenever findings exist, so both are ignored.
pub fn parse_buildifier(
    stdout: &[u8],
    stderr: &[u8],
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "buildifier";
    let text = std::str::from_utf8(stdout).map_err(|err| ParseError::Json {
        tool: TOOL,
        detail: err.to_string(),
    })?;
    let report: BuildifierReport = serde_json::from_str(text).map_err(|err| ParseError::Json {
        tool: TOOL,
        detail: err.to_string(),
    })?;
    let stderr_text = String::from_utf8_lossy(stderr);
    let mut findings = Vec::new();
    for file in &report.files {
        let checked = known(TOOL, files, &file.filename)?;
        if !file.valid {
            let (line, column, message) = buildifier_syntax(&stderr_text, &file.filename);
            let (start, end) = point(line, column);
            findings.push(FileFinding {
                file: checked.to_owned(),
                finding: Finding {
                    tool_id: TOOL.to_owned(),
                    rule_id: String::new(),
                    message,
                    severity: ToolSeverity::Error,
                    start,
                    end,
                    suggestions: Vec::new(),
                },
            });
            continue;
        }
        if !file.formatted {
            let (start, end) = point(1, 1);
            findings.push(FileFinding {
                file: checked.to_owned(),
                finding: Finding {
                    tool_id: TOOL.to_owned(),
                    rule_id: String::new(),
                    message: "file is not formatted".to_owned(),
                    severity: ToolSeverity::Warning,
                    start,
                    end,
                    suggestions: Vec::new(),
                },
            });
        }
        for warning in &file.warnings {
            findings.push(FileFinding {
                file: checked.to_owned(),
                finding: Finding {
                    tool_id: TOOL.to_owned(),
                    rule_id: warning.category.clone(),
                    message: warning.message.clone(),
                    severity: ToolSeverity::Warning,
                    start: TextPosition {
                        line: warning.start.line,
                        column: warning.start.column,
                    },
                    end: Some(TextPosition {
                        line: warning.end.line,
                        column: warning.end.column,
                    }),
                    suggestions: Vec::new(),
                },
            });
        }
    }
    Ok(findings)
}

/// Positions a `valid:false` file from its `<path>:<line>:<col>: <msg>`
/// stderr line, falling back to 1:1 when the tool names no position.
fn buildifier_syntax(stderr: &str, filename: &str) -> (u64, u64, String) {
    let prefix = format!("{filename}:");
    for line in stderr.lines() {
        if let Some(rest) = line.strip_prefix(&prefix) {
            let mut parts = rest.splitn(3, ':');
            if let (Some(line_text), Some(column_text), Some(message)) =
                (parts.next(), parts.next(), parts.next())
            {
                if let (Ok(line), Ok(column)) =
                    (line_text.parse::<u64>(), column_text.parse::<u64>())
                {
                    if line >= 1 && column >= 1 {
                        let message = message.trim().to_owned();
                        let message = if message.is_empty() {
                            "syntax error".to_owned()
                        } else {
                            message
                        };
                        return (line, column, message);
                    }
                }
            }
        }
    }
    (1, 1, "syntax error".to_owned())
}

// ---------------------------------------------------------------------------
// rustfmt
// ---------------------------------------------------------------------------

/// Parses rustfmt `--check` output. `code` is the process exit status:
/// nonzero without any parsed finding is a grammar mismatch.
pub fn parse_rustfmt(
    stdout: &[u8],
    stderr: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "rustfmt";
    let stdout_text = std::str::from_utf8(stdout).map_err(|err| ParseError::Shape {
        tool: TOOL,
        detail: err.to_string(),
    })?;
    let stderr_text = std::str::from_utf8(stderr).map_err(|err| ParseError::Shape {
        tool: TOOL,
        detail: err.to_string(),
    })?;
    let mut findings = Vec::new();
    for line in stdout_text.lines() {
        if let Some(header) = line.strip_prefix("Diff in ") {
            let (path, line) = rustfmt_header(header)?;
            let checked = known(TOOL, files, path)?;
            let (start, end) = point(line, 1);
            findings.push(FileFinding {
                file: checked.to_owned(),
                finding: Finding {
                    tool_id: TOOL.to_owned(),
                    rule_id: String::new(),
                    message: "file is not formatted".to_owned(),
                    severity: ToolSeverity::Warning,
                    start,
                    end,
                    suggestions: Vec::new(),
                },
            });
        }
    }
    let mut pending: Option<String> = None;
    for line in stderr_text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("error") {
            pending = Some(trimmed.to_owned());
            continue;
        }
        if let Some(arrow) = trimmed.strip_prefix("--> ") {
            if let Some(message) = pending.take() {
                let (path, line, column) = rustfmt_location(arrow)?;
                let checked = known(TOOL, files, path)?;
                let (start, end) = point(line, column);
                findings.push(FileFinding {
                    file: checked.to_owned(),
                    finding: Finding {
                        tool_id: TOOL.to_owned(),
                        rule_id: String::new(),
                        message,
                        severity: ToolSeverity::Error,
                        start,
                        end,
                        suggestions: Vec::new(),
                    },
                });
            }
        }
    }
    if findings.is_empty() && code != Some(0) {
        return Err(ParseError::Shape {
            tool: TOOL,
            detail: format!(
                "exit {} with no Diff headers or error blocks",
                code_name(code)
            ),
        });
    }
    Ok(findings)
}

/// Splits a `Diff in <path>:<line>:` header.
fn rustfmt_header(header: &str) -> Result<(&str, u64), ParseError> {
    let (rest, tail) = header
        .rsplit_once(':')
        .ok_or_else(|| missing("rustfmt", "Diff header", header))?;
    if !tail.is_empty() {
        return Err(ParseError::Shape {
            tool: "rustfmt",
            detail: format!("malformed Diff header: {header}"),
        });
    }
    let (path, line_text) = rest
        .rsplit_once(':')
        .ok_or_else(|| missing("rustfmt", "Diff header", header))?;
    let line = line_text.parse::<u64>().map_err(|_| ParseError::Shape {
        tool: "rustfmt",
        detail: format!("malformed Diff header: {header}"),
    })?;
    if path.is_empty() || line == 0 {
        return Err(ParseError::Shape {
            tool: "rustfmt",
            detail: format!("malformed Diff header: {header}"),
        });
    }
    Ok((path, line))
}

/// Splits a ` --> <path>:<line>:<col>` location.
fn rustfmt_location(arrow: &str) -> Result<(&str, u64, u64), ParseError> {
    let (rest, column_text) = arrow
        .rsplit_once(':')
        .ok_or_else(|| missing("rustfmt", "error location", arrow))?;
    let (path, line_text) = rest
        .rsplit_once(':')
        .ok_or_else(|| missing("rustfmt", "error location", arrow))?;
    let (line, column) = (line_text.parse::<u64>(), column_text.parse::<u64>());
    match (path.is_empty(), line, column) {
        (false, Ok(line), Ok(column)) if line >= 1 && column >= 1 => Ok((path, line, column)),
        _ => Err(ParseError::Shape {
            tool: "rustfmt",
            detail: format!("malformed error location: {arrow}"),
        }),
    }
}

fn missing(tool: &'static str, what: &str, line: &str) -> ParseError {
    ParseError::Shape {
        tool,
        detail: format!("malformed {what}: {line}"),
    }
}

fn code_name(code: Option<i32>) -> String {
    code.map_or_else(|| "signal".to_owned(), |code| code.to_string())
}

// ---------------------------------------------------------------------------
// Taplo
// ---------------------------------------------------------------------------

/// One `error:` / `┌─ <path>:<line>:<col>` block plus its caret detail.
struct TaploBlock {
    file: String,
    line: u64,
    column: u64,
    message: String,
}

/// Scans stderr for Taplo error blocks. The opener line supplies the
/// base message; the first caret line carrying text after `^` appends
/// the tool's own detail (such as `expected value`).
fn taplo_blocks(stderr: &str) -> Result<Vec<TaploBlock>, ParseError> {
    const TOOL: &str = "taplo";
    let mut blocks: Vec<TaploBlock> = Vec::new();
    let mut pending: Option<String> = None;
    let mut open: Option<usize> = None;
    for line in stderr.lines() {
        let trimmed = line.trim_start();
        if let Some(opener) = trimmed.strip_prefix("error:") {
            pending = Some(opener.trim().to_owned());
            open = None;
            continue;
        }
        if let Some(marker) = trimmed.find("┌─ ") {
            let location = trimmed[marker + "┌─ ".len()..].trim();
            let (path, line, column) = taplo_location(location)?;
            let base = pending.clone().ok_or_else(|| ParseError::Shape {
                tool: TOOL,
                detail: format!("location without an error opener: {location}"),
            })?;
            blocks.push(TaploBlock {
                file: path.to_owned(),
                line,
                column,
                message: base,
            });
            open = Some(blocks.len() - 1);
            continue;
        }
        if trimmed.starts_with("ERROR ") || trimmed.starts_with("INFO ") {
            open = None;
            continue;
        }
        if let Some(index) = open {
            // `rsplit` always yields at least one item, so this documents
            // the stdlib guarantee rather than branching on it.
            let caret = line
                .rsplit('^')
                .next()
                .expect("rsplit yields at least one item");
            let detail = caret.trim();
            if !detail.is_empty() && !detail.contains('│') && !detail.contains('─') {
                let block = &mut blocks[index];
                if !block.message.ends_with(detail) {
                    block.message.push_str(": ");
                    block.message.push_str(detail);
                }
                open = None;
            }
        }
    }
    Ok(blocks)
}

/// Splits a `<path>:<line>:<col>` location from the right so absolute
/// paths survive.
fn taplo_location(location: &str) -> Result<(&str, u64, u64), ParseError> {
    let (rest, column_text) = location
        .rsplit_once(':')
        .ok_or_else(|| missing("taplo", "error location", location))?;
    let (path, line_text) = rest
        .rsplit_once(':')
        .ok_or_else(|| missing("taplo", "error location", location))?;
    match (
        path.is_empty(),
        line_text.parse::<u64>(),
        column_text.parse::<u64>(),
    ) {
        (false, Ok(line), Ok(column)) if line >= 1 && column >= 1 => Ok((path, line, column)),
        _ => Err(ParseError::Shape {
            tool: "taplo",
            detail: format!("malformed error location: {location}"),
        }),
    }
}

/// Parses Taplo `lint` stderr into one error finding per block.
pub fn parse_taplo_lint(
    stderr: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "taplo";
    let stderr_text = std::str::from_utf8(stderr).map_err(|err| ParseError::Shape {
        tool: TOOL,
        detail: err.to_string(),
    })?;
    let mut findings = Vec::new();
    for block in taplo_blocks(stderr_text)? {
        let checked = known(TOOL, files, &block.file)?;
        let (start, end) = point(block.line, block.column);
        findings.push(FileFinding {
            file: checked.to_owned(),
            finding: Finding {
                tool_id: TOOL.to_owned(),
                rule_id: String::new(),
                message: block.message,
                severity: ToolSeverity::Error,
                start,
                end,
                suggestions: Vec::new(),
            },
        });
    }
    if findings.is_empty() && code != Some(0) {
        return Err(ParseError::Shape {
            tool: TOOL,
            detail: format!("exit {} with no parsed error blocks", code_name(code)),
        });
    }
    Ok(findings)
}

/// Parses Taplo `format --check` stderr: syntax blocks plus one format
/// finding per `not properly formatted` line.
pub fn parse_taplo_format_check(
    stderr: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "taplo";
    let stderr_text = std::str::from_utf8(stderr).map_err(|err| ParseError::Shape {
        tool: TOOL,
        detail: err.to_string(),
    })?;
    let mut findings = parse_taplo_lint(stderr, Some(0), files)?;
    for line in stderr_text.lines() {
        if let Some(marker) = line.find("the file is not properly formatted") {
            let rest = &line[marker..];
            let path = rest
                .find("path=\"")
                .and_then(|start| {
                    let quoted = &rest[start + "path=\"".len()..];
                    quoted.find('"').map(|end| &quoted[..end])
                })
                .ok_or_else(|| missing(TOOL, "format path", line))?;
            let checked = known(TOOL, files, path)?;
            let (start, end) = point(1, 1);
            findings.push(FileFinding {
                file: checked.to_owned(),
                finding: Finding {
                    tool_id: TOOL.to_owned(),
                    rule_id: String::new(),
                    message: "file is not formatted".to_owned(),
                    severity: ToolSeverity::Warning,
                    start,
                    end,
                    suggestions: Vec::new(),
                },
            });
        }
    }
    // `parse_taplo_lint` runs with a forced clean code so its own
    // exit-code check cannot fire here; enforce this mode's contract.
    if findings.is_empty() && code != Some(0) {
        return Err(ParseError::Shape {
            tool: TOOL,
            detail: format!("exit {} with no parsed findings", code_name(code)),
        });
    }
    Ok(findings)
}

// ---------------------------------------------------------------------------
// Vale
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ValeAlert {
    #[serde(rename = "Span")]
    span: Vec<u64>,
    #[serde(rename = "Check")]
    check: String,
    #[serde(rename = "Message")]
    message: String,
    #[serde(rename = "Severity")]
    severity: String,
    #[serde(rename = "Line")]
    line: u64,
}

fn vale_severity(level: &str) -> Result<ToolSeverity, ParseError> {
    match level {
        "error" => Ok(ToolSeverity::Error),
        "warning" => Ok(ToolSeverity::Warning),
        "suggestion" => Ok(ToolSeverity::Info),
        _ => Err(ParseError::Shape {
            tool: "vale",
            detail: format!("unknown severity: {level}"),
        }),
    }
}

/// Parses Vale `--output=JSON` stdout. Config envelopes (`Code`
/// `E100`/`E201`) become [`ParseError::ValeConfig`].
pub fn parse_vale(
    stdout: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "vale";
    let text = std::str::from_utf8(stdout).map_err(|err| ParseError::Json {
        tool: TOOL,
        detail: err.to_string(),
    })?;
    if text.trim().is_empty() {
        return Err(ParseError::Json {
            tool: TOOL,
            detail: "empty output".to_owned(),
        });
    }
    let value: serde_json::Value = serde_json::from_str(text).map_err(|err| ParseError::Json {
        tool: TOOL,
        detail: err.to_string(),
    })?;
    let report = value.as_object().ok_or_else(|| ParseError::Json {
        tool: TOOL,
        detail: "top-level JSON is not an object".to_owned(),
    })?;
    if let Some(code_value) = report.get("Code") {
        let detail = report
            .get("Text")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("vale reported a configuration error");
        return Err(ParseError::ValeConfig {
            detail: format!("{}: {}", code_value, detail.trim()),
        });
    }
    let mut findings = Vec::new();
    // Sorted iteration keeps multi-file output deterministic.
    let mut paths: Vec<&String> = report.keys().collect();
    paths.sort();
    for path in paths {
        let checked = known(TOOL, files, path)?;
        let alerts: Vec<ValeAlert> =
            serde_json::from_value(report[path].clone()).map_err(|err| ParseError::Json {
                tool: TOOL,
                detail: err.to_string(),
            })?;
        for alert in &alerts {
            let (start_column, end_column) = match alert.span.as_slice() {
                [start, end] if *start >= 1 && *end >= *start => (*start, *end + 1),
                _ => {
                    return Err(ParseError::Shape {
                        tool: TOOL,
                        detail: format!("malformed span for {}: {:?}", alert.check, alert.span),
                    });
                }
            };
            if alert.line == 0 {
                return Err(ParseError::Shape {
                    tool: TOOL,
                    detail: format!("zero line for {}", alert.check),
                });
            }
            findings.push(FileFinding {
                file: checked.to_owned(),
                finding: Finding {
                    tool_id: TOOL.to_owned(),
                    rule_id: alert.check.clone(),
                    message: alert.message.clone(),
                    severity: vale_severity(&alert.severity)?,
                    start: TextPosition {
                        line: alert.line,
                        column: start_column,
                    },
                    end: Some(TextPosition {
                        line: alert.line,
                        column: end_column,
                    }),
                    suggestions: Vec::new(),
                },
            });
        }
    }
    if findings.is_empty() && code != Some(0) {
        return Err(ParseError::Shape {
            tool: TOOL,
            detail: format!("exit {} with no parsed alerts", code_name(code)),
        });
    }
    Ok(findings)
}

// ---------------------------------------------------------------------------
// Markdown checker
// ---------------------------------------------------------------------------

/// One newline-delimited JSON finding line from the repo-owned Markdown
/// checker, keyed by the `--source` workspace path.
#[derive(Debug, Deserialize)]
struct MarkdownLine {
    path: String,
    line: u64,
    kind: String,
    message: String,
}

/// Parses Markdown checker stdout. `files` are the workspace paths from
/// the `--source` mappings, in stage order: the checker keys sibling
/// resolution and finding paths off those keys, so the backend re-roots
/// each validated path onto its scratch-absolute path before placement.
/// Findings are line-level points at column 1 with the kebab-case kind as
/// the rule; every kind is an error (broken links and structure fail the
/// lint gate). Clean output is empty stdout on exit 0; any other exit is
/// an operational failure, never a partial result.
pub fn parse_markdown_findings(
    stdout: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "markdown_check";
    const KINDS: &[&str] = &[
        "missing-file-target",
        "missing-anchor",
        "missing-heading",
        "duplicate-heading",
        "missing-language-tag",
    ];
    if code != Some(0) {
        return Err(ParseError::Shape {
            tool: TOOL,
            detail: format!("exit {}: findings exist only on exit 0", code_name(code)),
        });
    }
    let text = std::str::from_utf8(stdout).map_err(|err| ParseError::Json {
        tool: TOOL,
        detail: err.to_string(),
    })?;
    let mut findings = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let parsed: MarkdownLine = serde_json::from_str(line).map_err(|err| ParseError::Json {
            tool: TOOL,
            detail: err.to_string(),
        })?;
        if !KINDS.contains(&parsed.kind.as_str()) {
            return Err(ParseError::Shape {
                tool: TOOL,
                detail: format!("unknown finding kind {:?}", parsed.kind),
            });
        }
        if parsed.line == 0 {
            return Err(ParseError::Shape {
                tool: TOOL,
                detail: format!("zero line for {}", parsed.kind),
            });
        }
        let checked = known(TOOL, files, &parsed.path)?;
        let (start, end) = point(parsed.line, 1);
        findings.push(FileFinding {
            file: checked.to_owned(),
            finding: Finding {
                tool_id: TOOL.to_owned(),
                rule_id: parsed.kind,
                message: parsed.message,
                severity: ToolSeverity::Error,
                start,
                end,
                suggestions: Vec::new(),
            },
        });
    }
    Ok(findings)
}

// ---------------------------------------------------------------------------
// Clippy
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ClippyMessage {
    message: String,
    code: Option<ClippyCode>,
    level: String,
    #[serde(default)]
    spans: Vec<ClippySpan>,
    #[serde(default)]
    children: Vec<ClippyMessage>,
}

#[derive(Debug, Deserialize)]
struct ClippyCode {
    code: String,
}

#[derive(Debug, Deserialize)]
struct ClippySpan {
    file_name: String,
    byte_start: u64,
    byte_end: u64,
    line_start: u64,
    line_end: u64,
    column_start: u64,
    column_end: u64,
    is_primary: bool,
    suggested_replacement: Option<String>,
    suggestion_applicability: Option<String>,
}

fn clippy_level(level: &str) -> Result<ToolSeverity, ParseError> {
    match level {
        "warning" => Ok(ToolSeverity::Warning),
        "error" => Ok(ToolSeverity::Error),
        _ => Err(ParseError::Shape {
            tool: "clippy",
            detail: format!("unknown level: {level}"),
        }),
    }
}

/// Parses Clippy `--error-format=json` stderr, one diagnostic per line.
/// Span-less summaries are skipped; a nonzero exit with no placed
/// finding keeps the first skipped message as evidence.
pub fn parse_clippy(
    stderr: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "clippy";
    let stderr_text = std::str::from_utf8(stderr).map_err(|err| ParseError::Shape {
        tool: TOOL,
        detail: err.to_string(),
    })?;
    let mut findings = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    for line in stderr_text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value =
            serde_json::from_str(line).map_err(|err| ParseError::Shape {
                tool: TOOL,
                detail: format!("unparsable line: {err}"),
            })?;
        if value
            .get("$message_type")
            .and_then(serde_json::Value::as_str)
            != Some("diagnostic")
        {
            continue;
        }
        let diagnostic: ClippyMessage =
            serde_json::from_value(value).map_err(|err| ParseError::Shape {
                tool: TOOL,
                detail: format!("malformed diagnostic: {err}"),
            })?;
        let Some(span) = diagnostic
            .spans
            .iter()
            .filter(|span| files.contains(&span.file_name.as_str()))
            .find(|span| span.is_primary)
            .or_else(|| {
                diagnostic
                    .spans
                    .iter()
                    .find(|span| files.contains(&span.file_name.as_str()))
            })
        else {
            if diagnostic.spans.is_empty() {
                skipped.push(diagnostic.message);
            } else {
                return Err(ParseError::Shape {
                    tool: TOOL,
                    detail: format!(
                        "diagnostic outside the checked files: {}",
                        diagnostic.message
                    ),
                });
            }
            continue;
        };
        if span.line_start == 0
            || span.column_start == 0
            || span.line_end == 0
            || span.column_end == 0
        {
            return Err(ParseError::Shape {
                tool: TOOL,
                detail: format!("zero position in {}", diagnostic.message),
            });
        }
        let mut suggestions = Vec::new();
        collect_suggestions(&diagnostic.children, &span.file_name, &mut suggestions)?;
        let checked = known(TOOL, files, &span.file_name)?;
        findings.push(FileFinding {
            file: checked.to_owned(),
            finding: Finding {
                tool_id: TOOL.to_owned(),
                rule_id: diagnostic.code.map_or_else(String::new, |code| code.code),
                message: diagnostic.message,
                severity: clippy_level(&diagnostic.level)?,
                start: TextPosition {
                    line: span.line_start,
                    column: span.column_start,
                },
                end: Some(TextPosition {
                    line: span.line_end,
                    column: span.column_end,
                }),
                suggestions,
            },
        });
    }
    if findings.is_empty() && code != Some(0) {
        let detail = skipped.first().map_or_else(
            || format!("exit {} with no diagnostics", code_name(code)),
            |message| {
                format!(
                    "exit {} with no placed diagnostics: {message}",
                    code_name(code)
                )
            },
        );
        return Err(ParseError::Shape { tool: TOOL, detail });
    }
    Ok(findings)
}

/// Harvests `MachineApplicable` child spans for the diagnostic's file.
fn collect_suggestions(
    messages: &[ClippyMessage],
    file: &str,
    out: &mut Vec<Suggestion>,
) -> Result<(), ParseError> {
    for message in messages {
        for span in &message.spans {
            if let (Some(replacement), Some(applicability)) =
                (&span.suggested_replacement, &span.suggestion_applicability)
            {
                if applicability == "MachineApplicable" && span.file_name == file {
                    if span.byte_start > span.byte_end {
                        return Err(ParseError::Shape {
                            tool: "clippy",
                            detail: format!("inverted suggestion span in {}", message.message),
                        });
                    }
                    out.push(Suggestion {
                        start: span.byte_start,
                        end: span.byte_end,
                        replacement: replacement.as_bytes().to_vec(),
                    });
                }
            }
        }
        collect_suggestions(&message.children, file, out)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Exact outputs probed from the pinned M04 binaries; the fixtures
    // pin the grammars above, so a tool upgrade that changes its output
    // fails here instead of silently shifting findings.

    const BUILDIFIER_DIRTY: &str = r#"{"success":false,"files":[{"filename":"/s/dirty.bzl","formatted":false,"valid":true,"warnings":[{"start":{"line":1,"column":1},"end":{"line":1,"column":2},"category":"module-docstring","actionable":true,"autoFixable":false,"message":"The file has no module docstring.","url":"https://example.com"}]}]}"#;
    const BUILDIFIER_CLEAN: &str = r#"{"success":true,"files":[{"filename":"/s/clean.bzl","formatted":true,"valid":true,"warnings":[]}]}"#;
    const BUILDIFIER_BROKEN: &str = r#"{"success":false,"files":[{"filename":"/s/broken.bzl","formatted":false,"valid":false,"warnings":[]}]}"#;

    #[test]
    fn buildifier_reports_format_and_warnings() {
        let findings =
            parse_buildifier(BUILDIFIER_DIRTY.as_bytes(), b"", &["/s/dirty.bzl"]).expect("parsed");
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].file, "/s/dirty.bzl");
        assert_eq!(findings[0].finding.rule_id, "");
        assert_eq!(findings[0].finding.message, "file is not formatted");
        assert_eq!(findings[0].finding.severity, ToolSeverity::Warning);
        assert_eq!(findings[1].finding.rule_id, "module-docstring");
        assert_eq!(
            (findings[1].finding.start, findings[1].finding.end),
            (
                TextPosition { line: 1, column: 1 },
                Some(TextPosition { line: 1, column: 2 })
            )
        );
        assert!(
            parse_buildifier(BUILDIFIER_CLEAN.as_bytes(), b"", &["/s/clean.bzl"])
                .expect("parsed")
                .is_empty()
        );
    }

    #[test]
    fn buildifier_positions_syntax_errors_from_stderr() {
        let findings = parse_buildifier(
            BUILDIFIER_BROKEN.as_bytes(),
            b"/s/broken.bzl:3:1: syntax error",
            &["/s/broken.bzl"],
        )
        .expect("parsed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].finding.severity, ToolSeverity::Error);
        assert_eq!(
            (findings[0].finding.start, findings[0].finding.end),
            (TextPosition { line: 3, column: 1 }, None)
        );
        assert_eq!(findings[0].finding.message, "syntax error");
        // Exit code and `success` never decide: findings do.
        assert!(parse_buildifier(b"not json", b"crash", &["/s/broken.bzl"]).is_err());
        assert!(parse_buildifier(BUILDIFIER_CLEAN.as_bytes(), b"", &["/s/other.bzl"]).is_err());
    }

    #[test]
    fn rustfmt_reports_diff_headers_and_syntax_blocks() {
        let dirty = parse_rustfmt(
            b"Diff in /s/dirty.rs:1:\n-fn  main( ){}\n+fn main() {}\n",
            b"",
            Some(1),
            &["/s/dirty.rs"],
        )
        .expect("parsed");
        assert_eq!(dirty.len(), 1);
        assert_eq!(dirty[0].finding.message, "file is not formatted");
        assert_eq!(dirty[0].finding.severity, ToolSeverity::Warning);
        assert_eq!(dirty[0].finding.start, TextPosition { line: 1, column: 1 });

        let broken = parse_rustfmt(
            b"",
            b"error: this file contains an unclosed delimiter\n --> /s/broken.rs:1:12\n  |\n",
            Some(1),
            &["/s/broken.rs"],
        )
        .expect("parsed");
        assert_eq!(broken.len(), 1);
        assert_eq!(broken[0].finding.severity, ToolSeverity::Error);
        assert_eq!(
            broken[0].finding.start,
            TextPosition {
                line: 1,
                column: 12
            }
        );
        assert!(broken[0].finding.message.starts_with("error:"));

        assert!(parse_rustfmt(b"", b"", Some(0), &["/s/clean.rs"])
            .expect("parsed")
            .is_empty());
        assert!(parse_rustfmt(b"", b"", Some(1), &["/s/clean.rs"]).is_err());
        assert!(
            parse_rustfmt(b"Diff in /s/other.rs:1:\n", b"", Some(1), &["/s/clean.rs"]).is_err()
        );
    }

    const TAPLO_DIRTY_STDERR: &str = " INFO taplo:lint_files:collect_files: found files\nerror: invalid TOML\n  \u{250c}\u{2500} /s/dirty.toml:2:5\n  \u{2502}  \n2 \u{2502}   b = \n  \u{2502} \u{256d}\u{2500}\u{2500}\u{2500}\u{2500}^\n3 \u{2502} \u{2502} \n  \u{2502} \u{2570}^ expected value\n\nERROR taplo:lint_files: invalid file\n";

    #[test]
    fn taplo_lint_reports_blocks_with_caret_detail() {
        let findings = parse_taplo_lint(TAPLO_DIRTY_STDERR.as_bytes(), Some(1), &["/s/dirty.toml"])
            .expect("parsed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].file, "/s/dirty.toml");
        assert_eq!(findings[0].finding.severity, ToolSeverity::Error);
        assert_eq!(
            findings[0].finding.start,
            TextPosition { line: 2, column: 5 }
        );
        assert_eq!(findings[0].finding.message, "invalid TOML: expected value");
        assert!(
            parse_taplo_lint(b" INFO collect\n", Some(0), &["/s/clean.toml"])
                .expect("parsed")
                .is_empty()
        );
        // Fail-closed: nonzero exit with no blocks is a grammar mismatch.
        assert!(parse_taplo_lint(b" INFO collect\n", Some(1), &["/s/clean.toml"]).is_err());
    }

    #[test]
    fn taplo_format_check_adds_unformatted_files() {
        let stderr = b" INFO collect\nERROR taplo:format_files: the file is not properly formatted path=\"/s/fmt.toml\"\nERROR operation failed\n";
        let findings = parse_taplo_format_check(stderr, Some(1), &["/s/fmt.toml"]).expect("parsed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].finding.message, "file is not formatted");
        assert_eq!(findings[0].finding.severity, ToolSeverity::Warning);
        // Syntax blocks parse in format mode too.
        let mixed = [TAPLO_DIRTY_STDERR.as_bytes(), &stderr[..]].concat();
        let findings = parse_taplo_format_check(&mixed, Some(1), &["/s/dirty.toml", "/s/fmt.toml"])
            .expect("parsed");
        assert_eq!(findings.len(), 2);
        assert!(parse_taplo_format_check(b" INFO collect\n", Some(1), &["/s/fmt.toml"]).is_err());
    }

    const VALE_DIRTY: &str = r#"{
  "/s/vale/doc.md": [
    {
      "Action": {"Name": "", "Params": null},
      "Span": [9, 14],
      "Check": "Test.Cotton",
      "Description": "",
      "Link": "https://example.com",
      "Message": "Avoid cotton.",
      "Severity": "error",
      "Match": "cotton",
      "Line": 1
    }
  ]
}"#;
    const VALE_CONFIG_ERROR: &str = r#"{
  "Line": 0,
  "Path": "",
  "Text": "E100 [--config] Runtime error\n\npath '/s/nope.ini' does not exist\n\nExecution stopped with code 1.",
  "Code": "E100",
  "Span": 0
}"#;

    #[test]
    fn vale_reports_alerts_with_inclusive_spans() {
        let findings =
            parse_vale(VALE_DIRTY.as_bytes(), Some(1), &["/s/vale/doc.md"]).expect("parsed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].finding.rule_id, "Test.Cotton");
        assert_eq!(findings[0].finding.message, "Avoid cotton.");
        assert_eq!(findings[0].finding.severity, ToolSeverity::Error);
        // Span [9,14] is inclusive on both ends; placement is half-open.
        assert_eq!(
            (findings[0].finding.start, findings[0].finding.end),
            (
                TextPosition { line: 1, column: 9 },
                Some(TextPosition {
                    line: 1,
                    column: 15
                })
            )
        );
        assert!(parse_vale(b"{}", Some(0), &["/s/vale/clean.md"])
            .expect("parsed")
            .is_empty());
    }

    #[test]
    fn vale_surfaces_config_envelopes() {
        let err = parse_vale(VALE_CONFIG_ERROR.as_bytes(), Some(2), &["/s/vale/doc.md"])
            .expect_err("config error");
        assert!(matches!(err, ParseError::ValeConfig { .. }));
        assert!(err.to_string().contains("nope.ini"));
        assert!(parse_vale(b"", Some(2), &["/s/vale/doc.md"]).is_err());
        assert!(parse_vale(VALE_DIRTY.as_bytes(), Some(1), &["/s/other.md"]).is_err());
    }

    const CLIPPY_LINT: &str = r#"{"$message_type":"diagnostic","message":"length comparison to zero","code":{"code":"clippy::len_zero","explanation":null},"level":"warning","spans":[{"file_name":"/s/len.rs","byte_start":19,"byte_end":33,"line_start":2,"line_end":2,"column_start":8,"column_end":22,"is_primary":true,"text":[],"label":null,"suggested_replacement":null,"suggestion_applicability":null,"expansion":null}],"children":[{"message":"using `is_empty` is clearer and more explicit","code":null,"level":"help","spans":[{"file_name":"/s/len.rs","byte_start":19,"byte_end":33,"line_start":2,"line_end":2,"column_start":8,"column_end":22,"is_primary":true,"text":[],"label":null,"suggested_replacement":"\"x\".is_empty()","suggestion_applicability":"MachineApplicable","expansion":null}],"children":[],"rendered":null}],"rendered":null}
{"$message_type":"diagnostic","message":"1 warning emitted","code":null,"level":"warning","spans":[],"children":[],"rendered":null}"#;
    const CLIPPY_BROKEN: &str = r#"{"$message_type":"diagnostic","message":"this file contains an unclosed delimiter","code":null,"level":"error","spans":[{"file_name":"/s/broken.rs","byte_start":11,"byte_end":11,"line_start":1,"line_end":1,"column_start":12,"column_end":12,"is_primary":true,"text":[],"label":null,"suggested_replacement":null,"suggestion_applicability":null,"expansion":null}],"children":[],"rendered":null}
{"$message_type":"diagnostic","message":"aborting due to 1 previous error","code":null,"level":"error","spans":[],"children":[],"rendered":null}"#;

    #[test]
    fn clippy_places_diagnostics_and_harvests_suggestions() {
        let findings =
            parse_clippy(CLIPPY_LINT.as_bytes(), Some(0), &["/s/len.rs"]).expect("parsed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].finding.rule_id, "clippy::len_zero");
        assert_eq!(findings[0].finding.severity, ToolSeverity::Warning);
        assert_eq!(
            (findings[0].finding.start, findings[0].finding.end),
            (
                TextPosition { line: 2, column: 8 },
                Some(TextPosition {
                    line: 2,
                    column: 22
                })
            )
        );
        assert_eq!(
            findings[0].finding.suggestions,
            vec![Suggestion {
                start: 19,
                end: 33,
                replacement: b"\"x\".is_empty()".to_vec(),
            }]
        );

        let broken =
            parse_clippy(CLIPPY_BROKEN.as_bytes(), Some(1), &["/s/broken.rs"]).expect("parsed");
        assert_eq!(broken.len(), 1);
        assert_eq!(broken[0].finding.rule_id, "");
        assert_eq!(broken[0].finding.severity, ToolSeverity::Error);

        assert!(parse_clippy(b"", Some(0), &["/s/clean.rs"])
            .expect("parsed")
            .is_empty());
        // Fail-closed: nonzero exit with only summaries keeps the evidence.
        let err = parse_clippy(
            CLIPPY_LINT.lines().nth(1).expect("summary").as_bytes(),
            Some(1),
            &["/s/len.rs"],
        )
        .expect_err("unplaced failure");
        assert!(err.to_string().contains("1 warning emitted"));
    }

    #[test]
    fn error_display_is_stable() {
        assert!(ParseError::Json {
            tool: "vale",
            detail: "x".to_owned()
        }
        .to_string()
        .contains("vale"));
        assert!(ParseError::UnknownFile {
            tool: "taplo",
            path: "/s/x".to_owned()
        }
        .to_string()
        .contains("/s/x"));
        assert!(ParseError::ValeConfig {
            detail: "E100".to_owned()
        }
        .to_string()
        .contains("E100"));
    }
}

#[cfg(test)]
mod grammar_errors {
    use super::*;

    // Every error constructor above needs a witness: the coverage gate
    // holds exact 100%, so untested grammar branches fail the build.

    #[test]
    fn buildifier_rejects_bytes_and_positions_outside_the_grammar() {
        assert!(parse_buildifier(b"\xff\xfe", b"", &["/s/x.bzl"]).is_err());
        let broken = r#"{"success":false,"files":[{"filename":"/s/broken.bzl","formatted":false,"valid":false,"warnings":[]}]}"#;
        // Zero line falls through to the 1:1 fallback.
        let fallback = parse_buildifier(
            broken.as_bytes(),
            b"/s/broken.bzl:0:5: bad line\nnoise without prefix\n/s/broken.bzl:a:b: bad numbers\n/s/broken.bzl:nocolons",
            &["/s/broken.bzl"],
        )
        .expect("parsed");
        assert_eq!(fallback.len(), 1);
        assert_eq!(
            (fallback[0].finding.start, fallback[0].finding.end),
            (TextPosition { line: 1, column: 1 }, None)
        );
        // An empty message after the location still names a syntax error.
        let empty = parse_buildifier(broken.as_bytes(), b"/s/broken.bzl:3:1:", &["/s/broken.bzl"])
            .expect("parsed");
        assert_eq!(empty[0].finding.message, "syntax error");
        assert_eq!(empty[0].finding.start, TextPosition { line: 3, column: 1 });
    }

    #[test]
    fn rustfmt_rejects_bytes_headers_and_locations() {
        assert!(parse_rustfmt(b"\xff", b"", Some(1), &["/s/x.rs"]).is_err());
        assert!(parse_rustfmt(b"", b"\xff", Some(1), &["/s/x.rs"]).is_err());
        for header in [
            "Diff in nocolon",
            "Diff in /s/x.rs:1:extra",
            "Diff in /s/x.rs:abc:",
            "Diff in :0:",
        ] {
            assert!(
                parse_rustfmt(header.as_bytes(), b"", Some(1), &["/s/x.rs"]).is_err(),
                "header: {header}"
            );
        }
        // A location without a pending error opener is skipped.
        assert!(
            parse_rustfmt(b"", b" --> /s/x.rs:1:1\n", Some(0), &["/s/x.rs"])
                .expect("parsed")
                .is_empty()
        );
        assert!(parse_rustfmt(
            b"",
            b"error: boom\n --> /s/x.rs:a:b\n",
            Some(1),
            &["/s/x.rs"]
        )
        .is_err());
    }

    #[test]
    fn taplo_rejects_orphan_blocks_bad_locations_and_bytes() {
        assert!(parse_taplo_lint(
            "  \u{250c}\u{2500} /s/x.toml:1:5\n".as_bytes(),
            Some(1),
            &["/s/x.toml"]
        )
        .is_err());
        assert!(parse_taplo_lint(
            "error: bad\n  \u{250c}\u{2500} /s/x.toml:a:b\n".as_bytes(),
            Some(1),
            &["/s/x.toml"]
        )
        .is_err());
        assert!(parse_taplo_lint(b"\xff", Some(1), &["/s/x.toml"]).is_err());
        assert!(parse_taplo_format_check(b"\xff", Some(1), &["/s/x.toml"]).is_err());
        // A format path without a quoted value names no file.
        assert!(parse_taplo_format_check(
            b"ERROR taplo:format_files: the file is not properly formatted\n",
            Some(1),
            &["/s/x.toml"]
        )
        .is_err());
    }

    #[test]
    fn vale_rejects_bytes_json_shapes_and_alert_fields() {
        assert!(parse_vale(b"\xff", Some(2), &["/s/x.md"]).is_err());
        assert!(parse_vale(b"{nope", Some(2), &["/s/x.md"]).is_err());
        assert!(parse_vale(b"[1,2]", Some(2), &["/s/x.md"]).is_err());
        assert!(parse_vale(b"{}", Some(1), &["/s/x.md"]).is_err());
        let missing_line = r#"{"/s/x.md": [{"Span": [9, 14], "Check": "T.C", "Message": "m", "Severity": "error"}]}"#;
        assert!(parse_vale(missing_line.as_bytes(), Some(1), &["/s/x.md"]).is_err());
        for span in ["[0, 2]", "[3, 2]", "[1]", "[]"] {
            let alert = format!(
                r#"{{"/s/x.md": [{{"Span": {span}, "Check": "T.C", "Message": "m", "Severity": "error", "Line": 1}}]}}"#
            );
            assert!(
                parse_vale(alert.as_bytes(), Some(1), &["/s/x.md"]).is_err(),
                "span: {span}"
            );
        }
        let zero_line = r#"{
            "/s/x.md": [{"Span": [9, 14], "Check": "T.C", "Message": "m", "Severity": "error", "Line": 0}]
        }"#;
        assert!(parse_vale(zero_line.as_bytes(), Some(1), &["/s/x.md"]).is_err());
        let fatal = r#"{
            "/s/x.md": [{"Span": [9, 14], "Check": "T.C", "Message": "m", "Severity": "fatal", "Line": 1}]
        }"#;
        assert!(parse_vale(fatal.as_bytes(), Some(1), &["/s/x.md"]).is_err());
        // A warning maps below error; a suggestion maps to info.
        let levels = r#"{
            "/s/x.md": [
                {"Span": [1, 2], "Check": "T.W", "Message": "w", "Severity": "warning", "Line": 1},
                {"Span": [1, 2], "Check": "T.S", "Message": "s", "Severity": "suggestion", "Line": 2}
            ]
        }"#;
        let findings = parse_vale(levels.as_bytes(), Some(1), &["/s/x.md"]).expect("parsed");
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].finding.severity, ToolSeverity::Warning);
        assert_eq!(findings[1].finding.severity, ToolSeverity::Info);
    }

    #[test]
    fn markdown_parses_workspace_keyed_findings() {
        let stdout = concat!(
            "{\"path\":\"doc/guide.md\",\"line\":3,\"kind\":\"missing-file-target\",\"message\":\"link target \\\"nope.md\\\" does not match a checked source\"}\n",
            "{\"path\":\"doc/guide.md\",\"line\":7,\"kind\":\"missing-anchor\",\"message\":\"anchor \\\"#nope\\\" not found\"}\n",
        );
        let findings =
            parse_markdown_findings(stdout.as_bytes(), Some(0), &["doc/guide.md"]).expect("parsed");
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].file, "doc/guide.md");
        assert_eq!(findings[0].finding.tool_id, "markdown_check");
        assert_eq!(findings[0].finding.rule_id, "missing-file-target");
        assert_eq!(findings[0].finding.severity, ToolSeverity::Error);
        assert_eq!(
            (findings[0].finding.start, findings[0].finding.end),
            (TextPosition { line: 3, column: 1 }, None)
        );
        assert_eq!(findings[1].finding.rule_id, "missing-anchor");
        assert!(findings[0].finding.suggestions.is_empty());
        // Clean output is empty stdout on exit 0; blank lines are skipped.
        assert!(parse_markdown_findings(b"\n", Some(0), &["doc/guide.md"])
            .expect("parsed")
            .is_empty());
    }

    #[test]
    fn markdown_rejects_exits_kinds_lines_and_files() {
        let clean =
            "{\"path\":\"doc/guide.md\",\"line\":1,\"kind\":\"missing-heading\",\"message\":\"m\"}";
        // Findings exist only on exit 0: any other exit is an action
        // failure even with parseable lines.
        assert!(parse_markdown_findings(clean.as_bytes(), Some(1), &["doc/guide.md"]).is_err());
        assert!(parse_markdown_findings(clean.as_bytes(), None, &["doc/guide.md"]).is_err());
        assert!(parse_markdown_findings(b"", Some(2), &["doc/guide.md"]).is_err());
        // Non-UTF-8 and malformed lines are grammar errors.
        assert!(parse_markdown_findings(b"\xff", Some(0), &["doc/guide.md"]).is_err());
        assert!(parse_markdown_findings(b"{nope", Some(0), &["doc/guide.md"]).is_err());
        let missing_field = r#"{"path":"doc/guide.md","line":1,"kind":"missing-heading"}"#;
        assert!(
            parse_markdown_findings(missing_field.as_bytes(), Some(0), &["doc/guide.md"]).is_err()
        );
        let unknown_kind = r#"{"path":"doc/guide.md","line":1,"kind":"bad-kind","message":"m"}"#;
        assert!(
            parse_markdown_findings(unknown_kind.as_bytes(), Some(0), &["doc/guide.md"]).is_err()
        );
        let zero_line =
            r#"{"path":"doc/guide.md","line":0,"kind":"missing-heading","message":"m"}"#;
        assert!(parse_markdown_findings(zero_line.as_bytes(), Some(0), &["doc/guide.md"]).is_err());
        let elsewhere = r#"{"path":"other.md","line":1,"kind":"missing-heading","message":"m"}"#;
        assert!(parse_markdown_findings(elsewhere.as_bytes(), Some(0), &["doc/guide.md"]).is_err());
    }

    fn diagnostic(message: &str, level: &str, spans: &str, children: &str) -> String {
        format!(
            r#"{{"$message_type": "diagnostic", "message": {message}, "code": null, "level": "{level}", "spans": [{spans}], "children": [{children}], "rendered": null}}"#
        )
    }

    fn span(file: &str, line: u64, column: u64, primary: bool) -> String {
        format!(
            concat!(
                r#"{{"file_name": "{file}", "byte_start": 0, "byte_end": 1, "#,
                r#""line_start": {line}, "line_end": {line}, "#,
                r#""column_start": {column}, "column_end": {column}, "#,
                r#""is_primary": {primary}, "text": [], "label": null, "#,
                r#""suggested_replacement": null, "suggestion_applicability": null, "expansion": null}}"#
            ),
            file = file,
            line = line,
            column = column,
            primary = primary
        )
    }

    #[test]
    fn clippy_rejects_bytes_lines_and_unplaced_diagnostics() {
        assert!(parse_clippy(b"\xff", Some(1), &["/s/x.rs"]).is_err());
        assert!(parse_clippy(b"  \n", Some(0), &["/s/x.rs"])
            .expect("parsed")
            .is_empty());
        assert!(parse_clippy(b"hello\n", Some(0), &["/s/x.rs"]).is_err());
        let artifact = "{\"$message_type\": \"artifact\", \"x\": 1}\n";
        assert!(parse_clippy(artifact.as_bytes(), Some(0), &["/s/x.rs"])
            .expect("parsed")
            .is_empty());
        let bare = "{\"$message_type\": \"diagnostic\"}\n";
        assert!(parse_clippy(bare.as_bytes(), Some(0), &["/s/x.rs"]).is_err());
        // Spans outside the checked files cannot be attributed.
        let outside = diagnostic(
            "\"elsewhere\"",
            "warning",
            &span("/other.rs", 1, 1, true),
            "",
        );
        assert!(parse_clippy(outside.as_bytes(), Some(0), &["/s/x.rs"]).is_err());
        // Zero positions are a grammar mismatch, never a point range.
        let zero = diagnostic("\"zero\"", "warning", &span("/s/x.rs", 1, 0, true), "");
        assert!(parse_clippy(zero.as_bytes(), Some(0), &["/s/x.rs"]).is_err());
        // Unknown levels fail closed even when the span would place.
        let noted = diagnostic("\"noted\"", "note", &span("/s/x.rs", 1, 1, true), "");
        assert!(parse_clippy(noted.as_bytes(), Some(0), &["/s/x.rs"]).is_err());
        // A bare nonzero exit with no diagnostics names the exit.
        let err = parse_clippy(b"", Some(1), &["/s/x.rs"]).expect_err("empty failure");
        assert!(err.to_string().contains('1'));
    }

    #[test]
    fn clippy_keeps_only_machine_applicable_suggestions() {
        let maybe = format!(concat!(
            r#"{{"message": "maybe", "code": null, "level": "help", "spans": ["#,
            r#"{{"file_name": "/s/x.rs", "byte_start": 0, "byte_end": 1, "#,
            r#""line_start": 1, "line_end": 1, "column_start": 1, "column_end": 2, "#,
            r#""is_primary": true, "text": [], "label": null, "#,
            r#""suggested_replacement": "y", "suggestion_applicability": "MaybeIncorrect", "#,
            r#""expansion": null}}], "children": [], "rendered": null}}"#
        ));
        let plain = format!(concat!(
            r#"{{"message": "plain", "code": null, "level": "help", "spans": ["#,
            r#"{{"file_name": "/s/x.rs", "byte_start": 0, "byte_end": 1, "#,
            r#""line_start": 1, "line_end": 1, "column_start": 1, "column_end": 2, "#,
            r#""is_primary": true, "text": [], "label": null, "#,
            r#""suggested_replacement": null, "suggestion_applicability": null, "#,
            r#""expansion": null}}], "children": [], "rendered": null}}"#
        ));
        // A non-primary span still places when it is the only in-file span.
        let secondary = span("/s/x.rs", 2, 3, false);
        let line = diagnostic(
            "\"lint\"",
            "warning",
            &secondary,
            &format!("{maybe}, {plain}"),
        );
        let findings = parse_clippy(line.as_bytes(), Some(0), &["/s/x.rs"]).expect("parsed");
        assert_eq!(findings.len(), 1);
        assert_eq!(
            (findings[0].finding.start, findings[0].finding.end),
            (
                TextPosition { line: 2, column: 3 },
                Some(TextPosition { line: 2, column: 3 })
            )
        );
        assert!(findings[0].finding.suggestions.is_empty());
        // Inverted suggestion spans are a grammar mismatch.
        let inverted = format!(concat!(
            r#"{{"message": "fix", "code": null, "level": "help", "spans": ["#,
            r#"{{"file_name": "/s/x.rs", "byte_start": 5, "byte_end": 2, "#,
            r#""line_start": 1, "line_end": 1, "column_start": 1, "column_end": 2, "#,
            r#""is_primary": true, "text": [], "label": null, "#,
            r#""suggested_replacement": "y", "suggestion_applicability": "MachineApplicable", "#,
            r#""expansion": null}}], "children": [], "rendered": null}}"#
        ));
        let bad = diagnostic(
            "\"lint\"",
            "warning",
            &span("/s/x.rs", 1, 1, true),
            &inverted,
        );
        assert!(parse_clippy(bad.as_bytes(), Some(0), &["/s/x.rs"]).is_err());
    }
}
