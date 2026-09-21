//! Roslyn SARIF output grammar.
//!
//! Per-family module of [`crate::parsers`]: the pinned-shape contract
//! and [`ParseError`] semantics live in the parent module docs.
//!
//! Roslyn emits SARIF 2.1 per `csc /errorlog` invocation (one run per
//! TFM/RID pivot). The adapter concatenates per-pivot runs into one
//! log with a single schema plus version in deterministic pivot order
//! and unions results with per-pivot provenance; single-SARIF and
//! merged-single-run are rejected.
//!
//! See: `docs/quality/tool-integrations.md#initial-adapter-qualification`

use serde::Deserialize;

use super::{known, FileFinding, ParseError};
use crate::{Finding, TextPosition, ToolSeverity};

#[derive(Debug, Deserialize)]
struct SarifLog {
    #[serde(default)]
    runs: Vec<SarifRun>,
}

#[derive(Debug, Deserialize)]
struct SarifRun {
    #[serde(default)]
    results: Vec<SarifResult>,
}

#[derive(Debug, Deserialize)]
struct SarifResult {
    #[serde(rename = "ruleId")]
    rule_id: String,
    #[serde(default)]
    level: Option<String>,
    #[serde(default)]
    message: Option<SarifMessage>,
    #[serde(default)]
    locations: Vec<SarifLocation>,
}

