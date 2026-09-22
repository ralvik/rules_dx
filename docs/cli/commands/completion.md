# `dx completion`

Implementation status: implemented
(`dx completion` dispatch over the single CLI command-definition source).

## Invocation

`dx completion <shell>` (shells `bash|zsh|fish|powershell`) prints a static completion script for the named shell to stdout and exits `0`. An unknown shell name fails with `unknown-shell` and a non-zero exit. The command writes no files and mutates no shell state. There is no `--check` mode; `--check` fails pre-exec (exit `2`) via the adoption gate. `--dry-run` prints `would render completion for <shell>` without rendering; plans are summaries, suppressed under `--quiet`.

## Generation Source

Scripts are generated at runtime by the `dx` binary itself from the single CLI command-definition source, following the `kubectl`/`gh` generator-subcommand convention: a framework facility (of the `clap_complete` class) renders every supported shell from the command table, so no script is ever handwritten or stored. The owning source is the `Command` vocabulary in `cli/cli/src/args/command.rs` plus the `Cli` grammar in `cli/cli/src/args/grammar.rs` (re-exported through `cli/cli/src/args.rs`), rendered by `cli/cli/src/args/completion.rs`. Adding a command or flag regenerates every script, so completion cannot drift from the [command reference](README.md) or [ADR 0006](../../decisions/0006-cli-command-surface.md). Fixture tests assert every command and flag appears in each supported shell's output, plus the value sets (`--output` `text|diff|json`, `--fail-on` `info|warning|error`, `<shell>` `bash|zsh|fish|powershell`) via the parse-time `BadOutput`/`BadFailOn`/`UnknownShell` errors and the exact `COMPLETION_SHELLS` list (pinned by `value_sets_and_man_parity_are_pinned`): value sets validate at parse time, not via shell value completion, since the grammar uses `String`.

Anchor-stability: the powershell insertion renders functional
`CompletionResult` entries only at the exact generator anchor
(`break` closing the `'dx'` case). A `clap_complete` upgrade that shifts
the template fails closed (`unknown-shell`) instead of emitting a
silently-drifted script via fallbacks (whitespace-tolerant and header
fallbacks removed; silent drift rejected). Bash (`*)` fallback) and zsh
(targets line) likewise fail closed on template drift. Pinned by the
powershell anchor-stability assertions in `completion_renders_from_single_source`.

## Dynamic Candidates

Scope and task positions complete dynamically through a completion-time callback into the binary, matching the dynamic-candidate pattern used by the same convention. Each generated script invokes `dx __complete <typed...> <current>` at completion time (last word is the current prefix, `""` for a new word) and offers one candidate per line.

`__complete` is hidden: never a `Command` variant, never in `--help` or usage, so the 32-command registry stays exact. It always exits `0` with no stderr, so completion never breaks typing; unknown shapes yield no output.

First-slot tasks come from the same tables as execution: `watch` offers the watchable commands, `hooks` its verbs then (`run`) its triggers, `new` its languages, `audit` its families, `completion` its shells, `update`/`bump` their dependency sets. All other scope slots offer Bazel labels: `//...` plus `//dir/...` per directory holding a `BUILD.bazel`/`BUILD` marker (bounded scan; hidden, `bazel-*`, and symlinked directories skipped; external `@` and path prefixes yield nothing so shell file completion owns them). The per-slot dispatch in `cli/cli/src/args/complete.rs` matches exhaustively over `Command`, so adding a command breaks compilation until its slot is classified. Fixture tests pin the fish task payloads verbatim plus the callback marker in every shell.

## Installation

Installation is a one-time shell setup, not a per-directory step: evaluate the script from the shell rc file guarded by `command -v dx`, or place it in the shell's completions directory. Do not evaluate it from `.envrc`; direnv re-evaluates on every directory change, and completion setup must not run on every `cd`. Regenerate after upgrading `dx`: the embedded callback tracks the binary it was generated from.

Per-shell homes: bash `~/.bash_completion.d/dx` (or `/usr/share/bash-completion/completions/dx`), zsh a `${fpath}` entry named `_dx`, fish `~/.config/fish/completions/dx.fish`, powershell a file dot-sourced from `$PROFILE`. Generate each file with `dx completion <shell>`.

## Manual Page

Build the manual page with `bazel build //cli/cli:man_pages` (emits `man/dx.1` under `bazel-bin/cli/cli/`, section 1). Preview with `man --local-file bazel-bin/cli/cli/man/dx.1`; install system-wide to `/usr/share/man/man1/dx.1`, then `mandb`. The draft release ships `man/dx.1` alongside the `dx` binary (see the [release runbook](../../deploy/release-runbook.md)). The CLI is a single command with a command-word value, so the grammar yields one page; per-command pages arrive only if the grammar ever gains subcommands. `man/dx.1` renders from the same `cli_command` grammar as `--help` (`dx_man` via `clap_mangen::Man::new(cli_command())`, pinned by `value_sets_and_man_parity_are_pinned`), so manual-page vs help parity holds by construction.

## Direnv Composition

Direnv and completion compose without integration. Direnv places `.dx/bin` on `PATH` per directory, which keeps the `dx` binary visible so lazily-loaded completions resolve. No completion state lives in `.envrc`, and completion never reads direnv state.
