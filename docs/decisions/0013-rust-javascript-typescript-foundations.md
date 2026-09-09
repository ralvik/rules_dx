# ADR 0013: Rust and JavaScript/TypeScript Foundations

## Status

Accepted.

## Context

Rust, JavaScript, and TypeScript need first-class build, test, dependency, generation, quality, and
developer-environment behavior. Their ecosystems already have mature Bazel rules, package-manager
models, compiler semantics, and platform integrations. Reimplementing those facilities in
`rules_dx` would create competing Cargo, Rust, Node, pnpm, or TypeScript engines.

Rust is the first application foundation after the shared quality-core dogfood. It establishes the
first concrete language generation and native-environment integration. Python and
JavaScript/TypeScript are later foundations; later similarities must not drive a speculative
cross-language engine or universal language model into the Rust implementation.

The upstream ecosystems expose different provider, transition, toolchain, IDE, and environment
capabilities. A uniform user experience therefore cannot imply identical internal APIs or promise
version behavior that an authoritative upstream graph cannot represent.

## Decision

### Upstream Foundations

Rust uses an exact latest-stable `hermeticbuild/rules_rs` release and its pinned, patched
`rules_rust` stack as the preferred authority for toolchains, compilation, Cargo dependencies,
providers, proc macros, build scripts, generated sources, and rust-analyzer integration.
`rules_dx` does not implement another compiler rule, Cargo resolver, crate graph, linker model,
build-script protocol, or proc-macro transition. Exact release and patched-commit identities
are provisional pending O16/O24/O27/O37 stack qualification; `latest-stable` here states the
selection policy, not a frozen pin. This record does not pin a version through prose.

The tested-stack manifest records both the selected `rules_rs` release and the identity of its
underlying patched `rules_rust`. Both follow the strict dependency and compatibility-exception
policy in [ADR 0008](0008-dependency-currency.md). This preferred stack remains selected. Direct
`rules_rust` is an established upstream alternative to compare if substantial fixes are needed, not
an automatic fallback. Any switch requires explicit contract/API review and approval through
[Open Decisions](../open-decisions.md), plus fixture-proven conformance; it does not permit a
project-owned Rust engine.

