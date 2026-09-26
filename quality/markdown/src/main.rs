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

// LCOV_EXCL_START - reason: thin shim, issue: 1055, policy: docs/testing/strategy-details.md#coverage
fn main() {
    // Structured diagnostics: init is idempotent and emits
    // nothing by default; `RUST_LOG` overrides the warn filter. Library
    // error lines route through `tracing::error!` with identical text.
    dx_output::init_diagnostics(false);
    let args: Vec<String> = std::env::args().skip(1).collect();
    // LCOV_EXCL_STOP - reason: end thin shim, issue: 1055, policy: docs/testing/strategy-details.md#coverage
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
