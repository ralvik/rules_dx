# `dx completion`

Provisional surface under [O61](../../open-decisions.md); exact shell list and script mechanics are pending.

## Invocation

`dx completion <shell>` prints a static completion script for the named shell to stdout and exits `0`. An unknown shell name fails with `unknown-shell` and a non-zero exit. The command writes no files and mutates no shell state.

## Generation Source

Scripts are generated at runtime by the `dx` binary itself from the single CLI command-definition source, following the `kubectl`/`gh` generator-subcommand convention: a framework facility (of the `clap_complete` class) renders every supported shell from the command table, so no script is ever handwritten or stored. The owning source is the [M08 command planner](../../milestones/M08-target-resolution-basic-commands.md#deliverables). Adding a command or flag regenerates every script, so completion cannot drift from the [command reference](README.md) or [ADR 0006](../../decisions/0006-cli-command-surface.md). Fixture tests assert every command and flag appears in each supported shell's output.

## Installation

Installation is a one-time shell setup, not a per-directory step: evaluate the script from the shell rc file guarded by `command -v dx`, or place it in the shell's completions directory. Do not evaluate it from `.envrc`; direnv re-evaluates on every directory change, and completion setup must not run on every `cd`.

## Direnv Composition

Direnv and completion compose without integration. Direnv places `.dx/bin` on `PATH` per directory, which keeps the `dx` binary visible so lazily-loaded completions resolve. No completion state lives in `.envrc`, and completion never reads direnv state.

## Out Of Scope

Dynamic candidate completion (target labels, task names, and similar repository-derived candidates) stays a future option; v1 completes commands, flags, and fixed value sets only. The reserved extension point is a completion-time callback into the binary, matching the dynamic-candidate pattern used by the same convention.
