//! M04 WP3 Markdown check binary: thin CLI shim over the checker library.
//! Check semantics, JSON output, and exit codes live in the library and are
//! unit-tested there.
//!
//! Usage:
//! ```text
//! quality_markdown --source WS_PATH=EXEC_PATH [--source ...]
//!     [--sibling WS_PATH=EXEC_PATH ...]
//! ```
//! Exit `0` when every source was checked (findings print as JSON lines on
//! stdout); exit `2` on bad arguments, unreadable files, or non-UTF-8 input.

// LCOV_EXCL_START - reason: thin binary shim; CLI file I/O is covered by library run_cli unit tests with injected readers, not host I/O.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = quality_markdown::run_cli(
        &args,
        &|path| std::fs::read(path).map_err(|err| err.to_string()),
        &mut |line| println!("{line}"),
        &mut |line| eprintln!("{line}"),
    );
    std::process::exit(code);
}
// LCOV_EXCL_STOP - reason: end of thin binary shim exclusion.
