//! Per-tool output grammars for the pinned binaries plus Python.
//!
//! Each parser maps one tool's check output onto [`FileFinding`] values
//! addressed by the scratch-absolute path the tool reported; the caller
//! re-roots that path onto the workspace path before placement. Parsers
//! never spawn processes and never invent positions: anything outside the
//! pinned grammar is a [`ParseError`], which the runner surfaces as an
//! action failure. Every entry point first enforces [`MAX_OUTPUT_BYTES`]
//! so unbounded tool output fails closed as [`ParseError::TooLarge`].
//!
//! Pinned shapes (probed against the binaries, Python probes in
//! the evidence):
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
//! * JVM SARIF (Checkstyle `-f sarif`, PMD `-f sarif`, SpotBugs
//!   `-sarif`, ktlint `--reporter=sarif`, all on stdout): SARIF 2.1.0
//!   `runs[].results[]` with `ruleId`, `level`, `message.text`, and
//!   `locations[].physicalLocation` (`artifactLocation.uri` plus
//!   `region.startLine/startColumn/endLine/endColumn`). URIs are `file:`
//!   paths or workspace-relative mirrors; missing `level` is a warning,
//!   `error`/`warning`/`note`/`none` map onto [`ToolSeverity`], anything
//!   else is a grammar mismatch. Clean is empty `results` on exit 0;
//!   findings exit 1.
//! * google-java-format `--dry-run --set-exit-if-changed` and ktfmt
//!   `--dry-run`: stdout lists the absolute paths that would change,
//!   one per line (clean prints nothing). Each listed path becomes one
//!   `1:1` format finding (empty rule, `file is not formatted`,
//!   warning). Clean is empty output on exit 0; findings exit 1.
//! * Error Prone (javac log on stderr, stdout empty): stderr lines
//!   `<path>:<line>: error|warning: [Check] <message>` (unbracketed
//!   javac headers report under rule `javac`, never silently dropped).
//!   Indented continuations (source echo, `^` caret, `(see ...)` link,
//!   `Did you mean ...?`, `symbol:`/`location:` detail), `Note:` lines,
//!   and `N error(s)/warning(s)` summaries are skipped. Column forms,
//!   `-Werror` summaries, and pathless global warnings are grammar
//!   mismatches until observed upstream. Check-only: findings never
//!   carry suggestions; fixes travel as declared `error-prone.patch`
//!   outputs (see `commands::error_prone_patch`), never `IN_PLACE`.
//! * Scalafmt `--check`: stdout unified diff with `--- a/<path>` headers;
//!   exit 0 clean, exit 1 dirty with one `1:1` finding per header.
//! * Scalafix callback NDJSON: one JSON record per line with rule plus
//!   positions; console-parse rejected, exit 0 only, check-only.
//! * CSharpier `check`: stdout one unformatted path per line; exit 0
//!   clean, exit 1 dirty with one `1:1` finding per line.
//! * Fantomas `check --json`: stdout JSON `{files:[{path,status}]}`;
//!   exit 0 all unchanged, exit 99 with `needs-formatting` findings.
//! * Roslyn aggregated SARIF 2.1: union of per-pivot `runs[].results[]`
//!   with single schema plus version; single-SARIF and merged-run rejected.
//! * FSharpLint library NDJSON: one JSON record per line with rule plus
//!   full range; console-parse rejected, exit 0 only, check-only.
//! * Buf `lint --error-format=json`: stdout JSONL, one object per line
//!   with `path,start_line,start_column,end_line,end_column,type,
//!   message`; no SARIF, clean is empty on exit 0, findings exit 1.
//! * Buf `format --diff --exit-code`: stdout unified diff with
//!   `--- a/<path>` headers; exit 0 clean, exit 1 dirty.
//! * qmlformat check: stdout one unformatted path per line; exit 0
//!   clean, exit 1 dirty with one `1:1` finding per line.
//! * qmllint `--json -`: stdout JSON `{diagnostics:[{file,line,
//!   column,rule,message,severity}]}`; exit 0 clean, exit 1 dirty.
//! * clang-format check: stdout unified diff with `--- a/<path>` headers;
//!   exit 0 clean, exit 1 dirty with one `1:1` finding per header.
//! * `gofumpt -d`: stdout unified diff with `--- a/<path>` headers;
//!   exit 0 with empty output clean, exit 0 with diff dirty with one
//!   `1:1` finding per header (like `gofmt -d`); any other exit is a
//!   grammar mismatch.
//! * clang-tidy text diagnostics on stderr: one
//!   `<path>:<line>:<col>: <warning|error>: <message> [<check>]` line
//!   per finding; the trailing `[check]` names the rule. Exit 0 clean,
//!   exit 1 dirty, check-only.
//! * cppcheck `--xml --xml-version=2` on stderr: `<error id severity
//!   msg>` elements with a first `<location file line column>` each;
//!   severity `error` maps onto error, `warning`/`style`/`performance`/
//!   `portability` onto warning, `information` onto info. Exit 0 with no
//!   `<error ` elements clean, exit 1 dirty, check-only.
//! * staticcheck `-f json`: stdout JSON array, one object per finding
//!   (`code`, `severity`, `location:{file,line,column}`, optional `end`,
//!   `message`). A missing `end` is a point range. Clean is `[]` on
//!   exit 0; findings exit 1, check-only.
//! * `go vet` text diagnostics on stderr: one `<path>:<line>:<col>:
//!   <message>` line per finding (split from the left because messages
//!   contain colons); every diagnostic is a warning. Exit 0 clean,
//!   exit 1 dirty, check-only.
//! * errcheck text diagnostics on stdout: one `<path>:<line>:<col>:
//!   <message>` line per finding; every diagnostic is a warning. Exit 0
//!   clean, exit 1 dirty, check-only.
//! * Interpreted/file-family cohort: diff formatters (`cue`, `jsonnetfmt`,
//!   `pkl`, `modfmt`, `terraform -check -diff`, `yamlfmt -lint`, `shfmt
//!   -d`, `standardrb --check`, `djlint --reformat --check`) emit unified
//!   diff with `--- a/<path>` headers (one `1:1` finding per header);
//!   lint tools parse text (`djlint --lint`, `psscriptanalyzer`,
//!   `yamllint`, `shellcheck --format=gcc`, `keep-sorted`) or JSON
//!   (`stylelint --formatter json`, `rubocop --format json`); all
//!   check-only except formatters whole-file rewrite.
//!
//! See: `docs/quality/tool-integrations.md#initial-adapter-qualification`

