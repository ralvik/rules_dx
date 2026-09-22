//! Shard writer binary: thin CLI shim over the shard codec library.
//!
//! The `dx_codegen_shard` Starlark rule invokes this writer with one
//! contributor's validated record fields; the binary re-validates and emits
//! the binary `DxCodegenShard` protobuf. All record semantics live in the
//! library and are unit-tested there.
//!
//! Usage:
//! ```text
//! codegen_shard_writer --producer LABEL --language LANG \
//!   --entry LOGICAL_PATH|IMPORT_ROOT|NAMESPACE[--entry ...] \
//!   --entry LOGICAL_PATH|IMPORT_ROOT|NAMESPACE|EXEC_PATH[--entry ...] \
//!   --entry LOGICAL_PATH|IMPORT_ROOT|NAMESPACE|EXEC_PATH|REPLACES [--entry ...] \
//!   --output OUT.dxcodegen.pb
//! ```
//!
//! The three-part entry form declares a logical-only entry (empty
//! exec path, requiring no materialized artifact). The four-part form
//! declares the BEP-matching exec-path suffix for the backing artifact.
//! The five-part form additionally declares the replacement contract:
//! REPLACES must equal LOGICAL_PATH with non-empty EXEC_PATH,
//! identifying the replaced checked-in source and the replacing artifact.

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
use std::path::PathBuf;

use clap::{error::ErrorKind, Parser};
use codegen_shard::{
    decode_validated, encode_validated,
    proto::{DxCodegenEntry, DxCodegenShard},
};

fn usage() -> String {
    "usage: codegen_shard_writer --producer LABEL --language LANG --entry LOGICAL|ROOT|NAMESPACE[|EXEC] [--entry ...] --output OUT".into()
}

