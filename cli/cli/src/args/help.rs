//! Help rendering for the `dx` CLI.
//!
//! Split from `super` (`args.rs`): owns [`help_command_in`],
//! [`render_top_help`], [`per_command_flags`], and
//! [`render_command_help`]. Internal only (`pub(crate)`); the public
//! `crate::args` surface is unchanged.

use super::command::Command;
use super::grammar::{Cli, VALUE_OPTIONS};

/// Finds the command word for `--help` routing: the first positional
/// token that parses as [`Command`], skipping flag payloads exactly
/// like [`super::split_bazel_verbatim`]. Stops at `--` (everything after is
/// Bazel-owned). Returns `None` for top-level help when no command
/// word is present or the first positional is not a command.
///
/// Stays hand-rolled with [`super::split_bazel_verbatim`] (fallback):
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
    // Brand line: `render_long_help` below shows `long_about`
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
/// reconciliation: `--bazel` (clean only, forwards
/// `bazel clean`) and `--configured` (owners/deps/why only, selects
/// `cquery`) keep their names because they mean different things, and
/// both stay distinct from the `dx bazel` passthrough command. The
/// per-command usage + flags lines below name that distinction so the
/// collision is documented, not hidden.
pub(crate) fn per_command_flags(command: Command) -> &'static str {
    match command {
        Command::Check => {
            "Per-command flags: --check/--fail-on/--report pass through per phase (check only; non-mutating umbrella over format+lint+typecheck+generate, stop-on-first-failure)."
        }
        Command::Fix => {
            "Per-command flags: --check/--fail-on/--report pass through per phase (fix only; mutating by default with no rerun, run `dx check` to validate)."
        }
        Command::Clean => {
            "Per-command flags: --bazel (also run `bazel clean` after pruning; default never touches Bazel outputs; distinct from `dx bazel`, which forwards raw args)."
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
        Command::Migrate => {
            "Per-command flags: --from <version> --to <version> (migrate only; both Cargo semver, upgrade-only gate)."
        }
        Command::Audit => {
            "Per-command flags: --fail-on info|warning|error, --report sarif|spdx (audit only; --check and `-- --bazel-options` do not apply; --output diff has no patch)."
        }
        Command::Update => {
            "Per-command flags: --check (preset stale gate; selectors ignored) (update only; --fail-on/--report and `-- --bazel-options` do not apply; --output diff has no patch)."
        }
        Command::Bump => {
            "Per-command flags: none (exactly one `set:package` plus version; --check/--fail-on/--report and `-- --bazel-options` do not apply; --output diff has no patch)."
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
            "Usage: dx [global-options] build|test [--here] [--debug|--release] [scope ...] [-- bazel-options ...]"
        }
        Command::Coverage => {
            "Usage: dx [global-options] coverage [--here] [--min-coverage 0-100] [scope ...] [-- bazel-options ...]"
        }
        Command::Owners => "Usage: dx [global-options] owners [--configured] <scope> ...",
        Command::Deps => "Usage: dx [global-options] deps [--configured] <scope> ...",
        Command::Why => "Usage: dx [global-options] why [--configured] <file> <label>",
        Command::Version => {
            "Usage: dx [global-options] version [--check] [--pin <version>|--rollback]"
        }
        Command::Update => "Usage: dx [global-options] update [selector ...]",
        Command::Bump => "Usage: dx [global-options] bump <set:package> <version>",
        Command::Audit => "Usage: dx [global-options] audit [--here] [security|license] [scope ...]",
        Command::Migrate => {
            "Usage: dx [global-options] migrate --from <version> --to <version> [scope ...]"
        }
        Command::Lint | Command::Typecheck | Command::Format | Command::Generate => {
            "Usage: dx [global-options] <command> [--here] [scope ...] [-- bazel-options ...]"
        }
        Command::Check | Command::Fix => {
            "Usage: dx [global-options] check|fix [--here] [scope ...] [-- bazel-options ...]"
        }
        _ => "Usage: dx [global-options] <command> [scope ...] [-- bazel-options ...]",
    };
    let scopes = match command {
        Command::Clean => "Scopes: none (clean takes no scopes).",
        Command::Bazel => "Scopes: none (raw Bazel forwarding; no dx scope resolution).",
        Command::Deploy => "Scopes: exactly one main-workspace label (//pkg:target); patterns (//...), multiple labels, and file/path scopes are usage failures.",
        Command::Run => "Scopes: explicit Bazel labels/patterns (//..., //pkg:target, @repo//...), or workspace-relative files/dirs resolved via Bazel query. Requires a scope (empty scope is a usage error); file/dir scopes need exactly one runnable.",
        Command::Update => "Scopes: dependency-set/package/target selectors (cargo|npm|maven|nuget|go, set:package, labels/paths); bare run updates all sets.",
        Command::Bump => "Scopes: exactly one `set:package` plus one new version (bazel|cargo|github-actions|go|maven|npm|nuget); never batch.",
        Command::Audit => "Scopes: optional `security|license` family plus dependency-set/package/target selectors; bare run audits //... (both families, security first). Pass --here (--cwd alias) for the current directory tree instead (optionally after the family); --here cannot be combined with explicit scopes and never changes the no-flag default.",
        Command::Migrate => "Scopes: explicit Bazel labels/patterns or workspace-relative files/dirs reusing generation scope resolution; external scopes rejected. No scope selects //....",
        Command::Lint
        | Command::Typecheck
        | Command::Format
        | Command::Generate
        | Command::Build
        | Command::Test
        | Command::Coverage
        | Command::Check
        | Command::Fix => "Scopes: explicit Bazel labels/patterns (//..., //pkg:target, @repo//...), or workspace-relative files/dirs resolved via Bazel query. Graph-scope commands select //... when no scope is supplied. Pass --here (--cwd alias) for the current directory tree instead (//path/...; //... at the root); --here cannot be combined with explicit scopes and never changes the no-flag default.",
        _ => "Scopes: explicit Bazel labels/patterns (//..., //pkg:target, @repo//...), or workspace-relative files/dirs resolved via Bazel query. Graph-scope commands select //... when no scope is supplied; other commands follow per-command defaults (see docs/cli/commands/README.md#scope-defaults).",
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

#[cfg(test)]
mod tests {
    use super::super::{parse, ArgsError, Command};
    use super::render_command_help;

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn top_level_help_lists_commands_and_flags() {
        for flag in ["--help", "-h"] {
            let text = match parse(&args(&[flag])) {
                Err(ArgsError::Help { text }) => text,
                other => panic!("{flag}: want Help, got {other:?}"),
            };
            for needle in [
                "dx",
                "lint",
                "build",
                "--workspace",
                "--output",
                "--fail-on",
                "Exit codes",
            ] {
                assert!(text.contains(needle), "{flag}: missing {needle:?}");
            }
        }
    }

    #[test]
    fn help_process_exits_zero_with_brand() {
        let root = std::env::var("TEST_SRCDIR").expect("TEST_SRCDIR is set under Bazel");
        let workspace = std::env::var("TEST_WORKSPACE").expect("TEST_WORKSPACE is set under Bazel");
        let binary = std::path::Path::new(&root)
            .join(workspace)
            .join("cli/cli/dx");
        assert_cmd::Command::new(binary)
            .arg("--help")
            .assert()
            .success()
            .stdout(predicates::str::contains("Transparent UI over Bazel"));
    }

    #[test]
    fn per_command_help_covers_usage_scopes_exits_and_output() {
        for argv in [
            vec!["lint", "--help"],
            vec!["--help", "lint"],
            vec!["clean", "-h"],
        ] {
            let text = match parse(&args(&argv)) {
                Err(ArgsError::Help { text }) => text,
                other => panic!("{argv:?}: want Help, got {other:?}"),
            };
            let command = argv
                .iter()
                .find(|word| Command::parse(word).is_some())
                .expect("command");
            for needle in [
                command,
                "Usage:",
                "Scopes:",
                "Exit codes:",
                "Output:",
                "--workspace",
            ] {
                assert!(text.contains(needle), "{argv:?}: missing {needle:?}");
            }
        }
    }

    #[test]
    fn per_command_help_names_owned_flags_and_reconciles_bazel_naming() {
        let clean = match parse(&args(&["clean", "--help"])) {
            Err(ArgsError::Help { text }) => text,
            other => panic!("clean --help: want Help, got {other:?}"),
        };
        assert!(
            clean.contains("clean [--dry-run] [--bazel]"),
            "clean usage:\n{clean}"
        );
        assert!(clean.contains("--bazel"), "clean flags:\n{clean}");
        assert!(
            clean.contains("distinct from `dx bazel`"),
            "clean disambiguation:\n{clean}"
        );
        assert!(
            clean.contains("never touches Bazel outputs") || clean.contains("never Bazel outputs"),
            "clean surprise:\n{clean}"
        );
        let fix_help = match parse(&args(&["fix", "--help"])) {
            Err(ArgsError::Help { text }) => text,
            other => panic!("fix --help: want Help, got {other:?}"),
        };
        assert!(fix_help.contains("no rerun"), "fix surprise:\n{fix_help}");
        assert!(
            fix_help.contains("dx check"),
            "fix rerun guidance:\n{fix_help}"
        );
        let check_help = match parse(&args(&["check", "--help"])) {
            Err(ArgsError::Help { text }) => text,
            other => panic!("check --help: want Help, got {other:?}"),
        };
        assert!(
            check_help.contains("non-mutating"),
            "check mode:\n{check_help}"
        );
        for command in ["owners", "deps", "why"] {
            let text = match parse(&args(&[command, "--help"])) {
                Err(ArgsError::Help { text }) => text,
                other => panic!("{command} --help: want Help, got {other:?}"),
            };
            assert!(text.contains("[--configured]"), "{command} usage:\n{text}");
            assert!(
                text.contains("distinct from `dx clean --bazel`"),
                "{command} disambiguation:\n{text}"
            );
        }
        let coverage = match parse(&args(&["coverage", "--help"])) {
            Err(ArgsError::Help { text }) => text,
            other => panic!("coverage --help: want Help, got {other:?}"),
        };
        assert!(
            coverage.contains("--min-coverage"),
            "coverage flags:\n{coverage}"
        );
        let build = match parse(&args(&["build", "--help"])) {
            Err(ArgsError::Help { text }) => text,
            other => panic!("build --help: want Help, got {other:?}"),
        };
        assert!(build.contains("--debug"), "build flags:\n{build}");
        let version = match parse(&args(&["version", "--help"])) {
            Err(ArgsError::Help { text }) => text,
            other => panic!("version --help: want Help, got {other:?}"),
        };
        assert!(version.contains("--pin"), "version flags:\n{version}");
        for argv in [["lint", "--help"], ["status", "--help"]] {
            let text = match parse(&args(&argv)) {
                Err(ArgsError::Help { text }) => text,
                other => panic!("{argv:?}: want Help, got {other:?}"),
            };
            assert!(text.contains("Per-command flags:"), "{argv:?}:\n{text}");
        }
        let bazel_help = render_command_help(Command::Bazel);
        assert!(
            bazel_help.contains("Per-command flags:"),
            "bazel help:\n{bazel_help}"
        );
    }

    #[test]
    fn help_value_option_payload_is_not_a_command() {
        let text = match parse(&args(&["--output", "bazel", "--help"])) {
            Err(ArgsError::Help { text }) => text,
            other => panic!("want Help, got {other:?}"),
        };
        assert!(text.contains("--output"));
    }
}
