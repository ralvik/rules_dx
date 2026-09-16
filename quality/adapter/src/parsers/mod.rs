//! Per-tool output grammars for the pinned M04 binaries plus M15 Python.
//!
//! Each parser maps one tool's check output onto [`FileFinding`] values
//! addressed by the scratch-absolute path the tool reported; the caller
//! re-roots that path onto the workspace path before placement. Parsers
//! never spawn processes and never invent positions: anything outside the
//! pinned grammar is a [`ParseError`], which the runner surfaces as an
//! action failure.
//!
//! Pinned shapes (probed against the M04 binaries, M15 Python probes in
//! the M15 evidence):
//!
//! * Buildifier `--mode=check --format=json --lint=warn`: stdout JSON
//!   `{success, files:[{filename, formatted, valid, warnings:[...]}]}`.
//!   `success` is false whenever findings exist and the exit code is
//!   always 0, so both are ignored: the file entries decide. A
//!   `valid:false` entry is a syntax error positioned from the
//!   `<path>:<line>:<col>` stderr line; `formatted:false` is one format
//!   finding; each warning is one finding under its category.
//! * rustfmt `--check`: stdout `Diff in <path>:<line>:` headers (one per
//!   file, first-differing line) plus stderr `error` / ` --> <path>:...`
//!   blocks for syntax errors. A nonzero exit with neither is a grammar
//!   mismatch, never a clean result.
//! * Taplo `lint`: stderr `error:` / `  ┌─ <path>:<line>:<col>` blocks
//!   with `^` caret detail lines. A nonzero exit with no parsed block is
//!   a grammar mismatch (version-qualified fail-closed text parsing).
//! * Taplo `format --check`: the same syntax blocks plus
//!   `the file is not properly formatted path="<path>"` lines, one
//!   format finding each.
//! * Vale `--output=JSON`: stdout object mapping each checked path to its
//!   alert list (`Span:[start,end]` columns are inclusive on both ends,
//!   so placement uses `end + 1`). An envelope object with a `Code` key
//!   (`E100`/`E201`, exit 2) is [`ParseError::ValeConfig`], never a
//!   finding.
//! * Markdown checker (`//quality/markdown:quality_markdown`): stdout is
//!   newline-delimited JSON, one `{"path","line","kind","message"}` object
//!   per finding and nothing at all when clean. `path` is the workspace
//!   key from the `--source` mapping (the caller re-roots it onto the
//!   scratch-absolute path before placement), `line` is one-based, `kind`
//!   is one of the frozen kebab-case finding kinds. Findings exist only on
//!   exit 0: any other exit is a [`ParseError::Shape`], never a partial
//!   result.
//! * Clippy `--error-format=json --emit=metadata`: stderr JSON
//!   diagnostics, one per line. Span-less summaries (`N warnings
//!   emitted`, `aborting due to ...`) are skipped; every other
//!   diagnostic needs a primary span in a checked file. `level`
//!   maps warning/error; other levels with placed spans are a grammar
//!   mismatch. `MachineApplicable` child spans become byte [`Suggestion`]
//!   values; Clippy columns are half-open `[start, end)`, Vale-adjusted
//!   spans aside, every tool column here counts Unicode scalar values.
//! * rustc `--error-format=json --emit=metadata --crate-type=lib`:
//!   same JSON diagnostic grammar as Clippy (both are `rustc`
//!   diagnostics); `tool_id` is `rustc`, findings are typecheck
//!   diagnostics, and suggestions are parsed but never applied by the
//!   runner (typecheck is check-only).
//! * Ruff `check --output-format json`: stdout JSON array, one object per
//!   finding (`code`, `filename`, `location:{column,row}`,
//!   `end_location:{column,row}`, `message`, `severity`). `severity`
//!   `error`/`warning` map onto [`ToolSeverity`]; anything else is a
//!   grammar mismatch. Clean is `[]` on exit 0; findings exit 1. `fix`
//!   edits are ignored: the runner converges via `check --fix`
//!   re-runs, never by applying parsed edits.
//! * Ruff `format --check --output-format json`: the same JSON array, but
//!   every entry carries `code: "unformatted"` (one entry per unformatted
//!   file at its first-differing hunk); any other code is a grammar
//!   mismatch. Message and severity map verbatim like lint.
//! * Ty `check --output-format concise --no-progress`: stdout diagnostic
//!   lines `<path>:<line>:<col>: <severity>[<code>] <message>` plus a
//!   `Found N diagnostic(s)` summary and, when clean, `All checks
//!   passed!` on exit 0. Diagnostics are start points (concise carries
//!   no end); `error`/`warning` map onto [`ToolSeverity`]. Anything else
//!   on stdout is a grammar mismatch.
//! * tsc `--noEmit` (pinned `@npm_typescript//:tsc` 5.9.3): stdout
//!   diagnostic lines `<path>(<line>,<col>): error TS<code>: <message>`,
//!   exit 2 with findings and empty stdout on exit 0. Diagnostics are
//!   start points (classic tsc carries no end); every diagnostic is an
//!   error. Anything else on stdout is a grammar mismatch.
//! * pydoclint `--quiet`: violations on stderr as a bare `<path>` header
//!   line plus `    <line>: <DOCxxx>: <message>` lines (stdout empty).
//!   Findings are line-level points at column 1; every violation is an
//!   error. Line `0` (whole-file `DOC002` syntax errors: the file cannot
//!   be parsed, so no line exists) places a point at 1:1 with the tool's
//!   message verbatim. Clean is empty output on exit 0.
//! * flake8 `--isolated --format %(path)s:%(row)s:%(col)s:%(code)s:
//!   %(text)s`: stdout lines `path:row:col:code:message` (stderr empty),
//!   split from the left because messages contain colons. Findings are
//!   points at the reported 1-based position; `E`/`F` codes are errors,
//!   `W`/`C` codes are warnings, any other family is a grammar mismatch.
//!   Clean is empty output on exit 0; findings exit 1.
//! * pylint `--output-format=json`: stdout JSON array, one object per
//!   message (`type`, `symbol`, `message`, `message-id`, `line`, `column`,
//!   `endLine`, `endColumn`, `path`). Columns are 0-based, so placement
//!   adds one; a null end is a point range. `fatal`/`error` are errors,
//!   `warning`/`refactor`/`convention` are warnings, `info`/
//!   `information` is info; anything else is a grammar mismatch. Clean is
//!   `[]` on exit 0; findings exit with the bit-encoded class mask.
//! * Biome `lint --reporter=json`: stdout JSON
//!   `{summary, diagnostics[{severity, message, category,
//!   location{path,start{line,column},end}}], command}`. `severity`
//!   `warning`/`error`/`info` map onto [`ToolSeverity`]; `category` is the
//!   rule ID (`lint/...` or `parse`); positions are 1-based and must be
//!   nonzero. Clean is `[]` on exit 0; findings exit 1. `fix` edits are
//!   ignored: Biome lint is check-only and converges on format.
//! * Biome `format --reporter=json` (check): the same envelope, but each
//!   unformatted file yields one `category: "format"` diagnostic at
//!   `0:0` with no diff. The parser normalizes each to one `1:1` format
//!   finding (`rule_id` empty, `file is not formatted`, warning), mirroring
//!   Taplo/Buildifier; clean is `[]` on exit 0, findings exit 1.
//! * ESLint `-c <config> -f json`: stdout JSON array, one object per file
//!   (`filePath`, `messages[{ruleId, severity, message, line, column,
//!   endLine, endColumn, fatal}]`). `severity` 2 is error, 1 is warning;
//!   anything else is a grammar mismatch. A null `ruleId` with `fatal`
//!   is a syntax error (empty rule ID); a null `ruleId` without `fatal`
//!   is an ignored file (outside the base path or matching no config)
//!   and fails as a grammar mismatch, never a silent pass. Missing ends
//!   are point ranges. `fix`/`suggestions` are ignored: the runner
//!   converges via `--fix` re-runs, never by applying parsed edits. Clean
//!   is empty messages on exit 0; findings exit 1.
//! * Prettier `--no-config --no-editorconfig --check`: findings are the
//!   stderr `[warn] <file>` lines (one per unformatted file, reported
//!   relative to the working directory even for absolute arguments, so
//!   the caller passes workspace-relative mirror paths like Ty and
//!   re-anchors them). Each becomes one `1:1` format finding (empty rule,
//!   `file is not formatted`, warning). The `[warn] Code style issues`
//!   summary and `Checking formatting...` stdout are skipped. Clean is
//!   exit 0 with no warn lines; findings exit 1.

