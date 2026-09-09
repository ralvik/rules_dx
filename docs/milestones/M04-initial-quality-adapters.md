# M04: Rust, Starlark, TOML, And Markdown Acquisition And Adapters

## Outcome

The repository's initial source classes have hermetic, adapter-tested quality integrations usable through direct Bazel.

## Scope

Add rustfmt/Clippy, Buildifier, Taplo, and Vale adapters with their authoritative-toolchain or standalone
acquisition paths. Markdown also receives repository link and structure validation. Prettier is deferred.

## Contract References

- [Tool integration decision](../decisions/0007-tool-integration-model.md), [tools](../tools/), [quality](../quality/), [testing](../testing/), and open decision [O20](../open-decisions.md).

## Deliverables

- Adapters and typed native-config inputs for Rust, Starlark, TOML, and Markdown.
- Lazy checksummed artifact metadata as generated checked-in files with a regeneration
  command, and authoritative Rust toolchain bindings.

## Work Packages

1. Prove rustfmt/Clippy toolchain identity and standalone Buildifier, Taplo, and Vale acquisition.
2. Implement capability manifests, native config, diagnostics, and edits.
3. Integrate link/structure validation as repository-owned Markdown checks.

## Milestone-Specific Evidence

- Each adapter passes VCS-isolation, exact-input, no-config, edit, cache, and empty-`PATH` fixtures.
- [Initial adapter evidence](../quality/quality-testing.md#initial-adapter-evidence) proves Vale's
  actionable config-required failure and Taplo's version-qualified fail-closed text parsing.
- Acquisition is lazy by execution platform; adapter-tested does not imply supported.

## Out Of Scope

- Prettier, CLI quality commands, repository-wide dogfood enforcement, and other adapters.

## Completion Report Additions

- Record tool versions, delivery identities, semantic classes, native configs, and adapter-test cells.
