use serde::Deserialize;

use super::{check_output_size, code_name, FileFinding, ParseError};
use crate::{Finding, TextPosition, ToolSeverity};

#[derive(Debug, Deserialize)]
struct SarifLog {
    #[serde(default)]
    runs: Vec<SarifRun>,
}

#[derive(Debug, Deserialize)]
struct SarifRun {
    #[serde(default, rename = "originalUriBaseIds")]
    bases: std::collections::BTreeMap<String, SarifBase>,
    #[serde(default)]
    results: Vec<SarifResult>,
}

#[derive(Debug, Deserialize)]
struct SarifBase {
    #[serde(default)]
    uri: String,
}

#[derive(Debug, Deserialize)]
struct SarifResult {
    #[serde(rename = "ruleId", default)]
    rule_id: Option<String>,
    #[serde(default)]
    level: Option<String>,
    #[serde(default)]
    message: SarifMessage,
    #[serde(default)]
    locations: Vec<SarifLocation>,
}

#[derive(Debug, Default, Deserialize)]
struct SarifMessage {
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct SarifLocation {
    #[serde(rename = "physicalLocation", default)]
    physical: Option<SarifPhysical>,
}

#[derive(Debug, Deserialize)]
struct SarifPhysical {
    #[serde(rename = "artifactLocation", default)]
    artifact: Option<SarifArtifact>,
    #[serde(default)]
    region: Option<SarifRegion>,
}

#[derive(Debug, Deserialize)]
struct SarifArtifact {
    #[serde(default)]
    uri: String,
    #[serde(default, rename = "uriBaseId")]
    base_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SarifRegion {
    #[serde(rename = "startLine")]
    start_line: u64,
    #[serde(rename = "startColumn", default)]
    start_column: Option<u64>,
    #[serde(rename = "endLine", default)]
    end_line: Option<u64>,
    #[serde(rename = "endColumn", default)]
    end_column: Option<u64>,
}

fn sarif_severity(tool: &'static str, level: Option<&str>) -> Result<ToolSeverity, ParseError> {
    match level {
        None => Ok(ToolSeverity::Warning),
        Some("error") => Ok(ToolSeverity::Error),
        Some("warning") => Ok(ToolSeverity::Warning),
        Some("note") | Some("none") => Ok(ToolSeverity::Info),
        Some(other) => Err(ParseError::Shape {
            tool,
            detail: format!("unknown level: {other}"),
        }),
    }
}

fn strip_file_uri(uri: &str) -> &str {
    uri.strip_prefix("file://")
        .or_else(|| uri.strip_prefix("file:"))
        .unwrap_or(uri)
}

fn normalize_path(path: &str) -> String {
    let absolute = path.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    let joined = parts.join("/");
    if absolute {
        format!("/{joined}")
    } else {
        joined
    }
}

fn resolve_file<'a>(
    tool: &'static str,
    files: &[&'a str],
    uri: &str,
    base_id: Option<&str>,
    bases: &std::collections::BTreeMap<String, SarifBase>,
) -> Result<&'a str, ParseError> {
    let mut joined = String::new();
    if let Some(id) = base_id {
        let base = bases.get(id).ok_or_else(|| ParseError::Shape {
            tool,
            detail: format!("unknown uriBaseId: {id}"),
        })?;
        let base_path = strip_file_uri(&base.uri);
        joined.push_str(base_path.strip_suffix('/').unwrap_or(base_path));
        joined.push('/');
        joined.push_str(strip_file_uri(uri));
    } else {
        joined.push_str(strip_file_uri(uri));
    }
    let normalized = normalize_path(&joined);
    let path = normalized.strip_prefix("./").unwrap_or(&normalized);
    if let Some(hit) = files.iter().find(|file| **file == path) {
        return Ok(*hit);
    }
    let suffix = if path.starts_with('/') {
        path.to_owned()
    } else {
        format!("/{path}")
    };
    {
        let mut hits = files.iter().filter(|file| file.ends_with(suffix.as_str()));
        if let Some(hit) = hits.next() {
            if hits.next().is_none() {
                return Ok(*hit);
            }
        }
    }
    if path.starts_with('/') {
        let raw = strip_file_uri(uri);
        let raw_suffix = format!("/{raw}");
        let mut hits = files
            .iter()
            .filter(|file| file.ends_with(raw_suffix.as_str()));
        if let Some(hit) = hits.next() {
            if hits.next().is_none() {
                return Ok(*hit);
            }
        }
    }
    Err(ParseError::UnknownFile {
        tool,
        path: uri.to_owned(),
    })
}

pub fn parse_sarif(
    tool: &'static str,
    stdout: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    check_output_size(tool, stdout)?;
    let log: SarifLog = serde_json::from_slice(stdout).map_err(|err| ParseError::Json {
        tool,
        detail: err.to_string(),
    })?;
    let mut findings = Vec::new();
    for run in &log.runs {
        for result in &run.results {
            if result.message.text.is_empty() {
                return Err(ParseError::Shape {
                    tool,
                    detail: "result with empty message".to_owned(),
                });
            }
            let severity = sarif_severity(tool, result.level.as_deref())?;
            let rule_id = result.rule_id.clone().unwrap_or_default();
            if result.locations.is_empty() {
                return Err(ParseError::Shape {
                    tool,
                    detail: "result with no locations".to_owned(),
                });
            }
            for location in &result.locations {
                let physical = location.physical.as_ref().ok_or(ParseError::Shape {
                    tool,
                    detail: "location without physicalLocation".to_owned(),
                })?;
                let artifact = physical.artifact.as_ref().ok_or(ParseError::Shape {
                    tool,
                    detail: "location without artifact uri".to_owned(),
                })?;
                if artifact.uri.is_empty() {
                    return Err(ParseError::Shape {
                        tool,
                        detail: "location without artifact uri".to_owned(),
                    });
                }
                let region = physical.region.as_ref().ok_or(ParseError::Shape {
                    tool,
                    detail: "location without region".to_owned(),
                })?;
                if region.start_line < 1 {
                    return Err(ParseError::Shape {
                        tool,
                        detail: format!("bad start line {}", region.start_line),
                    });
                }
                let start_column = region.start_column.unwrap_or(1);
                if start_column < 1 {
                    return Err(ParseError::Shape {
                        tool,
                        detail: format!("bad start column {start_column}"),
                    });
                }
                let end = match (region.end_line, region.end_column) {
                    (Some(line), Some(column)) => {
                        if line < 1 || column < 1 {
                            return Err(ParseError::Shape {
                                tool,
                                detail: "bad end position".to_owned(),
                            });
                        }
                        Some(TextPosition { line, column })
                    }
                    (None, None) => None,
                    _ => {
                        return Err(ParseError::Shape {
                            tool,
                            detail: "partial end position".to_owned(),
                        });
                    }
                };
                let checked = resolve_file(
                    tool,
                    files,
                    &artifact.uri,
                    artifact.base_id.as_deref(),
                    &run.bases,
                )?;
                findings.push(FileFinding {
                    file: checked.to_owned(),
                    finding: Finding {
                        tool_id: tool.to_owned(),
                        rule_id: rule_id.clone(),
                        message: result.message.text.clone(),
                        severity,
                        start: TextPosition {
                            line: region.start_line,
                            column: start_column,
                        },
                        end,
                        suggestions: Vec::new(),
                    },
                });
            }
        }
    }
    if findings.is_empty() && code != Some(0) {
        return Err(ParseError::Shape {
            tool,
            detail: format!("exit {} with no results", code_name(code)),
        });
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOOL: &str = "sarif-test";

    fn log(results: &str) -> Vec<u8> {
        format!("{{\"version\":\"2.1.0\",\"runs\":[{{\"tool\":{{\"driver\":{{\"name\":\"t\"}}}},\"results\":[{results}]}}]}}").into_bytes()
    }

    #[test]
    fn sarif_reports_point_and_range() {
        let stdout = log(
            r#"{"ruleId":"R1","level":"error","message":{"text":"boom"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"file:/s/a.java"},"region":{"startLine":2,"startColumn":8}}}]},{"ruleId":"R2","level":"warning","message":{"text":"warn"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"/s/a.java"},"region":{"startLine":3,"startColumn":1,"endLine":3,"endColumn":5}}}]}"#,
        );
        let findings = parse_sarif(TOOL, &stdout, Some(1), &["/s/a.java"]).expect("parsed");
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].finding.rule_id, "R1");
        assert_eq!(findings[0].finding.severity, ToolSeverity::Error);
        assert!(findings[0].finding.end.is_none());
        let end = findings[1].finding.end.expect("extent");
        assert_eq!((end.line, end.column), (3, 5));
        let clean_json = log("");
        let clean = parse_sarif(TOOL, &clean_json, Some(0), &["/s/a.java"]).expect("clean");
        assert!(clean.is_empty());
        assert!(parse_sarif(TOOL, &clean_json, Some(1), &["/s/a.java"]).is_err());
        assert!(parse_sarif(TOOL, b"not json", Some(1), &["/s/a.java"]).is_err());
    }

