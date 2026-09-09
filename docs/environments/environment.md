# Developer Environments

## Scope

`dx env` is the language-neutral interface for persistent developer and IDE
environments. Language integrations derive contents from Bazel-owned rules and
providers; the CLI resolves selection and invokes Bazel but does not inspect
dependency manifests, run package managers directly, or infer language contents.

The PATH/environment bootstrap is delivered before language-native environments. Rust is the first
language-native projection used by the repository; Python's `.venv` is a later integration rather
than the generic or first environment model. Shared on-disk state is defined in
[Managed Environment State](managed-state.md).

With no argument, `dx env` invokes canonical `//dx:env`, refreshes `.dx/bin`, and
selects repository defaults for every application integration represented by authoritative
repository manifests and targets. Before `dx` is available,
the equivalent bootstrap is:

```sh
bazel run //dx:env
```

`dx env <target>` accepts exactly one explicit target label. It builds the target's
environment inputs and selects exact native projections for every supported application
integration represented in the target's analyzed transitive closure. It requires no generated or
handwritten sibling environment target.

Paths, directories, target patterns, multiple labels, profiles, and language
selectors are not accepted. Neither mode launches an interactive shell. Arguments
after `--` are Bazel command options placed before workflow targets, not application
arguments for environment tools.

## Language Composition

Each approved language owns its canonical storage, native layout, and conventional
facades. Python may expose `.venv`; Node, Rust, and future integrations retain their
ecosystem-native shapes.

A target-scoped invocation updates only integrations represented in the selected target closure.
Previously selected integrations absent from that closure preserve their
currently selected environment. A target with no supported language environment is
a successful no-op for language facades and does not change `.dx/bin`. No-argument
`dx env` restores repository defaults for every integration represented in the repository graph.

`dx env` does not build or select generated-source projections. It reads current
codegen selection metadata only to report a scope mismatch and identify `codegen` plus
the required scope kind without rendering argv. The environment operation still succeeds,
and it never invokes codegen implicitly.

`dx setup [target]` is the explicit combined workflow. It prepares environment and
codegen generations for the same root or exact-target scope and commits both only after
both validate successfully. It does not invoke `dx generate` or update tracked Bazel
metadata.

Setup uses one Bazel build request rather than running the env and codegen commands
sequentially. The request selects the union of roots needed by both workflows, applies
both provider-collection aspects, requests both private capability output groups, and emits
one BEP stream. Each aspect remains provider-selective, while Bazel shares package
loading, configured-target analysis, dependency closures, and common actions. The CLI
separates the returned plans only after Bazel completes, then prepares both generations
before the shared commit.

