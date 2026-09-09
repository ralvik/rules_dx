# ADR 0014: Tested Platform Release Stack

## Status

Accepted.

## Context

Consumers want language foundations and developer tools to work together without selecting and
reconciling versions of every Bazel ruleset, compiler, runtime, package manager, and quality
tool. Requiring an explicit language list also makes adding a supported language a multi-step
platform configuration task. Making all foundations eager would instead download and analyze
large unused toolchains.

Bzlmod resolves all declared module dependencies before lazily evaluating module extensions.
The root module may select newer compatible transitive module versions or apply overrides, so a
dependency cannot force one global graph on its consumers.

## Decision

### Required platforms

This table is the single source for the v1 required-host set. All other
documents link here instead of restating it. Exact pins, hosts, floors, SDK/CRT
identities, and qualified routes remain owned by [O14 and O37](../open-decisions.md);
nothing below pins a version.

| Platform | V1 status | Notes |
| --- | --- | --- |
| Linux x86_64 glibc | Required, primary bootstrap | First host; release CI must cover it |
| Linux arm64 glibc | Required | Native workflow, not cross-only |
| Linux x86_64/arm64 static musl | Required profiles | Dynamic musl is not an initial requirement |
| macOS arm64 | Required | Pinned acquired SDK; SDK version is not the deployment floor |
| macOS x86_64 | Best-effort | Qualify when a host is available; record gaps without blocking required-host release |
| Windows x86_64 MSVC-compatible | Required, backend blocked | Hermetic acquisition + MSVC compatibility gates unchanged |
| Windows arm64 | Out of v1 scope | Not a claim of impossibility |
| Other cross-build routes | Optional expansion | Only after the initial cohort passes; see the provisional [native plan](../native-toolchains.md) |

Linux-first means incremental host bring-up order (M00 starts Linux-first,
local-only), not a scope reduction: v1 retains full required-host coverage.

Each `rules_dx` release defines one tested default platform stack. The release pins the exact
Bazel version, direct Bazel ruleset versions, managed compiler/runtime defaults, package-manager
tools, test integrations, quality tools, and internal Rust dependencies used by release tests.
A checked-in generated manifest is the source of truth for that stack and feeds release notes,
documentation, update automation, and compatibility tests.

The Windows native-toolchain baseline requires MSVC-compatible C/C++ and Rust native-dependency
workflows, including qualified interoperability with prebuilt MSVC C++ libraries. It must not
require all native dependencies to be rebuilt for GNU/MinGW. This is not a promise of compatibility
with every vendor library or arbitrary STL/CRT combination. Different platforms may use different
native-toolchain backends behind the consistent user-facing rules; one compiler distribution across
all platforms is not required.

