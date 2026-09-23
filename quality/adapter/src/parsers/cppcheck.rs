//! Cppcheck output grammar.
//!
//! Per-family module of [`crate::parsers`]: the pinned-shape contract
//! and [`ParseError`] semantics live in the parent module docs.
//!
//! See: `docs/quality/tool-integrations.md#initial-adapter-qualification`

use super::{check_output_size, code_name, known, missing, FileFinding, ParseError};
use crate::{Finding, TextPosition, ToolSeverity};

/// Finds the value of `name="..."` inside one XML element tag.
/// Returns the unescaped value or `None` when absent or unterminated.
fn attribute(tag: &str, name: &str) -> Option<String> {
    let key = format!("{name}=\"");
    let start = tag.find(&key)? + key.len();
    let rest = &tag[start..];
    let end = rest.find('"')?;
    let raw = &rest[..end];
    Some(
        raw.replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&"),
    )
}

/// Parses `cppcheck --xml --xml-version=2` diagnostics on stderr.
/// `files` are the scratch-absolute paths the tool checked.
///
/// Each `<error id severity msg>` element with its first `<location
/// file line column>` becomes one finding; locations without a column
/// place at column 1. Severity maps `error` onto error, `warning`,
/// `style`, `performance`, and `portability` onto warning, and
/// `information` onto info; anything else is a grammar mismatch.
/// Clean is exit 0 with no `<error ` elements; findings exit 1. Any
/// other shape is a grammar mismatch, never a silent pass.
pub fn parse_cppcheck(
    stderr: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "cppcheck";
    check_output_size(TOOL, stderr)?;
    let text = std::str::from_utf8(stderr).map_err(|err| ParseError::Shape {
        tool: TOOL,
        detail: err.to_string(),
    })?;
    let mut findings = Vec::new();
    let mut search = text;
    while let Some(start) = search.find("<error ") {
        let tag_end = search[start..].find('>').ok_or_else(|| ParseError::Shape {
            tool: TOOL,
            detail: "unterminated error element".to_owned(),
        })?;
        let tag = &search[start..start + tag_end];
        let id = attribute(tag, "id").ok_or_else(|| missing(TOOL, "id", tag))?;
        let severity_word =
            attribute(tag, "severity").ok_or_else(|| missing(TOOL, "severity", tag))?;
        let severity = match severity_word.as_str() {
            "error" => ToolSeverity::Error,
            "warning" | "style" | "performance" | "portability" => ToolSeverity::Warning,
            "information" => ToolSeverity::Info,
            _ => return Err(missing(TOOL, "severity", tag)),
        };
        let message = attribute(tag, "msg").ok_or_else(|| missing(TOOL, "message", tag))?;
        if message.is_empty() {
            return Err(missing(TOOL, "message", tag));
        }
        let after = &search[start + tag_end..];
        let loc_start = after
            .find("<location ")
            .ok_or_else(|| missing(TOOL, "location", tag))?;
        let loc_end = after[loc_start..]
            .find('>')
            .ok_or_else(|| ParseError::Shape {
                tool: TOOL,
                detail: "unterminated location element".to_owned(),
            })?;
        let loc = &after[loc_start..loc_start + loc_end];
        let path = attribute(loc, "file").ok_or_else(|| missing(TOOL, "file", loc))?;
        let checked = known(TOOL, files, &path)?;
        let line_no: u64 = attribute(loc, "line")
            .ok_or_else(|| missing(TOOL, "line", loc))?
            .parse()
            .map_err(|_| missing(TOOL, "line", loc))?;
        let col_no: u64 = match attribute(loc, "column") {
            Some(raw) => raw.parse().map_err(|_| missing(TOOL, "column", loc))?,
            None => 1,
        };
        let start_pos = TextPosition {
            line: line_no,
            column: col_no,
        };
        findings.push(FileFinding {
            file: checked.to_owned(),
            finding: Finding {
                tool_id: TOOL.to_owned(),
                rule_id: id,
                message,
                severity,
                start: start_pos,
                end: Some(start_pos),
                suggestions: Vec::new(),
            },
        });
        search = &after[loc_start + loc_end..];
    }
    if findings.is_empty() && code != Some(0) {
        return Err(ParseError::Shape {
            tool: TOOL,
            detail: format!("exit {} with no error elements", code_name(code)),
        });
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIRTY: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<results version=\"2\">\n  <cppcheck version=\"2.21.0\"/>\n  <errors>\n    <error id=\"nullPointer\" severity=\"error\" msg=\"Possible null pointer dereference: slot\" verbose=\"Possible null pointer dereference: slot\">\n      <location file=\"cc/tests/fixtures/cppcheck/Sample.c\" line=\"8\" column=\"10\"/>\n    </error>\n  </errors>\n</results>\n";
    const CLEAN: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<results version=\"2\">\n  <cppcheck version=\"2.21.0\"/>\n  <errors>\n  </errors>\n</results>\n";

    #[test]
    fn cppcheck_reports_xml_errors() {
        let findings = parse_cppcheck(
            DIRTY.as_bytes(),
            Some(1),
            &["cc/tests/fixtures/cppcheck/Sample.c"],
        )
        .expect("parsed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].file, "cc/tests/fixtures/cppcheck/Sample.c");
        assert_eq!(findings[0].finding.rule_id, "nullPointer");
        assert_eq!(
            findings[0].finding.message,
            "Possible null pointer dereference: slot"
        );
        let clean = parse_cppcheck(
            CLEAN.as_bytes(),
            Some(0),
            &["cc/tests/fixtures/cppcheck/Sample.c"],
        )
        .expect("parsed");
        assert!(clean.is_empty());
        assert!(parse_cppcheck(b"", Some(1), &["cc/tests/fixtures/cppcheck/Sample.c"]).is_err());
        assert!(parse_cppcheck(DIRTY.as_bytes(), Some(1), &["other.c"]).is_err());
        assert!(parse_cppcheck(b"<error ", Some(1), &["x"]).is_err());
        assert!(parse_cppcheck(&[0xff], Some(1), &["x"]).is_err());
    }
}
