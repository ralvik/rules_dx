//! Argument tokenizer and `clap`-error mapping (frozen legacy contract).
//!
//! Split from [`super::parser`]: owns the Bazel-verbatim tokenizer
//! (`split_bazel_verbatim`, `tokenize`, `parse_tokens`) and the
//! `clap`-failure mapping (`map_clap_error` plus its `invalid_token`,
//! `invalid_value`, `recover_token`, `leading_flag`,
//! `is_command_positional` helpers). The full `parse` validation
//! (scope shapes, per-command option ownership, output-contract gates,
//! profile flags) stays in [`super::parser`].
//! Strict dx CLI surface (See: `docs/cli/cli-contract.md`): exact long
//! names only, help flag-only with no `help` verb (`disable_help_subcommand`),
//! `dx bazel` tails forward verbatim while every other shape parses whole;
//! attached `=value` echoes the whole token and missing values name the
//! bare flag, pinned by strict fixtures plus help goldens.

use clap::Parser;

use super::command::Command;
use super::grammar::{Cli, VALUE_OPTIONS};
use super::{help, suggest, ArgsError};

/// Finds the `bazel` command word when it owns the tail: the first
/// positional token, skipping value-option payloads exactly like the
/// legacy hand-rolled tokenizer did, so `dx bazel ...` forwards verbatim
/// while `dx --output bazel build` still binds `bazel` as the output
/// value. Returns `None` once a bare `--` is seen (everything after it
/// is Bazel-owned regardless of command) or when a value option is
/// missing its payload (the full parse then reports the missing value).
///
/// Stays hand-rolled (fallback): it routes `argv` *before* the
/// grammar runs, deciding which prefix clap parses and which tail forwards
/// verbatim. A `value_parser` runs inside parsing on one value and cannot
/// own the tail, and the Bazel tail is foreign syntax by contract.
fn split_bazel_verbatim(args: &[String]) -> Option<usize> {
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--" {
            return None;
        }
        if arg.starts_with('-') {
            let name = arg.split_once('=').map_or(arg.as_str(), |(name, _)| name);
            if !arg.contains('=') && VALUE_OPTIONS.contains(&name) {
                match args.get(index + 1) {
                    Some(next) if !next.starts_with("--") && next != "--" => index += 2,
                    _ => return None,
                }
                continue;
            }
            index += 1;
            continue;
        }
        return (arg == "bazel").then_some(index);
    }
    None
}

