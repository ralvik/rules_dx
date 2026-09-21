//! SHA-256 checksum writer for `archive_deploy`.
//!
//! Owning contract: `docs/deploy/authoring.md` (Path C).

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

// LCOV_EXCL_START - reason: thin binary shim; argument handling is verified by genrule execution, not unit coverage.
use std::path::Path;

fn usage() -> i32 {
    eprintln!("usage: hasher <src> <out>");
    1
}

fn run(argv: &[String]) -> i32 {
    if argv.len() != 3 {
        return usage();
    }
    let src = Path::new(&argv[1]);
    let dst = Path::new(&argv[2]);
    match dx_deploy_tools::hash_file(src, dst) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("hasher: cannot hash {}: {error}", src.display());
            1
        }
    }
}

fn main() {
    std::process::exit(run(&std::env::args().collect::<Vec<_>>()));
}
// LCOV_EXCL_STOP - reason: end of thin binary shim exclusion.