Rust native dependencies follow the Windows compatibility baseline and platform-specific native
backend permission in [ADR 0014](0014-tested-platform-release-stack.md#decision). That permission
does not replace the preferred Rust foundation or approve new provider mappings.

JavaScript uses exact latest-stable `aspect_rules_js` as the preferred authority for Node and pnpm
dependency semantics. TypeScript uses exact latest-stable `aspect_rules_ts` for TypeScript compilation
and project semantics. JavaScript and TypeScript tests use exact latest-stable `aspect_rules_jest`
for Jest execution. These upstream choices remain preferred and selected. `rules_dx` does not
implement a replacement package installer, pnpm resolver, TypeScript compiler, or JavaScript test
runner to address upstream gaps.

For every language, `rules_dx` exposes only narrow conventional wrappers, providers, and setup. It
owns opinionated defaults and compatibility at that boundary but does not re-export the complete
upstream API. Consumers needing advanced upstream APIs may declare and load the upstream
module directly. Exact public symbols and provider mappings remain fixture-gated in
[Open Decisions](../open-decisions.md) and [Testing](../testing/).

### Core-Language Completion And Remediation

Complete core Rust, Python, JavaScript, and TypeScript support on every required platform is required,
not conditional on the current upstream stack working without changes. The
[Support Matrix](../product/support-matrix.md) and [Testing](../testing/) define the required support
and evidence; this requirement does not claim that unverified capabilities already work. Python's
preferred foundation remains defined by [ADR 0010](0010-python-foundation.md).

Focused upstream rules patches are allowed when pinned reproducibly and tested against the accepted
contracts and required platforms. Prefer upstreaming those fixes. Small missing integration pieces
may be owned by `rules_dx` with a clearly bounded scope and maintenance responsibility, while
preserving upstream language semantics and verified public provider boundaries.

If the preferred stack needs substantial fixes, compare established alternative upstream rules
before considering replacement rule implementations. Any alternative requires explicit contract/API
review and approval before adoption, including compatibility impact and the required conformance
evidence; it is never a silent fallback. This strategy does not authorize a whole ruleset rewrite or
replacement language engine, dependency solver, compiler, or runtime.

Until a compliant remedy passes the required evidence, the affected capability fails closed and its
support claim remains blocked. Upstream failure does not justify weaker contracts, private graph
coupling, or dropping required core-language/platform scope, and it does not preclude remediation.

### First-Party Generation

Rust, JavaScript, and TypeScript use first-party `rules_dx` Gazelle extensions to generate stable
wrapper targets. Generation is project-owned because source discovery and the uniform generated
surface are product behavior; compilation, package resolution, and language semantics remain
upstream-owned. No extension may inspect private serialized upstream graphs or recreate an
ecosystem resolver.

Rust is the first concrete extension. It is implemented directly against Gazelle and the public
upstream rule/provider surface without first inventing a shared language framework. Shared mechanics
may be extracted only after Python or JavaScript/TypeScript proves concrete reuse. There is no
parallel language engine, universal source model, or runtime plugin framework.

Rust preserves native crate ownership: one authoritative crate root owns its recursively loaded
module tree, while independent test crates retain their own native ownership. JavaScript and
TypeScript use reusable per-source ownership with separate tests and thin executable entries. All
languages fail closed rather than duplicate a source, broaden unrelated production dependencies, or
change native language semantics to fit a common shape.

The complete source, ownership, target-kind, test, entry-point, naming, Cargo, pnpm, merge, and
resource contracts are owned by [Rust Generation](../generation/rust.md) and
[JavaScript And TypeScript Generation](../generation/javascript-typescript.md). Vue, Svelte, Astro,
and MDX are separate named integrations under
[Framework Adapter Generation](../generation/framework-adapters.md), never implicit core-language
fallbacks or one generic container parser.

### Dependency Authority

`Cargo.toml`, `Cargo.lock`, Cargo target/dependency policy, and verified public Rust-rule metadata are
authoritative for Rust ecosystem semantics. `package.json`, `pnpm-lock.yaml`, importer selection, and
verified public Aspect metadata are authoritative for JavaScript and TypeScript. A source-only local
or standard-library graph may generate where the language contract defines an unambiguous shape,
but generation does not invent missing ecosystem policy.

External references resolve only from dependencies active in the generated target's authoritative
scope. Lockfile presence alone is insufficient. Imports do not enable Cargo features, optional
dependencies, development scope, pnpm importers, platform packages, or package-manager settings.
Unknown, ambiguous, inactive, or insufficiently described dependencies fail rather than being
guessed, omitted, installed, or deferred to a later build.

The uniform strict-resolution and user-owned exception policy is authoritative under
[Generation](../generation/). Exact syntax recognizers, Cargo target mappings, pnpm importer/store
mappings, version selection, tests, coverage, and other upstream adaptations remain unresolved where
listed in [Open Decisions](../open-decisions.md).

### Environment And IDE Direction

Each language participates in the language-neutral `dx env` and managed-state contracts while
retaining its ecosystem-native layout. Environment contents come from analyzed authoritative Bazel
providers and selected dependency artifacts, never from checkout scans, ambient installers, a
second resolution, or reconstructed lockfile semantics.

Rust IDE behavior reuses the patched upstream rust-analyzer discovery, proc-macro, generated-source,
formatting, and Bazel-backed flycheck pipeline rather than constructing a separate static crate
graph. The narrowly approved Rust discovery and flycheck launchers may refresh Rust analysis through
their separate Bazel IDE path, but they do not mutate selected environment or codegen state.

Node environments preserve pnpm importer semantics and conventional importer-local `node_modules`
facades over Bazel-selected artifacts. They do not flatten a multi-importer graph or perform a
host-side install. Exact Rust and Node provider mappings and persistent shapes remain evidence-gated
through [Open Decisions](../open-decisions.md).

Selection, native layouts, immutable state, atomic commits, carry-forward, generated-code separation,
ownership, and retention are authoritative under [Environments](../environments/), including the
[Rust](../environments/rust.md) and [Node](../environments/node.md) contracts. Rust remains the
first language-native projection; Python and JavaScript/TypeScript follow it rather than redefining
the generic environment architecture.

### Quality, Versions, And Laziness

Rust quality uses the authoritative selected toolchain's rustfmt and Clippy. JavaScript and
TypeScript use Biome as the curated default formatter and linter, with Prettier available as a
formatter alternative and ESLint as an opt-in linter. TypeScript compiler diagnostics belong to
typecheck and derive from authoritative TypeScript project context rather than a reconstructed
compiler invocation.

Capability ownership, source classes, formatter composition, diagnostics, and fixes are defined by
[Quality](../quality/). The frozen tool inventory, acquisition, runtime sharing, and exact curated
defaults are defined by [Tools](../tools/). Those contracts, rather than this ADR, own detailed tool
algorithms and mappings.

Version selection follows [ADR 0012](0012-language-toolchain-versions.md) only where the upstream
rules can configure the complete relevant graph. Unsupported version controls fail rather than
becoming ignored settings or host-tool workarounds.

All foundations are automatically available but operationally lazy. An unused foundation must not
activate configured application targets or actions, resolve application packages, register usable
toolchain payloads, create environment projections, or download compiler/runtime/package payloads.
The required platform, consumer, remote, compatibility, and laziness evidence is authoritative under
[Testing](../testing/); this ADR alone makes no support claim.

## Consequences

- Rust is the first application and first concrete generation foundation; Python and
  JavaScript/TypeScript are later foundations.
- Consumers receive concise stable defaults while mature upstream projects retain compiler,
  package-manager, test-runner, provider, and IDE semantics.
- `rules_dx` owns wrapper compatibility and first-party BUILD generation, but not parallel language
  build or dependency engines.
- Native crate ownership and per-source JavaScript/TypeScript ownership remain distinct where their
  language semantics differ.
- Cargo and pnpm locks are authoritative only together with target scope; source references cannot
  activate dependencies.
- Rust and Node environments compose through shared managed state while preserving ecosystem-native
  layouts and authoritative upstream graphs.
- Quality tools remain capability-oriented and lazy rather than expanding application wrapper APIs.
- Upstream gaps require remediation through focused, tested, pinned patches, bounded integration, or
  an explicitly reviewed and approved alternative upstream. Unresolved gaps block conformance and
  support claims, not remediation; they do not authorize private graph coupling or a project-owned
  replacement engine.

## Rejected Alternatives

- Implementing Rust, Cargo, Node, pnpm, TypeScript, or Jest engines inside `rules_dx`.
- Re-exporting complete upstream rulesets as stable `rules_dx` APIs.
- Treating lint and format integration as complete application-language support.
- Claiming one identical multi-version or environment implementation across ecosystems despite
  different upstream capabilities.
- Building a universal language model or extracting cross-language helpers before later foundations
  prove actual reuse.
- Reconstructing crate graphs, rust-analyzer state, package stores, TypeScript compiler inputs, or
  lockfile semantics from private data or filesystem scans.
- Treating framework containers as ordinary JavaScript/TypeScript or parsing all frameworks through
  one generic adapter.
- Eagerly activating or downloading every automatically available language foundation.
