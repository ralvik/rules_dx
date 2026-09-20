//! Gate verdict for the coverage gate.
//!
//! Split from `super` (`lib.rs`): owns [`FileVerdict`], [`GateVerdict`],
//! [`is_covered_language`], [`evaluate`], and [`render`] plus the
//! per-file checker (`check_file`). Re-exported through `super` so the
//! public paths stay `dx_lcov::{FileVerdict, GateVerdict,
//! is_covered_language, evaluate, render}`. Distinct from the `parse`
//! module (combined-LCOV parsing), the `ignores` module (source-level
//! exclusion markers), and the `inventory`/`run` modules (repo inventory
//! and CLI).

use std::collections::BTreeMap;

use super::{find_ignores, is_ignored, FileHits, LcovError, ELIGIBLE, SUPPORT};

/// Per-file verdict with exact counts and uncovered locations.
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct FileVerdict {
    pub path: String,
    pub covered: u64,
    pub eligible: u64,
    pub uncovered: Vec<u32>,
    pub ignored: u64,
}

/// Whole-gate verdict. `passed` is true only with zero errors, zero uncovered
/// lines, and a non-empty denominator.
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct GateVerdict {
    pub files: Vec<FileVerdict>,
    pub covered: u64,
    pub eligible: u64,
    pub passed: bool,
    pub errors: Vec<String>,
    /// Instrumented sources outside the covered languages, observed in the
    /// report but counted nowhere.
    pub other_sources: Vec<String>,
}

/// Whether LCOV `SF` records for `path` carry gated line data. Rust and Go
/// use the pinned Bazel llvm-cov/go integrations; Python and
/// JavaScript/TypeScript participate in `bazel coverage` through the
/// repo's pytest/jest wrappers (see `docs/testing/generation.md`). Any
/// other extension lands in `other_sources` and counts nowhere; Starlark
/// line data stays a hard error until the M00 measurement route exists.
pub fn is_covered_language(path: &str) -> bool {
    path.ends_with(".rs")
        || path.ends_with(".go")
        || path.ends_with(".py")
        || path.ends_with(".js")
        || path.ends_with(".jsx")
        || path.ends_with(".ts")
        || path.ends_with(".tsx")
}

fn is_starlark(path: &str) -> bool {
    path.ends_with(".bzl")
}

/// Evaluate one eligible file against its report hits.
fn check_file(
    path: &str,
    hits: &FileHits,
    load_source: &dyn Fn(&str) -> Result<String, LcovError>,
    errors: &mut Vec<String>,
) -> Option<FileVerdict> {
    let source = match load_source(path) {
        Ok(text) => text,
        Err(error) => {
            errors.push(error.to_string());
            return None;
        }
    };
    let ignores = match find_ignores(path, &source) {
        Ok(valid) => valid,
        Err(message) => {
            errors.push(message.to_string());
            return None;
        }
    };
    let mut covered = 0;
    let mut eligible = 0;
    let mut ignored = 0;
    let mut uncovered = Vec::new();
    let mut numbered: Vec<u32> = hits.lines.keys().copied().collect();
    numbered.sort();
    for line in numbered {
        if is_ignored(&ignores, line) {
            ignored += 1;
        } else if hits.lines[&line] > 0 {
            covered += 1;
            eligible += 1;
        } else {
            uncovered.push(line);
            eligible += 1;
        }
    }
    if hits.lines.is_empty() && ignores.singles.is_empty() && ignores.ranges.is_empty() {
        errors.push(format!(
            "no instrumented lines and no validated ignores for eligible source: {path}"
        ));
    }
    Some(FileVerdict {
        path: path.to_string(),
        covered,
        eligible,
        uncovered,
        ignored,
    })
}

