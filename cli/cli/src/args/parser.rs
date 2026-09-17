//! Invocation parsing for the `dx` CLI (issue #236).
//!
//! Split from `super` (`args.rs`): owns the Bazel-verbatim tokenizer,
//! `clap`-error mapping, and the full `parse` validation (scope shapes,
//! per-command option ownership, output-contract gates, profile flags).
//! The `clap` grammar (`Cli`, `VALUE_OPTIONS`, `cli_command`) lives in
//! the [`super::grammar`] sibling; this module re-exports it so the
//! `crate::args::parser::{Cli, VALUE_OPTIONS, cli_command}` paths stay
//! stable. Re-exported through `super` so the public paths stay
//! `crate::args::parse` and `crate::args::cli_command`.
//!
//! Named `parser` (not `parse`) so the module and the `parse` function
//! can coexist without a namespace collision; the domain is the `parse`
//! half of the `args→command/parse/suggest/help/completion` split.

use clap::Parser;
use dx_output::{OutputMode, Threshold};

use super::command::Command;
use super::{help, suggest};
use super::{ArgsError, Invocation, ReportRequest};

pub use super::grammar::cli_command;
pub(crate) use super::grammar::{Cli, VALUE_OPTIONS};

/// Maps one scope positional onto its shape-specific parse failure:
/// empty scopes name the repository-wide default, package-relative
/// labels name the `//` qualification, and anything else keeps the
/// generic label/path guidance (typo paths and `@` scopes fail later
/// in resolution with `PathNotFound`/`ExternalScope` context).
fn scope_error(scope: &str) -> ArgsError {
    if scope.is_empty() {
        ArgsError::EmptyScope
    } else if scope.starts_with(':') {
        ArgsError::RelativeLabel {
            scope: scope.to_owned(),
        }
    } else {
        ArgsError::InvalidScope {
            scope: scope.to_owned(),
        }
    }
}

/// Finds the `bazel` command word when it owns the tail: the first
/// positional token, skipping value-option payloads exactly like the
/// legacy hand-rolled tokenizer did, so `dx bazel ...` forwards verbatim
/// while `dx --output bazel build` still binds `bazel` as the output
/// value. Returns `None` once a bare `--` is seen (everything after it
/// is Bazel-owned regardless of command) or when a value option is
/// missing its payload (the full parse then reports the missing value).
///
/// Stays hand-rolled (issue #233 fallback): it routes `argv` *before* the
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
fn invalid_token(error: &clap::Error) -> Option<String> {
    match error.get(clap::error::ContextKind::InvalidArg) {
        Some(clap::error::ContextValue::String(token)) => Some(token.clone()),
        Some(clap::error::ContextValue::Strings(tokens)) => tokens.first().cloned(),
        _ => None,
    }
}

/// Reads the clap invalid-value context (empty when an option value is
/// missing, the offending value otherwise).
fn invalid_value(error: &clap::Error) -> Option<String> {
    match error.get(clap::error::ContextKind::InvalidValue) {
        Some(clap::error::ContextValue::String(value)) => Some(value.clone()),
        Some(clap::error::ContextValue::Strings(values)) => values.first().cloned(),
        _ => None,
    }
}

/// Recovers the exact offending `argv` token for an unknown option:
/// clap reports the bare flag name for `--flag=value` spellings, while
/// the contract pins the whole token.
fn recover_token(args: &[String], token: Option<String>) -> String {
    let token = token.unwrap_or_default();
    if args.contains(&token) {
        return token;
    }
    let inline = format!("{token}=");
    if let Some(arg) = args.iter().rfind(|arg| arg.starts_with(&inline)) {
        return arg.clone();
    }
    token
}