/// Reads the clap invalid-argument context (`--flag <VALUE>` render or
/// bare token) as a string.
/// Shared plumbing; the suggestion/command mapping below stays local.
/// See: `cli/output/src/clap_errors.rs`.
fn invalid_token(error: &clap::Error) -> Option<String> {
    let token = dx_output::invalid_token(error);
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

/// Reads the clap invalid-value context (empty when an option value is
/// missing, the offending value otherwise).
/// Shared plumbing. See: `cli/output/src/clap_errors.rs`.
fn invalid_value(error: &clap::Error) -> Option<String> {
    dx_output::rejected_value(error)
}

/// Recovers the exact offending `argv` token for an unknown option:
/// clap reports the bare flag name for `--flag=value` spellings, while
/// the contract pins the whole token.
/// Shared plumbing. See: `cli/output/src/clap_errors.rs`.
fn recover_token(args: &[String], token: Option<String>) -> String {
    dx_output::recover_unknown_token(args, &token.unwrap_or_default())
}

/// Extracts the leading `--flag` from a clap missing-value render such
/// as `--output <OUTPUT>`.
/// Shared plumbing. See: `cli/output/src/clap_errors.rs`.
fn leading_flag(token: &str) -> String {
    dx_output::leading_flag(token).to_owned()
}

/// True when a clap invalid-argument render names the command
/// positional (`<COMMAND>`): the only `InvalidValue` source that is a
/// command word rather than an option value.
fn is_command_positional(token: &str) -> bool {
    token
        .trim_matches(|cut| cut == '<' || cut == '>' || cut == '[' || cut == ']')
        .eq_ignore_ascii_case("command")
}

/// Maps a clap parse failure back onto [`ArgsError`] so the contract
/// surface never changes: unknown commands (including `ValueEnum`
/// rejections of the command positional) stay unknown commands with
/// typo hints, unknown flags (including `=value` on booleans) stay
/// unknown options, and missing option values stay missing values.
/// `--help`/`-h` and `--version`/`-V` render from the same grammar
///  as [`ArgsError::Help`], never as usage errors.
fn map_clap_error(args: &[String], error: &clap::Error) -> ArgsError {
    use clap::error::ErrorKind;
    match error.kind() {
        ErrorKind::DisplayHelp => {
            let text = match help::help_command_in(args) {
                Some(command) => help::render_command_help(command),
                None => help::render_top_help(),
            };
            ArgsError::Help { text }
        }
        ErrorKind::DisplayVersion => {
            use clap::CommandFactory;
            ArgsError::Help {
                text: Cli::command().render_version().to_string(),
            }
        }
        ErrorKind::UnknownArgument | ErrorKind::TooManyValues => {
            let option = recover_token(args, invalid_token(error));
            let suggestion =
                suggest::clap_suggestion(error).or_else(|| suggest::suggest_option(&option));
            ArgsError::UnknownOption { option, suggestion }
        }
        ErrorKind::InvalidValue => {
            let token = invalid_token(error).unwrap_or_default();
            if invalid_value(error).is_none_or(|value| value.is_empty()) {
                ArgsError::MissingValue {
                    option: leading_flag(&token),
                }
            } else if is_command_positional(&token) {
                // The command positional is a `ValueEnum`, so unknown
                // command words fail here, not in `parse`: map them back
                // onto the unknown-command surface with typo hints
                //. A lone `-` still reads as an unknown
                // option, exactly like the pre-`ValueEnum` tokenizer did.
                let value = invalid_value(error).unwrap_or(token);
                if value.starts_with('-') {
                    let option = recover_token(args, Some(value.clone()));
                    let suggestion = suggest::clap_suggestion(error)
                        .or_else(|| suggest::suggest_option(&option));
                    ArgsError::UnknownOption { option, suggestion }
                } else {
                    let command = recover_token(args, Some(value.clone()));
                    let suggestion = suggest::clap_command_suggestion(error)
                        .or_else(|| suggest::suggest_command(&command));
                    ArgsError::UnknownCommand {
                        command,
                        suggestion,
                    }
                }
            } else {
                let option = recover_token(args, Some(token));
                let suggestion =
                    suggest::clap_suggestion(error).or_else(|| suggest::suggest_option(&option));
                ArgsError::UnknownOption { option, suggestion }
            }
        }
        _ => ArgsError::UnknownOption {
            option: error
                .render()
                .to_string()
                .lines()
                .next()
                .unwrap_or("dx")
                .trim()
                .to_owned(),
            suggestion: None,
        },
    }
}

/// Runs the clap tokenizer over `args` (without the executable name).
fn parse_tokens(args: &[String]) -> Result<Cli, ArgsError> {
    Cli::try_parse_from(std::iter::once("dx".to_owned()).chain(args.iter().cloned()))
        .map_err(|error| map_clap_error(args, &error))
}

/// Tokenizes `args` into the clap-classified [`Cli`] plus the verbatim
/// Bazel tail. `dx bazel` owns every token after the command word, so
/// its tail never reaches clap; every other shape parses whole.
pub(crate) fn tokenize(args: &[String]) -> Result<(Cli, Vec<String>), ArgsError> {
    if let Some(at) = split_bazel_verbatim(args) {
        // The prefix holds flags only (the scan stops at the first
        // positional and bails at `--`), so no positionals are lost; the
        // command word itself is the `bazel` token the scan stopped at.
        let mut cli = parse_tokens(&args[..at])?;
        cli.command = Some(Command::Bazel);
        // The first `--` still separates (it is dropped, the rest
        // forwards), exactly like the loop's separator check running
        // before the verbatim arm did.
        let mut bazel_options = Vec::new();
        let mut tail = args[at + 1..].iter();
        for arg in tail.by_ref() {
            if arg == "--" {
                break;
            }
            bazel_options.push(arg.clone());
        }
        bazel_options.extend(tail.cloned());
        return Ok((cli, bazel_options));
    }
    let mut cli = parse_tokens(args)?;
    let bazel_options = std::mem::take(&mut cli.bazel_options);
    Ok((cli, bazel_options))
}
