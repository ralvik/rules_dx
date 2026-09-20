# ADR 0020: Remove The `dx docs` Placeholder Command

## Status

Accepted. Supersedes the `dx docs` bullet in
[ADR 0006](./0006-cli-command-surface.md). Reintroduction with real
extraction/validation remains open work under issues #786 and #779-#785
([roadmap](../roadmap.md));
removal is complete.

## Context

[ADR 0006](./0006-cli-command-surface.md) selected `dx docs` for Bazel-owned
documentation extraction, validation, and rendering. The shipped command never
did that: `dx docs --check` validated flag shape (one rule: `--port`
requires `--serve`), printed a planned-action line, and exited 0 without
opening any Markdown, resolving the scope, or running extraction,
validation, or rendering. Its only real effects were proving `//dx/cli:dx`
compiles and rejecting malformed flags.

Meanwhile the green `docs-check` job guarded `docs-publish`, so the command
name overpromised in front of a deploy gate: the
docs Pages 404 deployed
through a green check. A placeholder that reports success in front of a
deploy gate is worse than no placeholder.

## Decision

Delete the `dx docs` command surface (`Command::Docs`, `execute_docs`,
`plan_docs`, `--serve`/`--port` flags, usage strings, completion entry) and
the `docs --check` step in the reusable docs workflow. Keep the `dx/docs`
IR/mode planning library: it remains the
open contract for
reintroduction, not a delivered invocation. The docs-content gate in CI is
`dx lint --check` (repository-owned link/structure audit plus `vale` over
`//docs:corpus`).

## Consequences

- `dx docs` fails as `unknown command`; the command registry, usage
  strings, and shell completions no longer list it.
- The [docs command reference](../cli/commands/docs.md) is a stub pointing
  at open work under issues #786 and #779-#785 ([roadmap](../roadmap.md)).
- Reintroducing the command alongside real extraction/validation behind
  the invocation remains open work under issues #786 and #779-#785
  ([roadmap](../roadmap.md)).
