//! Invocation parsing for the `dx` CLI.
//!
//! Split from `super` (`args.rs`): owns the full `parse` validation
//! (scope shapes, per-command option ownership, output-contract gates,
//! profile flags). The small value helpers (`scope_error`,
//! `parse_report`, `parse_min_coverage`) live in the [`super::values`]
//! sibling. The Bazel-verbatim tokenizer
//! and `clap`-error mapping live in the [`super::tokenizer`] sibling.
//! The `clap` grammar (`Cli`, `cli_command`) lives in
//! the [`super::grammar`] sibling; this module re-exports it so the
//! `crate::args::parser::{Cli, cli_command}` paths stay
//! stable (`VALUE_OPTIONS` stays in `grammar` and is imported directly
//! by its users).
//! Re-exported through `super` so the public paths stay
//! `crate::args::parse` and `crate::args::cli_command`.
//!
//! Named `parser` (not `parse`) so the module and the `parse` function
//! can coexist without a namespace collision; the domain is the `parse`
//! half of the `args→command/parse/suggest/help/completion` split.

use dx_output::{OutputMode, Threshold};

use super::command::Command;
use super::tokenizer::tokenize;
use super::values::{parse_min_coverage, parse_report, scope_error};
use super::{ArgsError, Invocation};

pub use super::grammar::cli_command;
pub(crate) use super::grammar::Cli;

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
        from,
        to,
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
    if from.as_deref().is_some_and(str::is_empty) {
        return Err(ArgsError::MissingValue {
            option: "--from".to_owned(),
        });
    }
    if to.as_deref().is_some_and(str::is_empty) {
        return Err(ArgsError::MissingValue {
            option: "--to".to_owned(),
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
        // Managed environment/codegen/setup commands run one
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
        // Audit plans through `dx_audit`: family selection
        // plus scope spellings, non-mutating, with live Gitleaks plus
        // advisory/vuln/SPDX backends, SARIF/SPDX reports and
        // `--fail-on` thresholds. `--check` is meaningless (audit never
        // mutates), Bazel forwards do not apply (no Bazel collection
        // build; live auditors run directly), and version/clean-only
        // flags do not apply.
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
        // Update plans through `dx_update`  plus the vendored
        // preset fragment: dependency-set / package/target
        // selectors with exact syntax in `dx_update::selector`, mutating
        // without confirmation. `dx update --check` is the preset stale
        // gate (non-mutating, exit 0 clean / 1 stale, copying the
        // `generate --check` exit contract); it ignores selectors and
        // checks only the fragment. Thresholds and standard reports do
        // not apply on this path; aggregate exit/report mappings follow
        // `dx_update::outcome`/`report` in default mode.
        if fail_on_name != "warning" {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--fail-on".to_owned(),
            });
        }
        // `update` supports `--output=json` (dry-run planning emits
        // `command_started`/`command_finished`; live execution adds
        // per-set `notice`/`error` events); `--output=diff` has no patch
        // to emit so it fails fast here, with the shared `supports_diff`
        // gate below as backup.
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
    if command == Command::Bump {
        // Bump plans through `dx_bump`: exactly one
        // `set:package` selector plus one new version
        // (`dx bump <selector> <version>`), mutating without confirmation.
        // Thresholds, standard reports, and check mode do not apply on
        // this path; never batch (one requirement per invocation).
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
        // `bump` supports `--output=json` like `update` (dry-run planning
        // emits `command_started`/`command_finished`; live execution adds
        // the widen `notice`/`error`); `--output=diff` has no patch to
        // emit so it fails fast here, with the shared `supports_diff`
        // gate below as backup.
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
        if targets.len() != 2 {
            return Err(ArgsError::MissingValue {
                option: "<selector> <version>".to_owned(),
            });
        }
        for scope in &targets {
            if scope.is_empty() || scope.starts_with(':') {
                return Err(scope_error(scope));
            }
        }
    }
    if command == Command::Migrate {
        // Migrate plans through `dx_adopt::plan_migrate`:
        // `dx migrate --from <version> --to <version> [scope ...]`,
        // both Cargo-flavor semver with a major-release-only gate plus
        // one manifest per major hop. Scope selection reuses generation
        // scope resolution verbatim; external scopes are rejected like
        // workflow commands during execution. Thresholds, standard
        // reports, and check mode do not apply on this path; live
        // execution fails closed until the first major-release manifest
        // lands (module at `0.0.0`, no releases cut).
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
        // `migrate` supports `--output=json` like `update`/`bump`
        // (dry-run planning emits `command_started`/`command_finished`;
        // live execution fails closed with `migrate_failed`);
        // `--output=diff` has no patch to emit so it fails fast here,
        // with the shared `supports_diff` gate below as backup.
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
        if from.is_none() || to.is_none() {
            return Err(ArgsError::MissingValue {
                option: "--from <version> --to <version>".to_owned(),
            });
        }
        for scope in &targets {
            if scope.is_empty() || scope.starts_with(':') {
                return Err(scope_error(scope));
            }
        }
    }
    // `--from`/`--to` belong to `migrate` only: every other command
    // fails fast instead of silently ignoring the versions.
    if command != Command::Migrate && (from.is_some() || to.is_some()) {
        return Err(ArgsError::UnsupportedOption {
            command: command.name(),
            option: if from.is_some() {
                "--from".to_owned()
            } else {
                "--to".to_owned()
            },
        });
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
        //: `dx run` is a local-only single-target launcher with prose
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
    // Uniform `--output` contract: shared `supports_json` /
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
    // Profile flags: `--debug`/`--release` are mutually
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
    // Flag ownership: `--rollback` belongs to `version` only
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
        from,
        to,
    })
}

#[cfg(test)]
#[path = "parser_tests_a.rs"]
mod parser_tests_a;
#[cfg(test)]
#[path = "parser_tests_b.rs"]
mod parser_tests_b;
