//! Thin CLI shim over the checker library.
//! Check semantics, JSON output, and exit codes live in the library and are
//! unit-tested there.
//!
//! Contract: `docs/quality/tool-integrations.md`.
//!
//! Usage:
//! ```text
//! quality_markdown --source WS_PATH=EXEC_PATH [--source ...]
//!     [--sibling WS_PATH=EXEC_PATH ...]
//! ```
//! Exit `0` when every source was checked (findings print as JSON lines on
//! stdout); exit `2` on bad arguments, unreadable files, or non-UTF-8 input.

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
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
fn main() {
    // Structured diagnostics: init is idempotent and emits
    // nothing by default; `RUST_LOG` overrides the warn filter. Library
    // error lines route through `tracing::error!` with identical text.
    dx_output::init_diagnostics(false);
    let args: Vec<String> = std::env::args().skip(1).collect();
    // LCOV_EXCL_STOP - policy: docs/testing/README.md#coverage
    let code = quality_markdown::run_cli(
        &args,
        &|path| {
            std::fs::read(path).map_err(|err| quality_markdown::MarkdownError::Io {
                message: err.to_string(),
            })
        },
        &mut |line| println!("{line}"),
        &mut |line| tracing::error!("{line}"),
    );
    std::process::exit(code);
}
