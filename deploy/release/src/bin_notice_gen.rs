//! Aggregated NOTICE generator for `notice_bundle`.
//!
//! Owning contract: `docs/deploy/release-runbook.md` (NOTICE release path).

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

// LCOV_EXCL_START - reason: thin shim, issue: 1055, policy: docs/testing/strategy-details.md#coverage
use std::path::PathBuf;

fn run(argv: &[String]) -> i32 {
    if argv.len() < 5 {
        return dx_release_tools::bin_usage(
            "notice_gen",
            "<manifest> <out> <root> <text> [<text> ...]",
        );
    }
    let manifest = PathBuf::from(&argv[1]);
    let dst = PathBuf::from(&argv[2]);
    let texts: Vec<PathBuf> = argv[4..].iter().map(PathBuf::from).collect();
    match dx_release_tools::write_notice(&manifest, &dst, &argv[3], &texts) {
        Ok(()) => 0,
        Err(error) => dx_release_tools::bin_cannot_write("notice_gen", &dst, error),
    }
}

fn main() {
    std::process::exit(run(&std::env::args().collect::<Vec<_>>()));
}
// LCOV_EXCL_STOP - reason: end thin shim, issue: 1055, policy: docs/testing/strategy-details.md#coverage
