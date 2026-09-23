# Developer Environments

`dx env` is the language-neutral interface for persistent developer and IDE
environments. Language integrations derive contents from Bazel-owned rules
and providers; the CLI resolves selection and invokes Bazel but does not
inspect dependency manifests, run package managers directly, or infer
language contents. Open work is tracked in GitHub issues.

## Scope

With no argument, `dx env` invokes canonical `//dx:env`, refreshes `.dx/bin`,
and selects repository defaults for every application integration represented
by authoritative repository manifests and targets. Before `dx` is available,
the equivalent bootstrap is:

```sh
bazel run //dx:env
```

`dx env <target>` accepts exactly one explicit target label. It builds the
target's environment inputs and selects exact native projections for every
supported application integration in the target's analyzed transitive
closure. It requires no generated or handwritten sibling environment target.

Paths, directories, target patterns, multiple labels, profiles, and language
selectors are not accepted. Neither mode launches an interactive shell.
Arguments after `--` are Bazel command options placed before workflow
targets, not application arguments for environment tools.

## Language Composition

Each approved language owns its canonical storage, native layout, and
conventional facades. Python may expose `.venv`; Node and Rust retain their
ecosystem-native shapes.

A target-scoped invocation updates only integrations represented in the
selected target closure. Previously selected integrations absent from that
closure keep their currently selected environment. A target with no supported
language environment is a successful no-op for language facades and does not
change `.dx/bin`. No-argument `dx env` restores repository defaults for every
integration in the repository graph.

`dx env` does not build or select generated-source projections. It reads
current codegen selection metadata only to report a scope mismatch. The
environment operation still succeeds and never invokes codegen implicitly.

`dx setup [target]` is the explicit combined workflow. It prepares
environment and codegen generations for the same root or exact-target scope
and commits both only after both validate successfully. It does not invoke
`dx generate` or update tracked Bazel metadata. Setup uses one Bazel build
request for both workflows rather than running env and codegen sequentially.

Language and LSP configuration refers to `.dx/setups/current/generated`, not
to a codegen generation captured by the environment identity. A codegen
refresh therefore becomes visible without rebuilding the environment.

There is no public contribution or invocation protocol for persistent
language environments in v1. Implementations remain language-specific and
directly consume their authoritative rules and verified provider data. They
use provider-derived filesystem links for sources, dependencies, and
toolchains. Generated outputs use the independent symlink-only codegen
projection. Copying is not a fallback.

For a mixed-language target, every represented integration contributes to one
complete environment plan and must prepare and validate successfully before
selection. The immutable layout, identity, pointer, carry-forward, reuse,
commit lock, retention, and unmanaged-path policy are defined once in
[Managed Environment State](managed-state.md).

## Repository And Target Roots

No-argument `dx env` runs canonical `//dx:env`. Repository-wide language
roots are collected from the current declared BUILD graph through the frozen
`//...` root-selection baseline. Each contributing configured target emits
one normalized binary Protobuf environment-plan shard; a private provider
collects shards and required artifacts, exposed through one private output
group. Language adapters partition the merged plan by runtime, ABI,
dependency universe, and other integration-specific identity.

Symlink-only native environments require every referenced Bazel artifact to
be present locally before commit. Under remote cache or execution, the
request downloads the private environment output group. Artifact downloading
remains Bazel-owned.

`dx env <target>` bypasses repository root selection and analyzes only the
requested configured target and its transitive closure. Exact-target
correctness therefore does not depend on a recent Gazelle run.

Setup includes only targets currently declared in BUILD files. Unowned files
or missing wrappers are ignored without a freshness error; users run
`dx generate` separately when they want Gazelle to update that graph.

## Tool Exposure

The stable PATH-facing location is `<workspace>/.dx/bin`. It exposes the
module-matched `dx`, user-facing tools from represented application
integrations, and quality tools selected by default or overridden
policy-family configuration.

Unused foundations, removed adapters, unselected alternatives, and internal
runner, protocol, apply, and evaluator executables are not exposed. Selection
depends only on authoritative graph facts plus declared policy.

The repository-default environment contains exactly the `doctor` probe, the
module-matched `dx` CLI, and the user-facing `quality_markdown` binary.
Internal pipeline binaries are not members.