Setup is capability-aware. If the selected exact target lacks environment or codegen capability,
its absent side follows the shared carry-forward or first-selection empty-counterpart rules in
[Selection And Carry-Forward](managed-state.md#selection-and-carry-forward).

Language and LSP configuration refers to `.dx/setups/current/generated`, not to a codegen
generation captured by the environment identity. An explicit codegen refresh therefore
becomes visible without rebuilding or reselecting the environment.

Persistent language environments have no public contribution or invocation protocol in
v1. Implementations remain language-specific and directly consume their authoritative
rules and verified provider data; v1 supports end-user workflows, not third-party
environment plugins. See [O35](../open-decisions.md). They use provider-derived filesystem links for sources,
dependencies, and toolchains. Generated outputs use the independent symlink-only codegen projection.
Copying is not a fallback.

Collection prefers complete authoritative transitive upstream providers. If an
upstream ruleset does not expose enough metadata, a narrow tested ruleset adapter maps
only its semantic dependency edges into the private environment-plan provider.
Unsupported rule kinds fail closed. Generic aspects never traverse a broad union of
attributes such as `deps`, `runtime_deps`, `data`, and tool edges, avoiding unnecessary
configured-aspect work and accidental environment contents.

For a mixed-language target, every represented integration contributes to one complete environment
plan and must prepare and validate successfully before selection. The immutable layout, complete
identity, current pointer, carry-forward, empty counterparts, reuse checks, commit lock, retention,
and unmanaged-path policy are defined once in [Managed Environment State](managed-state.md).

## Repository And Target Roots

No-argument `dx env` runs canonical `//dx:env`. Gazelle maintains that rule's private
tool bootstrap, while repository-wide language roots are collected from the current
declared BUILD graph through the same
provisional root-selection strategy as codegen. The correctness baseline applies
language-specific plan aspects to `//...`; the leading optimization candidate passes a
temporary Bazel-query-produced target-pattern file back to Bazel. Each contributing
configured target emits one normalized binary Protobuf environment-plan shard. A private
environment provider collects shards and required artifacts in transitive depsets, and
one private environment output group exposes both through BEP. Shards use a reserved
controlled filename suffix; Rust deterministically merges them and verifies that every
referenced execution path is present in the same BEP-reported output group. Language
adapters partition the merged plan by runtime, ABI, dependency universe, and other
integration-specific identity. `EnvironmentInfo` remains PATH-tool-only and is not this
internal plan contract.

Symlink-only native environments require every referenced Bazel artifact to be present
locally before commit. Under remote cache or execution, the request downloads the private
environment output group while leaving unrelated build outputs eligible to remain
remote-only. Artifact downloading remains Bazel-owned; the CLI does not fetch remote
cache objects itself.

`dx env <target>` bypasses repository root selection. A provider-collecting aspect analyzes
only the requested configured target and its transitive closure, then the language
integrations prepare the exact projections described by those providers. Exact-target
correctness therefore does not depend on a recent Gazelle run.

Setup includes only targets currently declared in BUILD files. Unowned files or missing
wrappers are ignored without a freshness error; users run `dx generate` separately when
they want Gazelle to update that graph. A query-produced target list is expected to
reduce discovery only; it does not avoid loading and analyzing selected roots.
Repository-wide candidates and warm behavior require the benchmark evidence in
[Generated Code](codegen.md) before one becomes normative. Among equivalent-semantics
candidates the fastest wins on both cold and warm, with warm weighted above cold for
internal paths, per [O34](../open-decisions.md).

## Tool Exposure

The stable PATH-facing location is `<workspace>/.dx/bin`. It exposes the
module-matched `dx`, user-facing tools from represented application integrations, and quality
tools selected by default or overridden policy-family configuration, including language
executables, formatters, linters, type
checkers, auditors, test tools, generators, and package tools.

Unused foundations, removed adapters, unselected alternatives, and internal runner,
protocol, apply, and evaluator executables are not exposed. Selection depends only on
authoritative graph facts plus declared policy; `//dx:env` does not recursively scan sources or
observe command history.

All managed projections follow the shared symlink-only and host-capability policy in
[Installation And Ownership](managed-state.md#installation-and-ownership).

Symlinks refer to current Bazel output artifacts rather than copying runtime closures.
Executable links preserve arguments, working directory, environment, exit status, and
signal behavior.

## Public Tool API

The public PATH-tool API is loaded from `@rules_dx//env:defs.bzl`. The convenience rule is
`environment_tool(name, executable, bin_name)`. Built-in integrations and consumer
repositories use the same validated constructors and `EnvironmentInfo`
collection provider; there is no private first-party contribution path. Third-party
language-integration plugins are deferred past v1 per [O35](../open-decisions.md).

`EnvironmentInfo` carries zero or more PATH-tool records and composes transitively.
Each record contains:

- One platform-neutral logical `bin_name`.
- Optional logical `aliases` for the same executable.
- The executable's Bazel `FilesToRunProvider`.

Each selected quality adapter contributes the same `FilesToRunProvider` and hermetic
runtime/package closure used by its Bazel quality actions. `.dx/bin` does not resolve or
install a separate editor version. Selection is configuration-based rather than source-based,
so a selected tool is present even when no currently declared source target is applicable;
disabled and unselected tools are absent. Exact tool-version parity for editors and direct
commands is guaranteed only through these managed entries and checked-in native configs.
Built-in quality records use conventional upstream `bin_name` values. If selected tools claim
the same host-normalized command name, environment configuration fails and reports all
claimants; no automatic `dx-` prefix, suffix, or precedence rule is applied.

Quality PATH entries invoke raw managed upstream tools rather than wrappers that apply Bazel
action policy. Direct invocations retain upstream argv, working directory, config discovery,
streams, and exit status. Action-only controls such as VCS-ignore isolation, normalized
machine results, sandbox paths, and denied network remain confined to Bazel quality actions.
The shared executable and runtime guarantee version parity only.

Every primary name and alias creates an equivalent PATH entry and participates in
global collision validation. Materialization maps logical names to host-native
filenames, including `.exe` where required. Names must be non-empty single filename
components and reject separators, `.`, `..`, explicit executable suffixes, and
host-reserved names. Collision checks follow host filename semantics, including
case-insensitive comparison on Windows.

Repositories add tools with:

```starlark
environment_tool(
    name = "my_cli_env",
    executable = "//tools:my_cli",
    bin_name = "my-cli",
)
```

The target is listed in `environment_config.tools`. Explicit `bin_name` keeps the
developer command stable when a Bazel label changes.

## Ownership And Refresh

`.dx/bin` is independent managed bootstrap state rather than part of setup selection. Its complete
installation, atomic refresh, ownership marker, unmanaged-path refusal, and shell-configuration
policy are defined in [Managed Environment State](managed-state.md#installation-and-ownership).

Prefer upstream-supported automatic refresh under the
[automatic-workflow policy](../product/scope.md#automatic-workflows). The approved managed Rust
rust-analyzer discovery/flycheck launchers and Go `gopls` package driver may invoke Bazel on editor
requests to obtain current analysis metadata and required artifacts. Use a separate IDE output base
and preserve the selected configuration and exact-target scope; do not mutate the selected environment
or codegen projection, tracked BUILD files, or application manifests/locks.

For Go, reuse rules_go's upstream `GOPACKAGESDRIVER` integration rather than creating a static package
snapshot or replacement package graph. Editor requests may trigger Bazel queries/builds and lazy
acquisition of their declared dependencies. Document that latency and work, propagate failures rather
than reporting a successful incomplete package graph, and qualify cancellation/concurrency and target
isolation. Exact mappings, cgo completion, and required-platform behavior remain O46 gates; this
exception does not admit the complete Go foundation.

`bazel clean`, output-base changes, or removed outputs can invalidate links. An automatic editor
request can run only while its managed executable remains usable; a dangling symlink cannot repair
itself. Ordinary entries do not gain a self-repair wrapper or automatic setup-selection refresh.
Recovery for missing tools and facades remains `dx env` or bootstrap `bazel run //dx:env`.
Other automatic refresh routes require evidence and corresponding domain-contract updates, not a
blanket prohibition or permission to add custom background machinery.

## Direnv Integration

direnv support is optional and follows the `bazel_env` pattern: per-directory automatic `PATH`
management without automatic Bazel execution. POSIX shells use a
committed `.envrc` next to `MODULE.bazel`; Windows uses documented manual `PATH` guidance first.

The committed `.envrc` contains `PATH_add` for `.dx/bin` only, a `watch_file` on `.dx/bin` so
direnv reloads after refresh, and a missing-directory guard that fails with actionable
`dx env` or `bazel run //dx:env` regeneration guidance. It never invokes Bazel from the shell
hook and never exports variables beyond `PATH` in v1, preserving the PATH-tools-only boundary
in [O35](../open-decisions.md). Unlike `bazel_env`, the watched path is the workspace-stable
`.dx/bin` rather than a configuration-specific `bazel-out` directory. `dx` never writes shell
profiles; the `.envrc` file itself is scaffolded absent-only by `dx init` and refused when an
unmanaged `.envrc` already exists. direnv requires its shell hook plus `direnv allow`; without
direnv, export `.dx/bin` manually or from the `dx env` status output, including in CI.

Windows keeps manual `PATH` guidance as the supported path: direnv on Windows depends on a
compatible bash (Git Bash via `bash_path`) plus hook setup, has no `cmd.exe` hook, and still
requires symlink capability for `.dx/bin` with failure before mutation and no junction or copy
fallback. A user-owned PowerShell snippet may be documented later; `dx` writes no profile or
registry state.

## Distribution

The Bazel-first installation path is `bazel run //dx:env`, which exposes the `dx`
version matching the consumer's pinned module. Optional checksummed standalone
binaries are also published for Linux x86_64/arm64, macOS x86_64/arm64, and Windows
x86_64. Standalone installation requires neither a local Rust toolchain nor Bazel.

The documented standalone installation workflow automatically verifies artifact integrity
and authenticity against an approved publisher identity before installing or executing
the downloaded `dx` binary. Invalid or unavailable verification fails installation;
there is no checksum-only fallback or optional verification step. A checksum delivered
alongside the binary is not by itself proof of publisher identity. Signing technology,
trust-root and verifier bootstrap, identity binding, and verification inputs remain
technical qualification work under [O38 and O39](../open-decisions.md), without adding
a Bazel or Rust prerequisite to standalone installation.

The approved v1 destinations are the Bazel Central Registry for the `rules_dx` module
and GitHub Releases for standalone `dx` binaries. This selects destinations only, not
credentials, permissions, registry submission procedures, or the publication sequence;
those remain under [O45](../open-decisions.md). Publication still requires explicit
milestone approval and qualified release artifacts.

Release hosting, signing, and verification services must satisfy the
[free-infrastructure constraint](../testing/README.md#infrastructure-budget) without
weakening artifact verification or required host coverage.

Starting with v1, the module and CLI share Semantic Versioning 2.0.0 release versions
(`MAJOR.MINOR.PATCH`). Incompatible changes to documented public APIs, including Starlark
symbols/providers and CLI commands, flags, and behavior, require a major release.
Backward-compatible public additions and deprecations require a minor release;
backward-compatible bug fixes use a patch release. Internal implementation details are
outside this compatibility promise; see the [public API boundary](../architecture/README.md#public-and-private-apis).
Native tool diagnostics and formatting may evolve under the existing
[tool-update policy](../quality/tool-integrations.md#common-adapter-contract), without making
every upstream finding or formatting change a public API break. Protocol schema versions
retain their own documented compatibility rules; release versioning does not replace them.

Maintenance targets only the latest release line. Older release lines receive no
backports, including security or critical fixes. Consumers may retain older pins,
but must upgrade to the latest maintained release to receive fixes. This maintenance
policy does not change the [CLI/module compatibility contract](../cli/cli-contract.md#workspace-discovery).

## Test Requirements

Environment tests must cover:

- Bootstrap on every required host and exact CLI/module version matching.
- Argument, working-directory, environment, exit-status, and signal fidelity.
- Paths with spaces, native executable names, Windows case-folding, and reserved
  names.
- Stale behavior after `bazel clean` and output-base changes without implicit Bazel
  startup.
- Rust dynamic discovery and flycheck Bazel startup in a separate IDE output base,
  package-scoped repository behavior, exact-target constraints, and no environment or
  codegen selection mutation.
- Go upstream package-driver Bazel startup on editor requests, separate IDE output-base/configuration
  isolation, exact-target constraints, failure propagation, and no environment/codegen selection or
  tracked-file mutation; record cgo and platform gaps rather than claiming generic IDE parity.
- Configured tool inclusion, inactive/private tool exclusion, transitive provider
  composition, aliases, runfiles, and collision diagnostics.
- First-party and consumer-repository tool contributions with identical behavior.
- Mixed-language preparation and facade-commit failures without partial selection,
  absent-language preservation, no-capability no-op behavior, and restoration of all
  defaults by no-argument `dx env`.
- One combined Bazel setup plan with shared roots/analysis, two provider-selective
  aspects and output groups, and one BEP stream.
- Managed root `node_modules` projection from Bazel/pnpm-selected artifacts without
  per-package workspace mutation or a second dependency resolution.
- Stable managed importer-local `node_modules` facades backed by rules_js-generated
  importer targets and one shared package store.
- Equivalent `//...` and query-produced repository-root candidates, exact-target
  bypass, declared-BUILD-only behavior, and measured cold/warm incrementality.

Language-specific environment tests live with each language environment contract.
Rust is the first language-native integration; see the
[Rust foundation](../decisions/0013-rust-javascript-typescript-foundations.md) for its accepted
behavior. [Python Environment](python-environment.md) defines the later concrete `.venv` projection.
Generated-source ownership and tests are defined in [Generated Code](codegen.md).
Shared state, pointer, reuse, concurrency, symlink, ownership, and retention tests are defined in
[Managed Environment State](managed-state.md#test-requirements).
Cross-cutting fixture, platform, and evidence rules remain in
[Testing Strategy](../testing/README.md).
