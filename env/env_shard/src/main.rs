//! Shard writer binary: thin CLI shim over the shard codec library.
//!
//! The `dx_env_shard` Starlark rule invokes this writer with one
//! contributor's validated record fields; the binary re-validates and emits
//! the binary `DxEnvShard` protobuf. All record semantics live in the
//! library and are unit-tested there.
//!
//! Usage:
//! ```text
//! env_shard_writer --producer LABEL --integration LANG \
//!   --entry KEY|VALUE [--entry ...] \
//!   --entry KEY|VALUE|EXEC_PATH [--entry ...] \
//!   --output OUT.dxenv.pb
//! ```
//!
//! The two-part entry form declares a logical-only identity input (empty
//! exec path, requiring no materialized artifact). The three-part form
//! declares the BEP-matching exec-path suffix for the backing artifact.

// LCOV_EXCL_START - reason: thin binary shim; CLI parsing and file I/O failures are operational action failures verified by build and shard-emission execution, not unit coverage.
use std::path::PathBuf;

use clap::{
    error::{ContextKind, ContextValue, ErrorKind},
    Parser,
};
use env_shard::{
    decode_validated, encode_validated,
    proto::{DxEnvEntry, DxEnvShard},
};

/// Shard writer failure (issue #230).
///
/// Variants render the legacy operational messages verbatim.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EnvShardError {
    /// `clap` tokenizing failure mapped onto the legacy surface.
    #[error("{message}")]
    Args { message: String },
    /// `--entry` value without `KEY|VALUE[|EXEC_PATH]` shape.
    #[error("bad --entry {raw:?}: want KEY|VALUE[|EXEC_PATH]")]
    BadEntry { raw: String },
    /// Required field or usage error.
    #[error("{message}")]
    Usage { message: String },
    /// Codec validation failed.
    #[error("{detail}")]
    Codec { detail: String },
    /// Output write failed.
    #[error("{detail}")]
    Io { detail: String },
}

fn usage() -> String {
    "usage: env_shard_writer --producer LABEL --integration LANG --entry KEY|VALUE[|EXEC] [--entry ...] --output OUT".into()
}

/// `argv` tokenizer. `--entry` appends in argument order; scalars keep
/// last-wins repeats; every value option consumes the next token
/// unconditionally (even a `--`-led token), matching the legacy hand loop.
/// Only tokenizing moves to `clap`; entry-shape validation is untouched.
#[derive(Parser)]
#[command(disable_help_flag = true)]
struct Cli {
    #[arg(long, allow_hyphen_values = true, overrides_with = "producer")]
    producer: Option<String>,
    #[arg(long, allow_hyphen_values = true, overrides_with = "integration")]
    integration: Option<String>,
    #[arg(long, allow_hyphen_values = true)]
    entry: Vec<String>,
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
/// Only [`ErrorKind::UnknownArgument`] and [`ErrorKind::InvalidValue`] (a
/// present flag with no consumable value) are reachable: every option takes
/// plain strings, so no value parser, conflict, or count error can fire.
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
        _ => error
            .to_string()
            .lines()
            .next()
            .unwrap_or("invalid arguments")
            .to_owned(),
    }
}

fn parse_args(args: &[String]) -> Result<Cli, EnvShardError> {
    Cli::try_parse_from(
        std::iter::once("env_shard_writer").chain(args.iter().map(|arg| arg as &str)),
    )
    .map_err(|error| EnvShardError::Args {
        message: parse_error(error, args),
    })
}

fn parse_entry(raw: &str) -> Result<DxEnvEntry, EnvShardError> {
    let parts: Vec<&str> = raw.split('|').collect();
    match parts.len() {
        2 => Ok(DxEnvEntry {
            key: parts[0].into(),
            value: parts[1].into(),
            exec_path: String::new(),
        }),
        3 => Ok(DxEnvEntry {
            key: parts[0].into(),
            value: parts[1].into(),
            exec_path: parts[2].into(),
        }),
        _ => Err(EnvShardError::BadEntry {
            raw: raw.to_owned(),
        }),
    }
}

fn run(args: &[String]) -> Result<(), EnvShardError> {
    let cli = parse_args(args)?;
    let mut entries = Vec::with_capacity(cli.entry.len());
    for raw in &cli.entry {
        entries.push(parse_entry(raw)?);
    }
    let output = cli.output.map(PathBuf::from);
    let shard = DxEnvShard {
        producer: cli
            .producer
            .ok_or_else(|| EnvShardError::Usage { message: usage() })?,
        integration: cli
            .integration
            .ok_or_else(|| EnvShardError::Usage { message: usage() })?,
        entries,
    };
    let bytes = encode_validated(&shard).map_err(|error| EnvShardError::Codec {
        detail: error.to_string(),
    })?;
    // Read back before writing so a codec regression fails the action
    // instead of emitting bytes the CLI would reject.
    decode_validated(&bytes).map_err(|error| EnvShardError::Codec {
        detail: error.to_string(),
    })?;
    let output = output.ok_or_else(|| EnvShardError::Usage { message: usage() })?;
    dx_atomic_fs::write_atomic(&output, &bytes).map_err(|error| EnvShardError::Io {
        detail: error.to_string(),
    })
}

fn main() {
    // Structured diagnostics (issue #232): init is idempotent and emits
    // nothing by default; `RUST_LOG` overrides the warn filter. Failures
    // report via `tracing::error!` with the legacy message text, so action
    // diagnostics keep their content while gaining filter control.
    dx_output::init_diagnostics(false);
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(error) = run(&args) {
        tracing::error!("env_shard_writer: {error}");
        std::process::exit(1);
    }
}
// LCOV_EXCL_STOP - reason: end of thin binary shim exclusion.
