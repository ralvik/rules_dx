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
`target_compatible_with = ["@platforms//os:linux"]`. See the
[shell contract](../testing/tools.md#shell-and-host-tool-contract).

Allowed ref: `Bash-only harness is Linux-only (issue #299 shell contract).`

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
`tools/bazelrc/preset.py`.

## Gazelle Boundary

Hand comments stay out of regen-owned stanzas. Bare `# keep` only where
the generator requires it. See [generation](../generation/README.md) and
the [common contract](../generation/common.md).