#[derive(Debug, Deserialize)]
struct SarifMessage {
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct SarifLocation {
    #[serde(rename = "physicalLocation")]
    #[serde(default)]
    physical: Option<SarifPhysical>,
}

#[derive(Debug, Deserialize)]
struct SarifPhysical {
    #[serde(rename = "artifactLocation")]
    #[serde(default)]
    artifact: Option<SarifArtifact>,
    #[serde(default)]
    region: Option<SarifRegion>,
}

#[derive(Debug, Deserialize)]
struct SarifArtifact {
    #[serde(default)]
    uri: String,
}

#[derive(Debug, Deserialize)]
struct SarifRegion {
    #[serde(rename = "startLine")]
    start_line: u64,
    #[serde(rename = "startColumn")]
    #[serde(default = "default_column")]
    start_column: u64,
    #[serde(rename = "endLine")]
    #[serde(default)]
    end_line: Option<u64>,
    #[serde(rename = "endColumn")]
    #[serde(default)]
    end_column: Option<u64>,
}

fn default_column() -> u64 {
    1
}

fn roslyn_severity(level: Option<&str>) -> Result<ToolSeverity, ParseError> {
    match level.unwrap_or("warning") {
        "error" => Ok(ToolSeverity::Error),
        "warning" => Ok(ToolSeverity::Warning),
        "note" | "info" | "none" => Ok(ToolSeverity::Info),
        other => Err(ParseError::Shape {
            tool: "roslyn",
            detail: format!("unknown level: {other}"),
        }),
    }
}

/// Parses aggregated Roslyn SARIF bytes. `files` are the
/// workspace-relative paths the pivots checked (artifact URIs are
/// re-rooted to workspace-relative form before lookup).
///
/// Clean is a log with no results. Every result needs exactly one
/// location with a known artifact plus a nonzero start line; missing
/// ends are point ranges. Unknown artifacts and malformed regions
/// fail closed.
pub fn parse_roslyn(sarif: &[u8], files: &[&str]) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "roslyn";
    let text = std::str::from_utf8(sarif).map_err(|err| ParseError::Shape {
        tool: TOOL,
        detail: err.to_string(),
    })?;
    let log: SarifLog = serde_json::from_str(text).map_err(|err| ParseError::Json {
        tool: TOOL,
        detail: err.to_string(),
    })?;
    let mut findings = Vec::new();
    for run in &log.runs {
        for result in &run.results {
            if result.rule_id.is_empty() {
                return Err(ParseError::Shape {
                    tool: TOOL,
                    detail: "result without ruleId".to_owned(),
                });
            }
            let location = result.locations.first().ok_or_else(|| ParseError::Shape {
                tool: TOOL,
                detail: format!("result {} without location", result.rule_id),
            })?;
            let physical = location
                .physical
                .as_ref()
                .ok_or_else(|| ParseError::Shape {
                    tool: TOOL,
                    detail: format!("result {} without physicalLocation", result.rule_id),
                })?;
            let uri = physical
                .artifact
                .as_ref()
                .map(|artifact| artifact.uri.as_str())
                .unwrap_or("");
            if uri.is_empty() {
                return Err(ParseError::Shape {
                    tool: TOOL,
                    detail: format!("result {} without artifact uri", result.rule_id),
                });
            }
            let normalized = uri.strip_prefix("./").unwrap_or(uri);
            let checked = known(TOOL, files, normalized)?;
            let region = physical.region.as_ref().ok_or_else(|| ParseError::Shape {
                tool: TOOL,
                detail: format!("result {} without region", result.rule_id),
            })?;
            if region.start_line == 0 || region.start_column == 0 {
                return Err(ParseError::Shape {
                    tool: TOOL,
                    detail: format!("result {} with zero start", result.rule_id),
                });
            }
            let end = match (region.end_line, region.end_column) {
                (Some(line), Some(column)) => {
                    if line == 0 || column == 0 {
                        return Err(ParseError::Shape {
                            tool: TOOL,
                            detail: format!("result {} with zero end", result.rule_id),
                        });
                    }
                    Some(TextPosition { line, column })
                }
                (None, None) => None,
                _ => {
                    return Err(ParseError::Shape {
                        tool: TOOL,
                        detail: format!("result {} with partial end", result.rule_id),
                    });
                }
            };
            let message = result
                .message
                .as_ref()
                .map(|message| message.text.clone())
                .unwrap_or_default();
            findings.push(FileFinding {
                file: checked.to_owned(),
                finding: Finding {
                    tool_id: TOOL.to_owned(),
                    rule_id: result.rule_id.clone(),
                    message,
                    severity: roslyn_severity(result.level.as_deref())?,
                    start: TextPosition {
                        line: region.start_line,
                        column: region.start_column,
                    },
                    end,
                    suggestions: Vec::new(),
                },
            });
        }
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SINGLE: &str = r#"{"version": "2.1.0", "runs": [{"results": [{"ruleId": "CA1822", "level": "warning", "message": {"text": "Member 'Greet' does not access instance data"}, "locations": [{"physicalLocation": {"artifactLocation": {"uri": "csharp/tests/fixtures/roslyn/Sample.cs"}, "region": {"startLine": 7, "startColumn": 19, "endLine": 7, "endColumn": 24}}}]}]}]}"#;

    #[test]
    fn roslyn_reports_sarif_results() {
        let findings = parse_roslyn(
            SINGLE.as_bytes(),
            &["csharp/tests/fixtures/roslyn/Sample.cs"],
        )
        .expect("parsed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].finding.rule_id, "CA1822");
        assert_eq!(findings[0].finding.severity, ToolSeverity::Warning);
        assert_eq!(
            findings[0].finding.start,
            TextPosition {
                line: 7,
                column: 19
            }
        );
        let clean = parse_roslyn(
            r#"{"version": "2.1.0", "runs": []}"#.as_bytes(),
            &["csharp/tests/fixtures/roslyn/Sample.cs"],
        )
        .expect("parsed");
        assert!(clean.is_empty());
        assert!(parse_roslyn(SINGLE.as_bytes(), &["other.cs"]).is_err());
        assert!(parse_roslyn(b"not json", &["x"]).is_err());
        assert!(parse_roslyn(&[0xff], &["x"]).is_err());
        let no_rule = r#"{"runs": [{"results": [{"ruleId": "", "locations": []}]}]}"#;
        assert!(parse_roslyn(no_rule.as_bytes(), &["x"]).is_err());
    }
}
