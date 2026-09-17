//! Shell-completion rendering for the `dx` CLI (issue #236).
//!
//! Split from `super` (`args.rs`): owns [`COMPLETION_SHELLS`] and
//! [`render_completion`]. Re-exported through `super` so the public path
//! stays `crate::args::{COMPLETION_SHELLS, render_completion}`.

use super::command::Command;
use super::parser::Cli;
use super::ArgsError;

/// Shells covered by `dx completion` (contract freeze).
pub const COMPLETION_SHELLS: &[&str] = &["bash", "zsh", "fish", "powershell"];

/// Renders one completion script from the [`Cli`] grammar definition
/// (issue #202): commands, flags, and fixed value sets come from the
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
    // command (issue #202).
    match shell {
        "fish" => {
            use clap::ValueEnum;
            text.push_str("\n# dx commands from the single Command source (issue #202)\n");
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
            } else {
                text.push_str("\n# dx commands from the single Command source (issue #202)\n");
                for cmd in Command::value_variants() {
                    text.push_str(&format!("# dx {}\n", cmd.name()));
                }
            }
        }
        _ => {}
    }
    Ok(text)
}