/// Evaluate the gate.
///
/// `inventory` maps repo-owned paths to [`ELIGIBLE`] or [`SUPPORT`];
/// `bazel_sources` lists the repo-owned `*.rs`/`*.bzl` files declared by
/// Bazel; `report` is the parsed combined LCOV; `load_source` reads workspace
/// sources. Support files with hits are skipped silently; report entries
/// outside the covered languages are listed separately and never merged
/// into gated counts, except
/// that Starlark entries carrying line data fail until a measurement route
/// and classification exist.
pub fn evaluate(
    inventory: &BTreeMap<String, String>,
    bazel_sources: &[String],
    report: &BTreeMap<String, FileHits>,
    load_source: &dyn Fn(&str) -> Result<String, LcovError>,
) -> GateVerdict {
    let mut verdict = GateVerdict::default();
    if report.is_empty() {
        verdict.errors.push(
            "missing coverage report: no SF records; run bazel coverage //... --combined_report=lcov first"
                .to_string(),
        );
    }
    for (path, hits) in report {
        if is_covered_language(path) {
            match inventory.get(path.as_str()) {
                None => verdict
                    .errors
                    .push(format!("instrumented source not in inventory: {path}")),
                Some(disposition) if disposition == SUPPORT => {}
                Some(disposition) if disposition == ELIGIBLE => {
                    if let Some(file) = check_file(path, hits, load_source, &mut verdict.errors) {
                        verdict.files.push(file);
                    }
                }
                Some(disposition) => verdict.errors.push(format!(
                    "unknown disposition {disposition:?} for {path}; want \"eligible\" or \"support\""
                )),
            }
        } else {
            verdict.other_sources.push(path.clone());
            if is_starlark(path) && !hits.lines.is_empty() {
                verdict.errors.push(format!(
                    "unexpected Starlark line data for {path}: no Starlark line route exists in M00"
                ));
            }
        }
    }
    for source in bazel_sources {
        if !inventory.contains_key(source) {
            verdict.errors.push(format!(
                "Bazel-declared source missing from inventory: {source}"
            ));
        }
    }
    for (path, disposition) in inventory {
        if disposition == ELIGIBLE {
            if !bazel_sources.contains(path) {
                verdict.errors.push(format!(
                    "inventory eligible source not declared by Bazel: {path}"
                ));
            }
            if !report.contains_key(path) {
                verdict.errors.push(format!(
                    "eligible source absent from coverage report: {path}"
                ));
            }
        } else if disposition != SUPPORT {
            verdict.errors.push(format!(
                "unknown disposition {disposition:?} for {path}; want \"eligible\" or \"support\""
            ));
        }
    }
    for file in &verdict.files {
        verdict.covered += file.covered;
        verdict.eligible += file.eligible;
    }
    let mut clean = verdict.errors.is_empty();
    for file in &verdict.files {
        if !file.uncovered.is_empty() {
            clean = false;
        }
    }
    if clean && verdict.eligible > 0 {
        verdict.passed = true;
    } else if clean {
        verdict
            .errors
            .push("no executable lines in scope: an empty denominator is never a pass".to_string());
    }
    verdict
}

