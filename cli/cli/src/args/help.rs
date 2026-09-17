//! Help rendering for the `dx` CLI (issue #236).
//!
//! Split from `super` (`args.rs`): owns [`help_command_in`],
//! [`render_top_help`], [`per_command_flags`], and
//! [`render_command_help`]. Internal only (`pub(crate)`); the public
//! `crate::args` surface is unchanged.

use super::command::Command;
use super::parser::{Cli, VALUE_OPTIONS};

/// Finds the command word for `--help` routing: the first positional
/// token that parses as [`Command`], skipping flag payloads exactly
/// like [`super::split_bazel_verbatim`]. Stops at `--` (everything after is
/// Bazel-owned). Returns `None` for top-level help when no command
/// word is present or the first positional is not a command.
///
/// Stays hand-rolled with [`super::split_bazel_verbatim`] (issue #233 fallback):
/// help routing inspects `argv` before the grammar runs, so it cannot
/// itself be a `value_parser`.
pub(crate) fn help_command_in(args: &[String]) -> Option<Command> {
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
                    _ => index += 1,
                }
                continue;
            }
            index += 1;
            continue;
        }
        return Command::parse(arg);
    }
    None
}

/// Renders top-level `--help` from the [`Cli`] grammar definition (one
/// source for parsing/help/completions; no hand-maintained flag list).
/// The command list is derived from [`Command`] (the same source as
/// parsing), because commands are a validated positional rather than
/// clap subcommands.
pub(crate) fn render_top_help() -> String {
    use clap::{CommandFactory, ValueEnum};
    let mut out = String::new();
    // Brand line (issue #225): `render_long_help` below shows `long_about`
    // but not `about`, so `--help` would otherwise omit the brand that `-h`
    // shows. Prepend it so both spellings carry the same identity.
    if let Some(about) = Cli::command().get_about() {
        out.push_str(&format!("dx - {about}\n\n"));
    }
    out.push_str("Commands:\n");
    for command in Command::value_variants() {
        out.push_str(&format!(
            "  {:<12} {}\n",
            command.name(),
            command.describe()
        ));
    }
    out.push('\n');
    out.push_str(&Cli::command().render_long_help().to_string());
    out
}

/// Renders per-command `--help`: one-line summary from
/// [`Command::describe`], usage/scopes/exit/output lines consistent
/// with `docs/cli/cli-contract.md`, then the full grammar help so the
/// flag list can never drift.
///
/// Issue #211 reconciliation: `--bazel` (clean only, forwards
/// `bazel clean`) and `--configured` (owners/deps/why only, selects
/// `cquery`) keep their names because they mean different things, and
/// both stay distinct from the `dx bazel` passthrough command. The
/// per-command usage + flags lines below name that distinction so the
/// collision is documented, not hidden.
pub(crate) fn per_command_flags(command: Command) -> &'static str {
    match command {
        Command::Clean => {
            "Per-command flags: --bazel (also run `bazel clean` after pruning; distinct from `dx bazel`, which forwards raw args)."
        }
        Command::Owners | Command::Deps | Command::Why => {
            "Per-command flags: --configured (use `bazel cquery` instead of `bazel query`; distinct from `dx clean --bazel`, which forwards `bazel clean`)."
        }
        Command::Coverage => {
            "Per-command flags: --min-coverage <0-100> (coverage only; collects without enforcing when absent)."
        }
        Command::Build | Command::Test | Command::Run | Command::Deploy => {
            "Per-command flags: --debug | --release (build/run/test/deploy only; mutually exclusive; bare invocation means dev, except deploy means release)."
        }
        Command::Version => {
            "Per-command flags: --check (drift check), --pin <version>, --rollback (version only; --pin and --rollback conflict)."
        }
        Command::Bazel => {
            "Per-command flags: none (raw Bazel forwarding; dx-owned options must precede the command word and most are rejected)."
        }
        _ => {
            "Per-command flags: --check/--fail-on/--report/--min-coverage/--debug/--release/--bazel/--pin/--rollback/--configured are owned per command; unsupported uses fail with `option \"--flag\" is not supported by dx <command>`."
        }
    }
}

pub(crate) fn render_command_help(command: Command) -> String {
    let usage = match command {
        Command::Clean => "Usage: dx [global-options] clean [--dry-run] [--bazel]",
        Command::Bazel => "Usage: dx [global-options] bazel [-- bazel-args ...]",
        Command::Run => "Usage: dx [global-options] run [--debug|--release] <target> [-- app-args ...]",
        Command::Deploy => "Usage: dx [global-options] deploy [--debug|--release] <label> [-- app-args ...]",
        Command::Build | Command::Test => {
            "Usage: dx [global-options] build|test [--debug|--release] [scope ...] [-- bazel-options ...]"
        }
        Command::Coverage => {
            "Usage: dx [global-options] coverage [--min-coverage 0-100] [scope ...] [-- bazel-options ...]"
        }
        Command::Owners => "Usage: dx [global-options] owners [--configured] <scope> ...",
        Command::Deps => "Usage: dx [global-options] deps [--configured] <scope> ...",
        Command::Why => "Usage: dx [global-options] why [--configured] <file> <label>",
        Command::Version => {
            "Usage: dx [global-options] version [--check] [--pin <version>|--rollback]"
        }
        _ => "Usage: dx [global-options] <command> [scope ...] [-- bazel-options ...]",
    };
    let scopes = match command {
        Command::Clean => "Scopes: none (clean takes no scopes).",
        Command::Bazel => "Scopes: none (raw Bazel forwarding; no dx scope resolution).",
        Command::Deploy => "Scopes: exactly one main-workspace label (//pkg:target); patterns (//...), multiple labels, and file/path scopes are usage failures.",
        Command::Run => "Scopes: explicit Bazel labels/patterns (//..., //pkg:target, @repo//...), or workspace-relative files/dirs resolved via Bazel query. No scope selects //....",
        _ => "Scopes: explicit Bazel labels/patterns (//..., //pkg:target, @repo//...), or workspace-relative files/dirs resolved via Bazel query. No scope selects //....",
    };
    let mut out = String::new();
    out.push_str(&format!(
        "dx {} - {}\n\n",
        command.name(),
        command.describe()
    ));
    out.push_str(usage);
    out.push('\n');
    out.push_str(per_command_flags(command));
    out.push_str("\n\n");
    out.push_str(scopes);
    out.push_str("\nExit codes: 0 success; 2 usage/scope/owner errors; 1 operational failures; Bazel-authoritative failures preserve Bazel's code.");
    out.push_str(
        "\nOutput: --output text|diff|json; --report <format>=<destination> (repeatable).",
    );
    out.push_str("\n\n");
    out.push_str(&render_top_help());
    out
}
