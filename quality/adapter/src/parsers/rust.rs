use serde::Deserialize;

use super::{check_output_size, code_name, known, FileFinding, ParseError};
use crate::{Finding, Suggestion, TextPosition, ToolSeverity};

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

fn clippy_level(tool: &'static str, level: &str) -> Result<ToolSeverity, ParseError> {
    match level {
        "warning" => Ok(ToolSeverity::Warning),
        "error" => Ok(ToolSeverity::Error),
        _ => Err(ParseError::Shape {
            tool,
            detail: format!("unknown level: {level}"),
        }),
    }
}

pub fn parse_clippy(
    stderr: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    check_output_size("clippy", stderr)?;
    parse_rust_diagnostics("clippy", stderr, code, files)
}

pub fn parse_rustc(
    stderr: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    check_output_size("rustc", stderr)?;
    parse_rust_diagnostics("rustc", stderr, code, files)
}

fn parse_rust_diagnostics(
    tool: &'static str,
    stderr: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    check_output_size(tool, stderr)?;
    let stderr_text = std::str::from_utf8(stderr).map_err(|err| ParseError::Shape {
        tool,
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
                tool,
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
                tool,
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
                    tool,
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
                tool,
                detail: format!("zero position in {}", diagnostic.message),
            });
        }
        let mut suggestions = Vec::new();
        collect_suggestions(
            tool,
            &diagnostic.children,
            &span.file_name,
            &mut suggestions,
        )?;
        let checked = known(tool, files, &span.file_name)?;
        findings.push(FileFinding {
            file: checked.to_owned(),
            finding: Finding {
                tool_id: tool.to_owned(),
                rule_id: diagnostic.code.map_or_else(String::new, |code| code.code),
                message: diagnostic.message,
                severity: clippy_level(tool, &diagnostic.level)?,
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
        return Err(ParseError::Shape { tool, detail });
    }
    Ok(findings)
}

fn collect_suggestions(
    tool: &'static str,
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
                            tool,
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
        collect_suggestions(tool, &message.children, file, out)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let err = parse_clippy(
            CLIPPY_LINT.lines().nth(1).expect("summary").as_bytes(),
            Some(1),
            &["/s/len.rs"],
        )
        .expect_err("unplaced failure");
        assert!(err.to_string().contains("1 warning emitted"));
    }

    const RUSTC_TYPE_ERROR: &str = r#"{"$message_type":"diagnostic","message":"mismatched types","code":{"code":"E0308","explanation":null},"level":"error","spans":[{"file_name":"/s/type.rs","byte_start":27,"byte_end":32,"line_start":2,"line_end":2,"column_start":9,"column_end":14,"is_primary":true,"text":[],"label":"expected `i32`, found `&str`","suggested_replacement":null,"suggestion_applicability":null,"expansion":null}],"children":[],"rendered":null}
{"$message_type":"diagnostic","message":"aborting due to 1 previous error","code":null,"level":"error","spans":[],"children":[],"rendered":null}"#;

    #[test]
    fn rustc_parses_the_shared_diagnostic_grammar_as_typecheck() {
        let findings =
            parse_rustc(RUSTC_TYPE_ERROR.as_bytes(), Some(1), &["/s/type.rs"]).expect("parsed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].file, "/s/type.rs");
        assert_eq!(findings[0].finding.tool_id, "rustc");
        assert_eq!(findings[0].finding.rule_id, "E0308");
        assert_eq!(findings[0].finding.severity, ToolSeverity::Error);
        assert_eq!(
            (findings[0].finding.start, findings[0].finding.end),
            (
                TextPosition { line: 2, column: 9 },
                Some(TextPosition {
                    line: 2,
                    column: 14
                })
            )
        );
        assert!(parse_rustc(b"", Some(0), &["/s/clean.rs"])
            .expect("parsed")
            .is_empty());
        let err = parse_rustc(
            RUSTC_TYPE_ERROR.lines().nth(1).expect("summary").as_bytes(),
            Some(1),
            &["/s/type.rs"],
        )
        .expect_err("unplaced failure");
        assert!(err.to_string().contains("rustc"));
        assert!(err.to_string().contains("aborting due to"));
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
        let outside = diagnostic(
            "\"elsewhere\"",
            "warning",
            &span("/other.rs", 1, 1, true),
            "",
        );
        assert!(parse_clippy(outside.as_bytes(), Some(0), &["/s/x.rs"]).is_err());
        let zero = diagnostic("\"zero\"", "warning", &span("/s/x.rs", 1, 0, true), "");
        assert!(parse_clippy(zero.as_bytes(), Some(0), &["/s/x.rs"]).is_err());
        let noted = diagnostic("\"noted\"", "note", &span("/s/x.rs", 1, 1, true), "");
        assert!(parse_clippy(noted.as_bytes(), Some(0), &["/s/x.rs"]).is_err());
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