/// Extracts the leading `--flag` from a clap missing-value render such
/// as `--output <OUTPUT>`.
fn leading_flag(token: &str) -> String {
    token.split_whitespace().next().unwrap_or(token).to_owned()
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
/// (issue #203) as [`ArgsError::Help`], never as usage errors.
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
                // (issue #199). A lone `-` still reads as an unknown
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
fn tokenize(args: &[String]) -> Result<(Cli, Vec<String>), ArgsError> {
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

/// Parses one `--report` value into its `format=destination` shape.
fn parse_report(value: &str) -> Result<ReportRequest, ArgsError> {
    match value.split_once('=') {
        Some((format, destination)) if !format.is_empty() && !destination.is_empty() => {
            Ok(ReportRequest {
                format: format.to_owned(),
                destination: destination.to_owned(),
            })
        }
        _ => Err(ArgsError::BadReport {
            value: value.to_owned(),
        }),
    }
}

/// Parses a `--min-coverage` value into an integer percent 0-100.
fn parse_min_coverage(value: &str) -> Result<u32, ArgsError> {
    match value.parse::<u32>() {
        Ok(percent) if percent <= 100 => Ok(percent),
        _ => Err(ArgsError::BadMinCoverage {
            value: value.to_owned(),
        }),
    }
}

/// Parses a full `dx` command line without the executable name.
///
/// Global options may appear before or after the command; the first
/// positional argument selects the command. Later positionals are
/// explicit Bazel labels, patterns, or workspace-relative file and
/// directory paths resolved through Bazel during execution; only
/// package-relative labels and empty scopes fail here. Arguments after
/// the first bare `--` forward to Bazel as command options verbatim.
pub fn parse(args: &[String]) -> Result<Invocation, ArgsError> {
    let (cli, bazel_options) = tokenize(args)?;
    let Cli {
        workspace,
        dry_run,
        quiet,
        verbose,
        output,
        report,
        fail_on,
        min_coverage: min_coverage_name,
        check,
        debug,
        release,
        bazel_clean,
        pin,
        rollback,
        configured,
        command: command_name,
        targets,
        ..
    } = cli;
    if workspace.as_deref().is_some_and(str::is_empty) {
        return Err(ArgsError::MissingValue {
            option: "--workspace".to_owned(),
        });
    }
    if pin.as_deref().is_some_and(str::is_empty) {
        return Err(ArgsError::MissingValue {
            option: "--pin".to_owned(),
        });
    }
    let output_name = output.unwrap_or_else(|| "text".to_owned());
    let fail_on_name = fail_on.unwrap_or_else(|| "warning".to_owned());
    let mut reports = Vec::new();
    for value in &report {
        reports.push(parse_report(value)?);
    }
    let mut min_coverage: Option<u32> = None;
    if let Some(value) = &min_coverage_name {
        min_coverage = Some(parse_min_coverage(value)?);
    }
    let command = command_name.ok_or(ArgsError::MissingCommand)?;
    // `dx bazel` tails never reach this check: the verbatim forwarding
    // above owns every token after the command word.
    if command != Command::Bazel {
        for scope in &targets {
            if scope == "-" {
                return Err(ArgsError::UnknownOption {
                    option: scope.clone(),
                    suggestion: None,
                });
            }
            if scope.is_empty() || scope.starts_with(':') {
                return Err(scope_error(scope));
            }
        }
    }
    if command == Command::Clean {
        // `dx clean [--dry-run] [--bazel]` prunes validated unselected
        // managed state with no scopes and no quality/workflow options:
        // `--dry-run` deletes nothing (a `--bazel` forward is listed,
        // never run), and only `--workspace`, `--dry-run`, `--quiet`,
        // and `--bazel` apply.
        if check {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--check".to_owned(),
            });
        }
        if fail_on_name != "warning" {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--fail-on".to_owned(),
            });
        }
        if output_name != "text" {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: format!("--output={output_name}"),
            });
        }
        if let Some(request) = reports.first() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: format!("--report={}={}", request.format, request.destination),
            });
        }
        if let Some(scope) = targets.first() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: scope.clone(),
            });
        }
        if !bazel_options.is_empty() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--".to_owned(),
            });
        }
    } else if bazel_clean {
        return Err(ArgsError::UnsupportedOption {
            command: command.name(),
            option: "--bazel".to_owned(),
        });
    }
    if command.is_managed() {
        // Managed environment/codegen/setup commands (M25 WP5) run one
        // Bazel collection request behind a canonical selection with
        // text prose only: no check mode, no finding thresholds, no
        // standard reports, and no version/clean-only flags.
        // `--bazel` is rejected by the clean-ownership arm above;
        // `--rollback`/`--configured` by the catch-alls below. Scope is
        // repository-wide by default or one exact target label, validated
        // through the shared setup scope rules (identical across the
        // codegen, env, and setup libs by contract); user Bazel options
        // after `--` forward to the collection build.
        if check {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--check".to_owned(),
            });
        }
        if fail_on_name != "warning" {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--fail-on".to_owned(),
            });
        }
        if output_name != "text" {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: format!("--output={output_name}"),
            });
        }
        if let Some(request) = reports.first() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: format!("--report={}={}", request.format, request.destination),
            });
        }
        if pin.is_some() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--pin".to_owned(),
            });
        }
        if let Err(error) = dx_setup::resolve_scope(&targets) {
            return Err(match error {
                dx_setup::ScopeError::MultipleTargets { .. } => ArgsError::UnsupportedOption {
                    command: command.name(),
                    // `MultipleTargets` guarantees at least two
                    // positionals, so the second one exists.
                    option: targets[1].clone(),
                },
                dx_setup::ScopeError::TargetPattern { value }
                | dx_setup::ScopeError::NotTargetLabel { value } => scope_error(&value),
            });
        }
    }
    if command == Command::Audit {
        // Audit plans through `dx_audit` (M26 WP1): family selection
        // plus scope spellings, non-mutating, with SARIF reports and
        // `--fail-on` thresholds. `--check` is meaningless (audit never
        // mutates), Bazel forwards do not apply (no collection build
        // yet), and version/clean-only flags do not apply.
        // Family parsing itself stays in `dx_audit::plan_audit`; args
        // only preserve positionals verbatim (family or scopes).
        if check {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--check".to_owned(),
            });
        }
        if pin.is_some() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--pin".to_owned(),
            });
        }
        if !bazel_options.is_empty() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--".to_owned(),
            });
        }
        // Scopes use the shared label/path shape; the first positional
        // may also be a family (`security`/`license`) preserved verbatim
        // for `dx_audit::plan_audit`. Only package-relative labels and
        // empty scopes fail here.
        for scope in &targets {
            if scope.is_empty() || scope.starts_with(':') {
                return Err(scope_error(scope));
            }
        }
    }
    if command == Command::Update {
        // Update plans through `dx_update` (M26 WP2): dependency-set /
        // package selectors preserved verbatim, mutating without
        // confirmation. Thresholds, standard reports, and check mode do
        // not apply on this path; exact aggregate exit/report mappings
        // stay under O12 qualification.
        if check {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--check".to_owned(),
            });
        }
        if fail_on_name != "warning" {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--fail-on".to_owned(),
            });
        }
        // `update` supports `--output=json` (dry-run planning and the
        // deferred-live error stream `command_started`/`command_finished`
        // like `audit`); `--output=diff` has no patch to emit so it fails
        // fast here, with the shared `supports_diff` gate below as backup.
        if output_name == "diff" {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--output=diff".to_owned(),
            });
        }
        if let Some(request) = reports.first() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: format!("--report={}={}", request.format, request.destination),
            });
        }
        if pin.is_some() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--pin".to_owned(),
            });
        }
        if !bazel_options.is_empty() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--".to_owned(),
            });
        }
        for scope in &targets {
            if scope.is_empty() || scope.starts_with(':') {
                return Err(scope_error(scope));
            }
        }
    }
    if command.is_workflow() {
        // Workflow commands run Bazel verbs directly with Bazel-owned
        // status: finding thresholds and check-mode mutation previews do
        // not apply, so explicit uses fail fast instead of silently
        // doing nothing.
        if check {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--check".to_owned(),
            });
        }
        if fail_on_name != "warning" {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--fail-on".to_owned(),
            });
        }
    }
    // `--min-coverage` belongs to `coverage` only: every other command
    // (workflow siblings, quality, umbrellas, managed, adoption) fails
    // fast instead of silently ignoring the threshold.
    if min_coverage.is_some() && command != Command::Coverage {
        return Err(ArgsError::UnsupportedOption {
            command: command.name(),
            option: "--min-coverage".to_owned(),
        });
    }
    let output = OutputMode::parse(&output_name, quiet).map_err(|_| ArgsError::BadOutput {
        value: output_name.clone(),
    })?;
    let fail_on = Threshold::parse(&fail_on_name).map_err(|_| ArgsError::BadFailOn {
        value: fail_on_name.clone(),
    })?;
    if command.is_adoption() {
        // Adoption/inspect surfaces run local helpers or thin query
        // forwarding: quality-only thresholds/reports and Bazel forwards
        // do not apply. `--check` belongs to `version` (pin drift)
        // only; `--pin` belongs to `version`
        // only. `--rollback`
        // and `--configured` ownership is enforced by the catch-all
        // below.
        if fail_on_name != "warning" {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--fail-on".to_owned(),
            });
        }
        if let Some(request) = reports.first() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: format!("--report={}={}", request.format, request.destination),
            });
        }
        if bazel_clean {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--bazel".to_owned(),
            });
        }
        if !bazel_options.is_empty() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--".to_owned(),
            });
        }
        if check && command != Command::Version {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--check".to_owned(),
            });
        }
        if pin.is_some() && command != Command::Version {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--pin".to_owned(),
            });
        }
        match command {
            Command::Status | Command::Version => {
                if !targets.is_empty() && command == Command::Status {
                    return Err(ArgsError::UnsupportedOption {
                        command: command.name(),
                        option: targets[0].clone(),
                    });
                }
                if !targets.is_empty() && command == Command::Version && pin.is_none() {
                    return Err(ArgsError::UnsupportedOption {
                        command: command.name(),
                        option: targets[0].clone(),
                    });
                }
            }
            Command::Completion => {
                if targets.len() != 1 {
                    return Err(ArgsError::MissingValue {
                        option: "<shell>".to_owned(),
                    });
                }
            }
            Command::Hooks | Command::Watch | Command::Owners | Command::Deps
                if targets.is_empty() =>
            {
                return Err(ArgsError::MissingValue {
                    option: "<scope>".to_owned(),
                });
            }
            // `dx why` resolves one ownership edge per call: exactly one
            // file scope and one target label, e.g.
            // `dx why src/main.rs //app:server`.
            Command::Why if targets.len() != 2 => {
                return Err(ArgsError::MissingValue {
                    option: "<file> <label>".to_owned(),
                });
            }
            _ => {}
        }
    }
    if command == Command::Bazel {
        // `dx bazel` forwards arguments unchanged to the workspace
        // Bazel launcher: no scope resolution, no quality thresholds or
        // reports, and text terminal output only (the child inherits
        // stdio). dx-owned options are rejected only when given before
        // the command word (everything after it already forwarded
        // verbatim above); `--rollback` and `--configured` ownership is
        // enforced by the catch-all below.
        if check {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--check".to_owned(),
            });
        }
        if fail_on_name != "warning" {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--fail-on".to_owned(),
            });
        }
        if output_name != "text" {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: format!("--output={output_name}"),
            });
        }
        if let Some(request) = reports.first() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: format!("--report={}={}", request.format, request.destination),
            });
        }
        if pin.is_some() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--pin".to_owned(),
            });
        }
        if bazel_clean {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--bazel".to_owned(),
            });
        }
    }
    if command == Command::Run || command == Command::Deploy {
        // O52: `dx run` is a local-only single-target launcher with prose
        // lifecycle on stderr. Machine-owned stdout modes are rejected
        // pre-exec so the application keeps the terminal. `dx deploy`
        // shares the terminal contract: text only, no reports, args
        // after `--` forward verbatim to the program.
        if !matches!(output, OutputMode::Text { .. }) {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: format!("--output={output_name}"),
            });
        }
        if let Some(request) = reports.first() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: format!("--report={}={}", request.format, request.destination),
            });
        }
    }
    // Uniform `--output` contract (issue #200): shared `supports_json` /
    // `supports_diff` gate so no command silently ignores a machine-output
    // request. Commands with their own output arm above (clean, managed,
    // update, `bazel`, `run`) already returned the same error; this gate
    // owns adoption/inspect (only `status` supports JSON, none supports
    // diff), `audit` diff, and workflow `build`/`test`/`coverage` diff.
    // Quality, generate, umbrellas, `audit`/`update` JSON, workflow JSON,
    // and `status` JSON pass through to streaming NDJSON execution.
    if output_name == "json" && !command.supports_json() {
        return Err(ArgsError::UnsupportedOption {
            command: command.name(),
            option: "--output=json".to_owned(),
        });
    }
    if output_name == "diff" && !command.supports_diff() {
        return Err(ArgsError::UnsupportedOption {
            command: command.name(),
            option: "--output=diff".to_owned(),
        });
    }
    // Profile flags (issue #179): `--debug`/`--release` are mutually
    // exclusive and belong to `build`, `run`, `test`, and `deploy`
    // only (every other command fails fast instead of silently
    // ignoring the profile).
    if debug && release {
        return Err(ArgsError::ConflictingProfiles);
    }
    if (debug || release)
        && !matches!(
            command,
            Command::Build | Command::Run | Command::Test | Command::Deploy
        )
    {
        return Err(ArgsError::UnsupportedOption {
            command: command.name(),
            option: if debug {
                "--debug".to_owned()
            } else {
                "--release".to_owned()
            },
        });
    }
    // Flag ownership (M30b): `--rollback` belongs to `version` only
    // and `--configured` to the inspect wrappers only. Command blocks
    // above already reject them on their own surfaces; this catch-all
    // keeps every other command (quality, umbrellas, `run`,
    // `generate`, `clean`, managed) failing fast instead of silently
    // ignoring them.
    if rollback && command != Command::Version {
        return Err(ArgsError::UnsupportedOption {
            command: command.name(),
            option: "--rollback".to_owned(),
        });
    }
    if configured
        && command != Command::Owners
        && command != Command::Deps
        && command != Command::Why
    {
        return Err(ArgsError::UnsupportedOption {
            command: command.name(),
            option: "--configured".to_owned(),
        });
    }
    Ok(Invocation {
        command,
        check,
        debug,
        release,
        workspace,
        dry_run,
        quiet,
        verbose,
        output,
        reports,
        fail_on,
        min_coverage,
        targets,
        bazel_options,
        bazel_clean,
        pin,
        rollback,
        configured,
    })
}

