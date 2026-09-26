use super::{check_output_size, code_name, known, point, FileFinding, ParseError};
use crate::{Finding, ToolSeverity};

pub fn parse_clang_format(
    stdout: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "clang_format";
    check_output_size(TOOL, stdout)?;
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

    const DIRTY: &str = "--- a/cc/tests/fixtures/clang_format/Sample.c\n+++ b/cc/tests/fixtures/clang_format/Sample.c\n@@ -1,3 +1,3 @@\n-int greet( const char*name){return 0;}\n+int greet(const char *name) {\n+  return 0;\n+}\n";

    #[test]
    fn clang_format_reports_diff_files() {
        let findings = parse_clang_format(
            DIRTY.as_bytes(),
            Some(1),
            &["cc/tests/fixtures/clang_format/Sample.c"],
        )
        .expect("parsed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].file, "cc/tests/fixtures/clang_format/Sample.c");
        assert_eq!(findings[0].finding.message, "file is not formatted");
        let clean = parse_clang_format(b"", Some(0), &["cc/tests/fixtures/clang_format/Sample.c"])
            .expect("parsed");
        assert!(clean.is_empty());
        assert!(
            parse_clang_format(b"", Some(1), &["cc/tests/fixtures/clang_format/Sample.c"]).is_err()
        );
        assert!(parse_clang_format(DIRTY.as_bytes(), Some(1), &["other.c"]).is_err());
        assert!(parse_clang_format(&[0xff], Some(1), &["x"]).is_err());
    }
}