/// Render the verdict with exact counts, uncovered locations, and errors.
/// The informational rate never decides; only exact counts do.
pub fn render(verdict: &GateVerdict) -> String {
    let mut out = String::new();
    if verdict.passed {
        out.push_str(&format!(
            "coverage gate: PASS {}/{} executable lines\n",
            verdict.covered, verdict.eligible
        ));
    } else {
        out.push_str(&format!(
            "coverage gate: FAIL {}/{} executable lines\n",
            verdict.covered, verdict.eligible
        ));
    }
    for file in &verdict.files {
        if file.uncovered.is_empty() {
            out.push_str(&format!(
                "  {}: {}/{} ({} ignored)\n",
                file.path, file.covered, file.eligible, file.ignored
            ));
        } else {
            let locations: Vec<String> = file
                .uncovered
                .iter()
                .map(|line| format!("{}:{line}", file.path))
                .collect();
            out.push_str(&format!(
                "  {}: {}/{} uncovered: {}\n",
                file.path,
                file.covered,
                file.eligible,
                locations.join(", ")
            ));
        }
    }
    if !verdict.errors.is_empty() {
        out.push_str("errors:\n");
        for error in &verdict.errors {
            out.push_str(&format!("  - {error}\n"));
        }
    }
    if !verdict.other_sources.is_empty() {
        out.push_str("other instrumented sources (not counted):\n");
        for source in &verdict.other_sources {
            out.push_str(&format!("  - {source}\n"));
        }
    }
    if verdict.eligible > 0 {
        let rate = verdict.covered as f64 * 100.0 / verdict.eligible as f64;
        out.push_str(&format!(
            "informational line rate: {rate:.2}% (exact counts decide, never rounding)\n"
        ));
    } else {
        out.push_str("informational line rate: n/a (no eligible lines)\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a marker suffix without spelling the contiguous literal in this
    /// file: the gate scans its own sources, so test data must not contribute
    /// directives. Every marker below lives inside string literals, which the
    /// comment scanner ignores.
    fn marker(kind: &str) -> String {
        ["LCOV", "_EXCL", kind].concat()
    }

    fn file_lines(lines: &[String]) -> String {
        let mut out = lines.join("\n");
        out.push('\n');
        out
    }

    fn hits(pairs: &[(u32, u64)]) -> FileHits {
        let mut file = FileHits::default();
        for (line, count) in pairs {
            file.lines.insert(*line, *count);
        }
        file
    }

    fn report_of(path: &str, pairs: &[(u32, u64)]) -> BTreeMap<String, FileHits> {
        let mut report = BTreeMap::new();
        report.insert(path.to_string(), hits(pairs));
        report
    }

    fn loader<'a>(
        files: BTreeMap<&'a str, String>,
    ) -> impl Fn(&str) -> Result<String, LcovError> + 'a {
        move |path| {
            files.get(path).cloned().ok_or_else(|| LcovError::Io {
                message: format!("no such fixture: {path}"),
            })
        }
    }

    fn eligible_inventory(paths: &[&str]) -> BTreeMap<String, String> {
        let mut inventory = BTreeMap::new();
        for path in paths {
            inventory.insert(path.to_string(), ELIGIBLE.to_string());
        }
        inventory
    }

    #[test]
    fn valid_ignore_shrinks_denominator_only_for_its_line() {
        let source = file_lines(&[
            "pub fn classify(n: u32) -> u32 {".to_string(),
            "    if n == 0 {".to_string(),
            format!(
                "        return 0; // {} - reason: fixture defensive branch.",
                marker("_LINE")
            ),
            "    }".to_string(),
            "    n".to_string(),
            "}".to_string(),
        ]);
        let files = BTreeMap::from([("elf.rs", source)]);
        let verdict = evaluate(
            &eligible_inventory(&["elf.rs"]),
            &["elf.rs".to_string()],
            &report_of("elf.rs", &[(1, 1), (2, 1), (3, 0), (5, 1)]),
            &loader(files),
        );
        assert!(verdict.passed, "{verdict:?}");
        assert_eq!(verdict.covered, 3);
        assert_eq!(verdict.eligible, 3);
        assert_eq!(verdict.files[0].ignored, 1);
    }

    #[test]
    fn uncovered_line_fails_with_location() {
        let files = BTreeMap::from([("elf.rs", "fn f() {\n    1\n}\n".to_string())]);
        let verdict = evaluate(
            &eligible_inventory(&["elf.rs"]),
            &["elf.rs".to_string()],
            &report_of("elf.rs", &[(1, 1), (2, 0)]),
            &loader(files),
        );
        assert!(!verdict.passed);
        assert_eq!(verdict.files[0].uncovered, vec![2]);
        let text = render(&verdict);
        assert!(text.contains("FAIL") && text.contains("elf.rs:2"), "{text}");
    }

    #[test]
    fn python_valid_ignore_shrinks_denominator() {
        let source = file_lines(&[
            "def classify(n):".to_string(),
            "    if n == 0:".to_string(),
            format!(
                "        return 0  # {} - reason: fixture defensive branch.",
                marker("_LINE")
            ),
            "    return n".to_string(),
        ]);
        let files = BTreeMap::from([("elf.py", source)]);
        let verdict = evaluate(
            &eligible_inventory(&["elf.py"]),
            &["elf.py".to_string()],
            &report_of("elf.py", &[(1, 1), (2, 1), (3, 0), (4, 1)]),
            &loader(files),
        );
        assert!(verdict.passed, "{verdict:?}");
        assert_eq!(verdict.covered, 3);
        assert_eq!(verdict.eligible, 3);
        assert_eq!(verdict.files[0].ignored, 1);
    }

    #[test]
    fn python_routes_to_files_not_other_sources() {
        let files = BTreeMap::from([("elf.py", "x = 1\n".to_string())]);
        let verdict = evaluate(
            &eligible_inventory(&["elf.py"]),
            &["elf.py".to_string()],
            &report_of("elf.py", &[(1, 1)]),
            &loader(files),
        );
        assert!(verdict.passed, "{verdict:?}");
        assert!(verdict.other_sources.is_empty());
        assert_eq!(verdict.files.len(), 1);
    }

    #[test]
    fn python_absent_eligible_source_fails() {
        let files = BTreeMap::from([("elf.py", "x = 1\n".to_string())]);
        let verdict = evaluate(
            &eligible_inventory(&["elf.py"]),
            &["elf.py".to_string()],
            &BTreeMap::new(),
            &loader(files),
        );
        assert!(!verdict.passed);
        assert!(
            verdict.errors.iter().any(|e| e.contains("absent")),
            "{verdict:?}"
        );
    }

    #[test]
    fn python_uncovered_line_fails_with_location() {
        let files = BTreeMap::from([("elf.py", "x = 1\ny = 2\n".to_string())]);
        let verdict = evaluate(
            &eligible_inventory(&["elf.py"]),
            &["elf.py".to_string()],
            &report_of("elf.py", &[(1, 1), (2, 0)]),
            &loader(files),
        );
        assert!(!verdict.passed);
        assert_eq!(verdict.files[0].uncovered, vec![2]);
        let text = render(&verdict);
        assert!(text.contains("FAIL") && text.contains("elf.py:2"), "{text}");
    }

    #[test]
    fn javascript_routes_to_files_with_slash_markers() {
        let source = file_lines(&[
            "export function f() {".to_string(),
            format!(
                "  return 0; // {} - reason: fixture defensive branch.",
                marker("_LINE")
            ),
            "}".to_string(),
        ]);
        let files = BTreeMap::from([("elf.js", source)]);
        let verdict = evaluate(
            &eligible_inventory(&["elf.js"]),
            &["elf.js".to_string()],
            &report_of("elf.js", &[(1, 1), (2, 0), (3, 1)]),
            &loader(files),
        );
        assert!(verdict.passed, "{verdict:?}");
        assert!(verdict.other_sources.is_empty());
        assert_eq!(verdict.covered, 2);
        assert_eq!(verdict.eligible, 2);
        assert_eq!(verdict.files[0].ignored, 1);
    }

    #[test]
    fn absent_eligible_source_fails() {
        let files = BTreeMap::from([("elf.rs", "fn f() {}\n".to_string())]);
        let verdict = evaluate(
            &eligible_inventory(&["elf.rs"]),
            &["elf.rs".to_string()],
            &BTreeMap::new(),
            &loader(files),
        );
        assert!(!verdict.passed);
        assert!(
            verdict.errors.iter().any(|e| e.contains("absent")),
            "{verdict:?}"
        );
        assert!(
            verdict.errors.iter().any(|e| e.contains("no SF records")),
            "{verdict:?}"
        );
    }

    #[test]
    fn unowned_bazel_source_fails() {
        let files = BTreeMap::from([("elf.rs", "fn f() {}\n".to_string())]);
        let verdict = evaluate(
            &eligible_inventory(&["elf.rs"]),
            &["elf.rs".to_string(), "ghost.rs".to_string()],
            &report_of("elf.rs", &[(1, 1)]),
            &loader(files),
        );
        assert!(!verdict.passed);
        assert!(
            verdict.errors.iter().any(|e| e.contains("ghost.rs")),
            "{verdict:?}"
        );
    }

    #[test]
    fn stale_eligible_inventory_fails() {
        let files = BTreeMap::from([("elf.rs", "fn f() {}\n".to_string())]);
        let verdict = evaluate(
            &eligible_inventory(&["elf.rs", "stale.rs"]),
            &["elf.rs".to_string()],
            &report_of("elf.rs", &[(1, 1)]),
            &loader(files),
        );
        assert!(!verdict.passed);
        assert!(
            verdict.errors.iter().any(|e| e.contains("stale.rs")),
            "{verdict:?}"
        );
    }

    #[test]
    fn unknown_disposition_fails() {
        let mut inventory = BTreeMap::new();
        inventory.insert("elf.rs".to_string(), "maybe".to_string());
        let files = BTreeMap::from([("elf.rs", "fn f() {}\n".to_string())]);
        let verdict = evaluate(
            &inventory,
            &["elf.rs".to_string()],
            &report_of("elf.rs", &[(1, 1)]),
            &loader(files),
        );
        assert!(!verdict.passed);
        assert!(
            verdict
                .errors
                .iter()
                .any(|e| e.contains("unknown disposition")),
            "{verdict:?}"
        );
    }

    #[test]
    fn zero_da_lines_without_ignores_fails() {
        let files = BTreeMap::from([("elf.rs", "fn f() {}\n".to_string())]);
        let verdict = evaluate(
            &eligible_inventory(&["elf.rs"]),
            &["elf.rs".to_string()],
            &report_of("elf.rs", &[]),
            &loader(files),
        );
        assert!(!verdict.passed);
        assert!(
            verdict
                .errors
                .iter()
                .any(|e| e.contains("no instrumented lines")),
            "{verdict:?}"
        );
    }

    #[test]
    fn fully_ignored_zero_da_file_passes_with_zero_counts() {
        let source = file_lines(&[
            format!("// {} - reason: shim.", marker("_START")),
            "fn main() {}".to_string(),
            format!("// {} - reason: shim end.", marker("_STOP")),
        ]);
        let live = file_lines(&["fn f() {}".to_string()]);
        let files = BTreeMap::from([("shim.rs", source), ("elf.rs", live)]);
        let mut report = report_of("elf.rs", &[(1, 2)]);
        report.insert("shim.rs".to_string(), FileHits::default());
        let verdict = evaluate(
            &eligible_inventory(&["elf.rs", "shim.rs"]),
            &["elf.rs".to_string(), "shim.rs".to_string()],
            &report,
            &loader(files),
        );
        assert!(verdict.passed, "{verdict:?}");
        assert_eq!(verdict.covered, 1);
        assert_eq!(verdict.eligible, 1);
    }

    #[test]
    fn empty_denominator_is_never_a_pass() {
        let source = file_lines(&[
            format!("// {} - reason: shim.", marker("_START")),
            "fn main() {}".to_string(),
            format!("// {} - reason: shim end.", marker("_STOP")),
        ]);
        let files = BTreeMap::from([("shim.rs", source)]);
        let mut report = BTreeMap::new();
        report.insert("shim.rs".to_string(), FileHits::default());
        let verdict = evaluate(
            &eligible_inventory(&["shim.rs"]),
            &["shim.rs".to_string()],
            &report,
            &loader(files),
        );
        assert!(!verdict.passed);
        assert!(
            verdict
                .errors
                .iter()
                .any(|e| e.contains("empty denominator")),
            "{verdict:?}"
        );
        let text = render(&verdict);
        assert!(text.contains("n/a (no eligible lines)"), "{text}");
    }

    #[test]
    fn support_hits_are_skipped_and_others_listed() {
        let live = "fn f() {}\n".to_string();
        let files = BTreeMap::from([("elf.rs", live)]);
        let mut inventory = eligible_inventory(&["elf.rs"]);
        inventory.insert("tool.rs".to_string(), SUPPORT.to_string());
        let mut report = report_of("elf.rs", &[(1, 1)]);
        report.insert("tool.rs".to_string(), hits(&[(1, 0)]));
        report.insert("notes.cc".to_string(), hits(&[(7, 1)]));
        let verdict = evaluate(&inventory, &["elf.rs".to_string()], &report, &loader(files));
        assert!(verdict.passed, "{verdict:?}");
        assert_eq!(verdict.covered, 1);
        assert_eq!(verdict.eligible, 1);
        assert_eq!(verdict.other_sources, vec!["notes.cc".to_string()]);
        let text = render(&verdict);
        assert!(
            text.contains("PASS") && text.contains("not counted"),
            "{text}"
        );
    }

    #[test]
    fn uninventoried_rust_in_report_fails() {
        let files = BTreeMap::from([("elf.rs", "fn f() {}\n".to_string())]);
        let mut report = report_of("elf.rs", &[(1, 1)]);
        report.insert("rogue.rs".to_string(), hits(&[(1, 1)]));
        let verdict = evaluate(
            &eligible_inventory(&["elf.rs"]),
            &["elf.rs".to_string()],
            &report,
            &loader(files),
        );
        assert!(!verdict.passed);
        assert!(
            verdict.errors.iter().any(|e| e.contains("rogue.rs")),
            "{verdict:?}"
        );
    }

    #[test]
    fn starlark_line_data_fails_until_route_exists() {
        let files = BTreeMap::from([("elf.rs", "fn f() {}\n".to_string())]);
        let mut report = report_of("elf.rs", &[(1, 1)]);
        report.insert("defs.bzl".to_string(), hits(&[(3, 1)]));
        let verdict = evaluate(
            &eligible_inventory(&["elf.rs"]),
            &["elf.rs".to_string()],
            &report,
            &loader(files),
        );
        assert!(!verdict.passed);
        assert!(
            verdict.errors.iter().any(|e| e.contains("defs.bzl")),
            "{verdict:?}"
        );
    }

    #[test]
    fn starlark_entry_without_lines_is_listed_only() {
        let files = BTreeMap::from([("elf.rs", "fn f() {}\n".to_string())]);
        let mut report = report_of("elf.rs", &[(1, 1)]);
        report.insert("defs.bzl".to_string(), FileHits::default());
        let verdict = evaluate(
            &eligible_inventory(&["elf.rs"]),
            &["elf.rs".to_string()],
            &report,
            &loader(files),
        );
        assert!(verdict.passed, "{verdict:?}");
        assert_eq!(verdict.other_sources, vec!["defs.bzl".to_string()]);
    }

    #[test]
    fn unreadable_source_fails() {
        let verdict = evaluate(
            &eligible_inventory(&["elf.rs"]),
            &["elf.rs".to_string()],
            &report_of("elf.rs", &[(1, 1)]),
            &loader(BTreeMap::new()),
        );
        assert!(!verdict.passed);
        assert!(
            verdict.errors.iter().any(|e| e.contains("no such fixture")),
            "{verdict:?}"
        );
    }

    #[test]
    fn invalid_source_markers_fail_evaluation() {
        let source = file_lines(&[format!("// {} - reason: stray stop.", marker("_STOP"))]);
        let files = BTreeMap::from([("elf.rs", source)]);
        let verdict = evaluate(
            &eligible_inventory(&["elf.rs"]),
            &["elf.rs".to_string()],
            &report_of("elf.rs", &[(1, 1)]),
            &loader(files),
        );
        assert!(!verdict.passed);
        assert!(
            verdict.errors.iter().any(|e| e.contains("without START")),
            "{verdict:?}"
        );
    }
}