pub mod biome;
pub mod buildifier;
pub mod eslint;
pub mod flake8;
pub mod markdown;
pub mod prettier;
pub mod pydoclint;
pub mod pylint;
pub mod ruff;
pub mod rust;
pub mod rustfmt;
pub mod taplo;
pub mod tsc;
pub mod ty;
pub mod vale;

pub use biome::{parse_biome_format, parse_biome_lint};
pub use buildifier::parse_buildifier;
pub use eslint::parse_eslint;
pub use flake8::parse_flake8;
pub use markdown::parse_markdown_findings;
pub use prettier::parse_prettier_check;
pub use pydoclint::parse_pydoclint;
pub use pylint::parse_pylint;
pub use ruff::{parse_ruff, parse_ruff_format};
pub use rust::{parse_clippy, parse_rustc};
pub use rustfmt::parse_rustfmt;
pub use taplo::{parse_taplo_format_check, parse_taplo_lint};
pub use tsc::parse_tsc;
pub use ty::parse_ty;
pub use vale::parse_vale;

use crate::{Finding, TextPosition};

/// One parsed finding, still addressed by the scratch-absolute path the
/// tool reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFinding {
    pub file: String,
    pub finding: Finding,
}

/// Grammar or attribution failure. Every variant is an action failure,
/// never a skipped finding.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    /// Tool stdout is not the pinned JSON grammar.
    #[error("{tool} output is not the pinned JSON grammar: {detail}")]
    Json { tool: &'static str, detail: String },
    /// Check output matches neither findings nor clean for the pinned
    /// tool, or a placed span/level is outside the pinned grammar.
    #[error("{tool} output is outside the pinned grammar: {detail}")]
    Shape { tool: &'static str, detail: String },
    /// A reported path is not one of the checked scratch files.
    #[error("{tool} reported an unchecked file: {path}")]
    UnknownFile { tool: &'static str, path: String },
    /// Vale refused without a usable config (E100/E201 envelope).
    #[error("vale needs a usable config: {detail}")]
    ValeConfig { detail: String },
}

/// Resolves a tool-reported path against the checked scratch files.
fn known<'a>(tool: &'static str, files: &[&'a str], path: &str) -> Result<&'a str, ParseError> {
    files
        .iter()
        .find(|file| **file == path)
        .copied()
        .ok_or_else(|| ParseError::UnknownFile {
            tool,
            path: path.to_owned(),
        })
}

fn point(line: u64, column: u64) -> (TextPosition, Option<TextPosition>) {
    (TextPosition { line, column }, None)
}

fn missing(tool: &'static str, what: &str, line: &str) -> ParseError {
    ParseError::Shape {
        tool,
        detail: format!("malformed {what}: {line}"),
    }
}

fn code_name(code: Option<i32>) -> String {
    code.map_or_else(|| "signal".to_owned(), |code| code.to_string())
}

#[cfg(test)]
mod tests {
    // Error-format stability lives with the shared error type;
    // per-family grammar pins live in their family modules.
    use super::ParseError;

    #[test]
    fn error_display_is_stable() {
        assert!(ParseError::Json {
            tool: "vale",
            detail: "x".to_owned()
        }
        .to_string()
        .contains("vale"));
        assert!(ParseError::UnknownFile {
            tool: "taplo",
            path: "/s/x".to_owned()
        }
        .to_string()
        .contains("/s/x"));
        assert!(ParseError::ValeConfig {
            detail: "E100".to_owned()
        }
        .to_string()
        .contains("E100"));
    }
}
