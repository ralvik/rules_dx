# CLI

Contracts for the `dx` command surface and its user-facing protocols:

- [CLI Contract](cli-contract.md): shared invocation, process, and launcher behavior.
- Helper Upstream Policy: re-evaluation triggers, adopt-vs-keep criteria, and per-item verdicts for hand-rolled helpers.
- [Command Reference](commands/README.md): behavior of each command.
- [Target Resolution](target-resolution.md): path and label normalization through Bazel.
- [Output Protocol](output-protocol.md): text, diff, NDJSON, streams, and errors.
- [Standard Reports](standard-reports.md): authoritative SARIF, JUnit XML, and LCOV format profiles.
- [dx completion](commands/completion.md): generated shell completions plus the `man/dx.1` manual page.

Related: [GitHub CI](../github-ci.md) defines consumer CI execution and reporting integration.
