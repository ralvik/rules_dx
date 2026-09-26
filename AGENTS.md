# Repository Instructions

This repository uses Bazel.

- Treat warnings as errors.
- Only user docs. Allowed: `docs/cli/commands/`, `docs/github-ci.md`, short READMEs, `examples/`. Delete anything else. Never write design rationale, history, alternatives, or implementation notes.
- Comments: max one short plain line, only when the why is not obvious. No links, no issue numbers, no `See:`/`Contract:`/`policy:`, no backtick chains, no restating the code. When in doubt, delete. Examples of what to delete: multi-line `///` rationale blocks, `.bazelrc` paragraphs like the Windows runfiles essay. One line like `# Windows needs the runfiles tree.` is the most ever allowed, and often nothing is better.
- Two exceptions only: `--help` text stays one short line per flag, and `LCOV_EXCL_*` markers keep their required short `reason:` plus `issue:`.
- Write plain and human. Short sentences. Commands first. No filler, no essays, no AI tone. If a paragraph explains why something exists, delete it.
