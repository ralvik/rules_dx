//! Shell-completion rendering for the `dx` CLI.
//!
//! Split from `super` (`args.rs`): owns [`COMPLETION_SHELLS`] and
//! [`render_completion`]. Re-exported through `super` so the public path
//! stays `crate::args::{COMPLETION_SHELLS, render_completion}`.

use super::command::Command;
use super::grammar::Cli;
use super::ArgsError;

/// Shells covered by `dx completion` (contract freeze).
pub const COMPLETION_SHELLS: &[&str] = &["bash", "zsh", "fish", "powershell"];

/// Line-start of the `break` closing the `'dx'` case in the rendered
/// powershell script, searched from the `'dx' {` header with whole-word
/// matching so tooltip text (`breaking`) never matches (See:
/// `docs/cli/commands/completion.md`).
fn powershell_case_break(text: &str) -> Option<usize> {
    let header = text.find("'dx' {")?;
    let region = &text[header..];
    let mut from = 0;
    while let Some(rel) = region[from..].find("break") {
        let abs_break = header + from + rel;
        let before_ok = text[..abs_break]
            .chars()
            .next_back()
            .is_none_or(|c| !(c.is_alphanumeric() || c == '_' || c == '-'));
        let after_ok = text[abs_break + "break".len()..]
            .chars()
            .next()
            .is_none_or(|c| !(c.is_alphanumeric() || c == '_' || c == '-'));
        if before_ok && after_ok {
            let line_start = text[..abs_break].rfind('\n').map_or(0, |i| i + 1);
            return Some(line_start);
        }
        from += rel + "break".len();
    }
    None
}

/// Renders one completion script from the [`Cli`] grammar definition
///: commands, flags, and fixed value sets come from the
/// same source that feeds parsing and `--help`, so generated scripts
/// cannot drift from the command reference. Generation is an explicit
/// `dx completion` cost only, never per-invocation. Unknown shells fail
/// with the contract's `unknown-shell` text.
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
            let anchor = "            break\n        }\n    })";
            if let Some(pos) = text.find(anchor) {
                text.insert_str(pos, &additions);
            } else if let Some(pos) = powershell_case_break(&text) {
                // Whitespace-tolerant fallback: same functional insertion
                // before the case-closing `break` (See:
                // `docs/cli/commands/completion.md`).
                text.insert_str(pos, &additions);
            } else if let Some(header) = text.find("'dx' {") {
                // Last functional fallback: commands before flags when the
                // case tail drifted beyond recognition.
                text.insert_str(header + "'dx' {".len(), &format!("\n{additions}"));
            } else {
                // Fail closed: never emit non-functional `# dx <cmd>`
                // comments. Pinned by the anchor-stability fixture.
                return Err(ArgsError::UnknownShell {
                    shell: shell.to_owned(),
                });
            }
        }
        _ => {}
    }
    Ok(text)
}
