//! Thin entry point over the gate library.
//! All branching logic lives in the library and is unit-tested there.
//!
//! Contract: `docs/testing/README.md#coverage`.

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

// The explicit `use` keeps the gate dependency visible to the Gazelle Rust
// scanner, which resolves imports from `use`/`extern crate` items only: the
// `dx_lcov::run` call path alone yields no import, so generation would
// strip the `:dx_lcov` dep the binary links.
use dx_lcov::run;

// LCOV_EXCL_START - policy: docs/testing/README.md#coverage
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(run(
        &args,
        &|path| {
            std::fs::read_to_string(path).map_err(|err| dx_lcov::LcovError::from(err.to_string()))
        },
        &mut |line| println!("{line}"),
    ));
}
// LCOV_EXCL_STOP - policy: docs/testing/README.md#coverage