#[cfg(test)]
mod tests {
    use super::super::{ArgsError, Command, ReportRequest};
    use super::parse;
    use dx_output::{OutputMode, Threshold};

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn bare_command_parses_with_defaults() {
        let got = parse(&args(&["lint"])).expect("parse");
        assert_eq!(got.command, Command::Lint);
        assert!(!got.check);
        assert!(!got.debug);
        assert!(!got.release);
        assert_eq!(got.workspace, None);
        assert!(!got.dry_run);
        assert!(!got.quiet);
        assert!(!got.verbose);
        assert_eq!(got.output, OutputMode::Text { quiet: false });
        assert!(got.reports.is_empty());
        assert_eq!(got.fail_on, Threshold::Warning);
        assert!(got.targets.is_empty());
        assert!(got.bazel_options.is_empty());
        assert_eq!(got.mode(), "default");
    }

    #[test]
    fn min_coverage_parses_for_coverage_only() {
        let got = parse(&args(&["coverage", "--min-coverage", "80"])).expect("parse");
        assert_eq!(got.command, Command::Coverage);
        assert_eq!(got.min_coverage, Some(80));
        let inline = parse(&args(&["coverage", "--min-coverage=100"])).expect("parse");
        assert_eq!(inline.min_coverage, Some(100));
        let zero = parse(&args(&["coverage", "--min-coverage=0"])).expect("parse");
        assert_eq!(zero.min_coverage, Some(0));
        let bare = parse(&args(&["coverage"])).expect("parse");
        assert_eq!(bare.min_coverage, None);
    }

