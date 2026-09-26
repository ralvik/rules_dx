use super::{check_output_size, code_name, known, point, FileFinding, ParseError};
use crate::{Finding, ToolSeverity};

pub fn parse_qmlformat(
    stdout: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "qmlformat";
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
        let normalized = trimmed.strip_prefix("./").unwrap_or(trimmed);
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
            detail: format!("exit {} with no unformatted paths", code_name(code)),
        });
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qmlformat_reports_unformatted_paths() {
        let stdout = "qml/Main.qml\n";
        let findings =
            parse_qmlformat(stdout.as_bytes(), Some(1), &["qml/Main.qml"]).expect("parsed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].file, "qml/Main.qml");
        let clean = parse_qmlformat(b"", Some(0), &["qml/Main.qml"]).expect("parsed");
        assert!(clean.is_empty());
        assert!(parse_qmlformat(b"", Some(1), &["qml/Main.qml"]).is_err());
        assert!(parse_qmlformat(stdout.as_bytes(), Some(1), &["other.qml"]).is_err());
        assert!(parse_qmlformat(&[0xff], Some(1), &["x"]).is_err());
    }
}
