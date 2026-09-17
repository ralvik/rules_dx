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
//!   --entry LOGICAL_PATH|IMPORT_ROOT|NAMESPACE|EXEC_PATH [--entry ...] \
//!   --output OUT.dxcodegen.pb
//! ```
//!
//! The three-part entry form declares a logical-only entry (empty
//! exec path, requiring no materialized artifact). The four-part form
//! declares the BEP-matching exec-path suffix for the backing artifact.

// LCOV_EXCL_START - reason: thin binary shim; CLI parsing and file I/O failures are operational action failures verified by build and shard-emission execution, not unit coverage.
use std::path::PathBuf;

use clap::{
    error::{ContextKind, ContextValue, ErrorKind},
    Parser,
};
use codegen_shard::{
    decode_validated, encode_validated,
    proto::{DxCodegenEntry, DxCodegenShard},
};

fn usage() -> String {
    "usage: codegen_shard_writer --producer LABEL --language LANG --entry LOGICAL|ROOT|NAMESPACE[|EXEC] [--entry ...] --output OUT".into()
}

/// `argv` tokenizer. `--entry` appends in argument order; scalars keep
/// last-wins repeats; every value option consumes the next token
/// unconditionally (even a `--`-led token), matching the legacy hand loop.
/// `--entry` values validate through [`parse_entry_value`] at tokenize time
/// (issue #233); shape failures map back onto the legacy `bad --entry …`
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
fn invalid_token(error: &clap::Error) -> String {
    match error.get(ContextKind::InvalidArg) {
        Some(ContextValue::String(token)) => token.clone(),
        Some(ContextValue::Strings(tokens)) => tokens.first().cloned().unwrap_or_default(),
        _ => String::new(),
    }
}

/// Map `clap` tokenizing failures onto the legacy [`usage`]-routed surface.
/// Reachable kinds: [`ErrorKind::UnknownArgument`], [`ErrorKind::InvalidValue`]
/// (a present flag with no consumable value), and [`ErrorKind::ValueValidation`]
/// (a `--entry` value rejected by [`parse_entry_value`], the only custom value
/// parser). No other parser, conflict, or count error can fire.
fn parse_error(error: clap::Error, args: &[String]) -> String {
    let token = invalid_token(&error);
    match error.kind() {
        // `clap` strips an attached `=value` from the reported token; the
        // legacy loop echoed the whole `argv` element, so recover it.
        ErrorKind::UnknownArgument => {
            let echoed = args
                .iter()
                .find(|arg| *arg == &token)
                .or_else(|| {
                    args.iter()
                        .find(|arg| arg.starts_with(&format!("{token}=")))
                })
                .map_or(token.clone(), Clone::clone);
            format!("unknown argument {echoed:?}\n{}", usage())
        }
        // The legacy loop reports a bare usage line here too.
        ErrorKind::InvalidValue => usage(),
        ErrorKind::ValueValidation => {
            // Only `--entry` carries a custom value parser, so any
            // validation failure is a rejected entry: report the legacy
            // `bad --entry …` text through the same parser the tokenizer
            // wraps.
            let raw = rejected_value(&error).unwrap_or_default();
            match parse_entry_value(&raw) {
                Err(legacy) => legacy,
                Ok(_) => error
                    .to_string()
                    .lines()
                    .next()
                    .unwrap_or("invalid arguments")
                    .to_owned(),
            }
        }
        _ => error
            .to_string()
            .lines()
            .next()
            .unwrap_or("invalid arguments")
            .to_owned(),
    }
}

/// Rejected `--entry` value behind a [`clap::Error`], if the error carries a
/// non-empty one.
fn rejected_value(error: &clap::Error) -> Option<String> {
    let invalid = error.get(ContextKind::InvalidValue)?;
    let raw = match invalid {
        ContextValue::String(value) => value.clone(),
        ContextValue::Strings(values) => values.first().cloned().unwrap_or_default(),
        _ => String::new(),
    };
    if raw.is_empty() {
        None
    } else {
        Some(raw)
    }
}

fn parse_args(args: &[String]) -> Result<Cli, String> {
    Cli::try_parse_from(
        std::iter::once("codegen_shard_writer").chain(args.iter().map(|arg| arg as &str)),
    )
    .map_err(|error| parse_error(error, args))
}

/// `clap` value parser for `--entry` (issue #233): the single source for
/// `LOGICAL|ROOT|NAMESPACE[|EXEC]` shape, so tokenizing accepts exactly 3-4
/// `|`-separated parts and rejections already carry the legacy
/// `bad --entry …` text that [`parse_error`] recovers from the error context.
fn parse_entry_value(raw: &str) -> Result<DxCodegenEntry, String> {
    let parts: Vec<&str> = raw.split('|').collect();
    match parts.len() {
        3 => Ok(DxCodegenEntry {
            logical_path: parts[0].into(),
            import_root: parts[1].into(),
            namespace: parts[2].into(),
            read_only: true,
            exec_path: String::new(),
        }),
        4 => Ok(DxCodegenEntry {
            logical_path: parts[0].into(),
            import_root: parts[1].into(),
            namespace: parts[2].into(),
            read_only: true,
            exec_path: parts[3].into(),
        }),
        _ => Err(format!(
            "bad --entry {raw:?}: want LOGICAL_PATH|IMPORT_ROOT|NAMESPACE[|EXEC_PATH]"
        )),
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let cli = parse_args(args)?;
    let output = cli.output.map(PathBuf::from);
    let shard = DxCodegenShard {
        producer: cli.producer.ok_or_else(usage)?,
        language: cli.language.ok_or_else(usage)?,
        entries: cli.entry,
    };
    let bytes = encode_validated(&shard).map_err(|error| error.to_string())?;
    // Read back before writing so a codec regression fails the action
    // instead of emitting bytes the CLI would reject.
    decode_validated(&bytes).map_err(|error| error.to_string())?;
    std::fs::write(output.ok_or_else(usage)?, bytes).map_err(|error| error.to_string())
}

fn main() {
    // Structured diagnostics (issue #232): init is idempotent and emits
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
// LCOV_EXCL_STOP - reason: end of thin binary shim exclusion.
