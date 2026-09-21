//! google-java-format output grammar.
//!
//! Per-family module of [`crate::parsers`]: the pinned-shape contract
//! and [`ParseError`] semantics live in the parent module docs.
//!
//! `google-java-format --dry-run --set-exit-if-changed` prints the
//! absolute paths of files that would change, one per line, to stdout
//! (clean prints nothing). Each listed path becomes one `1:1` format
//! finding (empty rule, `file is not formatted`, warning), mirroring
//! Prettier.

use super::{code_name, known, point, FileFinding, ParseError};
use crate::{Finding, ToolSeverity};

/// Parses google-java-format `--dry-run --set-exit-if-changed` stdout.
/// `files` are the absolute scratch paths passed to the tool.
pub fn parse_google_java_format(
    stdout: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "google_java_format";
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
        let checked = known(TOOL, files, trimmed)?;
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
    if findings.is_empty() && code != Some(0) {
        return Err(ParseError::Shape {
            tool: TOOL,
            detail: format!("exit {} with no paths", code_name(code)),
        });
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn google_java_format_reports_paths() {
        let stdout = "/s/Dirty.java\n";
        let findings = parse_google_java_format(stdout.as_bytes(), Some(1), &["/s/Dirty.java"])
            .expect("parsed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].file, "/s/Dirty.java");
        assert_eq!(findings[0].finding.message, "file is not formatted");
        let clean = parse_google_java_format(b"", Some(0), &["/s/Dirty.java"]).expect("clean");
        assert!(clean.is_empty());
        assert!(parse_google_java_format(b"", Some(1), &["/s/Dirty.java"]).is_err());
        assert!(parse_google_java_format(b"/s/other.java\n", Some(1), &["/s/Dirty.java"]).is_err());
        assert!(parse_google_java_format(&[0xff], Some(1), &["/s/Dirty.java"]).is_err());
    }
}
