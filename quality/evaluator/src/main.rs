//! M03 WP3 evaluator binary: thin CLI shim over the evaluator library.
//! Threshold semantics live in the library and are unit-tested there.
//!
//! Usage:
//! ```text
//! quality_evaluator --result RESULT.pb --fail_on info|warning|error --output MARKER
//! ```
//! The result is decoded and validated before any threshold comparison, so
//! malformed results fail at every threshold. A passing evaluation writes a
//! deterministic validation marker; a failing one exits nonzero with reasons
//! on stderr and writes no output.

// LCOV_EXCL_START - reason: thin binary shim; CLI parsing and file I/O failures are operational action failures verified by build and WP3 evaluator execution, not unit coverage.
use clap::{
    error::{ContextKind, ContextValue, ErrorKind},
    Parser,
};
use quality_evaluator::{evaluate, parse_threshold, Threshold};
use quality_result::decode_validated;

/// `argv` tokenizer (qualified under issue #316: frozen legacy contract).
/// Scalars keep last-wins repeats; every value option consumes the next
/// token unconditionally (even a `--`-led token), matching the legacy loop.
#[derive(Parser)]
#[command(disable_help_flag = true)]
struct Cli {
    /// Materialized result protobuf (`--result` keeps last-wins repeats).
    #[arg(long, allow_hyphen_values = true, overrides_with = "result")]
    result: Option<String>,
    /// Threshold policy name (`--fail_on` keeps last-wins repeats).
    /// Values validate through the canonical [`parse_threshold`] surface
    /// (issue #233): only `info|warning|error` tokenize; anything else maps
    /// back onto the legacy `unknown fail_on …` text in [`parse_error`].
    #[arg(
        long = "fail_on",
        allow_hyphen_values = true,
        overrides_with = "fail_on",
        value_parser = parse_fail_on
    )]
    fail_on: Option<Threshold>,
    /// Marker file written on pass (`--output` keeps last-wins repeats).
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

/// Map `clap` tokenizing failures onto the legacy `run()` error surface.
/// Reachable kinds: [`ErrorKind::UnknownArgument`] (incl. `--help`, which
/// was never a real flag here), [`ErrorKind::InvalidValue`] (a present flag
/// with no consumable value), and [`ErrorKind::ValueValidation`] (a
/// `--fail_on` value rejected by [`parse_fail_on`], the only custom value
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
            format!("unknown flag {echoed:?}")
        }
        ErrorKind::InvalidValue => {
            // `clap` renders the pending option as `--flag <VALUE>`; the
            // legacy message names the bare `--flag`. A missing value
            // carries no rejected value, so the `--fail_on` threshold
            // mapping below cannot misfire on it.
            let flag = token.split_whitespace().next().unwrap_or(&token);
            if flag == "--fail_on" {
                if let Some(raw) = rejected_value(&error) {
                    if let Err(legacy) = parse_threshold(&raw) {
                        return legacy.to_string();
                    }
                }
            }
            format!("missing value for {flag}")
        }
        ErrorKind::ValueValidation => {
            // Only `--fail_on` carries a custom value parser, so any
            // validation failure is a rejected threshold: report the legacy
            // `unknown fail_on …` text through the same `parse_threshold`
            // the parser wraps. The parser only runs on present values, so
            // the re-check rejects too — including an empty value, which
            // carries no value context but rejects the same way.
            let raw = rejected_value(&error).unwrap_or_default();
            match parse_threshold(&raw) {
                Err(legacy) => legacy.to_string(),
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

/// Rejected `--fail_on` value behind a [`clap::Error`], if the error
/// carries a non-empty one. Missing values carry none (or an empty one),
/// which the caller treats as missing rather than rejected.
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

/// `clap` value parser for `--fail_on` (issue #233): the single source is
/// [`parse_threshold`], so tokenizing accepts exactly `info|warning|error`
/// and rejections already carry the legacy `unknown fail_on …` text that
/// [`parse_error`] recovers from the error context.
fn parse_fail_on(raw: &str) -> Result<Threshold, String> {
    parse_threshold(raw).map_err(|error| error.to_string())
}

fn parse_args(args: &[String]) -> Result<Cli, String> {
    Cli::try_parse_from(
        std::iter::once("quality_evaluator").chain(args.iter().map(|arg| arg as &str)),
    )
    .map_err(|error| parse_error(error, args))
}

fn main() {
    // Structured diagnostics (issue #232): init is idempotent and emits
    // nothing by default; `RUST_LOG` overrides the warn filter. Failures
    // report via `tracing::error!` with the legacy message text, so action
    // diagnostics keep their content while gaining filter control.
    dx_output::init_diagnostics(false);
    if let Err(message) = run() {
        tracing::error!("quality_evaluator: {message}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cli = parse_args(&args)?;
    let result_path = cli.result.ok_or("--result is required")?;
    let threshold = cli.fail_on.ok_or("--fail_on is required")?;
    let output = cli.output.ok_or("--output is required")?;
    let bytes =
        std::fs::read(&result_path).map_err(|e| format!("cannot read {result_path:?}: {e}"))?;
    let result =
        decode_validated(&bytes).map_err(|e| format!("invalid result {result_path:?}: {e:?}"))?;
    let evaluation = evaluate(&result, threshold);
    if !evaluation.passed {
        return Err(evaluation.reasons.join("; "));
    }
    let marker = format!(
        "validated {} {} fail_on={}\n",
        result.producer,
        result.capability,
        threshold.name()
    );
    dx_atomic_fs::write_atomic(std::path::Path::new(&output), marker.as_bytes())
        .map_err(|e| format!("cannot write {output:?}: {e}"))?;
    Ok(())
}
// LCOV_EXCL_STOP - reason: end of thin binary shim exclusion.
