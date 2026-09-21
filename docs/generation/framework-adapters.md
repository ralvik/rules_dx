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

Exact upstream parsers, providers, target mappings, generated-region mappings, and test semantics
live in the Vue, Svelte, Astro, and MDX Gazelle extensions, pinned by their fixtures.
An adapter claims its format only after the complete fixture gate above passes; this contract
implies no broader support than the fixtures prove.

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

## Framework Mapping Qualification

Accepted. Each adapter keeps its provisional upstream; no switch is approved here.

Vue, Svelte, Astro, and MDX are qualified by focused end-to-end fixtures:
thin wrappers in `<fw>/rules/defs.bzl` preserving the upstream `JsInfo` provider and
adding `QualitySourcesInfo`, Gazelle extensions in `gazelle/<fw>/` with
parser/naming/lang fixtures plus focused tests and `testdata` generation goldens,
hello builds in `<fw>/tests/fixtures/hello/` as wrapper consumers with upstream parser/compiler
tests, provider-derived environment plans in `<fw>/env/`, and mixed-framework
composition in `examples/mixed/hello/` plus `gazelle/mixed/`.

Upstream parser/compiler: `vue_library` over `aspect_rules_js 3.4.1` with the Vue
3.5.42 runtime and `@vue/compiler-sfc 3.5.42`; `svelte_library` over the same
ruleset with the Svelte 5.57.0 runtime/compiler; `astro_library` over the same
ruleset with the standalone `@astrojs/compiler 4.0.0` Go+WASM compiler;
`mdx_library` over the same ruleset with `@mdx-js/mdx 3.1.1`. Pins live in
`package.json`, `pnpm-lock.yaml`, and `MODULE.bazel`
(`aspect_rules_js 3.4.1`, `aspect_rules_jest 0.26.0`).

Provider: each wrapper creates one private `<name>_upstream` `js_library` plus
one public forwarding rule preserving `JsInfo`, `DefaultInfo`, and
`InstrumentedFilesInfo` unchanged and adding only
`QualitySourcesInfo(direct_sources = {<fw>: <direct container>})`. Only
`js_library` and `JsInfo` are used upstream; execution and tests reuse the
JavaScript binary/test wrappers over parsed or compiled outputs, with no
separate `<fw>_binary` or `<fw>_test` wrapper.

Target and generated-region mappings: each Gazelle extension discovers only its
own container (`SupportedExts`: `.vue`, `.svelte`, `.astro`, `.mdx`), generates
one ordinary reusable one-source `<fw>_library` with deterministic
basename-derived names, and leaves core JS/TS sources and other containers
inert. Each container keeps one physical owner; virtual script, template,
frontmatter, client, style, prose, and expression regions never become
independent physical sources or core JS/TS targets. Generation never compiles
a container and never regex-extracts code.

Dependency: each parser extracts only its executable region (`<script>` for
Vue/Svelte, frontmatter plus client script for Astro, ESM import/export for
MDX) with a narrow block scanner; template, style, prose, fenced code, comments,
strings, template-literal specifiers, and computed imports stay inert. Relative
references normalize to basename without extension, bare specifiers stay
literal, and Node builtins filter via `IsStdLib`. `testdata` goldens prove
grouping, local `deps`, merge, and stale cleanup.

Test: each `<fw>/tests/fixtures/hello/` proves its regions through `javascript_test` over
`Hello.test.js` with `data` on the container, the shared `helper_lib`, and the
upstream compiler package. Vue asserts `parse` template/script/style regions
plus the helper edge; Svelte asserts modern `parse` fragment/instance/css plus
the helper edge; Astro asserts sync `parse` frontmatter/element regions with
no diagnostics; MDX asserts `compile` yields exactly the one first-party edge
with fenced-code imports excluded.

Environment/IDE: each `<fw>/env/plan.bzl` contributes a provider-derived
focused-target plan reading the preserved `JsInfo` transitive sources plus
`QualitySourcesInfo` direct sources, pinned by `<fw>/env/plan_tests.bzl` and
exercised by `<fw>/env:hello_lib_plan` over `//<fw>/tests/fixtures/hello:hello_lib`.
Binaries and tests share the same closure through `data`/runfiles;
repository/root/exact-target orchestration remains closed #506 (successors closed #787 and #788).

Quality-region: `vue`, `svelte`, `astro`, and `mdx` are frozen semantic
file classes in `quality/sources.bzl`, each owning its own policy family in
`quality/adapters.bzl` on classification-only terms: the container stays one
physical owner with virtual regions handed to execution-time integrations,
and no lint/format/typecheck adapter claims any framework class yet.

Composition: `examples/mixed/hello/` keeps one wrapper per container plus the
shared core `helper_lib`, with `hello_test` proving the shared helper edge,
per-container helper references, and no framework-to-framework imports.
`gazelle/mixed/` proves the partition is disjoint and complete for the closed
v1 set with case-sensitive extension matching, no fallback, and no eager work
for unused adapters. Required-core adapter mappings plus composition evidence
stay qualified seed-only under closed #510 (successors closed #796-#800)
(`quality/tests/fixtures/layer2_opens/pins.bzl` with `layer2_opens.expected`
via `bazel run //tools/ci:layer2_opens_qualification`, adapter-less as pass
rejected); no `Supported` claim until platform plus consumer
plus release evidence passes.

Pinned by `bazel run //tools/ci:foundation_maps` plus
`bazel run //tools/ci:layer2_opens_qualification`.
