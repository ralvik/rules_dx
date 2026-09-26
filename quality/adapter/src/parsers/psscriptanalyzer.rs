use super::{check_output_size, code_name, known, missing, point, FileFinding, ParseError};
use crate::{Finding, ToolSeverity};

pub fn parse_psscriptanalyzer(
    stdout: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "psscriptanalyzer";
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
        let tail = parts
            .next()
            .ok_or_else(|| missing(TOOL, "message", line))?
            .trim();
        if tail.is_empty() {
            return Err(missing(TOOL, "message", line));
        }
        let (rule, message) = match tail.strip_prefix('[').and_then(|t| t.split_once(']')) {
            Some((r, m)) if !r.is_empty() && !m.trim().is_empty() => {
                (r.trim().to_owned(), m.trim().to_owned())
            }
            _ => (String::new(), tail.to_owned()),
        };
        let (start, end) = point(line_no, col_no);
        findings.push(FileFinding {
            file: checked.to_owned(),
            finding: Finding {
                tool_id: TOOL.to_owned(),
                rule_id: rule,
                message,
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
    const DIRTY: &str = "Sample.ps1:4:1: [PSAvoidUsingWriteHost] Avoid using Write-Host\n";
    #[test]
    fn psscriptanalyzer_reports_bracketed_rule() {
        let findings =
            parse_psscriptanalyzer(DIRTY.as_bytes(), Some(1), &["Sample.ps1"]).expect("parsed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].finding.rule_id, "PSAvoidUsingWriteHost");
        let clean = parse_psscriptanalyzer(b"", Some(0), &["Sample.ps1"]).expect("parsed");
        assert!(clean.is_empty());
        assert!(parse_psscriptanalyzer(b"", Some(1), &["Sample.ps1"]).is_err());
        assert!(parse_psscriptanalyzer(DIRTY.as_bytes(), Some(1), &["other.ps1"]).is_err());
        assert!(parse_psscriptanalyzer(&[0xff], Some(1), &["x"]).is_err());
    }
}
