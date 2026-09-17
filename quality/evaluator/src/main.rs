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
use quality_evaluator::{evaluate, parse_threshold};
use quality_result::decode_validated;

#[derive(Parser)]
#[command(disable_help_flag = true)]
struct Cli {
    /// Materialized result protobuf (`--result` keeps last-wins repeats).
    #[arg(long, allow_hyphen_values = true, overrides_with = "result")]
    result: Option<String>,
    /// Threshold policy name (`--fail_on` keeps last-wins repeats).
    #[arg(
        long = "fail_on",
        allow_hyphen_values = true,
        overrides_with = "fail_on"
    )]
    fail_on: Option<String>,
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
/// Only [`ErrorKind::UnknownArgument`] (incl. `--help`, which was never a
/// real flag here) and [`ErrorKind::InvalidValue`] (a present flag with no
/// consumable value) are reachable: every option takes plain strings, so no
/// value parser, conflict, or count error can fire.
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
            // legacy message names the bare `--flag`.
            let flag = token.split_whitespace().next().unwrap_or(&token);
            format!("missing value for {flag}")
        }
        _ => error
            .to_string()
            .lines()
            .next()
            .unwrap_or("invalid arguments")
            .to_owned(),
    }
}

fn parse_args(args: &[String]) -> Result<Cli, String> {
    Cli::try_parse_from(
        std::iter::once("quality_evaluator").chain(args.iter().map(|arg| arg as &str)),
    )
    .map_err(|error| parse_error(error, args))
}

fn main() {
    if let Err(message) = run() {
        eprintln!("quality_evaluator: {message}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cli = parse_args(&args)?;
    let result_path = cli.result.ok_or("--result is required")?;
    let fail_on = cli.fail_on.ok_or("--fail_on is required")?;
    let output = cli.output.ok_or("--output is required")?;
    let bytes =
        std::fs::read(&result_path).map_err(|e| format!("cannot read {result_path:?}: {e}"))?;
    let result =
        decode_validated(&bytes).map_err(|e| format!("invalid result {result_path:?}: {e:?}"))?;
    let threshold = parse_threshold(&fail_on)?;
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
