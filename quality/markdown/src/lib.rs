//! WP3 repository-owned Markdown link/structure checker.
//!
//! Boundary (frozen in `docs/quality/tool-integrations.md`): this crate
//! parses one Markdown source plus its declared sibling-file closure and
//! reports structured findings. It is distinct from Vale: Vale owns prose
//! policy, this checker owns repository structure (relative link targets,
//! anchors, heading hierarchy, fenced code-block language tags). Remote URLs
//! are recorded as skipped and never fetched. Undeclared link targets fail
//! closed as findings, never as silent passes.
//!
//! Scope notes: inline links (`[text](target)`), images, references
//! (`[text][label]` with a `[label]: target` definition, including collapsed
//! `[text][]` and shortcut `[text]` forms), absolute-URI autolinks, and
//! multi-line links resolve through pulldown-cmark events; relative
//! `<./target>` autolinks resolve through a fallback scan (they are not
//! CommonMark autolinks, so the parser emits no event). A target that names
//! a directory resolves to its declared `README.md` index (`docs/cli/`
//! reads `docs/cli/README.md`); an undeclared index fails closed like any
//! undeclared target. Email autolinks (`<a@b.c>`) are wont-fix out of scope
//! (checker-owned: structure only, never prose/identity) and
//! ignored. Explicit references with no definition fail closed, keeping the
//! author's label spelling; bare `[text]` with no definition is literal
//! text. Code spans suppress link detection natively, including multi-line
//! spans; a stray backtick is literal text, so a link after it is reported
//! (fail-closed). HTML blocks contribute neither headings nor links. Only
//! ATX headings count (blockquote, list, and setext headings do not), and
//! heading slugs follow [`slug`] (`slug::slugify` over `deunicode`
//! transliteration: ASCII `a-z`/`0-9`/`-` only, collapsed and trimmed,
//! non-ASCII transliterated) with GitHub-style `-1`/`-2` deduplication
//! for repeat headings.
//!
//! Binary contract: [`run_cli`] checks `--source WS_PATH=EXEC_PATH` files
//! against a sibling closure (the union of `--source` and
//! `--sibling WS_PATH=EXEC_PATH` contents) and prints one JSON object per
//! finding on stdout:
//! `{"path": WS, "line": N, "kind": KEBAB, "message": TEXT}`.
//! Exit `0` when every source was checked, with or without findings;
//! unresolved links are findings, never operational failures. Exit `2` on
//! bad arguments, unreadable files, or non-UTF-8 input, with the reason on
//! stderr. Skipped remote targets are reported once each on stderr and never
//! fetched. Duplicate source workspace paths keep the first mapping.

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
