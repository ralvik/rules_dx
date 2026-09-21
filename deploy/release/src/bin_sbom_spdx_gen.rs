//! SPDX 2.3 generator for `sbom_release`.
//!
//! Owning contract: `docs/deploy/release-runbook.md` (SBOM release path).

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

// LCOV_EXCL_START - policy: docs/testing/README.md#coverage
use std::path::Path;

fn usage() -> i32 {
    eprintln!("usage: sbom_spdx_gen <artifact> <out> <package> <supplier>");
    1
}

fn run(argv: &[String]) -> i32 {
    if argv.len() != 5 {
        return usage();
    }
    let src = Path::new(&argv[1]);
    let dst = Path::new(&argv[2]);
    match dx_release_tools::write_spdx(src, dst, &argv[3], &argv[4]) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("sbom_spdx_gen: cannot write {}: {error}", dst.display());
            1
        }
    }
}

fn main() {
    std::process::exit(run(&std::env::args().collect::<Vec<_>>()));
}
// LCOV_EXCL_STOP - policy: docs/testing/README.md#coverage
