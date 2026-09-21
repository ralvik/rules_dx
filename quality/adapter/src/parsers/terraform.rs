//! Terraform fmt output grammar.
//!
//! Per-family module of [`crate::parsers`]: the pinned-shape contract
//! and [`ParseError`] semantics live in the parent module docs.
//!
//! See: `docs/quality/tool-integrations.md#initial-adapter-qualification`

use super::{code_name, known, point, FileFinding, ParseError};
use crate::{Finding, ToolSeverity};

/// Parses `terraform fmt -check -diff` stdout.
///
/// Whole-file rewrite with check/diff mode: unified diff markers
/// yield one `1:1` format finding per file; empty output on exit 0
/// is clean. Findings exit non-zero with diff markers.
pub fn parse_terraform(
    stdout: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "terraform";
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
    if code == Some(0) {
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
    const DIRTY: &str = "--- a/main.tf\n+++ b/main.tf\n@@ -1 +1 @@\n-BADFMT\n+fixed\n";
    #[test]
    fn terraform_reports_diff_files() {
        let findings = parse_terraform(DIRTY.as_bytes(), Some(3), &["main.tf"]).expect("parsed");
        assert_eq!(findings.len(), 1);
        let clean = parse_terraform(b"", Some(0), &["main.tf"]).expect("parsed");
        assert!(clean.is_empty());
        assert!(parse_terraform(b"", Some(3), &["main.tf"]).is_err());
        assert!(parse_terraform(DIRTY.as_bytes(), Some(3), &["other.tf"]).is_err());
        assert!(parse_terraform(&[0xff], Some(0), &["x"]).is_err());
    }
}
