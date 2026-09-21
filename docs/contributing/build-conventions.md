# BUILD Conventions

Accepted. One-line refs for repeated `BUILD.bazel` patterns. Full rationale
lives in the owning docs linked below; `BUILD.bazel` files carry only the
one-liner plus non-obvious attributes.

## Lane A

Normal targets bind workspace policy via direct `aspect_hints`. See
[native configuration](../quality/native-configuration.md#binding).

Allowed ref: `Lane A: direct aspect_hints binding; see this doc.`

## Corpus

Each package owns its checked-in sources via
`real_source_target(name = "corpus_*")` per content type. See the
[corpus dogfood](../contributing/local-workflows.md#corpus-dogfood) and
[source ownership](../quality/quality-sources.md).

Allowed ref: `Corpus: BUILD ownership; see this doc.`

## No-Coverage

Process-spawning tests carry `tags = ["no-coverage"]` and stay out of the
coverage denominator via `test_tag_filters=-no-coverage`. See
[testing strategy](../testing/README.md#coverage) and the
[verification matrix](../testing/verification-matrix.md#layers).

Allowed ref: `No-coverage: process-spawning test; see this doc.`

## Shell

Bash `sh_binary`/`sh_test` targets carry
`target_compatible_with = ["@platforms//os:linux"]` plus
`data = ["//tools/sh:bootstrap", "//tools/sh:lib"]` for the single-sourced
bootstrap loader plus `dx_bootstrap` (plus `data = ["//tools/sh:guards"]`
when the driver uses the table-driven guard rows). See the
[shell contract](../testing/tools.md#shell-and-host-tool-contract) for the
bash-only bootstrap floor plus guard maintenance (shared helpers plus
snapshot versus grep policy under issue #450, table rows under issue #653,
single-sourced bootstrap under issue #654).

Allowed ref: `Bash-only harness is Linux-only (issue #299 shell contract).`

## Visibility

Public (`//visibility:public`) is external API only: `//config`, `//dx`,
`//env`, `//generation`, `//quality` roots, `//<lang>/rules` wrappers, and
`//deploy/rules` macros. Everything else is repo-internal
(`//:__subpackages__`) or narrower (`//cli`, `//quality`, `//env`,
`//generation`, `//docs`, `//tools` scopes; `//visibility:private` for
`<lang>/env` test plans with no cross-package consumers). No new public
defaults or public target visibilities outside the allowlist. Guard:
`//tools/ci:visibility_guards` (issue #456).

## Comment Rules

Keep only what the target is plus non-obvious attributes
(`aspect_hints`, `target_compatible_with`, `tags`). Replace essays with a
link to the owning doc, ADR, or issue. Banned in `BUILD.bazel`: the
forwarder-plumbing essay, the coverage-denominator essay, the
no-corpus-entry essay, the dogfood-policy essay, `real_fixture_policy`,
the keep-regeneration essay, and `Release:` provenance lines.

## Generated Headers

Header is `GENERATED, do not edit` plus one regenerate command. Provenance
lives in generator inputs and locks; artifact metadata already carries
`url`/`sha256`. See `quality/artifacts/update.py` and
`tools/bazelrc/src/lib.rs`.

## Gazelle Boundary

Hand comments stay out of regen-owned stanzas. Bare `# keep` only where
the generator requires it. See [generation](../generation/README.md) and
the [common contract](../generation/common.md).
