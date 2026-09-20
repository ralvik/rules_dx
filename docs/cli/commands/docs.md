# `dx docs`

Removed. The `dx docs` command was deleted per
[ADR 0020](../../decisions/0020-remove-dx-docs-placeholder.md): the shipped
`--check` mode validated flag shape, not docs content, while its name
overpromised in front of the docs-publish deploy gate.

Reintroduction with real extraction/validation behind the invocation is
open under issue #581 (live successor to closed #421).
The design contracts it will implement are unchanged:

- [Documentation IR](../../documentation/doc-ir.md): common symbol model,
  language extensions, validation, fixtures, and drift policy.
- [Site build](../../documentation/site.md): Bazel cache-friendly action
  design and the decided mdBook renderer.

Recorded by [ADR 0020](../../decisions/0020-remove-dx-docs-placeholder.md),
which supersedes the `dx docs` bullet in
[ADR 0006](../../decisions/0006-cli-command-surface.md).
