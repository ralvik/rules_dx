# Framework Adapter Generation Contract

This contract defines the boundary for Vue, Svelte, Astro, and MDX generation over the core
[JavaScript and TypeScript](javascript-typescript.md) foundation. See
[ADR 0013](../decisions/0013-rust-javascript-typescript-foundations.md) and
[ADR 0015](../decisions/0015-first-party-gazelle-extensions.md) for rationale and the
[generation test matrix](../testing/generation.md#generate-command-and-gazelle-extensions) for
required evidence.

## Adapter Boundary

`.vue`, `.svelte`, `.astro`, and `.mdx` files are inert to the core JavaScript and TypeScript
extensions. Each format requires a separate named adapter over a stable upstream parser/compiler and
Bazel integration. No generic container parser, arbitrary framework registration API, regex
extraction, or implicit plain-JavaScript/TypeScript fallback is accepted.

An adapter may claim its format only after complete ownership, dependency, test, build, run, IDE,
quality, platform, merge, and stale-cleanup fixtures pass. The adapter follows the
[common generation contract](common.md), including strict target-scoped dependency resolution,
single ownership, conservative merge, user-owned resources, and fail-closed collisions.

Exact upstream parsers, providers, target mappings, generated-region mappings, and test semantics are
still unresolved independently in [O29 and O40-O42](../open-decisions.md); additional v1 framework
scope is O43. These adapters are planned, not implied to be implemented or supported by this
contract.

## Physical And Virtual Ownership

Each checked-in framework container has one physical source owner. Embedded script, expression,
template, style, frontmatter, client, and other semantic regions do not become independent physical
sources or core JS/TS targets. The named adapter uses its authoritative upstream parser/compiler to
hand virtual-region semantics to dependency, generated-output, IDE, and quality integrations without
duplicating the container or a region across owners.

Generation does not compile a container, regex-extract code, infer unsupported regions, or assign a
region to the core extension. Unsupported or ambiguous syntax fails or remains inert according to the
adapter's fixture-proven contract; it never falls back to a guessed JavaScript/TypeScript owner.

## Format-Specific Regions

Vue and Svelte adapters preserve one physical owner while representing script, template, style,
dependency, generated-output, and quality regions through their respective upstream integrations.
Neither adapter may duplicate virtual-region ownership or share a generic container parser.

Astro preserves the distinct semantics of frontmatter/server script, component/template content,
client scripts, dependencies, and generated outputs. These regions are not flattened into one
plain-JavaScript interpretation or assigned duplicate owners.

MDX keeps Markdown prose non-executable for dependency discovery. Only expressions identified by the
authoritative MDX parser/compiler contribute executable-region semantics or dependency references;
generation does not scan prose or code-like text heuristically.

Shared adapter mechanics may be extracted only after concrete adapters prove reuse. Mixed-framework
ownership and cross-framework imports must retain deterministic single ownership, with no eager work
for an unused adapter and no generic fallback.
