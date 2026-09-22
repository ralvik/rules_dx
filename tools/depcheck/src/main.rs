//! Thin CLI shim over the depcheck library.
//!
//! Owning contract: `docs/quality/quality-testing.md` (CLI args, exit codes,
//! offline/no-network, category/exception/obsolete semantics).

#![cfg_attr(
    not(test),
    deny(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::unreachable,
        clippy::todo
    )
)]

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
// `Locks` carries the full workspace lock set; boxing its `PathBuf`s would
// silence `large_enum_variant` but `clap` has no `ValueParser` for
// `Box<PathBuf>`, so the variant stays inline (See: `docs/quality/quality-testing.md`).
#[allow(clippy::large_enum_variant)]
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
    /// Verify all six workspace locks in one invocation (see `//tools:repin-all`).
    Locks {
        #[arg(long)]
        cargo_manifest: PathBuf,
        #[arg(long)]
        cargo_lock: PathBuf,
        #[arg(long)]
        uv_manifest: PathBuf,
        #[arg(long)]
        uv_lock: PathBuf,
        #[arg(long)]
        pnpm_manifest: PathBuf,
        #[arg(long)]
        pnpm_lock: PathBuf,
        #[arg(long)]
        go_manifest: PathBuf,
        #[arg(long)]
        go_lock: PathBuf,
        #[arg(long)]
        maven_artifacts: PathBuf,
        #[arg(long)]
        maven_lock: PathBuf,
        #[arg(long)]
        paket_manifest: PathBuf,
        #[arg(long)]
        paket_lock: PathBuf,
        #[arg(long)]
        ruby_manifest: PathBuf,
        #[arg(long)]
        ruby_lock: PathBuf,
    },
}

fn parse_eco(text: &str) -> Result<String, String> {
    const KNOWN: &[&str] = &[
        "rust", "python", "js", "ts", "go", "java", "kotlin", "scala", "csharp", "fsharp", "cc",
        "ruby",
    ];
    if KNOWN.contains(&text) {
        Ok(text.to_owned())
    } else {
        Err(format!(
            "invalid value '{text}' for '--ecosystem <ECOSYSTEM>'"
        ))
    }
}

/// Shared unknown-ecosystem failure (See: `docs/quality/quality-testing.md`, issue #914): both subcommands
/// reject unparsed ecosystems identically instead of copy-pasting the
/// `eprintln!` plus exit code.
fn unknown_ecosystem(ecosystem: &str) -> i32 {
    eprintln!("depcheck: ERROR: unknown ecosystem: {ecosystem}");
    2
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
                return unknown_ecosystem(&ecosystem);
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
                return unknown_ecosystem(&ecosystem);
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
        Command::Locks {
            cargo_manifest,
            cargo_lock,
            uv_manifest,
            uv_lock,
            pnpm_manifest,
            pnpm_lock,
            go_manifest,
            go_lock,
            maven_artifacts,
            maven_lock,
            paket_manifest,
            paket_lock,
            ruby_manifest,
            ruby_lock,
        } => {
            let mut out = String::new();
            let mut err = String::new();
            let code = dx_depcheck::cmd_locks(
                &dx_depcheck::WorkspaceLocks {
                    cargo_manifest: &cargo_manifest,
                    cargo_lock: &cargo_lock,
                    uv_manifest: &uv_manifest,
                    uv_lock: &uv_lock,
                    pnpm_manifest: &pnpm_manifest,
                    pnpm_lock: &pnpm_lock,
                    go_manifest: &go_manifest,
                    go_lock: &go_lock,
                    maven_artifacts: &maven_artifacts,
                    maven_lock: &maven_lock,
                    paket_manifest: &paket_manifest,
                    paket_lock: &paket_lock,
                    ruby_manifest: &ruby_manifest,
                    ruby_lock: &ruby_lock,
                },
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
