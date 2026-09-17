//! Combined-LCOV parsing for the coverage gate (issue #236).
//!
//! Split from `super` (`lib.rs`): owns [`FileHits`] and [`parse_lcov`]
//! (the `SF`/`DA` record parser that unions duplicate records and ignores
//! non-`DA` summaries). Re-exported through `super` so the public path
//! stays `dx_lcov::{FileHits, parse_lcov}`. Distinct from the `ignores`
//! module (source-level exclusion markers), the `verdict` module (gate
//! evaluation), and the `inventory`/`run` modules (repo inventory and CLI).

use std::collections::BTreeMap;

use super::LcovError;

/// Executable line hits for one source file, unioned across duplicate records.
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct FileHits {
    /// Line number (1-based) to hit count. A line is covered when hits > 0.
    pub lines: BTreeMap<u32, u64>,
}

/// Parse combined LCOV text into `SF` path to [`FileHits`].
///
/// Duplicate `SF` records for the same path are unioned per line (the maximum
/// hit count wins, preserving covered-ness). Only `DA` records define
/// executable lines; `FN`/`FNDA`/`BRDA`/`LH`/`LF` summaries are informational
/// and ignored. An empty report parses to an empty map; callers treat that as
/// a missing report.
pub fn parse_lcov(report: &str) -> Result<BTreeMap<String, FileHits>, LcovError> {
    let mut files: BTreeMap<String, FileHits> = BTreeMap::new();
    let mut current: Option<String> = None;
    for raw in report.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(path) = line.strip_prefix("SF:") {
            if path.is_empty() {
                return Err(LcovError::EmptySfPath);
            }
            current = Some(path.to_string());
            files.entry(path.to_string()).or_default();
        } else if let Some(rest) = line.strip_prefix("DA:") {
            let path = current.clone().ok_or_else(|| LcovError::DaOutsideSf {
                line: line.to_string(),
            })?;
            let mut parts = rest.split(',');
            let number_text = parts.next().unwrap_or_default();
            let lineno: u32 = number_text
                .parse()
                .map_err(|_| LcovError::MalformedLineNumber {
                    path: path.clone(),
                    line: line.to_string(),
                })?;
            if lineno == 0 {
                return Err(LcovError::MalformedLineNumber {
                    path: path.clone(),
                    line: line.to_string(),
                });
            }
            let hits_text = parts.next().unwrap_or_default();
            let hits: u64 = hits_text
                .parse()
                .map_err(|_| LcovError::MalformedHitCount {
                    path: path.clone(),
                    line: line.to_string(),
                })?;
            let slot = files
                .entry(path)
                .or_default()
                .lines
                .entry(lineno)
                .or_insert(0);
            if hits > *slot {
                *slot = hits;
            }
        } else if line == "end_of_record" {
            current = None;
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_unions_duplicate_records() {
        let report = parse_lcov(
            "TN:\nSF:a.rs\nDA:1,0\nDA:2,3\nend_of_record\nSF:a.rs\nDA:1,2\nDA:3,1\nend_of_record\n",
        )
        .unwrap();
        assert_eq!(report["a.rs"].lines[&1], 2);
        assert_eq!(report["a.rs"].lines[&2], 3);
        assert_eq!(report["a.rs"].lines[&3], 1);
    }

    #[test]
    fn parses_checksum_suffix_and_ignores_summaries() {
        let report = parse_lcov(
            "TN:\nSF:a.rs\nFN:1,main\nFNDA:1,main\nFNF:1\nFNH:1\nBRDA:2,0,0,0\nBRF:1\nBRH:0\nDA:1,1,abcd1234\nLH:1\nLF:1\nend_of_record\n",
        )
        .unwrap();
        assert_eq!(report["a.rs"].lines.len(), 1);
        assert_eq!(report["a.rs"].lines[&1], 1);
    }

    #[test]
    fn empty_report_parses_to_empty_map() {
        assert!(parse_lcov("").unwrap().is_empty());
        assert!(parse_lcov("\n  \n").unwrap().is_empty());
    }

    #[test]
    fn rejects_da_outside_sf() {
        let err = parse_lcov("DA:1,1\n").unwrap_err();
        assert!(err.to_string().contains("outside any SF"), "{err}");
    }

    #[test]
    fn rejects_malformed_da_numbers() {
        assert!(parse_lcov("SF:a.rs\nDA:x,1\nend_of_record\n").is_err());
        assert!(parse_lcov("SF:a.rs\nDA:1,x\nend_of_record\n").is_err());
        assert!(parse_lcov("SF:a.rs\nDA:1\nend_of_record\n").is_err());
    }

    #[test]
    fn rejects_zero_line_number() {
        assert!(parse_lcov("SF:a.rs\nDA:0,1\nend_of_record\n").is_err());
    }

    #[test]
    fn rejects_empty_sf_path() {
        assert!(parse_lcov("SF:\nDA:1,1\nend_of_record\n").is_err());
    }
}