All managed projections follow the shared symlink-only and host-capability
policy in [Installation And Ownership](managed-state.md#installation-and-ownership).
Symlinks refer to current Bazel output artifacts rather than copying runtime
closures. Executable links preserve arguments, working directory,
environment, exit status, and signal behavior.

## Public Tool API

The public PATH-tool API is loaded from `@rules_dx//env:defs.bzl`. The
convenience rule is `environment_tool(name, executable, bin_name)`. Built-in
integrations and consumer repositories use the same validated constructors
and `EnvironmentInfo` collection provider; there is no private first-party
contribution path.

There is no v1 persistent-environment plugin model and no `EnvironmentInfo`
extension point: `EnvironmentInfo` stays PATH-tool-only.

`EnvironmentInfo` carries zero or more PATH-tool records and composes
transitively. Each record contains one platform-neutral logical `bin_name`,
optional logical `aliases`, and the executable's Bazel `FilesToRunProvider`.

Each selected quality adapter contributes the same executable and hermetic
runtime/package closure used by its Bazel quality actions. A selected tool is
present even when no currently declared source target is applicable;
disabled and unselected tools are absent. If selected tools claim the same
host-normalized command name, configuration fails and reports all claimants;
no automatic prefix, suffix, or precedence rule is applied.

Quality PATH entries invoke raw managed upstream tools rather than wrappers
that apply Bazel action policy. Direct invocations retain upstream argv,
working directory, config discovery, streams, and exit status.

Every primary name and alias creates an equivalent PATH entry and
participates in global collision validation. Names must be non-empty single
filename components and reject separators, `.`, `..`, explicit executable
suffixes, and host-reserved names.

Repositories add tools with:

```starlark
environment_tool(
    name = "my_cli_env",
    executable = "//tools:my_cli",
    bin_name = "my-cli",
)
```

The target is listed in `environment_config.tools`. Explicit `bin_name` keeps
the developer command stable when a Bazel label changes.

## Ownership And Refresh

`.dx/bin` is independent managed bootstrap state rather than part of setup
selection. Its installation, atomic refresh, ownership marker,
unmanaged-path refusal, and shell-configuration policy are defined in
[Managed Environment State](managed-state.md#installation-and-ownership).

Prefer upstream-supported automatic refresh under the
[automatic-workflow policy](../product/scope.md#automatic-workflows). Approved
managed launchers may invoke Bazel on editor requests to obtain current
analysis metadata and required artifacts under a separate IDE output base,
without mutating the selected environment or codegen projection, tracked
BUILD files, or application manifests/locks.

For Go, reuse the upstream `GOPACKAGESDRIVER` integration rather than a
static package snapshot. Editor requests may trigger Bazel queries/builds;
propagate failures rather than reporting a successful incomplete graph. The
supported boundary is pure-Go packages; cgo completion is the explicit
exception (issue #789) with gaps recorded rather than generic IDE parity claimed.

`bazel clean`, output-base changes, or removed outputs can invalidate links.
Recovery for missing tools and facades remains `dx env` or bootstrap
`bazel run //dx:env`.

## Direnv Integration

direnv support is optional and follows the `bazel_env` pattern: per-directory
automatic `PATH` management without automatic Bazel execution. POSIX shells
use a committed `.envrc` next to `MODULE.bazel`; Windows uses documented
manual `PATH` guidance first.

The committed `.envrc` contains `PATH_add` for `.dx/bin` only, a
`watch_file` on `.dx/bin`, and a missing-directory guard with `dx env`
regeneration guidance. It never invokes Bazel from the shell hook and never
exports variables beyond `PATH` in v1. `dx` never writes shell profiles; the
`.envrc` file itself is scaffolded absent-only by `dx init` and refused when
an unmanaged `.envrc` already exists.

Windows keeps manual `PATH` guidance as the supported path: direnv on Windows
needs a compatible bash plus hook setup, has no `cmd.exe` hook, and still
requires symlink capability with failure before mutation and no junction or
copy fallback.

## Distribution

The Bazel-first installation path is `bazel run //dx:env`, which exposes the
`dx` version matching the consumer's pinned module. Standalone installation
requires neither a local Rust toolchain nor Bazel. Until releases are cut
with owner approval the Bazel-first path above is the supported installation.
There is no checksum-only fallback: a checksum delivered alongside a binary
is not by itself proof of publisher identity. Install-time
publisher-identity verification runs before install or exec. Airgapped hosts
install the same artifacts from the vendored bundle; see
[Offline Bootstrap](../deploy/offline-bootstrap.md).

The approved v1 destinations are the Bazel Central Registry for the
`rules_dx` module and GitHub Releases for standalone `dx` binaries.
Publication mechanics are implemented owner-gated dry-run-first per the
[release runbook](../deploy/release-runbook.md). Publication still requires
explicit approval and qualified release artifacts.

Publication dry-run dispatches `.github/workflows/publish-dry-run.yml`
manually from the Actions tab. It builds the seed-host binary and archive,
records digests, generates SBOM and provenance, exercises draft and signing
demos in dry-run mode, and checks the module shape without submitting. The
workflow needs only `contents: read` and stores no secrets.

The `github_deploy` rule defaults to `draft = True` with a placeholder tag
and always passes `--draft --verify-tag`, so the program never creates or
pushes tags itself.

Release hosting, signing, and verification services must satisfy the
[free-infrastructure constraint](../testing/strategy-details.md#infrastructure-budget)
without weakening artifact verification or required host coverage.

Starting with v1, the module and CLI share Semantic Versioning 2.0.0 release
versions (`MAJOR.MINOR.PATCH`). Incompatible changes to documented public
APIs require a major release; backward-compatible additions and deprecations
require a minor release; bug fixes use a patch release. Internal
implementation details are outside this compatibility promise.

Maintenance targets only the latest release line. Older release lines receive
no backports, including security or critical fixes.

## Test Requirements

Environment tests must cover bootstrap on every required host with exact
CLI/module version matching; argument, working-directory, environment,
exit-status, and signal fidelity; paths with spaces, native names, Windows
case-folding, and reserved names; stale behavior after `bazel clean` without
implicit Bazel startup; Rust and Go editor-driver Bazel startup in a separate
IDE output base with exact-target constraints and no selection mutation;
configured-tool inclusion with inactive/private-tool exclusion, transitive
composition, aliases, runfiles, and collision diagnostics; first-party and
consumer tool contributions; mixed-language preparation without partial
selection; one combined Bazel setup plan with shared roots and one BEP
stream; managed `node_modules` projection without workspace mutation;
equivalent `//...` and exact-target root behavior with declared-BUILD-only
evaluation.

Language-specific environment tests live with each language environment
contract. Generated-source ownership and tests are defined in
[Generated Code](codegen.md). Shared state, pointer, reuse, concurrency,
symlink, ownership, and retention tests are defined in
[Managed Environment State](managed-state.md#test-requirements).
Cross-cutting fixture, platform, and evidence rules remain in
[Testing Strategy](../testing/README.md).
