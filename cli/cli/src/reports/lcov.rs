//! LCOV report validation and line-rate computation (issue #236).
//!
//! Split from `super` (`reports.rs`): owns `validate_lcov` and
//! `coverage_line_rate`. Re-exported through `super` so the public path
//! stays `crate::reports::{validate_lcov, coverage_line_rate}`.

use std::collections::BTreeMap;

use dx_lcov::{find_ignores, is_covered_language, is_ignored, parse_lcov};

use super::ReportError;

/// Validates that `bytes` are a syntactically complete LCOV tracefile.
///
/// The exact bytes are preserved for the report; validation only
/// checks UTF-8, at least one `SF:` record with a non-empty path, no
/// `DA:` outside an `SF:` section, well-formed `DA:<line>,<hits>`
/// counters with `line >= 1`, and that every `SF:` section closes with
/// `end_of_record`. Unknown `FN`/`BRDA`/summary lines are ignored like
/// the M00 gate parser. Failures return [`ReportError::InvalidLcov`]
/// so callers emit no LCOV report.
pub fn validate_lcov(bytes: &[u8]) -> Result<(), ReportError> {
    let invalid = |detail: String| ReportError::InvalidLcov { detail };
    let text =
        std::str::from_utf8(bytes).map_err(|e| invalid(format!("LCOV is not UTF-8: {e}")))?;
    let mut sections = 0u64;
    let mut open = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(path) = line.strip_prefix("SF:") {
            if path.is_empty() {
                return Err(invalid("LCOV record with empty SF path".to_owned()));
            }
            if open {
                return Err(invalid("LCOV SF record before end_of_record".to_owned()));
            }
            open = true;
            sections += 1;
        } else if let Some(rest) = line.strip_prefix("DA:") {
            if !open {
                return Err(invalid(format!(
                    "LCOV DA record outside any SF record: {line}"
                )));
            }
            let mut parts = rest.split(',');
            let lineno: u32 = parts
                .next()
                .unwrap_or_default()
                .parse()
                .map_err(|_| invalid(format!("malformed LCOV DA line number: {line}")))?;
            if lineno == 0 {
                return Err(invalid(format!("malformed LCOV DA line number: {line}")));
            }
            parts
                .next()
                .unwrap_or_default()
                .parse::<u64>()
                .map_err(|_| invalid(format!("malformed LCOV DA hit count: {line}")))?;
        } else if line == "end_of_record" {
            if !open {
                return Err(invalid(
                    "LCOV end_of_record outside any SF record".to_owned(),
                ));
            }
            open = false;
        }
    }
    if sections == 0 {
        return Err(invalid("LCOV has no SF records".to_owned()));
    }
    if open {
        return Err(invalid("LCOV SF record without end_of_record".to_owned()));
    }
    Ok(())
}