pub mod biome;
pub mod buf;
pub mod buildifier;
pub mod checkstyle;
pub mod clang_format;
pub mod clang_tidy;
pub mod cppcheck;
pub mod csharpier;
pub mod cue;
pub mod djlint;
pub mod errcheck;
pub mod error_prone;
pub mod eslint;
pub mod fantomas;
pub mod flake8;
pub mod fsharplint;
pub mod gofumpt;
pub mod google_java_format;
pub mod govet;
pub mod jsonnetfmt;
pub mod keep_sorted;
pub mod ktfmt;
pub mod ktlint;
pub mod markdown;
pub mod modfmt;
pub mod pkl;
pub mod pmd;
pub mod prettier;
pub mod psscriptanalyzer;
pub mod pydoclint;
pub mod pylint;
pub mod qmlformat;
pub mod qmllint;
pub mod roslyn;
pub mod rubocop;
pub mod ruff;
pub mod rust;
pub mod rustfmt;
pub mod sarif;
pub mod scalafix;
pub mod scalafmt;
pub mod shellcheck;
pub mod shfmt;
pub mod spotbugs;
pub mod standardrb;
pub mod staticcheck;
pub mod stylelint;
pub mod taplo;
pub mod terraform;
pub mod tsc;
pub mod ty;
pub mod vale;
pub mod yamlfmt;
pub mod yamllint;

