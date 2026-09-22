//! LCOV report validation and line-rate computation.
//!
//! Split from `super` (`reports.rs`): owns `validate_lcov` and
//! `coverage_line_rate`. Re-exported through `super` so the public path
//! stays `crate::reports::{validate_lcov, coverage_line_rate}`. Both are
//! thin projections over `dx_lcov`: validation delegates to
//! `dx_lcov::validate_lcov_report`, rate computation merges via
//! `dx_lcov::merge_lcov_reports`, so the `lcov`-crate `SF`/`DA` union
//! lives once in `//cli/lcov`.
//! See: `docs/testing/README.md#coverage`.

use dx_lcov::{find_ignores, is_covered_language, is_ignored, merge_lcov_reports};

use super::ReportError;

/// Validates that `bytes` are a syntactically complete LCOV tracefile.
///
/// The exact bytes are preserved for the report; validation only
/// checks UTF-8 via [`std::str::from_utf8`] and delegates the structural
/// `SF`/`DA`/`end_of_record` checks to `dx_lcov::validate_lcov_report`
/// (the shared `lcov`-crate parser,). Unknown
/// `FN`/`BRDA`/summary lines are ignored like the gate parser.
/// Failures return [`ReportError::InvalidLcov`] so callers emit no LCOV
/// report.
pub fn validate_lcov(bytes: &[u8]) -> Result<(), ReportError> {
    let invalid = |detail: String| ReportError::InvalidLcov { detail };
    let text =
        std::str::from_utf8(bytes).map_err(|e| invalid(format!("LCOV is not UTF-8: {e}")))?;
    dx_lcov::validate_lcov_report(text).map_err(|e| invalid(e.to_string()))
}

/// Line-coverage rate over validated LCOV documents for
/// `dx coverage --min-coverage`.
///
/// Returns `(covered, eligible)` executable-line counts. Documents union
/// per `SF` path with maximum hits winning; source-level exclusion markers
/// are honored for the covered languages (`.rs`, `.go`, `.py`, `.js`,
/// `.jsx`, `.mjs`, `.cjs`, `.ts`, `.tsx`, `.mts`, `.cts`, JVM
/// `.java`/`.kt`/`.scala`, .NET `.cs`/`.fs`/`.fsi`, plus C/C++ `.c`/`.cc`/`.cpp`/`.cxx`/`.h`/`.hh`/`.hpp`/`.hxx`
///) through the shared `dx_lcov` scanner (a
/// `reason:` comment stays required exactly as under the retired gate;
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
    let merged = merge_lcov_reports(documents).map_err(&invalid)?;
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
        // Intentional bare marker as test data inside a string literal:
        // inert for this file's own gate, invalid for the rated source.
        let source = "// LCOV_EXCL_LINE\nfn a() {}\n";
        assert!(rate(&documents, &[("src/a.rs", source)]).is_err());
    }

    #[test]
    fn coverage_rate_rejects_malformed_lcov() {
        let documents = ["DA:1,1\nend_of_record\n"];
        assert!(rate(&documents, &[]).is_err());
    }

    #[test]
    fn coverage_rate_counts_wrapped_jvm_dotnet_sources() {
        for (path, source) in [
            ("src/A.java", "public class A {}\n"),
            ("src/A.kt", "fun f() = 1\n"),
            ("src/A.scala", "object A\n"),
            ("src/A.cs", "public static class A {}\n"),
            ("src/A.fs", "module A\n"),
            ("src/A.fsi", "module A\n"),
        ] {
            let document = format!("SF:{path}\nDA:1,1\nDA:2,0\nend_of_record\n");
            assert_eq!(
                rate(&[document.as_str()], &[(path, source)]),
                Ok((1, 2)),
                "{path}"
            );
        }
    }

    #[test]
    fn coverage_rate_honors_wrapped_lang_markers() {
        let documents = ["SF:src/A.java\nDA:1,1\nDA:2,0\nDA:3,0\nend_of_record\n"];
        let source = "public class A {\n// LCOV_EXCL_LINE - reason: generated.\n}\n";
        assert_eq!(rate(&documents, &[("src/A.java", source)]), Ok((1, 2)));
    }

    #[test]
    fn coverage_rate_counts_modern_js_ts_extensions() {
        for path in ["src/a.mjs", "src/a.cjs", "src/a.mts", "src/a.cts"] {
            let document = format!("SF:{path}\nDA:1,1\nDA:2,0\nend_of_record\n");
            assert_eq!(rate(&[document.as_str()], &[]), Ok((1, 2)), "{path}");
        }
    }
}
