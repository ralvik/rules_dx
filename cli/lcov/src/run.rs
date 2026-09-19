//! Gate CLI for the coverage gate (issue #236).
//!
//! Split from `super` (`lib.rs`): owns [`run`] (the `check --report
//! --inventory --sources [--root]` entry point returning 0 on pass, 1 on
//! gate failure, 2 on usage/configuration errors). Re-exported through
//! `super` so the public path stays `dx_lcov::run`. Distinct from the
//! `parse` module (combined-LCOV parsing), the `ignores` module
//! (source-level exclusion markers), the `verdict` module (gate
//! evaluation), and the `inventory` module (repo inventory).

use super::{evaluate, parse_inventory, parse_lcov, render, LcovError};
use clap::{
    error::{ContextKind, ContextValue, ErrorKind},
    Parser,
};

fn print_usage(print: &mut dyn FnMut(&str)) {
    print("usage: check --report <combined.lcov> --inventory <inventory.txt> --sources <sources.txt> [--root <dir>]");
}

/// `argv` tokenizer (issue #396: reuse pinned `clap`, qualified under
/// issue #316 frozen legacy contract). Every value option consumes the
/// next token unconditionally (even a `--`-led token) via
/// `allow_hyphen_values`, matching the legacy hand loop; repeats are
/// last-wins via `overrides_with`. A dangling value (including `--root`
/// with no value, previously silently ignored) is a usage error (exit 2).
#[derive(Parser, Debug)]
#[command(disable_help_flag = true, disable_version_flag = true)]
struct Cli {
    #[arg(long, allow_hyphen_values = true, overrides_with = "report")]
    report: Option<String>,
    #[arg(long, allow_hyphen_values = true, overrides_with = "inventory")]
    inventory: Option<String>,
    #[arg(long, allow_hyphen_values = true, overrides_with = "sources")]
    sources: Option<String>,
    #[arg(long, allow_hyphen_values = true, overrides_with = "root")]
    root: Option<String>,
    /// Legacy `--help`/`-h` arm: prints the usage line (exit 2).
    #[arg(long = "help", short = 'h', action = clap::ArgAction::SetTrue)]
    help: bool,
}

/// Raw `argv` token behind a [`clap::Error`], e.g. `--bogus` or `oops`.
fn invalid_token(error: &clap::Error) -> String {
    match error.get(ContextKind::InvalidArg) {
        Some(ContextValue::String(token)) => token.clone(),
        Some(ContextValue::Strings(tokens)) => tokens.first().cloned().unwrap_or_default(),
        _ => String::new(),
    }
}

/// Map `clap` tokenizing failures onto the legacy usage-routed surface:
/// every failure prints its reason plus the usage line (exit `2`).
/// Reachable kinds: [`ErrorKind::UnknownArgument`] and
/// [`ErrorKind::InvalidValue`] (a present flag with no consumable value).
/// No other parser, conflict, or count error can fire.
fn parse_error(error: clap::Error, args: &[String]) -> String {
    let token = invalid_token(&error);
    match error.kind() {
        // `clap` strips an attached `=value` from the reported token; the
        // legacy loop echoed the whole `argv` element, so recover it.
        ErrorKind::UnknownArgument => {
            let echoed = args
                .iter()
                .find(|arg| *arg == &token)
                .or_else(|| {
                    args.iter()
                        .find(|arg| arg.starts_with(&format!("{token}=")))
                })
                .map_or(token.clone(), Clone::clone);
            format!("unknown argument: {echoed}")
        }
        ErrorKind::InvalidValue => {
            // `clap` renders the pending option as `--flag <VALUE>`; the
            // legacy message names the bare `--flag`.
            let flag = token.split_whitespace().next().unwrap_or(&token);
            format!("missing value for {flag}")
        }
        _ => error
            .to_string()
            .lines()
            .next()
            .unwrap_or("invalid arguments")
            .to_owned(),
    }
}

fn parse_args(args: &[String]) -> Result<Cli, String> {
    Cli::try_parse_from(std::iter::once("check").chain(args.iter().map(|arg| arg as &str)))
        .map_err(|error| parse_error(error, args))
}

/// Run the gate CLI. Returns 0 on pass, 1 on gate failure, 2 on usage or
/// configuration errors. Missing report *evidence* fails the gate (1);
/// unreadable inventory/sources configuration is a usage error (2).
pub fn run(
    args: &[String],
    read_file: &dyn Fn(&str) -> Result<String, LcovError>,
    print: &mut dyn FnMut(&str),
) -> i32 {
    let cli = match parse_args(args) {
        Ok(cli) => cli,
        Err(message) => {
            print(&message);
            print_usage(print);
            return 2;
        }
    };
    if cli.help {
        print_usage(print);
        return 2;
    }
    let report_path = match cli.report {
        Some(path) => path,
        None => {
            print("missing required --report <combined LCOV>");
            print_usage(print);
            return 2;
        }
    };
    let inventory_path = match cli.inventory {
        Some(path) => path,
        None => {
            print("missing required --inventory <inventory file>");
            print_usage(print);
            return 2;
        }
    };
    let sources_path = match cli.sources {
        Some(path) => path,
        None => {
            print("missing required --sources <Bazel source list>");
            print_usage(print);
            return 2;
        }
    };
    let root = cli.root.unwrap_or_else(|| ".".to_string());
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
    use std::collections::BTreeMap;

    use super::*;

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
    fn run_rejects_dangling_root_flag() {
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
            ],
        );
        assert_eq!(code, 2, "{printed:?}");
        assert!(
            printed.iter().any(|line| line.contains("usage")),
            "{printed:?}"
        );
    }
}
