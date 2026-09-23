//! Scalafix library-API output grammar.
//!
//! Per-family module of [`crate::parsers`]: the pinned-shape contract
//! and [`ParseError`] semantics live in the parent module docs.
//!
//! Console-parse is rejected: rewritable rules emit patch-only diff
//! with no rule attribution, so the adapter wires via
//! `scalafix.interfaces.ScalafixMainCallback` over semantic-rule
//! artifacts. The entrypoint emits one JSON object per line (NDJSON)
//! with per-file rule IDs plus positions.
//!
//! See: `docs/quality/tool-integrations.md#initial-adapter-qualification`

use serde::Deserialize;

use super::{check_output_size, code_name, known, FileFinding, ParseError};
use crate::{Finding, TextPosition, ToolSeverity};

#[derive(Debug, Deserialize)]
struct ScalafixRecord {
    path: String,
    line: u64,
    column: u64,
    #[serde(default)]
    end_line: Option<u64>,
    #[serde(default)]
    end_column: Option<u64>,
    rule: String,
    message: String,
    severity: String,
}

fn scalafix_severity(level: &str) -> Result<ToolSeverity, ParseError> {
    match level {
        "error" => Ok(ToolSeverity::Error),
        "warning" => Ok(ToolSeverity::Warning),
        "info" => Ok(ToolSeverity::Info),
        _ => Err(ParseError::Shape {
            tool: "scalafix",
            detail: format!("unknown severity: {level}"),
        }),
    }
}

/// Parses Scalafix callback NDJSON stdout. `files` are the
/// scratch-absolute paths the entrypoint checked.
///
/// Clean is exit 0 with empty output. Findings exit 0 with at least
/// one record; any nonzero exit is an operational failure, never
/// findings. Missing ends are point ranges. Unknown paths and
/// malformed lines fail closed.
pub fn parse_scalafix(
    stdout: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "scalafix";
    check_output_size(TOOL, stdout)?;
    if code != Some(0) {
        return Err(ParseError::Shape {
            tool: TOOL,
            detail: format!("exit {} (want 0)", code_name(code)),
        });
    }
    let text = std::str::from_utf8(stdout).map_err(|err| ParseError::Shape {
        tool: TOOL,
        detail: err.to_string(),
    })?;
    let mut findings = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let record: ScalafixRecord =
            serde_json::from_str(trimmed).map_err(|err| ParseError::Json {
                tool: TOOL,
                detail: err.to_string(),
            })?;
        if record.line == 0 || record.column == 0 || record.rule.is_empty() {
            return Err(ParseError::Shape {
                tool: TOOL,
                detail: format!("malformed record: {trimmed}"),
            });
        }
        let checked = known(TOOL, files, &record.path)?;
        let end = match (record.end_line, record.end_column) {
            (Some(line), Some(column)) => {
                if line == 0 || column == 0 {
                    return Err(ParseError::Shape {
                        tool: TOOL,
                        detail: format!("malformed end: {trimmed}"),
                    });
                }
                Some(TextPosition { line, column })
            }
            (None, None) => None,
            _ => {
                return Err(ParseError::Shape {
                    tool: TOOL,
                    detail: format!("partial end: {trimmed}"),
                });
            }
        };
        findings.push(FileFinding {
            file: checked.to_owned(),
            finding: Finding {
                tool_id: TOOL.to_owned(),
                rule_id: record.rule,
                message: record.message,
                severity: scalafix_severity(&record.severity)?,
                start: TextPosition {
                    line: record.line,
                    column: record.column,
                },
                end,
                suggestions: Vec::new(),
            },
        });
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINT: &str = "{\"path\": \"/s/Sample.scala\", \"line\": 6, \"column\": 5, \"rule\": \"DisableSyntax.var\", \"message\": \"mutable state should be avoided\", \"severity\": \"error\"}\n{\"path\": \"/s/Sample.scala\", \"line\": 9, \"column\": 3, \"rule\": \"DisableSyntax.null\", \"message\": \"null should be avoided\", \"severity\": \"warning\"}\n";

    #[test]
    fn scalafix_reports_callback_records() {
        let findings =
            parse_scalafix(LINT.as_bytes(), Some(0), &["/s/Sample.scala"]).expect("parsed");
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].finding.rule_id, "DisableSyntax.var");
        assert_eq!(findings[0].finding.severity, ToolSeverity::Error);
        assert_eq!(findings[1].finding.rule_id, "DisableSyntax.null");
        let clean = parse_scalafix(b"", Some(0), &["/s/Sample.scala"]).expect("parsed");
        assert!(clean.is_empty());
        assert!(parse_scalafix(LINT.as_bytes(), Some(1), &["/s/Sample.scala"]).is_err());
        assert!(parse_scalafix(LINT.as_bytes(), Some(0), &["/s/other.scala"]).is_err());
        assert!(parse_scalafix(b"not json\n", Some(0), &["/s/Sample.scala"]).is_err());
        assert!(parse_scalafix(&[0xff], Some(0), &["/s/Sample.scala"]).is_err());
    }
}
