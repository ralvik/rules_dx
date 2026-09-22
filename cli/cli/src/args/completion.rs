//! Shell-completion rendering for the `dx` CLI.
//!
//! Split from `super` (`args.rs`): owns [`COMPLETION_SHELLS`] and
//! [`render_completion`]. Re-exported through `super` so the public path
//! stays `crate::args::{COMPLETION_SHELLS, render_completion}`.

use super::command::Command;
use super::complete::{
    completes_labels, AUDIT_FAMILIES, COMPLETE_SUBCOMMAND, DYNAMIC_MARKER, HOOK_TRIGGERS,
    HOOK_VERBS,
};
use super::grammar::Cli;
use super::ArgsError;

/// Shells covered by `dx completion` (contract freeze).
pub const COMPLETION_SHELLS: &[&str] = &["bash", "zsh", "fish", "powershell"];

/// Sorted space-joined rendering of one fixed candidate table for the
/// fish task lines below.
fn sorted_join(names: &[&str]) -> String {
    let mut sorted: Vec<&str> = names.to_vec();
    sorted.sort_unstable();
    sorted.join(" ")
}

/// Dependency-set hints shared by the fish `update`/`bump` task line,
/// drawn from the same [`dx_update::sets::SetId`] table as the
/// [`super::complete`] dispatch so new sets cannot drift.
fn update_set_names() -> Vec<&'static str> {
    dx_update::sets::SetId::ALL
        .iter()
        .map(|id| id.name())
        .collect()
}

/// Appends the bash dynamic section: the static `*)` fallback (empty
/// completions for scope positions) becomes a completion-time callback
/// into the binary, following the generator-subcommand convention
/// (See: `docs/cli/commands/completion.md`). Fails closed when the
/// generator template drifts beyond recognition.
fn bash_dynamic(text: &mut String) -> Result<(), ArgsError> {
    let anchor = "                *)\n                    COMPREPLY=()\n                    ;;";
    let dynamic = format!(
        "                *)\n                    #{DYNAMIC_MARKER}: completion-time callback into the binary (See: docs/cli/commands/completion.md).\n                    _dx_words=(\"${{COMP_WORDS[@]:1:COMP_CWORD}}\")\n                    if [[ ${{cur}} == \"\" ]]; then\n                        _dx_words+=(\"\")\n                    fi\n                    if command -v dx >/dev/null 2>&1; then\n                        COMPREPLY=( $(dx {COMPLETE_SUBCOMMAND} \"${{_dx_words[@]}}\" 2>/dev/null) )\n                    else\n                        COMPREPLY=()\n                    fi\n                    return 0\n                    ;;"
    );
    if text.contains(anchor) {
        *text = text.replacen(anchor, &dynamic, 1);
        Ok(())
    } else {
        Err(ArgsError::UnknownShell {
            shell: "bash".to_owned(),
        })
    }
}

/// Appends the zsh dynamic section: scope positions call the
/// `_dx_dynamic_targets` helper (completion-time callback into the
/// binary) instead of plain file completion (See:
/// `docs/cli/commands/completion.md`). Fails closed on template drift.
fn zsh_dynamic(text: &mut String) -> Result<(), ArgsError> {
    let anchor = "*::targets -- Later positionals\\: explicit scopes/targets:_default";
    let replacement =
        "*::targets -- Later positionals\\: explicit scopes/targets:_dx_dynamic_targets";
    if !text.contains(anchor) {
        return Err(ArgsError::UnknownShell {
            shell: "zsh".to_owned(),
        });
    }
    *text = text.replacen(anchor, replacement, 1);
    text.push_str(&format!(
        "\n#{DYNAMIC_MARKER}: completion-time callback into the binary (See: docs/cli/commands/completion.md).\n(( $+functions[_dx_dynamic_targets] )) ||\n_dx_dynamic_targets() {{\n    local -a _dx_words _dx_candidates\n    _dx_words=(${{words[2,-1]}})\n    if (( CURRENT > $#words )); then\n        _dx_words+=(\"\")\n    fi\n    _dx_candidates=(\"${{(@f)$(dx {COMPLETE_SUBCOMMAND} \"${{_dx_words[@]}}\" 2>/dev/null)}}\")\n    compadd -a _dx_candidates\n}}\n"
    ));
    Ok(())
}

/// Appends the fish dynamic section: fixed task lines per
/// task-taking command (derived from the same tables as the
/// [`super::complete`] dispatch, never copied) plus one label line
/// whose condition derives from [`completes_labels`] and whose values
/// call back into the binary at completion time (See:
/// `docs/cli/commands/completion.md`).
fn fish_dynamic(text: &mut String) {
    use clap::ValueEnum;
    text.push_str(&format!(
        "\n#{DYNAMIC_MARKER}: completion-time callback into the binary (See: docs/cli/commands/completion.md)\n"
    ));
    let mut line = |condition: &str, names: &[&str], desc: &str| {
        text.push_str(&format!(
            "complete -c dx -f -n '{condition}' -a '{}' -d '{desc}'\n",
            sorted_join(names)
        ));
    };
    line(
        "__fish_seen_subcommand_from watch",
        dx_adopt::WATCHABLE_COMMANDS,
        "watchable task",
    );
    line(
        "__fish_seen_subcommand_from hooks; and not __fish_seen_subcommand_from install uninstall status run",
        HOOK_VERBS,
        "hooks verb",
    );
    line(
        "__fish_seen_subcommand_from hooks; and __fish_seen_subcommand_from run",
        HOOK_TRIGGERS,
        "hook trigger",
    );
    line(
        "__fish_seen_subcommand_from new",
        dx_adopt::SUPPORTED_NEW_LANGUAGES,
        "project language",
    );
    line(
        "__fish_seen_subcommand_from audit; and not __fish_seen_subcommand_from license security",
        AUDIT_FAMILIES,
        "audit family",
    );
    line(
        "__fish_seen_subcommand_from completion",
        COMPLETION_SHELLS,
        "completion shell",
    );
    line(
        "__fish_seen_subcommand_from update bump",
        &update_set_names(),
        "dependency set",
    );
    let mut label_commands: Vec<&str> = Command::value_variants()
        .iter()
        .filter(|command| completes_labels(**command))
        .map(|command| command.name())
        .collect();
    label_commands.sort_unstable();
    text.push_str(&format!(
        "complete -c dx -n '__fish_seen_subcommand_from {}' -a '(if test -z (commandline -ct); dx {COMPLETE_SUBCOMMAND} (commandline -opc)[2..-1] \"\"; else; dx {COMPLETE_SUBCOMMAND} (commandline -opc)[2..-1]; end 2>/dev/null)'\n",
        label_commands.join(" ")
    ));
}

