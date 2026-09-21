//! Cue output grammar.
//!
//! Per-family module of [`crate::parsers`]: the pinned-shape contract
//! and [`ParseError`] semantics live in the parent module docs.
//!
//! See: `docs/quality/tool-integrations.md#initial-adapter-qualification`

use super::{code_name, known, point, FileFinding, ParseError};
use crate::{Finding, ToolSeverity};

/// Parses `cue fmt` diff stdout. `files` are the scratch-absolute paths
/// the tool checked.
///
/// Whole-file rewrite with check/diff mode: unified diff markers
/// (`--- ` headers) mentioning at least one checked file yield one
/// `1:1` format finding per file; empty output on exit 0 is clean.
/// Any other exit shape is a grammar mismatch, never a silent pass.
pub fn parse_cue(
    stdout: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "cue";
    let text = std::str::from_utf8(stdout).map_err(|err| ParseError::Shape {
        tool: TOOL,
        detail: err.to_string(),
    })?;
    let mut mentioned: Vec<String> = Vec::new();
    for line in text.lines() {
        if let Some(path) = line.strip_prefix("--- ") {
            let path = path.strip_prefix("a/").unwrap_or(path).trim();
            if path.is_empty() || path == "/dev/null" {
                continue;
            }
            if !mentioned.iter().any(|known| known == path) {
                mentioned.push(path.to_owned());
            }
        }
    }
    if mentioned.is_empty() {
        if code == Some(0) {
            return Ok(Vec::new());
        }
        return Err(ParseError::Shape {
            tool: TOOL,
            detail: format!("exit {} with no diff markers", code_name(code)),
        });
    }
    if code != Some(0) && code != Some(1) {
        return Err(ParseError::Shape {
            tool: TOOL,
            detail: format!("exit {} with diff markers", code_name(code)),
        });
    }
    let mut findings = Vec::with_capacity(mentioned.len());
    for path in mentioned {
        let checked = known(TOOL, files, &path)?;
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
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIRTY: &str =
        "--- a/cue/Sample.cue\n+++ b/cue/Sample.cue\n@@ -1 +1 @@\n-BADFMT\n+fixed\n";

    #[test]
    fn cue_reports_diff_files() {
        let findings = parse_cue(DIRTY.as_bytes(), Some(1), &["cue/Sample.cue"]).expect("parsed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].file, "cue/Sample.cue");
        assert_eq!(findings[0].finding.message, "file is not formatted");
        let clean = parse_cue(b"", Some(0), &["cue/Sample.cue"]).expect("parsed");
        assert!(clean.is_empty());
        assert!(parse_cue(b"", Some(1), &["cue/Sample.cue"]).is_err());
        assert!(parse_cue(DIRTY.as_bytes(), Some(1), &["other.cue"]).is_err());
        assert!(parse_cue(&[0xff], Some(0), &["x"]).is_err());
    }
}
