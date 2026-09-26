# Repository Instructions

This repository uses Bazel.

- Treat warnings as errors.
- Document only what a user needs to run dx. Delete the rest; never maintain design rationale in comments or docs.
- Comments: only when the why is not obvious from the code. One short plain line, no links, no issue numbers, no `See:`/`Contract:` chains, no restating the code. When in doubt, delete it. Exception: doc comments that generate user-visible text (clap `Parser`/`Args`/`ValueEnum` fields and variants feeding `--help` and completions) stay, one short line each.
- Keep READMEs short; usage in `docs/cli/commands/`, CI usage in `docs/github-ci.md`, examples in `examples/`. Do not add docs elsewhere.
- Write plain and human. Short sentences. No filler, no essays, no AI tone.