    #[test]
    fn sarif_resolves_base_id_joins() {
        let stdout = br#"{"version":"2.1.0","runs":[{"originalUriBaseIds":{"%SRCROOT%":{"uri":"file:///home/u/"}},"tool":{"driver":{"name":"k"}},"results":[{"ruleId":"R","level":"error","message":{"text":"m"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"../../tmp/x/Dirty.kt","uriBaseId":"%SRCROOT%"},"region":{"startLine":1,"startColumn":1}}}]}]}]}"#;
        let findings = parse_sarif(TOOL, stdout, Some(1), &["/tmp/x/Dirty.kt"]).expect("base join");
        assert_eq!(findings[0].file, "/tmp/x/Dirty.kt");
        let unknown_base = br#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"k"}},"results":[{"ruleId":"R","message":{"text":"m"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"a.java","uriBaseId":"%MISSING%"},"region":{"startLine":1}}}]}]}]}"#;
        assert!(parse_sarif(TOOL, unknown_base, Some(1), &["/tmp/x/Dirty.kt"]).is_err());
    }

    #[test]
    fn sarif_resolves_relative_and_level_defaults() {
        let stdout = log(
            r#"{"ruleId":"R","message":{"text":"m"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"src/a.java"},"region":{"startLine":1}}}]}"#,
        );
        let findings = parse_sarif(TOOL, &stdout, Some(1), &["/scratch/src/a.java"])
            .expect("relative resolves");
        assert_eq!(findings[0].file, "/scratch/src/a.java");
        assert_eq!(findings[0].finding.severity, ToolSeverity::Warning);
        assert_eq!(
            (
                findings[0].finding.start.line,
                findings[0].finding.start.column
            ),
            (1, 1)
        );
        let note = log(
            r#"{"ruleId":"R","level":"note","message":{"text":"m"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"/s/a.java"},"region":{"startLine":1,"startColumn":1}}}]}"#,
        );
        let findings = parse_sarif(TOOL, &note, Some(1), &["/s/a.java"]).expect("note");
        assert_eq!(findings[0].finding.severity, ToolSeverity::Info);
        let bad_level = log(
            r#"{"ruleId":"R","level":"fatal","message":{"text":"m"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"/s/a.java"},"region":{"startLine":1,"startColumn":1}}}]}"#,
        );
        assert!(parse_sarif(TOOL, &bad_level, Some(1), &["/s/a.java"]).is_err());
        let unknown = log(
            r#"{"ruleId":"R","level":"warning","message":{"text":"m"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"/s/other.java"},"region":{"startLine":1,"startColumn":1}}}]}"#,
        );
        assert!(parse_sarif(TOOL, &unknown, Some(1), &["/s/a.java"]).is_err());
        let empty_msg = log(
            r#"{"ruleId":"R","level":"warning","message":{"text":""},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"/s/a.java"},"region":{"startLine":1,"startColumn":1}}}]}"#,
        );
        assert!(parse_sarif(TOOL, &empty_msg, Some(1), &["/s/a.java"]).is_err());
        let no_loc = log(r#"{"ruleId":"R","message":{"text":"m"},"locations":[]}"#);
        assert!(parse_sarif(TOOL, &no_loc, Some(1), &["/s/a.java"]).is_err());
        let bad_line = log(
            r#"{"ruleId":"R","message":{"text":"m"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"/s/a.java"},"region":{"startLine":0,"startColumn":1}}}]}"#,
        );
        assert!(parse_sarif(TOOL, &bad_line, Some(1), &["/s/a.java"]).is_err());
    }
}
