//! `clap` grammar for the `dx` CLI (issue #236).
//!
//! Split from [`super::parser`]: owns the `clap` grammar (`Cli`),
//! the value-option table (`VALUE_OPTIONS`), and the grammar accessor
//! (`cli_command`). Re-exported through `super` and through `parser`
//! so the public paths stay `crate::args::cli_command` and
//! `crate::args::parser::{Cli, VALUE_OPTIONS, cli_command}`.
//!
//! Named `grammar` (not `parse`) so the `parse` function keeps its name;
//! the domain is the grammar half of the `args→command/parse/...` split.

use clap::Parser;

use super::command::Command;

/// Raw `dx` command-line tokens as classified by `clap`: flags may appear
/// before or after the command word, repeated scalars keep the last
/// occurrence, and slice shapes (`--report`, scopes, Bazel forwards) keep
/// `argv` order. `--help`/`-h` render from this same grammar definition
/// (issue #203): one source feeds parsing, help, and completions/man
/// pages, never hand-maintained usage strings.
#[derive(Parser)]
#[command(
    name = "dx",
    about = "Transparent UI over Bazel: quality, workflow, and environment commands",
    long_about = "dx [global-options] <command> [scope ...] [-- bazel-options ...]\n\nScopes: explicit Bazel labels/patterns (//..., //pkg:target, @repo//...), or workspace-relative files/dirs resolved via Bazel query. No scope selects //....\n\nExit codes: 0 success; 2 CLI-detected usage/scope/owner errors; 1 operational failures; Bazel-authoritative commands preserve Bazel's code.\n\nOutput: --output text|diff|json (NDJSON machine protocol on stdout, human text otherwise). See docs/cli/cli-contract.md.",
    version
)]
pub(crate) struct Cli {
    /// Override upward workspace discovery (find MODULE.bazel).
    #[arg(long, allow_negative_numbers = true, overrides_with = "workspace")]
    pub(crate) workspace: Option<String>,
    /// Resolve and summarize the plan without executing workflows/mutations.
    #[arg(long)]
    pub(crate) dry_run: bool,
    /// Suppress dx operation summaries (tool diagnostics still print).
    #[arg(long)]
    pub(crate) quiet: bool,
    /// Enable structured diagnostics on stderr via tracing (issue #222).
    /// Default stays byte-identical; `--verbose` adds info-level logs.
    /// Orthogonal to `--quiet` (summaries vs logs).
    #[arg(long)]
    pub(crate) verbose: bool,
    /// Select concise text, unified patches, or versioned NDJSON events.
    #[arg(long, allow_negative_numbers = true, overrides_with = "output")]
    pub(crate) output: Option<String>,
    /// Write a standard report (sarif/junit/lcov) to file or -; repeatable.
    #[arg(long, allow_negative_numbers = true)]
    pub(crate) report: Vec<String>,
    /// Lowest diagnostic severity that fails quality commands.
    #[arg(long, allow_negative_numbers = true, overrides_with = "fail_on")]
    pub(crate) fail_on: Option<String>,
    /// Required line-coverage percent (coverage only, 0-100).
    #[arg(long, allow_negative_numbers = true, overrides_with = "min_coverage")]
    pub(crate) min_coverage: Option<String>,
    /// Check mode (quality/version only; no mutations).
    #[arg(long)]
    pub(crate) check: bool,
    /// Use dx_debug config (build/run/test/deploy only; conflicts with --release).
    #[arg(long)]
    pub(crate) debug: bool,
    /// Use dx_release config (build/run/test/deploy only; conflicts with --debug).
    #[arg(long)]
    pub(crate) release: bool,
    /// Additionally forward `bazel clean` after pruning (clean only).
    #[arg(long = "bazel")]
    pub(crate) bazel_clean: bool,
    /// Re-pin to <version> (version only).
    #[arg(long, allow_negative_numbers = true, overrides_with = "pin")]
    pub(crate) pin: Option<String>,
    /// Re-pin the recorded previous release (version only).
    #[arg(long)]
    pub(crate) rollback: bool,
    /// Use cquery instead of query (owners/deps/why only).
    #[arg(long)]
    pub(crate) configured: bool,
    /// First positional: the command word (a [`Command`] value so the
    /// same grammar feeds parsing, `--help`, and shell completions).
    #[arg(value_enum)]
    pub(crate) command: Option<Command>,
    /// Later positionals: explicit scopes/targets.
    pub(crate) targets: Vec<String>,
    /// Everything after the first bare `--`, forwarded verbatim.
    #[arg(last = true)]
    pub(crate) bazel_options: Vec<String>,
}

/// Grammar accessor for build steps (issue #225): the `dx_man` binary
/// renders `man/dx.1` from this command so the manual page tracks the
/// same grammar as parsing, `--help`, and completions.
pub fn cli_command() -> clap::Command {
    use clap::CommandFactory;
    Cli::command()
}

/// Value options whose next token the tokenizer consumes as their value:
/// any token not starting with `--`, including single-dash spellings and
/// the empty string.
pub(crate) const VALUE_OPTIONS: &[&str] = &[
    "--workspace",
    "--output",
    "--report",
    "--fail-on",
    "--min-coverage",
    "--pin",
];
