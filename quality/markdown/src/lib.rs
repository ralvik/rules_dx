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

mod check;
mod frontmatter;
mod links;

#[cfg(test)]
#[path = "lib_tests.rs"]
mod lib_tests;

pub use check::{
    check_markdown, kind_id, run_cli, CheckOutcome, Finding, FindingKind, MarkdownError,
};
pub use frontmatter::slug;
pub use links::resolve_target;
