//! Checkstyle output grammar.
//!
//! Per-family module of [`crate::parsers`]: the pinned-shape contract
//! and [`ParseError`] semantics live in the parent module docs.
//!
//! Checkstyle `-f sarif` prints a SARIF 2.1.0 log to stdout (operational
//! chatter such as the Reflections scan line and the `Checkstyle ends
//! with N errors.` summary goes to stderr and is ignored here). URIs
//! are `file:` paths with point regions; `level` maps error/warning.

use super::{sarif::parse_sarif, FileFinding, ParseError};

/// Parses Checkstyle `-f sarif` stdout. `files` are the absolute
/// scratch paths passed to the tool.
pub fn parse_checkstyle(
    stdout: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    parse_sarif("checkstyle", stdout, code, files)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SARIF: &str = r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"Checkstyle"}},"results":[{"ruleId":"com.puppycrawl.tools.checkstyle.checks.imports.UnusedImportsCheck","level":"error","message":{"text":"Unused import - java.util.ArrayList."},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"file:/s/Dirty.java"},"region":{"startLine":2,"startColumn":8}}}]}]}]}"#;

    #[test]
    fn checkstyle_reports_sarif_points() {
        let findings =
            parse_checkstyle(SARIF.as_bytes(), Some(1), &["/s/Dirty.java"]).expect("parsed");
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].finding.rule_id,
            "com.puppycrawl.tools.checkstyle.checks.imports.UnusedImportsCheck"
        );
        assert!(findings[0].finding.end.is_none());
        let clean_json = r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"Checkstyle"}},"results":[]}]}"#;
        let clean =
            parse_checkstyle(clean_json.as_bytes(), Some(0), &["/s/Dirty.java"]).expect("clean");
        assert!(clean.is_empty());
        assert!(parse_checkstyle(clean_json.as_bytes(), Some(1), &["/s/Dirty.java"]).is_err());
        assert!(parse_checkstyle(b"not json", Some(1), &["/s/Dirty.java"]).is_err());
    }
}
