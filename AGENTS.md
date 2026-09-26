# Repository Instructions

This repository uses Bazel.

- Treat warnings as errors.
- Only user docs. Allowed: `docs/cli/commands/`, `docs/github-ci.md`, short READMEs, `examples/`. Delete anything else. Never write design rationale, history, alternatives, or implementation notes.
- Comments: delete by default. Keep max one short plain line, only when the why is weird and not obvious from the code. No links, no issue numbers, no `See:`/`Contract:`/`policy:`, no ADR, no backtick chains, no restating the code. When in doubt, delete. This includes single-line comments: `// Hello returns a greeting`, `"""Returns kwargs with ..."""`, `# Seed Go fixture`, `doc = \"...\"` that repeats the attribute name. Delete all of those. Assigned strings like `_HUB_BUILD = \"\"\"...\"\"\"` are code, not comments. Keep. Examples of what to delete: multi-line `///` rationale blocks, `.bazelrc` paragraphs like the Windows runfiles essay, `doc/..._doc` strings that repeat the code, file-header ` pins` lines. One line like `# Windows needs the runfiles tree.` is the most ever allowed, and often nothing is better.
- Two exceptions only: `--help` text stays one short line per flag, and `LCOV_EXCL_*` markers keep their required short `reason:` plus `issue:`.
- Write plain and human. Short sentences. Commands first. No filler, no essays, no AI tone. If a paragraph explains why something exists, delete it.
