//! Combined-LCOV parsing for the coverage gate (issue #236).
//!
//! Split from `super` (`lib.rs`): owns [`FileHits`], [`parse_lcov`], and
//! [`validate_lcov_report`] (the `SF`/`DA` record parser that unions
//! duplicate records and ignores non-`DA` summaries). Re-exported through
//! `super` so the public paths stay
//! `dx_lcov::{FileHits, parse_lcov, validate_lcov_report}`. Distinct from
//! the `ignores` module (source-level exclusion markers), the `verdict`
//! module (gate evaluation), and the `inventory`/`run` modules (repo
//! inventory and CLI).

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
/// Implemented on `lcov` records (issue #395): duplicate `SF` records for
/// the same path are unioned per line (the maximum hit count wins,
/// preserving covered-ness). Only `DA` records define executable lines;
/// `FN`/`FNDA`/`BRDA`/`LH`/`LF` summaries are informational and ignored.
/// An empty report parses to an empty map; callers treat that as a missing
/// report.
pub fn parse_lcov(report: &str) -> Result<BTreeMap<String, FileHits>, LcovError> {
    let mut files: BTreeMap<String, FileHits> = BTreeMap::new();
    let mut current: Option<String> = None;
    for raw in report.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if line == "end_of_record" {
            current = None;
            continue;
        }
        if !line.contains(':') {
            continue;
        }
        match line.parse::<lcov::Record>() {
            Ok(lcov::Record::SourceFile { path }) => {
                let path_str = path.to_string_lossy().into_owned();
                if path_str.is_empty() {
                    return Err(LcovError::EmptySfPath);
                }
                current = Some(path_str.clone());
                files.entry(path_str).or_default();
            }
            Ok(lcov::Record::LineData {
                line: lineno,
                count: hits,
                ..
            }) => {
                let path = current.clone().ok_or_else(|| LcovError::DaOutsideSf {
                    line: line.to_string(),
                })?;
                if lineno == 0 {
                    return Err(LcovError::MalformedLineNumber {
                        path: path.clone(),
                        line: line.to_string(),
                    });
                }
                let slot = files
                    .entry(path)
                    .or_default()
                    .lines
                    .entry(lineno)
                    .or_insert(0);
                if hits > *slot {
                    *slot = hits;
                }
            }
            Ok(lcov::Record::EndOfRecord) => {
                // `end_of_record` with a trailing `:fields` suffix is not the
                // canonical terminator; the exact match above owns the close.
                continue;
            }
            Ok(_) => {
                // `TN`/`FN`/`FNDA`/`BRDA`/summary records are informational.
            }
            Err(err) => {
                let kind = line.split_once(':').map(|(k, _)| k).unwrap_or("");
                if kind == "SF" {
                    return Err(LcovError::EmptySfPath);
                }
                if kind != "DA" {
                    continue;
                }
                if current.is_none() {
                    return Err(LcovError::DaOutsideSf {
                        line: line.to_string(),
                    });
                }
                let path = current.clone().unwrap_or_default();
                match err {
                    lcov::record::ParseRecordError::ParseIntError("line", _)
                    | lcov::record::ParseRecordError::FieldNotFound("line") => {
                        return Err(LcovError::MalformedLineNumber {
                            path,
                            line: line.to_string(),
                        });
                    }
                    _ => {
                        return Err(LcovError::MalformedHitCount {
                            path,
                            line: line.to_string(),
                        });
                    }
                }
            }
        }
    }
    Ok(files)
}

/// Strict structural validation for combined LCOV tracefiles.
///
/// Implemented on `lcov` records (issue #395) for the `dx coverage`
/// validator: requires at least one `SF` record, well-formed
/// `DA:<line>,<hits>` counters with `line >= 1`, no `DA` outside an `SF`
/// section, and that every `SF` section closes with `end_of_record`.
/// Unknown `FN`/`BRDA`/summary lines are ignored like the lenient parser.
pub fn validate_lcov_report(report: &str) -> Result<(), LcovError> {
    let mut sections = 0u64;
    let mut open = false;
    let mut current: Option<String> = None;
    for raw in report.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if line == "end_of_record" {
            if !open {
                return Err(LcovError::EndOfRecordOutsideSf);
            }
            open = false;
            current = None;
            continue;
        }
        if !line.contains(':') {
            continue;
        }
        match line.parse::<lcov::Record>() {
            Ok(lcov::Record::SourceFile { path }) => {
                let path_str = path.to_string_lossy().into_owned();
                if path_str.is_empty() {
                    return Err(LcovError::EmptySfPath);
                }
                if open {
                    return Err(LcovError::SfBeforeEndOfRecord);
                }
                open = true;
                sections += 1;
                current = Some(path_str);
            }
            Ok(lcov::Record::LineData { line: lineno, .. }) => {
                if !open {
                    return Err(LcovError::DaOutsideSf {
                        line: line.to_string(),
                    });
                }
                let path = current.clone().unwrap_or_default();
                if lineno == 0 {
                    return Err(LcovError::MalformedLineNumber {
                        path,
                        line: line.to_string(),
                    });
                }
            }
            Ok(lcov::Record::EndOfRecord) => {
                continue;
            }
            Ok(_) => {
                // Informational summaries are ignored.
            }
            Err(err) => {
                let kind = line.split_once(':').map(|(k, _)| k).unwrap_or("");
                if kind == "SF" {
                    return Err(LcovError::EmptySfPath);
                }
                if kind != "DA" {
                    continue;
                }
                if !open {
                    return Err(LcovError::DaOutsideSf {
                        line: line.to_string(),
                    });
                }
                let path = current.clone().unwrap_or_default();
                match err {
                    lcov::record::ParseRecordError::ParseIntError("line", _)
                    | lcov::record::ParseRecordError::FieldNotFound("line") => {
                        return Err(LcovError::MalformedLineNumber {
                            path,
                            line: line.to_string(),
                        });
                    }
                    _ => {
                        return Err(LcovError::MalformedHitCount {
                            path,
                            line: line.to_string(),
                        });
                    }
                }
            }
        }
    }
    if sections == 0 {
        return Err(LcovError::NoSfRecords);
    }
    if open {
        return Err(LcovError::SfWithoutEndOfRecord);
    }
    Ok(())
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

    #[test]
    fn ignores_unknown_records_like_hand_parser() {
        let report = parse_lcov("SF:a.rs\nFOO:1,2\nDA:1,1\nend_of_record\n").unwrap();
        assert_eq!(report["a.rs"].lines[&1], 1);
    }

    #[test]
    fn strict_validator_requires_closed_sections() {
        assert!(validate_lcov_report("SF:/a.rs\nDA:1,1\nend_of_record\n").is_ok());
        assert!(validate_lcov_report("").is_err());
        assert!(validate_lcov_report("SF:/a.rs\nDA:1,1\n").is_err());
        assert!(validate_lcov_report("SF:/a.rs\nSF:/b.rs\n").is_err());
        assert!(validate_lcov_report("end_of_record\n").is_err());
        assert!(validate_lcov_report("DA:1,1\nend_of_record\n").is_err());
    }
}
