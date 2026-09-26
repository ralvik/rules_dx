use super::{check_output_size, code_name, known, missing, point, FileFinding, ParseError};
use crate::{Finding, ToolSeverity};

pub fn parse_shellcheck(
    stdout: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    const TOOL: &str = "shellcheck";
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
        let (level, rest) = tail
            .split_once(':')
            .ok_or_else(|| missing(TOOL, "level", line))?;
        let level = level.trim();
        let rest = rest.trim();
        let severity = match level {
            "error" => ToolSeverity::Error,
            "warning" => ToolSeverity::Warning,
            "info" | "note" | "style" => ToolSeverity::Info,
            _ => return Err(missing(TOOL, "level", line)),
        };
        let (message, rule) = match rest.rsplit_once('[').and_then(|(m, r)| {
            r.strip_suffix(']')
                .map(|code| (m.trim().to_owned(), code.trim().to_owned()))
        }) {
            Some((m, r)) if !m.is_empty() && !r.is_empty() => (m, r),
            _ => (rest.to_owned(), String::new()),
        };
        if message.is_empty() {
            return Err(missing(TOOL, "message", line));
        }
        let (start, end) = point(line_no, col_no);
        findings.push(FileFinding {
            file: checked.to_owned(),
            finding: Finding {
                tool_id: TOOL.to_owned(),
                rule_id: rule,
                message,
                severity,
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
    const DIRTY: &str = "run.sh:3:1: warning: Double quote to prevent globbing [SC2086]\n";
    #[test]
    fn shellcheck_reports_gcc_lines() {
        let findings = parse_shellcheck(DIRTY.as_bytes(), Some(1), &["run.sh"]).expect("parsed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].finding.rule_id, "SC2086");
        let clean = parse_shellcheck(b"", Some(0), &["run.sh"]).expect("parsed");
        assert!(clean.is_empty());
        assert!(parse_shellcheck(b"", Some(1), &["run.sh"]).is_err());
        assert!(parse_shellcheck(DIRTY.as_bytes(), Some(1), &["other.sh"]).is_err());
        assert!(parse_shellcheck(&[0xff], Some(1), &["x"]).is_err());
    }
}
