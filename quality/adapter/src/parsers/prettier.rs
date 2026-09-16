//! Prettier output grammar.
//!
//! Per-family module of [`crate::parsers`]: the pinned-shape contract
//! and [`ParseError`] semantics live in the parent module docs.

use super::{code_name, known, point, FileFinding, ParseError};
use crate::{Finding, ToolSeverity};

/// Parses Prettier `--check` stderr. `files` are the workspace-relative
/// mirror paths (Prettier reports working-directory-relative paths even
/// for absolute arguments, so the caller re-anchors them like Ty).
pub fn parse_prettier_check(
    stderr: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "prettier";
    let text = std::str::from_utf8(stderr).map_err(|err| ParseError::Shape {
        tool: TOOL,
        detail: err.to_string(),
    })?;
    let mut findings = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        let Some(path) = trimmed.strip_prefix("[warn] ") else {
            if trimmed.is_empty() {
                continue;
            }
            return Err(ParseError::Shape {
                tool: TOOL,
                detail: format!("unexpected stderr line: {trimmed:?}"),
            });
        };
        if path.starts_with("Code style issues") {
            continue;
        }
        // No empty-path guard: `path` is the remainder after the
        // `"[warn] "` prefix on an already-trimmed line, so it cannot be
        // empty (a bare `"[warn] "` line trims to `"[warn]"` and fails
        // the prefix match above). Unknown paths fail closed in `known`.
        let normalized = path.strip_prefix("./").unwrap_or(path);
        let checked = known(TOOL, files, normalized)?;
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
            detail: format!("exit {} with no warn lines", code_name(code)),
        });
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prettier_check_reports_warn_lines() {
        let stderr = "[warn] src/a.js\n[warn] Code style issues found in the above file. Run Prettier with --write to fix.\n";
        let findings =
            parse_prettier_check(stderr.as_bytes(), Some(1), &["src/a.js"]).expect("parsed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].file, "src/a.js");
        assert_eq!(findings[0].finding.message, "file is not formatted");
        let clean = parse_prettier_check(b"", Some(0), &["src/a.js"]).expect("parsed");
        assert!(clean.is_empty());
        assert!(parse_prettier_check(b"", Some(1), &["src/a.js"]).is_err());
        assert!(parse_prettier_check(b"unexpected\n", Some(1), &["src/a.js"]).is_err());
        assert!(parse_prettier_check(stderr.as_bytes(), Some(1), &["src/other.js"]).is_err());
        assert!(parse_prettier_check(&[0xff], Some(1), &["src/a.js"]).is_err());
        let padded = "[warn] src/a.js\n\n";
        let findings =
            parse_prettier_check(padded.as_bytes(), Some(1), &["src/a.js"]).expect("parsed");
        assert_eq!(findings.len(), 1);
    }
}
