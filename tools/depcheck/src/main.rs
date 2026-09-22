//! Thin CLI shim over the depcheck library.
//!
//! Owning contract: `docs/quality/quality-testing.md` (CLI args, exit codes,
//! offline/no-network, category/exception/obsolete semantics).

#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

// LCOV_EXCL_START - policy: docs/testing/README.md#coverage
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "depcheck")]
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Verify manifest vs lock (offline, non-mutating).
    Consistency {
        #[arg(long, value_parser = parse_eco)]
        ecosystem: String,
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        lock: PathBuf,
    },
    /// Verify declared deps are used in owning scope.
    Usage {
        #[arg(long, value_parser = parse_eco)]
        ecosystem: String,
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        sources: PathBuf,
        #[arg(long)]
        exceptions: Option<PathBuf>,
    },
}

fn parse_eco(text: &str) -> Result<String, String> {
    const KNOWN: &[&str] = &[
        "rust", "python", "js", "ts", "go", "java", "kotlin", "scala", "csharp", "fsharp", "cc",
    ];
    if KNOWN.contains(&text) {
        Ok(text.to_owned())
    } else {
        Err(format!(
            "invalid value '{text}' for '--ecosystem <ECOSYSTEM>'"
        ))
    }
}

fn run() -> i32 {
    let cli = Cli::parse();
    // LCOV_EXCL_STOP - policy: docs/testing/README.md#coverage
    match cli.cmd {
        Command::Consistency {
            ecosystem,
            manifest,
            lock,
        } => {
            let Some(eco) = dx_depcheck::Ecosystem::parse(&ecosystem) else {
                eprintln!("depcheck: ERROR: unknown ecosystem: {ecosystem}");
                return 2;
            };
            let mut out = String::new();
            let mut err = String::new();
            let code = dx_depcheck::cmd_consistency(eco, &manifest, &lock, &mut out, &mut err);
            if !out.is_empty() {
                print!("{out}");
            }
            if !err.is_empty() {
                eprint!("{err}");
            }
            code
        }
        Command::Usage {
            ecosystem,
            manifest,
            sources,
            exceptions,
        } => {
            let Some(eco) = dx_depcheck::Ecosystem::parse(&ecosystem) else {
                eprintln!("depcheck: ERROR: unknown ecosystem: {ecosystem}");
                return 2;
            };
            let mut out = String::new();
            let mut err = String::new();
            let code = dx_depcheck::cmd_usage(
                eco,
                &manifest,
                &sources,
                exceptions.as_deref(),
                &mut out,
                &mut err,
            );
            if !out.is_empty() {
                print!("{out}");
            }
            if !err.is_empty() {
                eprint!("{err}");
            }
            code
        }
    }
}

fn main() {
    std::process::exit(run());
}
