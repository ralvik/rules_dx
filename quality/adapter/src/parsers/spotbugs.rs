//! SpotBugs output grammar.
//!
//! Per-family module of [`crate::parsers`]: the pinned-shape contract
//! and [`ParseError`] semantics live in the parent module docs.
//!
//! SpotBugs `-sarif` prints a SARIF 2.1.0 log to its output file with
//! ranged regions; `level` maps error/warning. The adapter is
//! target-coupled (it analyzes compiled classes from the authoritative
//! `JavaInfo`, never bare sources), so provider-less matrix cases never
//! exercise it; the parser is still fail-closed on the pinned grammar.

use super::{sarif::parse_sarif, FileFinding, ParseError};

/// Parses SpotBugs `-sarif` output. `files` are the absolute scratch
/// paths passed to the tool.
pub fn parse_spotbugs(
    stdout: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    parse_sarif("spotbugs", stdout, code, files)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SARIF: &str = r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"SpotBugs"}},"results":[{"ruleId":"NP_NULL_ON_SOME_PATH","level":"warning","message":{"text":"Possible null pointer dereference."},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"/s/Dirty.java"},"region":{"startLine":5,"startColumn":5,"endLine":5,"endColumn":9}}}]}]}]}"#;

    #[test]
    fn spotbugs_reports_sarif_ranges() {
        let findings =
            parse_spotbugs(SARIF.as_bytes(), Some(1), &["/s/Dirty.java"]).expect("parsed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].finding.rule_id, "NP_NULL_ON_SOME_PATH");
        let clean =
            r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"SpotBugs"}},"results":[]}]}"#;
        let clean = parse_spotbugs(clean.as_bytes(), Some(0), &["/s/Dirty.java"]).expect("clean");
        assert!(clean.is_empty());
        assert!(parse_spotbugs(b"not json", Some(1), &["/s/Dirty.java"]).is_err());
    }
}
