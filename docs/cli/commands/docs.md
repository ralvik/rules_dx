# `dx docs`

Removed. The `dx docs` command was deleted in
[issue #31](https://github.com/ralvik/rules_dx/issues/31): the shipped
`--check` mode validated flag shape, not docs content, while its name
overpromised in front of the docs-publish deploy gate.

Reintroduction with real extraction/validation behind the invocation is
tracked in [issue #10](https://github.com/ralvik/rules_dx/issues/10).
The design contracts it will implement are unchanged:

- [Documentation IR](../../documentation/doc-ir.md): common symbol model,
  language extensions, validation, fixtures, and drift policy.
- [Site build](../../documentation/site.md): Bazel cache-friendly action
  design and the decided mdBook renderer.

Recorded by [ADR 0020](../../decisions/0020-remove-dx-docs-placeholder.md),
which supersedes the `dx docs` bullet in
[ADR 0006](../../decisions/0006-cli-command-surface.md).