pub use biome::{parse_biome_format, parse_biome_lint};
pub use buf::{parse_buf_format, parse_buf_lint};
pub use buildifier::parse_buildifier;
pub use checkstyle::parse_checkstyle;
pub use clang_format::parse_clang_format;
pub use clang_tidy::parse_clang_tidy;
pub use cppcheck::parse_cppcheck;
pub use csharpier::parse_csharpier;
pub use cue::parse_cue;
pub use djlint::{parse_djlint, parse_djlint_format};
pub use errcheck::parse_errcheck;
pub use error_prone::parse_error_prone;
pub use eslint::parse_eslint;
pub use fantomas::parse_fantomas;
pub use flake8::parse_flake8;
pub use fsharplint::parse_fsharplint;
pub use gofumpt::parse_gofumpt;
pub use google_java_format::parse_google_java_format;
pub use govet::parse_govet;
pub use jsonnetfmt::parse_jsonnetfmt;
pub use keep_sorted::parse_keep_sorted;
pub use ktfmt::parse_ktfmt;
pub use ktlint::parse_ktlint;
pub use markdown::parse_markdown_findings;
pub use modfmt::parse_modfmt;
pub use pkl::parse_pkl;
pub use pmd::parse_pmd;
pub use prettier::parse_prettier_check;
pub use psscriptanalyzer::parse_psscriptanalyzer;
pub use pydoclint::parse_pydoclint;
pub use pylint::parse_pylint;
pub use qmlformat::parse_qmlformat;
pub use qmllint::parse_qmllint;
pub use roslyn::parse_roslyn;
pub use rubocop::parse_rubocop;
pub use ruff::{parse_ruff, parse_ruff_format};
pub use rust::{parse_clippy, parse_rustc};
pub use rustfmt::parse_rustfmt;
pub use scalafix::parse_scalafix;
pub use scalafmt::parse_scalafmt;
pub use shellcheck::parse_shellcheck;
pub use shfmt::parse_shfmt;
pub use spotbugs::parse_spotbugs;
pub use standardrb::parse_standardrb;
pub use staticcheck::parse_staticcheck;
pub use stylelint::parse_stylelint;
pub use taplo::{parse_taplo_format_check, parse_taplo_lint};
pub use terraform::parse_terraform;
pub use tsc::parse_tsc;
pub use ty::parse_ty;
pub use vale::parse_vale;
pub use yamlfmt::parse_yamlfmt;
pub use yamllint::parse_yamllint;

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
    /// Tool output exceeds the max output size guard.
    #[error("{tool} output exceeds max size {limit} bytes (got {bytes})")]
    TooLarge {
        tool: &'static str,
        bytes: usize,
        limit: usize,
    },
}

/// Maximum tool output bytes any parser accepts (supply-chain bound).
/// See: `docs/quality/tool-integrations.md#initial-adapter-qualification`
pub const MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;

