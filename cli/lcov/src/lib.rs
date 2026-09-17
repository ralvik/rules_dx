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
//! Marker recognition is textual per-extension comment syntax: `//` line
//! comments for C-like sources, `#` line comments for Python, Starlark,
//! TOML, shell, and YAML, and `<!-- ... -->` segments for Markdown and
//! HTML. A marker is honored only inside its language's comments and
//! outside string or char literals (byte-level scan honoring `"`/`'`
//! and backslash escapes). Markers inside block comments or raw strings
//! are out of scope; no eligible source uses those shapes. The `reason:`
//! lookup itself is a textual per-line match on the marker line or the
//! line directly above it, and the reason text after the colon must be
//! non-empty.
//!
//! Domain split (issue #236): combined-LCOV parsing (`FileHits`,
//! `parse_lcov`) lives in the `parse` module, source-level exclusion
//! markers (`Ignores`, `is_ignored`, `find_ignores`) live in the `ignores`
//! module, and gate evaluation (`FileVerdict`, `GateVerdict`,
//! `is_covered_language`, `evaluate`, `render`) lives in the `verdict`
//! module. This facade keeps the shared error and inventory dispositions;
//! the public paths stay `dx_lcov::{FileHits, parse_lcov, Ignores,
//! is_ignored, find_ignores, FileVerdict, GateVerdict,
//! is_covered_language, evaluate, render}` via the re-exports below.

// Issue #238: infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

use std::collections::BTreeMap;

pub mod ignores;
pub mod parse;
pub mod verdict;

pub use ignores::{find_ignores, is_ignored, Ignores};
pub use parse::{parse_lcov, FileHits};
pub use verdict::{evaluate, is_covered_language, render, FileVerdict, GateVerdict};

/// Typed LCOV gate failure (issue #230).
///
/// Every variant renders byte-identical to the historical `String` error
/// it replaces, so CLI operational diagnostics stay stable while callers
/// gain matchable structure instead of `format!` string plumbing.
/// Binary edges keep rendering via `Display` (`to_string()`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LcovError {
    /// `SF:` record carries no path.
    #[error("LCOV record with empty SF path")]
    EmptySfPath,
    /// `DA:` record appears before any `SF:` record.
    #[error("LCOV DA record outside any SF record: {line}")]
    DaOutsideSf { line: String },
    /// `DA:` line number is not a positive integer.
    #[error("malformed LCOV DA line number in {path}: {line}")]
    MalformedLineNumber { path: String, line: String },
    /// `DA:` hit count is not an integer.
    #[error("malformed LCOV DA hit count in {path}: {line}")]
    MalformedHitCount { path: String, line: String },
    /// Exclusion directive lacks a nearby non-empty `reason:`.
    #[error("coverage ignore without nearby reason at {path}:{lineno}: {directive} requires a reason: comment on the same or previous line")]
    MissingReason {
        path: String,
        lineno: usize,
        directive: String,
    },
    /// `START` opens while another range is open.
    #[error("nested range START at {path}:{lineno}")]
    NestedStart { path: String, lineno: usize },
    /// `STOP` closes with no open range.
    #[error("range STOP without START at {path}:{lineno}")]
    StopWithoutStart { path: String, lineno: usize },
    /// Marker prefix spells no known directive.
    #[error("unrecognized coverage ignore directive at {path}:{lineno}")]
    UnrecognizedDirective { path: String, lineno: usize },
    /// `START` never closes.
    #[error("unclosed range START at {path}:{start}")]
    UnclosedStart { path: String, start: usize },
    /// Inventory line is not `<disposition> <path>`.
    #[error("malformed inventory line {lineno}: {raw:?}")]
    MalformedInventory { lineno: usize, raw: String },
    /// Injected file read failed; carries the reader's message verbatim
    /// so gate output stays byte-identical.
    #[error("{message}")]
    Io { message: String },
}

impl From<String> for LcovError {
    fn from(message: String) -> Self {
        Self::Io { message }
    }
}

/// Inventory disposition for authored first-party implementation.
pub const ELIGIBLE: &str = "eligible";
/// Inventory disposition for classified non-implementation (build
/// declarations, schemas, fixtures, tooling inputs). Never in the denominator.
pub const SUPPORT: &str = "support";

/// Parse the inventory file: `<disposition> <repo-relative path>` per line;
/// blank lines and `#` comments are skipped.
pub fn parse_inventory(text: &str) -> Result<BTreeMap<String, String>, LcovError> {
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
            return Err(LcovError::MalformedInventory {
                lineno,
                raw: raw.to_string(),
            });
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
    read_file: &dyn Fn(&str) -> Result<String, LcovError>,
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
        read_file(&full).map_err(|error| LcovError::Io {
            message: format!("cannot read eligible source {full}: {error}"),
        })
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
                owned.get(path).cloned().ok_or_else(|| LcovError::Io {
                    message: format!("missing file: {path}"),
                })
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