The preferred native stack uses `rules_cc` with the Rust foundation in
[ADR 0013](0013-rust-javascript-typescript-foundations.md), `hermetic-llvm` on Linux/macOS, and a
separately qualified hermetic MSVC-compatible backend on Windows. Windows retains the complete
hermetic acquisition requirement: no separately installed Build Tools or host SDK prerequisite or
fallback is approved. Prefer existing upstream acquisition and toolchain rules over project-owned
compiler/SDK infrastructure. The provisional
[native qualification plan](../native-toolchains.md) selects the first backend/compiler candidates
and cross-build cohort from upstream evidence. Final backend adoption, exact compiler/SDK/STL/CRT
identities, licensing, and public mappings remain gated by
[Open Decisions](../open-decisions.md). If no existing route satisfies these requirements with
bounded integration, report the blocker rather than silently relaxing acquisition or compatibility.
No pin, host, floor, or acquisition identity in the provisional native plan is normative
through this record; O14/O37 own those values.
Prefer a published stable Windows-backend release; a necessary qualified upstream commit pin is
permitted under the narrow exception in [ADR 0008](0008-dependency-currency.md#decision).

The Windows consumer workflow may require explicit Microsoft EULA acceptance through the selected
upstream acquisition mechanism. Document the applicable terms and require deliberate acknowledgement;
do not automatically accept on the consumer's behalf or bypass upstream controls. Acceptance is not
permission to redistribute: applicable usage and distribution rights still require review. Merely
adding the module or using unrelated tools must not require acceptance or acquire restricted native
payloads. The exact mechanism remains open; this setup step does not relax hermetic acquisition,
MSVC compatibility, or qualification requirements.

Linux requires glibc and static-musl profiles on x86_64/arm64. Dynamic musl is not an initial
requirement. Prioritize Linux cross-builds and admit additional routes where upstream integration
keeps maintenance bounded, rather than requiring every host-to-target combination. Required native
platform workflows remain unchanged; cross-building and target execution require separate evidence.
The [feasibility review](../product/support-matrix.md#native-toolchain-alternatives) records the
alternatives and gaps. This selects a design direction, not exact release pins, complete C/C++
foundation admission, verified support, or an implementation milestone.

The normal consumer adds only the `rules_dx` module dependency. Every supported application
foundation is available automatically; there is no language enable list. Foundations remain
strictly lazy: an unused language may add modules to Bzlmod version resolution, but it must add no
configured targets, actions, module-extension dependency resolution, registered usable toolchain
payload, environment projection, compiler/runtime/package download, or application dependency
fetch.

Automatic availability is through stable `rules_dx` wrappers, providers, aspects, and generated
targets. Bzlmod does not make a dependency's transitive modules directly visible to a consumer;
users who intentionally load an upstream ruleset API add that module explicitly and leave the
normal one-dependency surface. `rules_dx` must not require consumers to add upstream modules for
its documented wrappers or generated workflows.

The tested stack includes the first-party Gazelle extensions and their authoritative metadata
adapters defined by ADR 0015. Future foundations reuse upstream generation only after uniform-
output conformance; functional gaps or incompatible output require a first-party extension without
moving ecosystem resolution or build semantics into `rules_dx`.

`dx generate` is always an available explicit generation workflow once the CLI is installed,
with a repository-wide default and explicit v1 scoped selection per
[ADR 0006](0006-cli-command-surface.md). Generation semantics — discovery, strict resolution,
merge behavior, naming, and manifest transport — are owned by
[ADR 0015](0015-first-party-gazelle-extensions.md) and the [generation contracts](../generation/common.md);
this record adds no separate generation rule. Source discovery must not invent external dependencies, compiler
versions, package-manager settings, or other project metadata. Strict resolution, unknown-import
failure behavior, and manifest authority follow [ADR 0015](0015-first-party-gazelle-extensions.md)
and the [generation contracts](../generation/common.md) without restatement here.

Writing a BUILD declaration is not creation or analysis of a Bazel configured target. Generation
does not build or test the discovered sources, select application dependency versions, register a
usable toolchain payload, prepare an environment or IDE projection, or execute application code
generators. It may materialize already locked package artifacts in a declared Bazel preparation
step only when their contents are required to derive dependency metadata under ADR 0015. Declared
targets and authoritative providers activate later application behavior when their separate
workflows analyze them. Semantic source classes independently activate lazy quality policy.

The tested stack is a default compatibility guarantee, not a root-module lock. Normal Bzlmod
selection and root overrides remain allowed. `rules_dx` does not inspect, warn about, or reject a
resolved graph merely because it differs from the tested defaults. Such a graph is outside the
tested-stack guarantee unless release CI covers it, but ordinary commands do not emit a warning.
Actual provider, API, toolchain, or execution incompatibility still fails at its normal Bazel
boundary with an actionable error.

Application dependency versions remain project-owned through authoritative lockfiles such as
`uv.lock`, `pnpm-lock.yaml`, `Cargo.lock`, and `go.sum`. The platform stack pins the integration,
runtime, and resolver versions; it does not replace ecosystem dependency resolution or introduce
a `rules_dx` application lockfile.

The [Support Matrix](../product/support-matrix.md) is the authority for current support claims.
Every language or file family states
whether it has build, test, dependency, generation, environment, IDE, coverage, and quality
support. A cell becomes supported only after its documented platform and consumer tests pass.
Incomplete foundations remain explicitly quality-only or planned rather than inheriting an
umbrella claim from the release stack.

## Consequences

- One `rules_dx` version is the normal user-facing platform version.
- Maintainers own compatibility across the complete default module graph and every supported
  foundation.
- Unused foundations have some module-resolution and ruleset-source metadata cost, but no heavy
  tool or application payload cost.
- Adding a foundation requires cold/warm module-overhead measurements, zero-unused-work proofs,
  and updates to the [Support Matrix](../product/support-matrix.md).
- Root overrides need no special CLI policy or warning path.
- Project lockfiles continue to select application libraries and packages.
- Support claims are granular and evidence-backed rather than inferred from tool availability.

## Rejected Alternatives

- Requiring consumers to maintain a language list in both module and BUILD configuration.
- Requiring a language list in `MODULE.bazel` as the normal activation mechanism.
- Eagerly downloading or registering every compiler/runtime payload.
- Forcing root-level version overrides or generating a second platform lockfile.
- Warning on every resolved module graph that differs from release defaults.
- Claiming complete language support from formatter or linter availability alone.
