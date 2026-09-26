use super::{check_output_size, code_name, known, FileFinding, ParseError};
use crate::{Finding, TextPosition, ToolSeverity};

fn json_fail(tool: &'static str, detail: String) -> ParseError {
    ParseError::Json { tool, detail }
}

fn shape_fail(tool: &'static str, detail: String) -> ParseError {
    ParseError::Shape { tool, detail }
}

fn position(value: &serde_json::Value, what: &str) -> Result<TextPosition, ParseError> {
    const TOOL: &str = "staticcheck";
    let line = value
        .get("line")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| shape_fail(TOOL, format!("finding {what} lacks a line")))?;
    let column = value
        .get("column")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| shape_fail(TOOL, format!("finding {what} lacks a column")))?;
    if line == 0 || column == 0 {
        return Err(shape_fail(
            TOOL,
            format!("finding {what} carries a zero position"),
        ));
    }
    Ok(TextPosition { line, column })
}

pub fn parse_staticcheck(
    stdout: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "staticcheck";
    check_output_size(TOOL, stdout)?;
    let text = std::str::from_utf8(stdout).map_err(|err| shape_fail(TOOL, err.to_string()))?;
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|err| json_fail(TOOL, err.to_string()))?;
    let items = value
        .as_array()
        .ok_or_else(|| shape_fail(TOOL, "top level is not an array".to_owned()))?;
    let mut findings = Vec::with_capacity(items.len());
    for item in items {
        let rule = item
            .get("code")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| shape_fail(TOOL, "finding lacks a code".to_owned()))?;
        if rule.is_empty() {
            return Err(shape_fail(TOOL, "finding carries an empty code".to_owned()));
        }
        let severity = match item.get("severity").and_then(serde_json::Value::as_str) {
            Some("error") => ToolSeverity::Error,
            Some("warning") => ToolSeverity::Warning,
            Some(other) => {
                return Err(shape_fail(TOOL, format!("unknown severity: {other}")));
            }
            None => return Err(shape_fail(TOOL, "finding lacks a severity".to_owned())),
        };
        let location = item
            .get("location")
            .ok_or_else(|| shape_fail(TOOL, "finding lacks a location".to_owned()))?;
        let path = location
            .get("file")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| shape_fail(TOOL, "finding lacks a file".to_owned()))?;
        let checked = known(TOOL, files, path)?;
        let start = position(location, "location")?;
        let end = match item.get("end") {
            Some(end) => Some(position(end, "end")?),
            None => None,
        };
        let message = item
            .get("message")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| shape_fail(TOOL, "finding lacks a message".to_owned()))?;
        if message.is_empty() {
            return Err(shape_fail(
                TOOL,
                "finding carries an empty message".to_owned(),
            ));
        }
        findings.push(FileFinding {
            file: checked.to_owned(),
            finding: Finding {
                tool_id: TOOL.to_owned(),
                rule_id: rule.to_owned(),
                message: message.to_owned(),
                severity,
                start,
                end,
                suggestions: Vec::new(),
            },
        });
    }
    if findings.is_empty() && code != Some(0) {
        return Err(shape_fail(
            TOOL,
            format!("exit {} with no findings", code_name(code)),
        ));
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIRTY: &str = "[{\"code\": \"SA4006\", \"severity\": \"warning\", \"location\": {\"file\": \"go/tests/fixtures/staticcheck/Sample.go\", \"line\": 4, \"column\": 2}, \"end\": {\"line\": 4, \"column\": 3}, \"message\": \"this value of x is never used\"}]";

    #[test]
    fn staticcheck_reports_json_findings() {
        let findings = parse_staticcheck(
            DIRTY.as_bytes(),
            Some(1),
            &["go/tests/fixtures/staticcheck/Sample.go"],
        )
        .expect("parsed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].file, "go/tests/fixtures/staticcheck/Sample.go");
        assert_eq!(findings[0].finding.rule_id, "SA4006");
        assert_eq!(findings[0].finding.message, "this value of x is never used");
        let clean = parse_staticcheck(b"[]", Some(0), &["go/tests/fixtures/staticcheck/Sample.go"])
            .expect("parsed");
        assert!(clean.is_empty());
        assert!(
            parse_staticcheck(b"[]", Some(1), &["go/tests/fixtures/staticcheck/Sample.go"])
                .is_err()
        );
        assert!(parse_staticcheck(DIRTY.as_bytes(), Some(1), &["other.go"]).is_err());
        assert!(parse_staticcheck(b"{}", Some(1), &["x"]).is_err());
        assert!(parse_staticcheck(&[0xff], Some(1), &["x"]).is_err());
    }
}