/// Appends the powershell dynamic section: past the `'dx'` case the
/// script calls back into the binary for scope/task positions (See:
/// `docs/cli/commands/completion.md`). Inserted before the final
/// prefix filter so binary candidates filter identically.
fn powershell_dynamic(text: &mut String) {
    let block = format!(
        "    if ($command -ne 'dx') {{\n        #{DYNAMIC_MARKER}: completion-time callback into the binary (See: docs/cli/commands/completion.md).\n        try {{\n            $dxWords = @()\n            for ($i = 1; $i -lt $commandElements.Count; $i++) {{\n                $element = $commandElements[$i]\n                if ($element -is [StringConstantExpressionAst] -and $element.Value -ne $wordToComplete) {{\n                    $dxWords += $element.Value\n                }}\n            }}\n            $dxDynamic = @(dx {COMPLETE_SUBCOMMAND} @dxWords \"$wordToComplete\" 2>$null)\n            foreach ($candidate in $dxDynamic) {{\n                if ($candidate -ne '') {{\n                    $completions += [CompletionResult]::new($candidate, $candidate, [CompletionResultType]::ParameterValue, $candidate)\n                }}\n            }}\n        }} catch {{}}\n    }}\n"
    );
    let anchor = "    $completions.Where{";
    if let Some(pos) = text.find(anchor) {
        text.insert_str(pos, &block);
    } else {
        text.push_str(&format!("\n{block}"));
    }
}

/// Renders one completion script from the [`Cli`] grammar definition
///: commands, flags, and fixed value sets come from the
/// same source that feeds parsing and `--help`, so generated scripts
/// cannot drift from the command reference. Scope and task positions
/// additionally carry a completion-time callback into the binary
/// (`dx __complete`); repository labels and per-command tasks resolve at
/// completion time from the same tables as parsing. Generation is an
/// explicit `dx completion` cost only, never per-invocation. Unknown
/// shells fail with the contract's `unknown-shell` text.
pub fn render_completion(shell: &str) -> Result<String, ArgsError> {
    use clap::CommandFactory;
    if !COMPLETION_SHELLS.contains(&shell) {
        return Err(ArgsError::UnknownShell {
            shell: shell.to_owned(),
        });
    }
    let generator =
        shell
            .parse::<clap_complete::aot::Shell>()
            .map_err(|_| ArgsError::UnknownShell {
                shell: shell.to_owned(),
            })?;
    let mut command = Cli::command();
    let mut script = Vec::new();
    clap_complete::generate(generator, &mut command, "dx", &mut script);
    let mut text = String::from_utf8(script).map_err(|_| ArgsError::UnknownShell {
        shell: shell.to_owned(),
    })?;
    // Fish/powershell generators omit positional `ValueEnum` values, so
    // commands would be missing there while bash/zsh list them. Append
    // command completions derived from [`Command`] (same source as
    // parsing), never hand-maintained, so every shell completes every
    // command.
    match shell {
        "fish" => {
            use clap::ValueEnum;
            text.push_str("\n# dx commands from the single Command source (issue #202; See: docs/cli/commands/completion.md)\n");
            for cmd in Command::value_variants() {
                let desc = cmd.describe().replace('\'', "\\'");
                text.push_str(&format!(
                    "complete -c dx -f -n '__fish_use_subcommand' -a {} -d '{}'\n",
                    cmd.name(),
                    desc
                ));
            }
            fish_dynamic(&mut text);
        }
        "powershell" => {
            use clap::ValueEnum;
            let mut additions = String::new();
            for cmd in Command::value_variants() {
                let desc = cmd.describe().replace('\'', "''");
                additions.push_str(&format!(
                    "            [CompletionResult]::new('{}', '{}', [CompletionResultType]::ParameterValue, '{}')\n",
                    cmd.name(),
                    cmd.name(),
                    desc
                ));
            }
            // Anchor-stability (See: `docs/cli/commands/completion.md`):
            // only the exact generator anchor inserts functional entries.
            // A `clap_complete` upgrade that shifts the template fails
            // closed here instead of emitting silently-drifted scripts via
            // whitespace-tolerant/header fallbacks (removed: silent drift).
            let anchor = "            break\n        }\n    })";
            if let Some(pos) = text.find(anchor) {
                text.insert_str(pos, &additions);
            } else {
                // Fail closed: never emit non-functional `# dx <cmd>`
                // comments nor fallback-positioned entries. Pinned by the
                // anchor-stability fixture.
                return Err(ArgsError::UnknownShell {
                    shell: shell.to_owned(),
                });
            }
            powershell_dynamic(&mut text);
        }
        "bash" => {
            bash_dynamic(&mut text)?;
        }
        "zsh" => {
            zsh_dynamic(&mut text)?;
        }
        _ => {}
    }
    Ok(text)
}