/// Line-coverage rate over validated LCOV documents for
/// `dx coverage --min-coverage`.
///
/// Returns `(covered, eligible)` executable-line counts. Documents union
/// per `SF` path with maximum hits winning; source-level exclusion markers
/// are honored for the covered languages (`.rs`, `.go`, `.py`, `.js`,
/// `.jsx`, `.ts`, `.tsx`) through the shared `dx_lcov` scanner (a
/// `reason:` comment stays required exactly as under the retired M00 gate;
/// see the marker syntax in `docs/testing/README.md`). Sources that fail to load count raw: Bazel may
/// instrument generated or external files outside the workspace.
/// Records outside the covered languages have no marker language and count
/// raw. Invalid markers fail the computation.
pub fn coverage_line_rate(
    documents: &[String],
    load: &dyn Fn(&str) -> Option<String>,
) -> Result<(u64, u64), ReportError> {
    let invalid = |e: dx_lcov::LcovError| ReportError::InvalidLcov {
        detail: e.to_string(),
    };
    let mut merged: BTreeMap<String, dx_lcov::FileHits> = BTreeMap::new();
    for document in documents {
        let parsed = parse_lcov(document).map_err(&invalid)?;
        for (path, hits) in parsed {
            let slot = merged.entry(path).or_default();
            for (line, count) in hits.lines {
                let cell = slot.lines.entry(line).or_insert(0);
                if count > *cell {
                    *cell = count;
                }
            }
        }
    }
    let mut covered = 0u64;
    let mut eligible = 0u64;
    for (path, hits) in &merged {
        let mut ignores = None;
        if is_covered_language(path) {
            if let Some(source) = load(path) {
                ignores = Some(find_ignores(path, &source).map_err(&invalid)?);
            }
        }
        for (line, count) in &hits.lines {
            let ignored = ignores
                .as_ref()
                .is_some_and(|valid| is_ignored(valid, *line));
            if ignored {
                continue;
            }
            eligible += 1;
            if *count > 0 {
                covered += 1;
            }
        }
    }
    Ok((covered, eligible))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lcov_validation_covers_all_branches() {
        let good = "SF:/a.rs\nDA:1,1\nend_of_record\n";
        assert_eq!(validate_lcov(good.as_bytes()), Ok(()));
        assert!(validate_lcov(&[0xff]).is_err());
        assert!(validate_lcov(b"DA:1,1\nend_of_record\n").is_err());
        assert!(validate_lcov(b"SF:\nDA:1,1\nend_of_record\n").is_err());
        assert!(validate_lcov(b"SF:/a.rs\nSF:/b.rs\n").is_err());
        assert!(validate_lcov(b"SF:/a.rs\nDA:bad,1\nend_of_record\n").is_err());
        assert!(validate_lcov(b"SF:/a.rs\nDA:0,1\nend_of_record\n").is_err());
        assert!(validate_lcov(b"SF:/a.rs\nDA:1,bad\nend_of_record\n").is_err());
        assert!(validate_lcov(b"end_of_record\n").is_err());
        assert!(validate_lcov(b"SF:/a.rs\nDA:1,1\n").is_err());
        assert!(validate_lcov(b"").is_err());
        // Blank lines and unknown FN/BRDA lines are ignored.
        let with_noise = "\nSF:/a.rs\nFN:1,fn\nBRDA:1,0,0,1\nDA:1,1\nend_of_record\n";
        assert_eq!(validate_lcov(with_noise.as_bytes()), Ok(()));
    }

    fn rate(documents: &[&str], sources: &[(&str, &str)]) -> Result<(u64, u64), ReportError> {
        let owned: Vec<String> = documents.iter().map(ToString::to_string).collect();
        coverage_line_rate(&owned, &|path| {
            sources
                .iter()
                .find(|(name, _)| *name == path)
                .map(|(_, text)| (*text).to_owned())
        })
    }

    #[test]
    fn coverage_rate_counts_hits_and_uncovered() {
        let documents = ["SF:src/a.py\nDA:1,1\nDA:2,0\nDA:3,5\nend_of_record\n"];
        assert_eq!(rate(&documents, &[]), Ok((2, 3)));
    }

    #[test]
    fn coverage_rate_unions_duplicate_records_with_max_hits() {
        let documents = [
            "SF:src/a.rs\nDA:1,0\nDA:2,1\nend_of_record\n".to_owned(),
            "SF:src/a.rs\nDA:1,3\nDA:3,0\nend_of_record\n".to_owned(),
        ];
        let source = "fn a() {}\nfn b() {}\nfn c() {}\n";
        assert_eq!(
            coverage_line_rate(&documents, &|_| Some(source.to_owned())),
            Ok((2, 3))
        );
    }

    #[test]
    fn coverage_rate_honors_exclusion_markers() {
        let documents = ["SF:src/a.rs\nDA:1,1\nDA:2,0\nDA:3,0\nend_of_record\n"];
        let source = "fn a() {}\n// LCOV_EXCL_LINE - reason: generated.\nfn c() {}\n";
        assert_eq!(rate(&documents, &[("src/a.rs", source)]), Ok((1, 2)));
    }

    #[test]
    fn coverage_rate_counts_unloadable_sources_raw() {
        let documents = ["SF:src/a.rs\nDA:1,1\nDA:2,0\nend_of_record\n"];
        assert_eq!(rate(&documents, &[]), Ok((1, 2)));
    }

    #[test]
    fn coverage_rate_rejects_invalid_markers() {
        let documents = ["SF:src/a.rs\nDA:1,1\nend_of_record\n"];
        let source = "// LCOV_EXCL_LINE\nfn a() {}\n";
        assert!(rate(&documents, &[("src/a.rs", source)]).is_err());
    }

    #[test]
    fn coverage_rate_rejects_malformed_lcov() {
        let documents = ["DA:1,1\nend_of_record\n"];
        assert!(rate(&documents, &[]).is_err());
    }
}