/// Writer failure.
///
/// Typed writer failure (thiserror) with source chaining for the codec
/// and I/O legs: `Display` keeps the frozen legacy strings
/// (`bad --entry …`, usage-routed, codec/IO detail) byte-identical while
/// callers gain matchable structure instead of `String` plumbing.
#[derive(Debug, thiserror::Error)]
pub enum WriterError {
    /// A `--entry` value with the wrong `|` shape.
    #[error("bad --entry {raw:?}: want LOGICAL_PATH|IMPORT_ROOT|NAMESPACE[|EXEC_PATH[|REPLACES]]")]
    BadEntry { raw: String },
    /// Usage-routed tokenizing failure (unknown argument, missing value,
    /// or missing required scalar): the message already carries the
    /// legacy `usage()` line.
    #[error("{0}")]
    Usage(String),
    /// The shard codec rejected the assembled record.
    #[error("{0}")]
    Codec(#[source] codegen_shard::Error),
    /// The validated bytes failed to write to the output path.
    #[error("{0}")]
    Io(#[source] std::io::Error),
}

/// `argv` tokenizer (frozen legacy contract).
/// `--entry` appends in argument order; scalars keep
/// last-wins repeats; every value option consumes the next token
/// unconditionally (even a `--`-led token), matching the legacy hand loop.
/// `--entry` values validate through [`parse_entry_value`] at tokenize time
///; shape failures map back onto the legacy `bad --entry …`
/// text via [`parse_error`].
#[derive(Parser)]
#[command(disable_help_flag = true)]
struct Cli {
    #[arg(long, allow_hyphen_values = true, overrides_with = "producer")]
    producer: Option<String>,
    #[arg(long, allow_hyphen_values = true, overrides_with = "language")]
    language: Option<String>,
    #[arg(long, allow_hyphen_values = true, value_parser = parse_entry_value)]
    entry: Vec<DxCodegenEntry>,
    #[arg(long, allow_hyphen_values = true, overrides_with = "output")]
    output: Option<String>,
}

/// Raw `argv` token behind a [`clap::Error`], e.g. `--bogus` or `oops`.
/// Shared plumbing; message formats stay local to the frozen contract.
/// See: `cli/output/src/clap_errors.rs` (`dx_output::invalid_token`).
fn invalid_token(error: &clap::Error) -> String {
    dx_output::invalid_token(error)
}

/// Map `clap` tokenizing failures onto the legacy [`usage`]-routed surface.
/// Reachable kinds: [`ErrorKind::UnknownArgument`], [`ErrorKind::InvalidValue`]
/// (a present flag with no consumable value), and [`ErrorKind::ValueValidation`]
/// (a `--entry` value rejected by [`parse_entry_value`], the only custom value
/// parser). No other parser, conflict, or count error can fire.
fn parse_error(error: clap::Error, args: &[String]) -> WriterError {
    let token = invalid_token(&error);
    match error.kind() {
        // `clap` strips an attached `=value` from the reported token; the
        // legacy loop echoed the whole `argv` element, so recover it.
        // See: `cli/output/src/clap_errors.rs`.
        ErrorKind::UnknownArgument => {
            let echoed = dx_output::recover_unknown_token(args, &token);
            WriterError::Usage(format!("unknown argument {echoed:?}\n{}", usage()))
        }
        // The legacy loop reports a bare usage line here too.
        ErrorKind::InvalidValue => WriterError::Usage(usage()),
        ErrorKind::ValueValidation => {
            // Only `--entry` carries a custom value parser, so any
            // validation failure is a rejected entry: report the legacy
            // `bad --entry …` text through the same parser the tokenizer
            // wraps.
            let raw = rejected_value(&error).unwrap_or_default();
            match parse_entry_value(&raw) {
                Err(legacy) => legacy,
                Ok(_) => WriterError::Usage(dx_output::first_line(&error)),
            }
        }
        _ => WriterError::Usage(dx_output::first_line(&error)),
    }
}

/// Rejected `--entry` value behind a [`clap::Error`], if the error carries a
/// non-empty one.
/// Shared plumbing. See: `cli/output/src/clap_errors.rs`.
fn rejected_value(error: &clap::Error) -> Option<String> {
    dx_output::rejected_value(error)
}

fn parse_args(args: &[String]) -> Result<Cli, WriterError> {
    Cli::try_parse_from(
        std::iter::once("codegen_shard_writer").chain(args.iter().map(|arg| arg as &str)),
    )
    .map_err(|error| parse_error(error, args))
}

/// `clap` value parser for `--entry`: the single source for
/// `LOGICAL|ROOT|NAMESPACE[|EXEC[|REPLACES]]` shape, so tokenizing accepts
/// exactly 3-5 `|`-separated parts and rejections already carry the legacy
/// `bad --entry …` text that [`parse_error`] recovers from the error context.
fn parse_entry_value(raw: &str) -> Result<DxCodegenEntry, WriterError> {
    let parts: Vec<&str> = raw.split('|').collect();
    match parts.len() {
        3 => Ok(DxCodegenEntry {
            logical_path: parts[0].into(),
            import_root: parts[1].into(),
            namespace: parts[2].into(),
            read_only: true,
            exec_path: String::new(),
            replaces: String::new(),
        }),
        4 => Ok(DxCodegenEntry {
            logical_path: parts[0].into(),
            import_root: parts[1].into(),
            namespace: parts[2].into(),
            read_only: true,
            exec_path: parts[3].into(),
            replaces: String::new(),
        }),
        5 => Ok(DxCodegenEntry {
            logical_path: parts[0].into(),
            import_root: parts[1].into(),
            namespace: parts[2].into(),
            read_only: true,
            exec_path: parts[3].into(),
            replaces: parts[4].into(),
        }),
        _ => Err(WriterError::BadEntry {
            raw: raw.to_owned(),
        }),
    }
}

fn run(args: &[String]) -> Result<(), WriterError> {
    let cli = parse_args(args)?;
    let output = cli.output.map(PathBuf::from);
    let shard = DxCodegenShard {
        producer: cli.producer.ok_or_else(|| WriterError::Usage(usage()))?,
        language: cli.language.ok_or_else(|| WriterError::Usage(usage()))?,
        entries: cli.entry,
    };
    // LCOV_EXCL_STOP - policy: docs/testing/README.md#coverage
    let bytes = encode_validated(&shard).map_err(WriterError::Codec)?;
    // Read back before writing so a codec regression fails the action
    // instead of emitting bytes the CLI would reject.
    decode_validated(&bytes).map_err(WriterError::Codec)?;
    std::fs::write(output.ok_or_else(|| WriterError::Usage(usage()))?, bytes)
        .map_err(WriterError::Io)
}

fn main() {
    // Structured diagnostics: init is idempotent and emits
    // nothing by default; `RUST_LOG` overrides the warn filter. Failures
    // report via `tracing::error!` with the legacy message text, so action
    // diagnostics keep their content while gaining filter control.
    dx_output::init_diagnostics(false);
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(error) = run(&args) {
        tracing::error!("codegen_shard_writer: {error}");
        std::process::exit(1);
    }
}
