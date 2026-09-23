//! ktlint output grammar.
//!
//! Per-family module of [`crate::parsers`]: the pinned-shape contract
//! and [`ParseError`] semantics live in the parent module docs.
//!
//! ktlint `--reporter=sarif` prints a SARIF 2.1.0 log to stdout with
//! ranged regions; `level` maps error/warning. Lint fixes run
//! `ktlint --format` in place (re-read like ESLint `--fix`); the
//! format capability itself stays owned by ktfmt in curated defaults.

use super::{check_output_size, sarif::parse_sarif, FileFinding, ParseError};

/// Parses ktlint `--reporter=sarif` stdout. `files` are the absolute
/// scratch paths passed to the tool.
pub fn parse_ktlint(
    stdout: &[u8],
    code: Option<i32>,
    files: &[&str],
) -> Result<Vec<FileFinding>, ParseError> {
    check_output_size("ktlint", stdout)?;
    parse_sarif("ktlint", stdout, code, files)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SARIF: &str = r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"ktlint"}},"results":[{"ruleId":"no-unused-imports","level":"error","message":{"text":"Unused import."},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"/s/Dirty.kt"},"region":{"startLine":1,"startColumn":8,"endLine":1,"endColumn":15}}}]}]}]}"#;

    #[test]
    fn ktlint_reports_sarif_ranges() {
        let findings = parse_ktlint(SARIF.as_bytes(), Some(1), &["/s/Dirty.kt"]).expect("parsed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].finding.rule_id, "no-unused-imports");
        let clean =
            r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"ktlint"}},"results":[]}]}"#;
        let clean = parse_ktlint(clean.as_bytes(), Some(0), &["/s/Dirty.kt"]).expect("clean");
        assert!(clean.is_empty());
        assert!(parse_ktlint(b"not json", Some(1), &["/s/Dirty.kt"]).is_err());
    }
}
