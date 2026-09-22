# Mixed Ownership Contract

This contract owns the mixed-framework ownership partition proved by
`gazelle/mixed/mixed.go`. It follows the [common contract](common.md) and
the [framework adapters](framework-adapters.md); rationale lives in
[ADR 0015](../decisions/0015-first-party-gazelle-extensions.md) and evidence
in the [test matrix](../testing/generation.md#generate-command-and-gazelle-extensions).

## Partition

Each v1 container keeps exactly one physical owner: `.vue` is vue-owned,
`.svelte` is svelte-owned, `.astro` is astro-owned, `.mdx` is mdx-owned,
core `.js`/`.jsx`/`.mjs`/`.cjs` stays javascript-owned, and
`.ts`/`.tsx`/`.mts`/`.cts` stays typescript-owned. Matching is
case-sensitive on the final extension with no lowercasing and no generic
fallback; unknown extensions have no owner.

## Laziness

An empty directory or one without a supported extension yields no rules.
Unused adapters do no eager work: the partition result never contains an
empty adapter entry, so callers assert laziness by checking for a nil or
empty map.

## Evidence

Accepted. Pinned by `bazel run //tools/ci:foundation_maps` plus
`bazel run //tools/ci:layer2_opens_qualification`, with composition proved
by `examples/mixed/hello/` plus `gazelle/mixed/`. No `Supported` claim
until platform plus consumer plus release evidence passes per the
[support matrix](../product/support-matrix.md).
