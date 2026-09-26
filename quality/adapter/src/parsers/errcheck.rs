use super::{check_output_size, code_name, known, missing, point, FileFinding, ParseError};
use crate::{Finding, ToolSeverity};

pub fn parse_errcheck(
    stdout: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "errcheck";
    check_output_size(TOOL, stdout)?;
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
        let (path, rest) = trimmed
            .split_once(':')
            .ok_or_else(|| missing(TOOL, "location", line))?;
        let checked = known(TOOL, files, path)?;
        let mut parts = rest.splitn(3, ':');
        let line_no: u64 = parts
            .next()
            .ok_or_else(|| missing(TOOL, "line", line))?
            .trim()
            .parse()
            .map_err(|_| missing(TOOL, "line", line))?;
        let col_no: u64 = parts
            .next()
            .ok_or_else(|| missing(TOOL, "column", line))?
            .trim()
            .parse()
            .map_err(|_| missing(TOOL, "column", line))?;
        let message = parts
            .next()
            .ok_or_else(|| missing(TOOL, "message", line))?
            .trim();
        if message.is_empty() {
            return Err(missing(TOOL, "message", line));
        }
        let (start, end) = point(line_no, col_no);
        findings.push(FileFinding {
            file: checked.to_owned(),
            finding: Finding {
                tool_id: TOOL.to_owned(),
                rule_id: String::new(),
                message: message.to_owned(),
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
            detail: format!("exit {} with no diagnostics", code_name(code)),
        });
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIRTY: &str =
        "go/tests/fixtures/errcheck/Sample.go:7:2: unchecked error: f.WriteString(\"hello\")\n";

    #[test]
    fn errcheck_reports_text_diagnostics() {
        let findings = parse_errcheck(
            DIRTY.as_bytes(),
            Some(1),
            &["go/tests/fixtures/errcheck/Sample.go"],
        )
        .expect("parsed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].file, "go/tests/fixtures/errcheck/Sample.go");
        assert_eq!(
            findings[0].finding.message,
            "unchecked error: f.WriteString(\"hello\")"
        );
        let clean = parse_errcheck(b"", Some(0), &["go/tests/fixtures/errcheck/Sample.go"])
            .expect("parsed");
        assert!(clean.is_empty());
        assert!(parse_errcheck(b"", Some(1), &["go/tests/fixtures/errcheck/Sample.go"]).is_err());
        assert!(parse_errcheck(DIRTY.as_bytes(), Some(1), &["other.go"]).is_err());
        assert!(parse_errcheck(b"nope\n", Some(1), &["x"]).is_err());
        assert!(parse_errcheck(&[0xff], Some(1), &["x"]).is_err());
    }
}
