//! `dx_man` build step (issue #225): renders the `dx` manual page from
//! the clap grammar so it can never drift from `--help`.
//!
//! Usage: `dx_man <output-file>`. The `man_pages` genrule wires this
//! into the build (`bazel build //cli/cli:man_pages` emits `man/dx.1`);
//! release packaging (issue #26) consumes that target. The CLI is a
//! single clap command with a command-word value enum rather than
//! subcommands, so the grammar yields one page; per-command pages arrive
//! if the grammar ever gains subcommands.

// Issue #238: infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

use std::io::Write;
use std::path::PathBuf;

// LCOV_EXCL_START - reason: thin build-step binary; man-page rendering is verified by the man_pages genrule build, not unit coverage.
fn main() {
    // Structured diagnostics (issue #232): build-step failures report via
    // `tracing::error!` with the legacy message text; init is idempotent
    // and emits nothing by default.
    dx_output::init_diagnostics(false);
    let out: PathBuf = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            tracing::error!("usage: dx_man <output-file>");
            std::process::exit(2);
        });
    let man = clap_mangen::Man::new(dx_cli::args::cli_command()).section("1");
    let mut buffer = Vec::new();
    man.render(&mut buffer).unwrap_or_else(|error| {
        tracing::error!("dx_man: render failed: {error}");
        std::process::exit(1);
    });
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).unwrap_or_else(|error| {
                tracing::error!("dx_man: cannot create {}: {error}", parent.display());
                std::process::exit(1);
            });
        }
    }
    let mut file = std::fs::File::create(&out).unwrap_or_else(|error| {
        tracing::error!("dx_man: cannot write {}: {error}", out.display());
        std::process::exit(1);
    });
    file.write_all(&buffer).unwrap_or_else(|error| {
        tracing::error!("dx_man: cannot write {}: {error}", out.display());
        std::process::exit(1);
    });
}
// LCOV_EXCL_STOP - reason: end of thin build-step binary exclusion.
