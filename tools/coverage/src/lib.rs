//! M00 implementation-coverage gate.
//!
//! Bazel-owned enforcement for the resolved coverage policy over the M00
//! eligible scope. The gate parses the combined LCOV report from
//! `bazel coverage --combined_report=lcov`, validates source-level exclusion
//! markers carrying nearby `reason:` comments, reconciles the
//! repository-owned inventory against Bazel-declared sources, and requires
//! exact 100% covered-over-eligible executable lines. Only `DA` records
//! define executable lines; blank and comment-only lines are not executable.
//! Missing reports, unowned or absent sources, and any uncovered
//! non-excluded executable line fail the gate. Percentages are informational
//! only and never decide the verdict.
//!
//! Marker recognition is textual Rust `//` line comments only: a marker is
//! honored only when it appears after the `//` comment start and outside
//! string or char literals (byte-level scan honoring `"`/`'` and backslash
//! escapes). Markers inside block comments, raw strings, or lifetimes-heavy
//! lines are out of scope for M00; no M00 source uses those shapes. The
//! `reason:` lookup itself is a textual per-line match on the marker line or
//! the line directly above it, and the reason text after the colon must be
//! non-empty.

#![deny(warnings)]

use std::collections::BTreeMap;

/// Inventory disposition for authored first-party implementation.
pub const ELIGIBLE: &str = "eligible";
/// Inventory disposition for classified non-implementation (build
/// declarations, schemas, fixtures, tooling inputs). Never in the denominator.
pub const SUPPORT: &str = "support";

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
pub fn parse_lcov(report: &str) -> Result<BTreeMap<String, FileHits>, String> {
    let mut files: BTreeMap<String, FileHits> = BTreeMap::new();
    let mut current: Option<String> = None;
    for raw in report.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(path) = line.strip_prefix("SF:") {
            if path.is_empty() {
                return Err("LCOV record with empty SF path".to_string());
            }
            current = Some(path.to_string());
            files.entry(path.to_string()).or_default();
        } else if let Some(rest) = line.strip_prefix("DA:") {
            let path = current
                .clone()
                .ok_or_else(|| format!("LCOV DA record outside any SF record: {line}"))?;
            let mut parts = rest.split(',');
            let number_text = parts.next().unwrap_or_default();
            let lineno: u32 = number_text
                .parse()
                .map_err(|_| format!("malformed LCOV DA line number in {path}: {line}"))?;
            if lineno == 0 {
                return Err(format!("malformed LCOV DA line number in {path}: {line}"));
            }
            let hits_text = parts.next().unwrap_or_default();
            let hits: u64 = hits_text
                .parse()
                .map_err(|_| format!("malformed LCOV DA hit count in {path}: {line}"))?;
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

/// Validated source-level exclusions for one file.
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct Ignores {
    /// Singly excluded lines with their reason text.
    pub singles: BTreeMap<u32, String>,
    /// Excluded ranges (inclusive start/end) with the opening reason text.
    pub ranges: Vec<(u32, u32, String)>,
}

/// Whether `line` (1-based) is excluded by `ignores`.
pub fn is_ignored(ignores: &Ignores, line: u32) -> bool {
    if ignores.singles.contains_key(&line) {
        return true;
    }
    for range in &ignores.ranges {
        if range.0 <= line && line <= range.1 {
            return true;
        }
    }
    false
}

/// Non-empty reason text after `reason:` on `line`, if present.
fn reason_value(line: &str) -> Option<String> {
    let marker = line.find("reason:")?;
    let value = line[marker + "reason:".len()..].trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

/// Reason for `directive` at 1-based `lineno`: `reason:` with non-empty text
/// on the same line or the line directly above it.
fn nearby_reason(
    path: &str,
    directive: &str,
    lineno: usize,
    lines: &[&str],
) -> Result<String, String> {
    if let Some(reason) = reason_value(lines[lineno - 1]) {
        return Ok(reason);
    }
    if lineno >= 2 {
        if let Some(reason) = reason_value(lines[lineno - 2]) {
            return Ok(reason);
        }
    }
    Err(format!(
        "coverage ignore without nearby reason at {path}:{lineno}: {directive} requires a reason: comment on the same or previous line"
    ))
}

/// Comment text after the `//` comment start, honoring `"`/`'` literals and
/// backslash escapes. Returns `None` when the line has no line comment.
fn line_comment(line: &str) -> Option<&str> {
    let bytes = line.as_bytes();
    let mut index = 0;
    let mut in_string = false;
    let mut in_char = false;
    let mut escaped = false;
    while index < bytes.len() {
        let byte = bytes[index];
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
        } else if in_char {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'\'' {
                in_char = false;
            }
        } else if byte == b'"' {
            in_string = true;
        } else if byte == b'\'' {
            in_char = true;
        } else if byte == b'/' && index + 1 < bytes.len() && bytes[index + 1] == b'/' {
            return Some(&line[index + 2..]);
        }
        index += 1;
    }
    None
}

/// Whether `rest` (text right after the common marker prefix) is `word`
/// followed by a non-word character or end of text.
fn take_word(rest: &str, word: &str) -> bool {
    if let Some(tail) = rest.strip_prefix(word) {
        !tail.starts_with(|c: char| c == '_' || c.is_alphanumeric())
    } else {
        false
    }
}

/// Validate the exclusion markers in the Rust source `source` for `path`.
///
/// Every LINE/START/STOP directive needs a nearby non-empty `reason:`; ranges
/// must open and close exactly once; any other spelling of the marker prefix
/// is an unrecognized directive and fails. Markers are honored only inside
/// `//` comments outside literals (see [`line_comment`]).
pub fn find_ignores(path: &str, source: &str) -> Result<Ignores, String> {
    let lines: Vec<&str> = source.lines().collect();
    let mut ignores = Ignores::default();
    let mut open: Option<(usize, String)> = None;
    for (index, line) in lines.iter().enumerate() {
        let lineno = index + 1;
        let comment = line_comment(line).unwrap_or_default();
        for (pos, _) in comment.match_indices("LCOV_EXCL") {
            let rest = &comment[pos + "LCOV_EXCL".len()..];
            if take_word(rest, "_LINE") {
                let reason = nearby_reason(path, "LINE directive", lineno, &lines)?;
                ignores.singles.insert(lineno as u32, reason);
            } else if take_word(rest, "_START") {
                let reason = nearby_reason(path, "START directive", lineno, &lines)?;
                if open.is_some() {
                    return Err(format!("nested range START at {path}:{lineno}"));
                }
                open = Some((lineno, reason));
            } else if take_word(rest, "_STOP") {
                let reason = nearby_reason(path, "STOP directive", lineno, &lines)?;
                match open.take() {
                    Some((start, start_reason)) => {
                        ignores
                            .ranges
                            .push((start as u32, lineno as u32, start_reason));
                        let _ = reason;
                    }
                    None => {
                        return Err(format!("range STOP without START at {path}:{lineno}"));
                    }
                }
            } else {
                return Err(format!(
                    "unrecognized coverage ignore directive at {path}:{lineno}"
                ));
            }
        }
    }
    if let Some((start, _)) = open {
        return Err(format!("unclosed range START at {path}:{start}"));
    }
    Ok(ignores)
}

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
    /// Non-Rust instrumented sources observed in the report; counted nowhere.
    pub other_sources: Vec<String>,
}

fn is_covered_language(path: &str) -> bool {
	path.ends_with(".rs") || path.ends_with(".go")
}

fn is_starlark(path: &str) -> bool {
    path.ends_with(".bzl")
}

/// Evaluate one eligible file against its report hits.
fn check_file(
    path: &str,
    hits: &FileHits,
    load_source: &dyn Fn(&str) -> Result<String, String>,
    errors: &mut Vec<String>,
) -> Option<FileVerdict> {
    let source = match load_source(path) {
        Ok(text) => text,
        Err(message) => {
            errors.push(message);
            return None;
        }
    };
    let ignores = match find_ignores(path, &source) {
        Ok(valid) => valid,
        Err(message) => {
            errors.push(message);
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
/// sources. Support files with hits are skipped silently; non-Rust report
/// entries are listed separately and never merged into Rust counts, except
/// that Starlark entries carrying line data fail until a measurement route
/// and classification exist.
pub fn evaluate(
    inventory: &BTreeMap<String, String>,
    bazel_sources: &[String],
    report: &BTreeMap<String, FileHits>,
    load_source: &dyn Fn(&str) -> Result<String, String>,
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

/// Parse the inventory file: `<disposition> <repo-relative path>` per line;
/// blank lines and `#` comments are skipped.
pub fn parse_inventory(text: &str) -> Result<BTreeMap<String, String>, String> {
    let mut inventory = BTreeMap::new();
    for (index, raw) in text.lines().enumerate() {
        let lineno = index + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let disposition = parts.next().unwrap_or_default();
        let path = parts.next().unwrap_or_default();
        if disposition.is_empty() || path.is_empty() || parts.next().is_some() {
            return Err(format!("malformed inventory line {lineno}: {raw:?}"));
        }
        inventory.insert(path.to_string(), disposition.to_string());
    }
    Ok(inventory)
}

fn print_usage(print: &mut dyn FnMut(&str)) {
    print("usage: check --report <combined.lcov> --inventory <inventory.txt> --sources <sources.txt> [--root <dir>]");
}

/// Run the gate CLI. Returns 0 on pass, 1 on gate failure, 2 on usage or
/// configuration errors. Missing report *evidence* fails the gate (1);
/// unreadable inventory/sources configuration is a usage error (2).
pub fn run(
    args: &[String],
    read_file: &dyn Fn(&str) -> Result<String, String>,
    print: &mut dyn FnMut(&str),
) -> i32 {
    let mut report_path: Option<String> = None;
    let mut inventory_path: Option<String> = None;
    let mut sources_path: Option<String> = None;
    let mut root = ".".to_string();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        index += 1;
        let value = args.get(index).cloned();
        match flag {
            "--report" => report_path = value,
            "--inventory" => inventory_path = value,
            "--sources" => sources_path = value,
            "--root" => {
                if let Some(dir) = value {
                    root = dir;
                }
            }
            _ => {
                print(&format!("unknown argument: {flag}"));
                print_usage(print);
                return 2;
            }
        }
        index += 1;
    }
    let report_path = match report_path {
        Some(path) => path,
        None => {
            print("missing required --report <combined LCOV>");
            print_usage(print);
            return 2;
        }
    };
    let inventory_path = match inventory_path {
        Some(path) => path,
        None => {
            print("missing required --inventory <inventory file>");
            print_usage(print);
            return 2;
        }
    };
    let sources_path = match sources_path {
        Some(path) => path,
        None => {
            print("missing required --sources <Bazel source list>");
            print_usage(print);
            return 2;
        }
    };
    let report_text = match read_file(&report_path) {
        Ok(text) => text,
        Err(message) => {
            print(&format!(
                "coverage gate: FAIL\nmissing report file {report_path}: {message}"
            ));
            return 1;
        }
    };
    let report = match parse_lcov(&report_text) {
        Ok(parsed) => parsed,
        Err(message) => {
            print(&format!("coverage gate: FAIL\n{message}"));
            return 1;
        }
    };
    let inventory_text = match read_file(&inventory_path) {
        Ok(text) => text,
        Err(message) => {
            print(&format!(
                "unreadable inventory file {inventory_path}: {message}"
            ));
            return 2;
        }
    };
    let inventory = match parse_inventory(&inventory_text) {
        Ok(parsed) => parsed,
        Err(message) => {
            print(&format!(
                "invalid inventory file {inventory_path}: {message}"
            ));
            return 2;
        }
    };
    let sources_text = match read_file(&sources_path) {
        Ok(text) => text,
        Err(message) => {
            print(&format!(
                "unreadable Bazel source list {sources_path}: {message}"
            ));
            return 2;
        }
    };
    let mut bazel_sources = Vec::new();
    for raw in sources_text.lines() {
        let line = raw.trim();
        if !line.is_empty() && !line.starts_with('#') {
            bazel_sources.push(line.to_string());
        }
    }
    let verdict = evaluate(&inventory, &bazel_sources, &report, &|path| {
        let full = if root == "." {
            path.to_string()
        } else {
            format!("{root}/{path}")
        };
        read_file(&full).map_err(|message| format!("cannot read eligible source {full}: {message}"))
    });
    print(&render(&verdict));
    if verdict.passed {
        0
    } else {
        1
    }
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
    ) -> impl Fn(&str) -> Result<String, String> + 'a {
        move |path| {
            files
                .get(path)
                .cloned()
                .ok_or_else(|| format!("no such fixture: {path}"))
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
        assert!(err.contains("outside any SF"), "{err}");
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
    fn single_line_ignore_needs_same_line_reason() {
        let source = file_lines(&[
            "pub fn f() -> u32 {".to_string(),
            format!("    // {} - reason: fixture.", marker("_LINE")),
            "    1".to_string(),
            "}".to_string(),
        ]);
        let ignores = find_ignores("t.rs", &source).unwrap();
        assert_eq!(ignores.singles.len(), 1);
        assert!(ignores.singles.contains_key(&2));
        assert!(is_ignored(&ignores, 2));
        assert!(!is_ignored(&ignores, 1));
        assert!(!is_ignored(&ignores, 3));
    }

    #[test]
    fn single_line_ignore_accepts_previous_line_reason() {
        let source = file_lines(&[
            "    // reason: fixture explains the next line.".to_string(),
            format!("    // {}", marker("_LINE")),
            "    1".to_string(),
        ]);
        let ignores = find_ignores("t.rs", &source).unwrap();
        assert!(ignores.singles.contains_key(&2));
    }

    #[test]
    fn single_line_ignore_accepts_marker_at_end_of_line() {
        let source = file_lines(&[
            "    // reason: fixture.".to_string(),
            format!("    // {}", marker("_LINE")),
            "    1".to_string(),
        ]);
        assert!(find_ignores("t.rs", &source)
            .unwrap()
            .singles
            .contains_key(&2));
    }

    #[test]
    fn missing_reason_fails_on_first_line() {
        let source = file_lines(&[format!("// {}", marker("_LINE")), "fn f() {}".to_string()]);
        let err = find_ignores("t.rs", &source).unwrap_err();
        assert!(err.contains("t.rs:1") && err.contains("reason"), "{err}");
    }

    #[test]
    fn missing_reason_fails_with_unrelated_previous_line() {
        let source = file_lines(&[
            "fn f() {".to_string(),
            "    // nothing here.".to_string(),
            format!("    // {} - oops, no reason key.", marker("_LINE")),
            "}".to_string(),
        ]);
        assert!(find_ignores("t.rs", &source).is_err());
    }

    #[test]
    fn empty_reason_fails() {
        let source = file_lines(&[
            "    // reason:   ".to_string(),
            format!("    // {}", marker("_LINE")),
            "    1".to_string(),
        ]);
        assert!(find_ignores("t.rs", &source).is_err());
    }

    #[test]
    fn distant_reason_fails() {
        let source = file_lines(&[
            "    // reason: too far above.".to_string(),
            "    // filler.".to_string(),
            format!("    // {}", marker("_LINE")),
        ]);
        assert!(find_ignores("t.rs", &source).is_err());
    }

    #[test]
    fn range_excludes_interior_and_boundaries() {
        let source = file_lines(&[
            "fn f() {".to_string(),
            format!("    // {} - reason: range opens.", marker("_START")),
            "    1".to_string(),
            format!("    // {} - reason: range closes.", marker("_STOP")),
            "}".to_string(),
        ]);
        let ignores = find_ignores("t.rs", &source).unwrap();
        assert_eq!(ignores.ranges.len(), 1);
        assert!(is_ignored(&ignores, 2));
        assert!(is_ignored(&ignores, 3));
        assert!(is_ignored(&ignores, 4));
        assert!(!is_ignored(&ignores, 1));
        assert!(!is_ignored(&ignores, 5));
    }

    #[test]
    fn stop_without_reason_fails() {
        let source = file_lines(&[
            format!("// {} - reason: opens.", marker("_START")),
            "code();".to_string(),
            format!("// {}", marker("_STOP")),
        ]);
        assert!(find_ignores("t.rs", &source).is_err());
    }

    #[test]
    fn stop_without_start_fails() {
        let source = file_lines(&[format!("// {} - reason: stray stop.", marker("_STOP"))]);
        let err = find_ignores("t.rs", &source).unwrap_err();
        assert!(err.contains("without START"), "{err}");
    }

    #[test]
    fn nested_start_fails() {
        let source = file_lines(&[
            format!("// {} - reason: outer.", marker("_START")),
            format!("// {} - reason: inner.", marker("_START")),
            format!("// {} - reason: close.", marker("_STOP")),
            format!("// {} - reason: close.", marker("_STOP")),
        ]);
        let err = find_ignores("t.rs", &source).unwrap_err();
        assert!(err.contains("nested"), "{err}");
    }

    #[test]
    fn unclosed_start_fails() {
        let source = file_lines(&[
            "fn f() {".to_string(),
            format!("    // {} - reason: never closed.", marker("_START")),
            "}".to_string(),
        ]);
        let err = find_ignores("t.rs", &source).unwrap_err();
        assert!(err.contains("unclosed") && err.contains("t.rs:2"), "{err}");
    }

    #[test]
    fn unrecognized_suffix_fails() {
        let source = file_lines(&[format!("// {} - reason: typo.", marker("_RANGE"))]);
        let err = find_ignores("t.rs", &source).unwrap_err();
        assert!(err.contains("unrecognized"), "{err}");
    }

    #[test]
    fn bare_prefix_fails() {
        let source = file_lines(&[format!("// {} - reason: bare.", marker(""))]);
        assert!(find_ignores("t.rs", &source).is_err());
    }

    #[test]
    fn word_boundary_suffix_fails() {
        let source = file_lines(&[format!("// {}X - reason: glued.", marker("_LINE"))]);
        assert!(find_ignores("t.rs", &source).is_err());
    }

    #[test]
    fn markers_inside_string_literals_are_ignored() {
        let tricky = format!("let s = \"code with // {} inside\";", marker("_LINE"));
        let source = file_lines(&[tricky, "real();".to_string()]);
        let ignores = find_ignores("t.rs", &source).unwrap();
        assert!(ignores.singles.is_empty());
        assert!(ignores.ranges.is_empty());
    }

    #[test]
    fn lexer_survives_escapes_and_char_literals() {
        let source = file_lines(&[
            "let s = \"a\\\"b\";".to_string(),
            "let q = '\\'';".to_string(),
            "let c = '/';".to_string(),
            "code();".to_string(),
        ]);
        assert!(find_ignores("t.rs", &source).unwrap().singles.is_empty());
    }

    #[test]
    fn marker_after_string_state_is_recognized() {
        let source = file_lines(&[
            "let s = \"a\\\"b\";".to_string(),
            format!("// {} - reason: after strings.", marker("_LINE")),
            "code();".to_string(),
        ]);
        let ignores = find_ignores("t.rs", &source).unwrap();
        assert!(ignores.singles.contains_key(&2));
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

    #[test]
    fn parses_inventory_with_comments_and_blanks() {
        let inventory = parse_inventory("# comment\n\neligible a.rs\nsupport b.rs  \n").unwrap();
        assert_eq!(inventory["a.rs"], ELIGIBLE);
        assert_eq!(inventory["b.rs"], SUPPORT);
    }

    #[test]
    fn rejects_malformed_inventory_lines() {
        assert!(parse_inventory("eligible\n").is_err());
        assert!(parse_inventory("eligible a.rs extra\n").is_err());
        assert!(parse_inventory("   \n lone\n").is_err());
    }

    fn run_harness(stored: BTreeMap<&str, &str>, args: &[&str]) -> (i32, Vec<String>) {
        let owned: BTreeMap<String, String> = stored
            .into_iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        let owned_args: Vec<String> = args.iter().map(|arg| arg.to_string()).collect();
        let mut printed = Vec::new();
        let code = run(
            &owned_args,
            &|path| {
                owned
                    .get(path)
                    .cloned()
                    .ok_or_else(|| format!("missing file: {path}"))
            },
            &mut |line| printed.push(line.to_string()),
        );
        (code, printed)
    }

    fn passing_store() -> BTreeMap<&'static str, &'static str> {
        BTreeMap::from([
            ("report.info", "SF:elf.rs\nDA:1,1\nend_of_record\n"),
            ("inventory.txt", "eligible elf.rs\n"),
            ("sources.txt", "elf.rs\n"),
            ("elf.rs", "fn f() {}\n"),
        ])
    }

    #[test]
    fn run_passes_on_covered_inventory() {
        let (code, printed) = run_harness(
            passing_store(),
            &[
                "--report",
                "report.info",
                "--inventory",
                "inventory.txt",
                "--sources",
                "sources.txt",
            ],
        );
        assert_eq!(code, 0);
        assert!(
            printed.iter().any(|line| line.contains("PASS 1/1")),
            "{printed:?}"
        );
    }

    #[test]
    fn run_honors_explicit_root() {
        let (code, printed) = run_harness(
            passing_store(),
            &[
                "--report",
                "report.info",
                "--inventory",
                "inventory.txt",
                "--sources",
                "sources.txt",
                "--root",
                "ws",
            ],
        );
        assert_eq!(code, 1, "{printed:?}");
        assert!(
            printed.iter().any(|line| line.contains("ws/elf.rs")),
            "{printed:?}"
        );
    }

    #[test]
    fn run_fails_on_uncovered_lines() {
        let mut stored = passing_store();
        stored.insert("report.info", "SF:elf.rs\nDA:1,0\nend_of_record\n");
        let (code, printed) = run_harness(
            stored,
            &[
                "--report",
                "report.info",
                "--inventory",
                "inventory.txt",
                "--sources",
                "sources.txt",
            ],
        );
        assert_eq!(code, 1);
        assert!(
            printed.iter().any(|line| line.contains("FAIL")),
            "{printed:?}"
        );
    }

    #[test]
    fn run_rejects_unknown_arguments() {
        let (code, printed) = run_harness(passing_store(), &["--bogus"]);
        assert_eq!(code, 2);
        assert!(
            printed.iter().any(|line| line.contains("usage")),
            "{printed:?}"
        );
    }

    #[test]
    fn run_rejects_dangling_flag_value() {
        let (code, _) = run_harness(passing_store(), &["--report"]);
        assert_eq!(code, 2);
    }

    #[test]
    fn run_requires_report_inventory_and_sources() {
        let full = [
            "--report",
            "report.info",
            "--inventory",
            "inventory.txt",
            "--sources",
            "sources.txt",
        ];
        let (code, _) = run_harness(passing_store(), &full[2..]);
        assert_eq!(code, 2);
        let (code, _) = run_harness(passing_store(), &[full[0], full[1], full[4], full[5]]);
        assert_eq!(code, 2);
        let (code, _) = run_harness(passing_store(), &[full[0], full[1], full[2], full[3]]);
        assert_eq!(code, 2);
    }

    #[test]
    fn run_reports_missing_report_file_as_gate_failure() {
        let (code, printed) = run_harness(
            BTreeMap::new(),
            &[
                "--report",
                "gone.info",
                "--inventory",
                "inventory.txt",
                "--sources",
                "sources.txt",
            ],
        );
        assert_eq!(code, 1);
        assert!(
            printed
                .iter()
                .any(|line| line.contains("missing report file")),
            "{printed:?}"
        );
    }

    #[test]
    fn run_reports_malformed_report_as_gate_failure() {
        let mut stored = passing_store();
        stored.insert("report.info", "SF:elf.rs\nDA:0,1\nend_of_record\n");
        let (code, _) = run_harness(
            stored,
            &[
                "--report",
                "report.info",
                "--inventory",
                "inventory.txt",
                "--sources",
                "sources.txt",
            ],
        );
        assert_eq!(code, 1);
    }

    #[test]
    fn run_rejects_bad_inventory_configuration() {
        let mut stored = passing_store();
        stored.insert("inventory.txt", "eligible\n");
        let (code, printed) = run_harness(
            stored,
            &[
                "--report",
                "report.info",
                "--inventory",
                "inventory.txt",
                "--sources",
                "sources.txt",
            ],
        );
        assert_eq!(code, 2, "{printed:?}");
        let mut stored = passing_store();
        stored.remove("inventory.txt");
        let (code, _) = run_harness(
            stored,
            &[
                "--report",
                "report.info",
                "--inventory",
                "inventory.txt",
                "--sources",
                "sources.txt",
            ],
        );
        assert_eq!(code, 2);
    }

    #[test]
    fn run_rejects_missing_sources_list() {
        let mut stored = passing_store();
        stored.remove("sources.txt");
        let (code, printed) = run_harness(
            stored,
            &[
                "--report",
                "report.info",
                "--inventory",
                "inventory.txt",
                "--sources",
                "sources.txt",
            ],
        );
        assert_eq!(code, 2, "{printed:?}");
    }

    #[test]
    fn run_ignores_dangling_root_flag() {
        let (code, _) = run_harness(
            passing_store(),
            &[
                "--report",
                "report.info",
                "--inventory",
                "inventory.txt",
                "--sources",
                "sources.txt",
                "--root",
            ],
        );
        assert_eq!(code, 0);
    }
}
