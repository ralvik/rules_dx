//! RuboCop output grammar.
//!
//! Per-family module of [`crate::parsers`]: the pinned-shape contract
//! and [`ParseError`] semantics live in the parent module docs.
//!
//! See: `docs/quality/tool-integrations.md#initial-adapter-qualification`

use super::{code_name, known, point, FileFinding, ParseError};
use crate::{Finding, ToolSeverity};

/// Parses `rubocop --format json` stdout over the release-assembled
/// Ruby closure. `files` are the workspace-relative mirror paths.
///
/// Pinned shape is `{files:[{path, offenses:[{severity, message,
/// cop_name, location:{line, column}}]}]}`. Severity `error`/`warning`
/// map onto [`ToolSeverity`]; `convention`/`refactor` map onto warning.
/// Clean is empty offenses on exit 0; findings exit non-zero.
pub fn parse_rubocop(
    stdout: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "rubocop";
    let value: serde_json::Value =
        serde_json::from_slice(stdout).map_err(|err| ParseError::Json {
            tool: TOOL,
            detail: err.to_string(),
        })?;
    let entries = value
        .get("files")
        .and_then(|v| v.as_array())
        .ok_or_else(|| ParseError::Shape {
            tool: TOOL,
            detail: "missing files".to_owned(),
        })?;
    let mut findings = Vec::new();
    for entry in entries {
        let path = entry
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ParseError::Shape {
                tool: TOOL,
                detail: "missing path".to_owned(),
            })?;
        let checked = known(TOOL, files, path)?;
        let offenses = entry
            .get("offenses")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ParseError::Shape {
                tool: TOOL,
                detail: "missing offenses".to_owned(),
            })?;
        for off in offenses {
            let line = off
                .get("location")
                .and_then(|l| l.get("line"))
                .and_then(|v| v.as_u64())
                .ok_or_else(|| ParseError::Shape {
                    tool: TOOL,
                    detail: "missing line".to_owned(),
                })?;
            let col = off
                .get("location")
                .and_then(|l| l.get("column"))
                .and_then(|v| v.as_u64())
                .ok_or_else(|| ParseError::Shape {
                    tool: TOOL,
                    detail: "missing column".to_owned(),
                })?;
            let message = off
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned();
            if message.is_empty() {
                return Err(ParseError::Shape {
                    tool: TOOL,
                    detail: "missing message".to_owned(),
                });
            }
            let rule = off
                .get("cop_name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned();
            let severity = match off.get("severity").and_then(|v| v.as_str()) {
                Some("error") | Some("fatal") => ToolSeverity::Error,
                Some("warning") | Some("convention") | Some("refactor") => ToolSeverity::Warning,
                other => {
                    return Err(ParseError::Shape {
                        tool: TOOL,
                        detail: format!("unknown severity {other:?}"),
                    })
                }
            };
            if line == 0 || col == 0 {
                return Err(ParseError::Shape {
                    tool: TOOL,
                    detail: "positions must be nonzero".to_owned(),
                });
            }
            let (start, end) = point(line, col);
            findings.push(FileFinding {
                file: checked.to_owned(),
                finding: Finding {
                    tool_id: TOOL.to_owned(),
                    rule_id: rule,
                    message,
                    severity,
                    start,
                    end,
                    suggestions: Vec::new(),
                },
            });
        }
    }
    if findings.is_empty() && code != Some(0) {
        return Err(ParseError::Shape {
            tool: TOOL,
            detail: format!("exit {} with no diagnostics", code_name(code)),
        });
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;
    const DIRTY: &str = r#"{"files": [{"path": "Sample.rb", "offenses": [{"severity": "convention", "message": "Use double quotes", "cop_name": "Style/StringLiterals", "location": {"line": 3, "column": 1}}]}]}"#;
    const CLEAN: &str = r#"{"files": [{"path": "Sample.rb", "offenses": []}]}"#;
    #[test]
    fn rubocop_reports_json_offenses() {
        let findings = parse_rubocop(DIRTY.as_bytes(), Some(1), &["Sample.rb"]).expect("parsed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].finding.rule_id, "Style/StringLiterals");
        let clean = parse_rubocop(CLEAN.as_bytes(), Some(0), &["Sample.rb"]).expect("parsed");
        assert!(clean.is_empty());
        assert!(parse_rubocop(b"", Some(1), &["Sample.rb"]).is_err());
        assert!(parse_rubocop(DIRTY.as_bytes(), Some(1), &["other.rb"]).is_err());
        assert!(parse_rubocop(&[0xff], Some(0), &["x"]).is_err());
    }
}
