//! `env` binary: thin CLI shim over the managed-environment bootstrap library.
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

// LCOV_EXCL_START - reason: thin binary shim; flag parsing, workspace discovery, and runfiles location are operational behaviors verified by build and bootstrap execution, not unit coverage.
use std::path::{Path, PathBuf};
use std::time::Duration;

use dx_env::{identity_hex, parse_staged, probe_symlink, refresh, RefreshOptions, RefreshOutcome};

/// Default tree metadata rlocation candidates, in order. The metadata file
/// has a stable name while tool link names vary, so one lookup locates the
/// staged tree: its parent directory plus `bin`.
const METADATA_CANDIDATES: &[(&str, &str)] = &[
    ("rules_dx/env/default_tree.metadata.json", "_main"),
    ("_main/env/default_tree.metadata.json", "_main"),
];

fn usage_error(message: &str) -> i32 {
    eprintln!("dx env: {message}");
    eprintln!(
        "usage: env [--workspace DIR] [--staged-bin DIR --metadata FILE] [--lock-timeout-ms N]"
    );
    2
}

fn run() -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut workspace: Option<String> = None;
    let mut staged_bin: Option<String> = None;
    let mut metadata: Option<String> = None;
    let mut lock_timeout_ms: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--workspace" => {
                index += 1;
                workspace = Some(match args.get(index) {
                    Some(value) => value.clone(),
                    None => return usage_error("missing value for --workspace"),
                });
            }
            "--staged-bin" => {
                index += 1;
                staged_bin = Some(match args.get(index) {
                    Some(value) => value.clone(),
                    None => return usage_error("missing value for --staged-bin"),
                });
            }
            "--metadata" => {
                index += 1;
                metadata = Some(match args.get(index) {
                    Some(value) => value.clone(),
                    None => return usage_error("missing value for --metadata"),
                });
            }
            "--lock-timeout-ms" => {
                index += 1;
                lock_timeout_ms = Some(match args.get(index) {
                    Some(value) => value.clone(),
                    None => return usage_error("missing value for --lock-timeout-ms"),
                });
            }
            "--help" | "-h" => {
                return usage_error("install the staged environment tree into .dx/bin")
            }
            other => return usage_error(&format!("unknown flag {other:?}")),
        }
        index += 1;
    }
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
    // `bazel run` executes with the working directory inside the runfiles
    // tree under bazel-out; `BUILD_WORKSPACE_DIRECTORY` points back at the
    // source workspace. An explicit `--workspace` still wins.
    let start = match workspace {
        Some(dir) => PathBuf::from(dir),
        None => std::env::var_os("BUILD_WORKSPACE_DIRECTORY")
            .map(PathBuf::from)
            .filter(|dir| dir.is_absolute())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))),
    };
    let (staged_bin, metadata) = match (staged_bin, metadata) {
        (Some(bin), Some(meta)) => (PathBuf::from(bin), PathBuf::from(meta)),
        _ => match locate_default_tree() {
            Some(paths) => paths,
            None => {
                eprintln!("dx env: default tree not found in runfiles; pass --staged-bin and --metadata explicitly");
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
                eprintln!("dx env: {error}");
                return 1;
            }
        },
        Err(error) => {
            eprintln!(
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
            eprintln!("dx env: {error}");
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
    std::process::exit(run());
}
// LCOV_EXCL_STOP - reason: end of thin binary shim exclusion.