/// Rejects oversized output before parsing (max output size guard).
/// Every parser calls this first so unbounded tool output fails closed
/// as [`ParseError::TooLarge`]. The fuzz property harness feeds
/// arbitrary bytes here; only `ParseError`, never panic (proptest-style
/// mutations run with a std-only xorshift, no new supply-chain dep).
pub fn check_output_size(tool: &'static str, bytes: &[u8]) -> Result<(), ParseError> {
    if bytes.len() > MAX_OUTPUT_BYTES {
        return Err(ParseError::TooLarge {
            tool,
            bytes: bytes.len(),
            limit: MAX_OUTPUT_BYTES,
        });
    }
    Ok(())
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
    use super::{check_output_size, ParseError, MAX_OUTPUT_BYTES};

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
        assert!(ParseError::TooLarge {
            tool: "sarif",
            bytes: MAX_OUTPUT_BYTES + 1,
            limit: MAX_OUTPUT_BYTES,
        }
        .to_string()
        .contains("max size"));
    }

    #[test]
    fn output_size_guard_rejects_oversized() {
        assert!(check_output_size("tsc", b"ok").is_ok());
        assert!(check_output_size("tsc", &vec![b'x'; MAX_OUTPUT_BYTES]).is_ok());
        let big = vec![b'x'; MAX_OUTPUT_BYTES + 1];
        assert_eq!(
            check_output_size("tsc", &big),
            Err(ParseError::TooLarge {
                tool: "tsc",
                bytes: big.len(),
                limit: MAX_OUTPUT_BYTES,
            })
        );
        // Every parser enforces the same guard first.
        assert!(super::tsc::parse_tsc(&big, Some(2), &["/s/a.ts"]).is_err());
        assert!(super::sarif::parse_sarif("sarif-test", &big, Some(1), &["/s/a.java"]).is_err());
    }

    fn xorshift(state: &mut u64) -> u64 {
        // std-only deterministic PRNG for the fuzz property harness.
        let mut x = *state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        *state = x;
        x
    }

    #[test]
    fn fuzz_parsers_never_panic_on_arbitrary_bytes() {
        // Property harness: arbitrary/truncated/mutated bytes yield Ok or
        // ParseError, never panic (cargo fuzz seeds reuse these corpora).
        // Covers representative grammar families: classic text, JSON,
        // SARIF, JSONL, dual-stream, and config-envelope parsers.
        let seeds: &[&[u8]] = &[
            b"{}",
            b"[]",
            b"not json",
            b"/s/a.ts(1,1): error TS1234: msg\n",
            br#"{"version":"2.1.0","runs":[]}"#,
            br#"[{"filePath":"/s/a.js","messages":[{"ruleId":"x","severity":2,"message":"m","line":1,"column":1}]}]"#,
            br#"{"success":false,"files":[{"filename":"/s/a.bzl","formatted":false,"valid":true,"warnings":[]}]}"#,
            b"path:1:1: E100 message\n",
            b"error: bad\n  \xe2\x94\x8c\xe2\x94\x80 /s/x.toml:1:5\n",
            b"Diff in /s/x.rs:1:\n-fn  main(){}\n+fn main() {}\n",
            b"--- a/x.py\n+++ b/x.py\n@@ -1 +1 @@\n-a\n+b\n",
            b"{\"path\":\"x.proto\",\"start_line\":1,\"start_column\":1,\"type\":\"T\",\"message\":\"m\"}\n",
            b"/s/Hello.java:3: error: [DeadException] msg\n1 error\n",
            b"\xff\xfe\x00",
        ];
        let mut state = 0x9E37_79B9_7F4A_7C15u64;
        for round in 0..400 {
            let seed = seeds[round % seeds.len()];
            let mut input = seed.to_vec();
            // Mutate: truncate, flip, or extend with PRNG bytes.
            match xorshift(&mut state) % 3 {
                0 => {
                    let keep = (xorshift(&mut state) as usize) % (input.len() + 1);
                    input.truncate(keep);
                }
                1 => {
                    if !input.is_empty() {
                        let at = (xorshift(&mut state) as usize) % input.len();
                        input[at] ^= (xorshift(&mut state) & 0xFF) as u8;
                    }
                }
                _ => {
                    let extra = (xorshift(&mut state) % 32) as usize;
                    for _ in 0..extra {
                        input.push((xorshift(&mut state) & 0xFF) as u8);
                    }
                }
            }
            let files = [
                "/s/a.ts",
                "/s/a.java",
                "/s/a.js",
                "/s/a.bzl",
                "/s/x.toml",
                "/s/x.rs",
                "/s/a.py",
                "/s/x.proto",
                "/s/Hello.java",
                "/s/dirty.toml",
            ];
            let _ = super::tsc::parse_tsc(&input, Some(2), &files);
            let _ = super::sarif::parse_sarif("fuzz", &input, Some(1), &files);
            let _ = super::ruff::parse_ruff(&input, Some(1), &files);
            let _ = super::ruff::parse_ruff_format(&input, Some(1), &files);
            let _ = super::vale::parse_vale(&input, Some(1), &files);
            let _ = super::rust::parse_clippy(&input, Some(1), &files);
            let _ = super::rust::parse_rustc(&input, Some(1), &files);
            let _ = super::buildifier::parse_buildifier(&input, &input, &files);
            let _ = super::eslint::parse_eslint(&input, Some(1), &files);
            let _ = super::ty::parse_ty(&input, Some(1), &files);
            let _ = super::taplo::parse_taplo_lint(&input, Some(1), &files);
            let _ = super::taplo::parse_taplo_format_check(&input, Some(1), &files);
            let _ = super::rustfmt::parse_rustfmt(&input, &input, Some(1), &files);
            let _ = super::error_prone::parse_error_prone(&input, &input, Some(1), &files);
            let _ = super::buf::parse_buf_lint(&input, Some(1), &files);
            let _ = super::buf::parse_buf_format(&input, Some(1), &files);
            let _ = super::djlint::parse_djlint(&input, Some(1), &files);
            let _ = super::djlint::parse_djlint_format(&input, Some(1), &files);
            let _ = super::shellcheck::parse_shellcheck(&input, Some(1), &files);
            let _ = super::prettier::parse_prettier_check(&input, Some(1), &files);
            let _ = super::govet::parse_govet(&input, Some(1), &files);
            let _ = super::gofumpt::parse_gofumpt(&input, Some(1), &files);
            let _ = super::markdown::parse_markdown_findings(&input, Some(0), &files);
            let _ = super::spotbugs::parse_spotbugs(&input, Some(1), &files);
        }
    }
}