    #[test]
    fn min_coverage_rejects_bad_values_and_other_commands() {
        assert_eq!(
            parse(&args(&["coverage", "--min-coverage=eighty"])),
            Err(ArgsError::BadMinCoverage {
                value: "eighty".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["coverage", "--min-coverage=101"])),
            Err(ArgsError::BadMinCoverage {
                value: "101".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["coverage", "--min-coverage"])),
            Err(ArgsError::MissingValue {
                option: "--min-coverage".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["test", "--min-coverage=80"])),
            Err(ArgsError::UnsupportedOption {
                command: "test",
                option: "--min-coverage".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["lint", "--min-coverage=80"])),
            Err(ArgsError::UnsupportedOption {
                command: "lint",
                option: "--min-coverage".to_owned(),
            })
        );
    }

    #[test]
    fn check_selects_check_mode() {
        let got = parse(&args(&["format", "--check"])).expect("parse");
        assert_eq!(got.command, Command::Format);
        assert!(got.check);
        assert_eq!(got.mode(), "check");
    }

    #[test]
    fn generate_parses_repo_wide_with_bazel_options() {
        let got = parse(&args(&["generate"])).expect("parse");
        assert_eq!(got.command, Command::Generate);
        assert!(!got.check);
        assert_eq!(got.output, OutputMode::Text { quiet: false });
        assert!(got.targets.is_empty());
        assert!(got.bazel_options.is_empty());
        assert_eq!(got.mode(), "default");
        let scoped =
            parse(&args(&["generate", "--check", "//a:one", "--", "--jobs=4"])).expect("parse");
        assert_eq!(scoped.command, Command::Generate);
        assert!(scoped.check);
        assert_eq!(scoped.targets, args(&["//a:one"]));
        assert_eq!(scoped.bazel_options, args(&["--jobs=4"]));
    }

