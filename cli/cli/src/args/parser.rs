//! Invocation parsing for the `dx` CLI (issue #236).
//!
//! Split from `super` (`args.rs`): owns the `clap` grammar (`Cli`),
//! the Bazel-verbatim tokenizer, `clap`-error mapping, and the full
//! `parse` validation (scope shapes, per-command option ownership,
//! output-contract gates, profile flags). Re-exported through `super`
//! so the public paths stay `crate::args::parse` and
//! `crate::args::cli_command`.
//!
//! Named `parser` (not `parse`) so the module and the `parse` function
//! can coexist without a namespace collision; the domain is the `parse`
//! half of the `args→command/parse/suggest/help/completion` split.

use clap::Parser;
use dx_output::{OutputMode, Threshold};

use super::command::Command;
use super::{help, suggest};
use super::{ArgsError, Invocation, ReportRequest};

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
    workspace: Option<String>,
    /// Resolve and summarize the plan without executing workflows/mutations.
    #[arg(long)]
    dry_run: bool,
    /// Suppress dx operation summaries (tool diagnostics still print).
    #[arg(long)]
    quiet: bool,
    /// Enable structured diagnostics on stderr via tracing (issue #222).
    /// Default stays byte-identical; `--verbose` adds info-level logs.
    /// Orthogonal to `--quiet` (summaries vs logs).
    #[arg(long)]
    verbose: bool,
    /// Select concise text, unified patches, or versioned NDJSON events.
    #[arg(long, allow_negative_numbers = true, overrides_with = "output")]
    output: Option<String>,
    /// Write a standard report (sarif/junit/lcov) to file or -; repeatable.
    #[arg(long, allow_negative_numbers = true)]
    report: Vec<String>,
    /// Lowest diagnostic severity that fails quality commands.
    #[arg(long, allow_negative_numbers = true, overrides_with = "fail_on")]
    fail_on: Option<String>,
    /// Required line-coverage percent (coverage only, 0-100).
    #[arg(long, allow_negative_numbers = true, overrides_with = "min_coverage")]
    min_coverage: Option<String>,
    /// Check mode (quality/version only; no mutations).
    #[arg(long)]
    check: bool,
    /// Use dx_debug config (build/run/test/deploy only; conflicts with --release).
    #[arg(long)]
    debug: bool,
    /// Use dx_release config (build/run/test/deploy only; conflicts with --debug).
    #[arg(long)]
    release: bool,
    /// Additionally forward `bazel clean` after pruning (clean only).
    #[arg(long = "bazel")]
    bazel_clean: bool,
    /// Re-pin to <version> (version only).
    #[arg(long, allow_negative_numbers = true, overrides_with = "pin")]
    pin: Option<String>,
    /// Re-pin the recorded previous release (version only).
    #[arg(long)]
    rollback: bool,
    /// Use cquery instead of query (owners/deps/why only).
    #[arg(long)]
    configured: bool,
    /// First positional: the command word (a [`Command`] value so the
    /// same grammar feeds parsing, `--help`, and shell completions).
    #[arg(value_enum)]
    command: Option<Command>,
    /// Later positionals: explicit scopes/targets.
    targets: Vec<String>,
    /// Everything after the first bare `--`, forwarded verbatim.
    #[arg(last = true)]
    bazel_options: Vec<String>,
}

/// Grammar accessor for build steps (issue #225): the `dx_man` binary
/// renders `man/dx.1` from this command so the manual page tracks the
/// same grammar as parsing, `--help`, and completions.
pub fn cli_command() -> clap::Command {
    use clap::CommandFactory;
    Cli::command()
}

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
