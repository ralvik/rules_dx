//! `env` binary: thin CLI shim over the managed-environment bootstrap library.
//!
//! Contract: `docs/environments/environment.md`.
//!
//! Refresh semantics, marker validation, and the atomic swap live in the
//! library and are unit-tested there. This shim owns process concerns
//! only: flag parsing, workspace discovery, staged-input location (explicit
//! flags or runfiles lookup of the default tree), outcome messaging, and
//! exit codes (0 success, 1 operational failure, 2 usage error per the CLI
//! output protocol).
//!
//! Usage:
//! ```text
//! env [--workspace DIR] [--staged-bin DIR --metadata FILE] [--lock-timeout-ms N]
//! ```
//! With no staged-input flags the default `environment_tree` travels in
//! this binary's runfiles; pass both flags to install any other tree.

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

// LCOV_EXCL_START - reason: thin binary shim; flag parsing, workspace discovery, and runfiles location are operational behaviors verified by build and bootstrap execution, not unit coverage.
use std::path::{Path, PathBuf};
use std::time::Duration;

use clap::{
    error::{ContextKind, ContextValue, ErrorKind},
    Parser,
};
use dx_env::{identity_hex, parse_staged, probe_symlink, refresh, RefreshOptions, RefreshOutcome};

/// Default tree metadata rlocation candidates, in order.
/// Single source for the default-tree lookup: the metadata file has a
/// stable name while tool link names vary, so one lookup locates the
/// staged tree (parent plus `bin`). Uses the standard `runfiles` library
/// (`rlocation_from`); prod code never reads `TEST_SRCDIR` directly.
/// Two entries cover Bzlmod (`_main/`) plus legacy (`rules_dx/`) layouts.
const METADATA_CANDIDATES: &[(&str, &str)] = &[
    ("rules_dx/env/default_tree.metadata.json", "_main"),
    ("_main/env/default_tree.metadata.json", "_main"),
];

fn usage_error(message: &str) -> i32 {
    // Structured diagnostics: usage failures report via
    // `tracing::error!` with the legacy message text.
    tracing::error!("dx env: {message}");
    tracing::error!(
        "usage: env [--workspace DIR] [--staged-bin DIR --metadata FILE] [--lock-timeout-ms N]"
    );
    2
}

/// `argv` tokenizer (frozen legacy contract).
/// Every option keeps the legacyshape: last-wins scalar
/// repeats and unconditional next-token consumption (even a `--`-led token),
/// so `--workspace --staged-bin DIR` still binds `--staged-bin` as the
/// workspace. Only tokenizing moves to `clap`; all value validation below
/// is untouched.
#[derive(Parser)]
#[command(disable_help_flag = true)]
struct Cli {
    #[arg(long, allow_hyphen_values = true, overrides_with = "workspace")]
    workspace: Option<String>,
    #[arg(long, allow_hyphen_values = true, overrides_with = "staged_bin")]
    staged_bin: Option<String>,
    #[arg(long, allow_hyphen_values = true, overrides_with = "metadata")]
    metadata: Option<String>,
    #[arg(long, allow_hyphen_values = true, overrides_with = "lock_timeout_ms")]
    lock_timeout_ms: Option<String>,
    /// Legacy `--help`/`-h` arm: prints the description line plus usage.
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

/// Map `clap` tokenizing failures onto [`usage_error`] messages. Only
/// [`ErrorKind::UnknownArgument`] and [`ErrorKind::InvalidValue`] (a present
/// flag with no consumable value) are reachable: every option takes plain
/// strings, so no value parser, conflict, or count error can fire.
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
            format!("unknown flag {echoed:?}")
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
    Cli::try_parse_from(std::iter::once("env").chain(args.iter().map(|arg| arg as &str)))
        .map_err(|error| parse_error(error, args))
}

fn run() -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cli = match parse_args(&args) {
        Ok(cli) => cli,
        Err(message) => return usage_error(&message),
    };
    if cli.help {
        return usage_error("install the staged environment tree into .dx/bin");
    }
    let (workspace, staged_bin, metadata, lock_timeout_ms) = (
        cli.workspace,
        cli.staged_bin,
        cli.metadata,
        cli.lock_timeout_ms,
    );
    if staged_bin.is_none() != metadata.is_none() {
        return usage_error("--staged-bin and --metadata must be passed together");
    }
    let lock_timeout = match lock_timeout_ms {
        None => dx_env::LOCK_TIMEOUT,
        Some(millis) => match millis.parse::<u64>() {
            Ok(millis) => Duration::from_millis(millis),
            Err(_) => return usage_error("--lock-timeout-ms must be a non-negative integer"),
        },
    };
    // Workspace start: single-sourced via
    // `dx_process::workspace_start` (shell: `tools/sh/lib.sh`).
    // An explicit `--workspace` still wins.
    let start = match workspace {
        Some(dir) => PathBuf::from(dir),
        None => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            dx_process::workspace_start(&cwd)
        }
    };
    let (staged_bin, metadata) = match (staged_bin, metadata) {
        (Some(bin), Some(meta)) => (PathBuf::from(bin), PathBuf::from(meta)),
        _ => match locate_default_tree() {
            Some(paths) => paths,
            None => {
                tracing::error!("dx env: default tree not found in runfiles; pass --staged-bin and --metadata explicitly");
                return 1;
            }
        },
    };
    let options = RefreshOptions {
        workspace_root: start,
        staged_bin,
        staged_metadata: metadata.clone(),
        os: std::env::consts::OS,
        lock_timeout,
    };
    let tools = match std::fs::read_to_string(&metadata) {
        Ok(text) => match parse_staged(&text) {
            Ok(tools) => tools,
            Err(error) => {
                tracing::error!("dx env: {error}");
                return 1;
            }
        },
        Err(error) => {
            tracing::error!(
                "dx env: cannot read staged metadata {}: {error}",
                metadata.display()
            );
            return 1;
        }
    };
    match refresh(&options, &probe_symlink) {
        Ok(RefreshOutcome::AlreadyCurrent) => {
            println!("dx env: already current (.dx/bin {})", identity_hex(&tools));
            0
        }
        Ok(RefreshOutcome::InstalledFresh) => {
            println!(
                "dx env: installed {} tool(s) (.dx/bin {})",
                tools.len(),
                identity_hex(&tools)
            );
            0
        }
        Ok(RefreshOutcome::InstalledReplacement) => {
            println!(
                "dx env: replaced managed tree with {} tool(s) (.dx/bin {})",
                tools.len(),
                identity_hex(&tools)
            );
            0
        }
        Err(error) => {
            tracing::error!("dx env: {error}");
            1
        }
    }
}

/// Locates the default staged tree through the runfiles manifest: the
/// metadata file has a stable name, and the staged links sit in the `bin`
/// directory beside it.
fn locate_default_tree() -> Option<(PathBuf, PathBuf)> {
    let runfiles = runfiles::Runfiles::create().ok()?;
    for (path, source_repo) in METADATA_CANDIDATES {
        if let Some(metadata) = runfiles.rlocation_from(Path::new(path), source_repo) {
            if metadata.is_file() {
                if let Some(parent) = metadata.parent() {
                    let staged_bin = parent.join(dx_env::BIN_DIR_NAME);
                    if staged_bin.is_dir() {
                        return Some((staged_bin, metadata));
                    }
                }
            }
        }
    }
    None
}

fn main() {
    // Structured diagnostics: init is idempotent and emits
    // nothing by default; `RUST_LOG` overrides the warn filter.
    dx_output::init_diagnostics(false);
    std::process::exit(run());
}
// LCOV_EXCL_STOP - reason: end of thin binary shim exclusion.