    #[test]
    fn globals_parse_before_and_after_command() {
        let got = parse(&args(&[
            "--workspace",
            "/repo",
            "--dry-run",
            "--quiet",
            "typecheck",
            "--output=json",
            "--fail-on=error",
        ]))
        .expect("parse");
        assert_eq!(got.command, Command::Typecheck);
        assert_eq!(got.workspace, Some("/repo".to_owned()));
        assert!(got.dry_run);
        assert!(got.quiet);
        assert_eq!(got.output, OutputMode::Json);
        assert_eq!(got.fail_on, Threshold::Error);
    }

    #[test]
    fn quiet_applies_to_text_output() {
        let got = parse(&args(&["lint", "--quiet"])).expect("parse");
        assert_eq!(got.output, OutputMode::Text { quiet: true });
    }

    #[test]
    fn verbose_parses_before_and_after_command_and_stays_orthogonal_to_quiet() {
        // Issue #222: `--verbose` enables tracing diagnostics without
        // changing the machine-output contract; `--quiet` still controls
        // summaries independently.
        let bare = parse(&args(&["lint", "--verbose"])).expect("parse");
        assert!(bare.verbose);
        assert!(!bare.quiet);
        let before = parse(&args(&["--verbose", "lint"])).expect("parse");
        assert!(before.verbose);
        let both = parse(&args(&["lint", "--quiet", "--verbose"])).expect("parse");
        assert!(both.quiet);
        assert!(both.verbose);
        assert_eq!(both.output, OutputMode::Text { quiet: true });
        let help = match parse(&args(&["--verbose", "--help"])) {
            Err(ArgsError::Help { text }) => text,
            other => panic!("want Help, got {other:?}"),
        };
        assert!(help.contains("--verbose"), "top-level help:\n{help}");
    }

