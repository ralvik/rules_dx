//! Preset update binary for `bazel run //tools/bazelrc:preset.update`.
//!
//! Owning contract: `docs/contributing/local-workflows.md` (preset update loop).

#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

// LCOV_EXCL_START - policy: docs/testing/README.md#coverage
use std::path::PathBuf;

fn usage() -> i32 {
    eprintln!("usage: preset.update [--verify-only]");
    1
}

fn run(argv: &[String]) -> i32 {
    let verify_only = argv.len() == 2 && argv[1] == "--verify-only";
    if argv.len() != 1 && !verify_only {
        return usage();
    }
    let workspace = match dx_preset::resolve_workspace() {
        Ok(workspace) => workspace,
        Err(error) => {
            eprintln!("preset.update: cannot resolve workspace: {error}");
            return 1;
        }
    };
    let source: PathBuf = dx_preset::source_dir(&workspace);
    if !source.is_dir() {
        eprintln!(
            "preset.update: source directory '{}' not found",
            source.display()
        );
        return 1;
    }
    if verify_only {
        match dx_preset::check_preset(&workspace) {
            Ok(()) => {
                println!("preset.update: preset.bazelrc verified");
                0
            }
            Err(dx_preset::PresetError::Stale { detail }) => {
                println!("{detail}");
                1
            }
            Err(error) => {
                eprintln!("{error}");
                1
            }
        }
    } else {
        match dx_preset::update_preset(&workspace) {
            Ok(()) => {
                println!("preset.update: wrote preset.bazelrc");
                0
            }
            Err(error) => {
                eprintln!("{error}");
                1
            }
        }
    }
}

fn main() {
    std::process::exit(run(&std::env::args().collect::<Vec<_>>()));
}
// LCOV_EXCL_STOP - policy: docs/testing/README.md#coverage
