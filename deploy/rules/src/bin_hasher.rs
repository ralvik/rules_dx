//! SHA-256 checksum writer for `archive_deploy`.
//!
//! Owning contract: `docs/deploy/authoring.md` (Path C).

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

// LCOV_EXCL_START - policy: docs/testing/README.md#coverage
use std::path::Path;

fn run(argv: &[String]) -> i32 {
    if argv.len() != 3 {
        return dx_deploy_tools::bin_usage("hasher", "<src> <out>");
    }
    let src = Path::new(&argv[1]);
    let dst = Path::new(&argv[2]);
    // LCOV_EXCL_STOP - policy: docs/testing/README.md#coverage
    match dx_deploy_tools::hash_file(src, dst) {
        Ok(()) => 0,
        Err(error) => dx_deploy_tools::bin_cannot("hasher", "hash", src, error),
    }
}

fn main() {
    std::process::exit(run(&std::env::args().collect::<Vec<_>>()));
}