    #[test]
    fn reports_are_repeatable_with_format_shape() {
        let got = parse(&args(&[
            "lint",
            "--report",
            "sarif=reports/lint.sarif",
            "--report=sarif=/tmp/extra.sarif",
        ]))
        .expect("parse");
        assert_eq!(
            got.reports,
            vec![
                ReportRequest {
                    format: "sarif".to_owned(),
                    destination: "reports/lint.sarif".to_owned(),
                },
                ReportRequest {
                    format: "sarif".to_owned(),
                    destination: "/tmp/extra.sarif".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn bazel_options_forward_verbatim_after_separator() {
        let got = parse(&args(&["lint", "--", "--jobs=4", "--", "nokeep_going"])).expect("parse");
        assert_eq!(got.bazel_options, args(&["--jobs=4", "--", "nokeep_going"]));
    }

    #[test]
    fn missing_command_fails() {
        assert_eq!(parse(&args(&[])), Err(ArgsError::MissingCommand));
        assert_eq!(parse(&args(&["--quiet"])), Err(ArgsError::MissingCommand));
    }

    #[test]
    fn unknown_command_fails() {
        assert_eq!(
            parse(&args(&["bogus"])),
            Err(ArgsError::UnknownCommand {
                command: "bogus".to_owned(),
                suggestion: None,
            })
        );
    }

    #[test]
    fn explicit_label_scope_parses_in_order() {
        let got = parse(&args(&["lint", "//a:one", "@repo//b/...", "//c/..."])).expect("parse");
        assert_eq!(got.targets, args(&["//a:one", "@repo//b/...", "//c/..."]));
    }

    #[test]
    fn path_scopes_parse_for_resolution() {
        let got = parse(&args(&[
            "lint",
            "src/main.rs",
            "quality/testdata/",
            "./x.py",
        ]))
        .expect("parse");
        assert_eq!(
            got.targets,
            args(&["src/main.rs", "quality/testdata/", "./x.py"])
        );
    }

    #[test]
    fn relative_and_empty_scope_fail() {
        assert_eq!(
            parse(&args(&["lint", ":corpus"])),
            Err(ArgsError::RelativeLabel {
                scope: ":corpus".to_owned(),
            })
        );
        assert_eq!(parse(&args(&["lint", ""])), Err(ArgsError::EmptyScope));
    }

    #[test]
    fn workflow_commands_parse_scopes_and_options() {
        for command in ["build", "test", "coverage"] {
            let got =
                parse(&args(&[command, "//a:one", "pkg/a.py", "--", "--jobs=4"])).expect("parse");
            assert_eq!(got.command.name(), command);
            assert_eq!(
                got.targets,
                args(&["//a:one", "pkg/a.py"]),
                "scopes parse for resolution"
            );
            assert_eq!(got.bazel_options, args(&["--jobs=4"]));
        }
    }

    #[test]
    fn workflow_commands_reject_quality_only_options() {
        assert_eq!(
            parse(&args(&["build", "--check"])),
            Err(ArgsError::UnsupportedOption {
                command: "build",
                option: "--check".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["test", "--fail-on=error"])),
            Err(ArgsError::UnsupportedOption {
                command: "test",
                option: "--fail-on".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["coverage", "--check", "--fail-on=info"])),
            Err(ArgsError::UnsupportedOption {
                command: "coverage",
                option: "--check".to_owned(),
            }),
            "check is reported before fail-on"
        );
        // Quality commands keep both options.
        assert!(parse(&args(&["lint", "--check", "--fail-on=error"])).is_ok());
    }

    #[test]
    fn unknown_options_fail() {
        assert_eq!(
            parse(&args(&["lint", "--jobs=4"])),
            Err(ArgsError::UnknownOption {
                option: "--jobs=4".to_owned(),
                suggestion: None,
            })
        );
        assert_eq!(
            parse(&args(&["lint", "-q"])),
            Err(ArgsError::UnknownOption {
                option: "-q".to_owned(),
                suggestion: None,
            })
        );
        assert_eq!(
            parse(&args(&["lint", "--dry-run=yes"])),
            Err(ArgsError::UnknownOption {
                option: "--dry-run=yes".to_owned(),
                suggestion: None,
            })
        );
        assert_eq!(
            parse(&args(&["-"])),
            Err(ArgsError::UnknownOption {
                option: "-".to_owned(),
                suggestion: None,
            })
        );
    }

    #[test]
    fn missing_values_fail() {
        assert_eq!(
            parse(&args(&["lint", "--output"])),
            Err(ArgsError::MissingValue {
                option: "--output".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["lint", "--workspace", "--quiet", "format"])),
            Err(ArgsError::MissingValue {
                option: "--workspace".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["lint", "--workspace="])),
            Err(ArgsError::MissingValue {
                option: "--workspace".to_owned(),
            })
        );
    }

    #[test]
    fn bad_values_fail() {
        assert_eq!(
            parse(&args(&["lint", "--output=yaml"])),
            Err(ArgsError::BadOutput {
                value: "yaml".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["lint", "--fail-on=never"])),
            Err(ArgsError::BadFailOn {
                value: "never".to_owned(),
            })
        );
        for bad in ["sarif", "=out.sarif", "sarif=", ""] {
            assert_eq!(
                parse(&args(&["lint", &format!("--report={bad}")])),
                Err(ArgsError::BadReport {
                    value: bad.to_owned(),
                })
            );
        }
    }

    #[test]
    fn inline_flag_values_and_empty_workspace_fail() {
        assert_eq!(
            parse(&args(&["lint", "--workspace", ""])),
            Err(ArgsError::MissingValue {
                option: "--workspace".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["lint", "--quiet=x"])),
            Err(ArgsError::UnknownOption {
                option: "--quiet=x".to_owned(),
                suggestion: None,
            })
        );
        assert_eq!(
            parse(&args(&["lint", "--check=x"])),
            Err(ArgsError::UnknownOption {
                option: "--check=x".to_owned(),
                suggestion: None,
            })
        );
    }

    #[test]
    fn clean_parses_dry_run_and_bazel() {
        let got = parse(&args(&["clean"])).expect("parse");
        assert_eq!(got.command, Command::Clean);
        assert!(!got.bazel_clean);
        assert!(!got.dry_run);
        let got = parse(&args(&["clean", "--dry-run", "--bazel"])).expect("parse");
        assert_eq!(got.command, Command::Clean);
        assert!(got.dry_run);
        assert!(got.bazel_clean);
        assert_eq!(Command::Clean.name(), "clean");
        assert!(!Command::Clean.is_workflow());
        assert!(!Command::Clean.is_umbrella());
    }

    #[test]
    fn clean_rejects_scopes_and_quality_options() {
        assert_eq!(
            parse(&args(&["clean", "--check"])),
            Err(ArgsError::UnsupportedOption {
                command: "clean",
                option: "--check".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["clean", "--fail-on=error"])),
            Err(ArgsError::UnsupportedOption {
                command: "clean",
                option: "--fail-on".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["clean", "--output=json"])),
            Err(ArgsError::UnsupportedOption {
                command: "clean",
                option: "--output=json".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["clean", "--report=sarif=out.sarif"])),
            Err(ArgsError::UnsupportedOption {
                command: "clean",
                option: "--report=sarif=out.sarif".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["clean", "//a:one"])),
            Err(ArgsError::UnsupportedOption {
                command: "clean",
                option: "//a:one".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["clean", "--", "--jobs=4"])),
            Err(ArgsError::UnsupportedOption {
                command: "clean",
                option: "--".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["clean", "--bazel=yes"])),
            Err(ArgsError::UnknownOption {
                option: "--bazel=yes".to_owned(),
                suggestion: None,
            })
        );
        // `--bazel` belongs to clean only.
        assert_eq!(
            parse(&args(&["lint", "--bazel"])),
            Err(ArgsError::UnsupportedOption {
                command: "lint",
                option: "--bazel".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["build", "--bazel"])),
            Err(ArgsError::UnsupportedOption {
                command: "build",
                option: "--bazel".to_owned(),
            })
        );
    }

    #[test]
    fn audit_update_parse_and_reject_unsupported_options() {
        let audit = parse(&args(&["audit"])).expect("parse audit");
        assert_eq!(audit.command, Command::Audit);
        assert_eq!(audit.command.name(), "audit");
        assert!(audit.command.is_audit_update());
        assert!(!audit.command.is_workflow());
        assert!(!audit.command.is_adoption());
        assert!(!audit.command.is_managed());
        assert!(audit.targets.is_empty());
        let families = parse(&args(&["audit", "security"])).expect("parse audit family");
        assert_eq!(families.targets, vec!["security".to_owned()]);
        let update = parse(&args(&["update"])).expect("parse update");
        assert_eq!(update.command, Command::Update);
        assert_eq!(update.command.name(), "update");
        assert!(update.command.is_audit_update());
        let selected = parse(&args(&["update", "crates"])).expect("parse update selector");
        assert_eq!(selected.targets, vec!["crates".to_owned()]);
        assert_eq!(
            parse(&args(&["audit", "--check"])),
            Err(ArgsError::UnsupportedOption {
                command: "audit",
                option: "--check".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["update", "--check"])),
            Err(ArgsError::UnsupportedOption {
                command: "update",
                option: "--check".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["update", "--fail-on=error"])),
            Err(ArgsError::UnsupportedOption {
                command: "update",
                option: "--fail-on".to_owned(),
            })
        );
        // `update` supports `--output=json` (issue #200): dry-run planning
        // and the deferred-live error stream NDJSON like `audit`.
        let got = parse(&args(&["update", "--output=json"])).expect("update json");
        assert_eq!(got.command, Command::Update);
        assert_eq!(got.output, OutputMode::Json);
        assert_eq!(
            parse(&args(&["update", "--output=diff"])),
            Err(ArgsError::UnsupportedOption {
                command: "update",
                option: "--output=diff".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["update", "--report=sarif=out.sarif"])),
            Err(ArgsError::UnsupportedOption {
                command: "update",
                option: "--report=sarif=out.sarif".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["update", "--", "--jobs=4"])),
            Err(ArgsError::UnsupportedOption {
                command: "update",
                option: "--".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["audit", "--", "--jobs=4"])),
            Err(ArgsError::UnsupportedOption {
                command: "audit",
                option: "--".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["audit", ":target"])),
            Err(ArgsError::RelativeLabel {
                scope: ":target".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["update", ":target"])),
            Err(ArgsError::RelativeLabel {
                scope: ":target".to_owned(),
            })
        );
    }

    #[test]
    fn run_rejects_machine_output_and_reports() {
        assert_eq!(
            parse(&args(&["run", "//app:bin", "--output=json"])),
            Err(ArgsError::UnsupportedOption {
                command: "run",
                option: "--output=json".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["run", "//app:bin", "--output=diff"])),
            Err(ArgsError::UnsupportedOption {
                command: "run",
                option: "--output=diff".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["run", "--report=junit=out.xml"])),
            Err(ArgsError::UnsupportedOption {
                command: "run",
                option: "--report=junit=out.xml".to_owned(),
            })
        );
        let got = parse(&args(&["run", "//app:bin", "--", "--port=8080"])).expect("parse run");
        assert_eq!(got.command, Command::Run);
        assert_eq!(got.targets, vec!["//app:bin".to_owned()]);
        assert_eq!(got.bazel_options, vec!["--port=8080".to_owned()]);
    }

    #[test]
    fn output_contract_has_no_silent_ignore() {
        // Issue #200: every command either supports a machine-output mode or
        // rejects it pre-exec with `UnsupportedOption`. Silent ignore (accept
        // the flag, print text anyway) is never allowed.
        //
        // JSON-capable: quality, generate, workflow build/test/coverage,
        // umbrellas, audit, update, status.
        for command in [
            "lint",
            "typecheck",
            "format",
            "generate",
            "build",
            "test",
            "coverage",
            "check",
            "fix",
            "audit",
            "update",
        ] {
            let got = parse(&args(&[command, "--output=json"])).expect("json capable");
            assert_eq!(got.output, OutputMode::Json, "command: {command}");
            assert!(got.command.supports_json(), "command: {command}");
        }
        let got = parse(&args(&["status", "--output=json"])).expect("status json");
        assert_eq!(got.output, OutputMode::Json);
        assert!(Command::Status.supports_json());
        // Diff-capable: patch producers only.
        for command in ["lint", "typecheck", "format", "generate", "check", "fix"] {
            let got = parse(&args(&[command, "--output=diff"])).expect("diff capable");
            assert_eq!(got.output, OutputMode::Diff, "command: {command}");
            assert!(got.command.supports_diff(), "command: {command}");
        }
        // Text-only exemptions fail fast on both machine modes.
        // (`bazel` takes dx flags before the command word; tokens after it
        // forward verbatim to the launcher.)
        for words in [
            vec!["clean", "--output=json"],
            vec!["clean", "--output=diff"],
            vec!["codegen", "--output=json"],
            vec!["env", "--output=diff"],
            vec!["setup", "--output=json"],
            vec!["--output=json", "bazel", "version"],
            vec!["init", "--output=json"],
            vec!["init", "proj", "--output=diff"],
            vec!["hooks", "status", "--output=json"],
            vec!["version", "--output=json"],
            vec!["watch", "test", "--output=json"],
            vec!["owners", "//a:one", "--output=json"],
            vec!["deps", "//a:one", "--output=diff"],
            vec!["why", "src/main.rs", "//a:one", "--output=json"],
            vec!["completion", "bash", "--output=json"],
        ] {
            assert!(
                matches!(
                    parse(&args(&words)),
                    Err(ArgsError::UnsupportedOption { .. })
                ),
                "words: {words:?}"
            );
        }
        // No patch to emit: JSON-capable but diff-rejecting commands fail
        // fast on `--output=diff` instead of printing empty stdout.
        for words in [
            vec!["build", "//a:one", "--output=diff"],
            vec!["test", "//a:one", "--output=diff"],
            vec!["coverage", "--output=diff"],
            vec!["audit", "--output=diff"],
            vec!["update", "--output=diff"],
            vec!["status", "--output=diff"],
        ] {
            assert_eq!(
                parse(&args(&words)),
                Err(ArgsError::UnsupportedOption {
                    command: words[0],
                    option: "--output=diff".to_owned(),
                }),
                "words: {words:?}"
            );
        }
    }

    #[test]
    fn bazel_forwards_verbatim_and_rejects_dx_options() {
        let got = parse(&args(&["bazel", "build", "//...", "--", "--jobs=4"])).expect("parse");
        assert_eq!(got.command, Command::Bazel);
        assert_eq!(got.command.name(), "bazel");
        assert!(!got.command.is_workflow());
        assert!(!got.command.is_adoption());
        assert!(got.targets.is_empty());
        assert_eq!(
            got.bazel_options,
            vec![
                "build".to_owned(),
                "//...".to_owned(),
                "--jobs=4".to_owned()
            ]
        );
        // Tokens after the command word forward verbatim even when
        // they look like dx options: dx globals must precede `bazel`.
        for words in [
            vec!["bazel", "--jobs=4"],
            vec!["bazel", "--check"],
            vec!["bazel", "--pin=0.1.0"],
            vec!["bazel", "version", "--configured"],
            vec!["bazel", "--", "--fail-on=error"],
        ] {
            let got = parse(&args(&words)).expect("verbatim");
            assert_eq!(got.command, Command::Bazel, "words: {words:?}");
            assert!(got.targets.is_empty(), "words: {words:?}");
        }
        let got = parse(&args(&["bazel", "build", "--jobs", "4"])).expect("verbatim");
        assert_eq!(
            got.bazel_options,
            vec!["build".to_owned(), "--jobs".to_owned(), "4".to_owned()]
        );
        // dx-owned options before the command word still fail fast so
        // launcher flags can never be misread as dx flags.
        for words in [
            vec!["--output=json", "bazel", "version"],
            vec!["--check", "bazel", "version"],
            vec!["--pin=0.1.0", "bazel", "version"],
            vec!["--configured", "bazel", "version"],
            vec!["--fail-on=error", "bazel", "build", "//..."],
            vec!["--report=sarif=x.sarif", "bazel", "build"],
        ] {
            assert!(
                matches!(
                    parse(&args(&words)),
                    Err(ArgsError::UnsupportedOption { .. })
                ),
                "words: {words:?}"
            );
        }
    }

    #[test]
    fn version_rollback_check_and_configured_parse() {
        let got = parse(&args(&["version", "--rollback"])).expect("parse");
        assert_eq!(got.command, Command::Version);
        assert!(got.rollback);
        let got = parse(&args(&["version", "--check"])).expect("parse");
        assert!(got.check);
        let got = parse(&args(&["deps", "--configured", "//a:one"])).expect("parse");
        assert_eq!(got.command, Command::Deps);
        assert!(got.configured);
        for words in [
            vec!["status", "--rollback"],
            vec!["status", "--check"],
            vec!["owners", "//a:one", "--pin=0.1.0"],
            vec!["build", "//a:one", "--configured"],
            vec!["lint", "--rollback"],
            vec!["check", "//...", "--configured"],
            vec!["status", "--pin=0.1.0"],
        ] {
            assert!(
                matches!(
                    parse(&args(&words)),
                    Err(ArgsError::UnsupportedOption { .. })
                ),
                "words: {words:?}"
            );
        }
    }

    #[test]
    fn managed_commands_parse_repo_and_exact_scopes() {
        for command in ["codegen", "env", "setup"] {
            let got = parse(&args(&[command])).expect("parse");
            assert!(got.command.is_managed());
            assert!(got.targets.is_empty());
            assert!(got.bazel_options.is_empty());
            let got = parse(&args(&[command, "//a:one", "--", "--jobs=4"])).expect("scoped parse");
            assert_eq!(got.targets, args(&["//a:one"]));
            assert_eq!(got.bazel_options, args(&["--jobs=4"]));
            let got = parse(&args(&[command, "@repo//pkg:lib"])).expect("external label");
            assert_eq!(got.targets, args(&["@repo//pkg:lib"]));
        }
        // Scope rules are the shared setup scope rules: multiple
        // positionals, patterns, and non-labels fail before execution.
        assert_eq!(
            parse(&args(&["codegen", "//a:one", "//b:two"])),
            Err(ArgsError::UnsupportedOption {
                command: "codegen",
                option: "//b:two".to_owned(),
            })
        );
        for words in [
            vec!["env", "//a/..."],
            vec!["setup", "src/main.rs"],
            vec!["codegen", ":target"],
            vec!["env", ""],
            vec!["setup", "--config=release"],
        ] {
            assert!(
                matches!(
                    parse(&args(&words)),
                    Err(ArgsError::EmptyScope)
                        | Err(ArgsError::RelativeLabel { .. })
                        | Err(ArgsError::InvalidScope { .. })
                        | Err(ArgsError::UnknownOption { .. })
                ),
                "words: {words:?}"
            );
        }
        // Quality-only, version-only, and clean-only options
        // fail fast on managed commands.
        for words in [
            vec!["codegen", "--check"],
            vec!["env", "--fail-on=error"],
            vec!["setup", "--output=json"],
            vec!["codegen", "--report=sarif=out.sarif"],
            vec!["env", "--pin=0.1.0"],
            vec!["setup", "--rollback"],
            vec!["codegen", "--configured"],
            vec!["env", "--configured"],
            vec!["setup", "--pin=0.2.0"],
            vec!["codegen", "--bazel"],
        ] {
            assert!(
                matches!(
                    parse(&args(&words)),
                    Err(ArgsError::UnsupportedOption { .. })
                ),
                "words: {words:?}"
            );
        }
    }

    #[test]
    fn why_requires_file_and_label() {
        let got = parse(&args(&["why", "src/a.rs", "//a:one"])).expect("parse");
        assert_eq!(got.command, Command::Why);
        assert_eq!(
            got.targets,
            vec!["src/a.rs".to_owned(), "//a:one".to_owned()]
        );
        for words in [
            vec!["why", "src/a.rs"],
            vec!["why", "src/a.rs", "//a:one", "//b:two"],
        ] {
            assert_eq!(
                parse(&args(&words)),
                Err(ArgsError::MissingValue {
                    option: "<file> <label>".to_owned(),
                }),
                "words: {words:?}"
            );
        }
    }
}
