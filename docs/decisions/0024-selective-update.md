# ADR 0024: Selective `dx update` Per-Set Support

## Status

Accepted.

## Context

`dx update` plans per dependency set through the approved Bazel
integrations ([Update command](../cli/commands/audit-update-bazel.md#dx-update)).
`cli/update/src/backend.rs` reports selective (`set:package`) updates as
`Unsupported` for Cargo, Maven, NuGet, and Go in V1 (whole-lock repin
only), while npm delegates package lists to the Bazel-pinned pnpm.
`cli/cli/src/exec/bump.rs` widens exactly one declared requirement and
leaves the resolver-owned follow-up (`dx update <set>` for Cargo/npm/Go)
manual. No open tracker owned the per-set selective-versus-full decision;
see issue #583.

## Decision

Selective support is per set, pinned by fixtures in
`cli/update/tests/fixtures/selective_update/` and qualified by
`bazel run //tools/ci:selective_update_qualification`:

- npm `SUPPORTED`: `bazel run @pnpm//:pnpm -- update [<pkg>...]`
  delegates the package list to the upstream pnpm updater. Transitive
  changes stay permitted under pnpm resolver semantics; package selection
  is not a single-entry guarantee.
- Cargo `WONT-FIX` in V1: `crate_universe` repin
  (`CARGO_BAZEL_REPIN=1 bazel build //rust/tests/fixtures/hello:hello`)
  refreshes the whole Cargo lock. There is no per-crate repin flag in the
  approved integration, so `cargo:<crate>` fails closed with
  `unsupported selective update` plus the `dx update cargo` hint.
- Maven `WONT-FIX` in V1: `rules_jvm_external` pin
  (`REPIN=1 bazel run @maven//:pin`) refreshes `maven_install.json` from
  the exact `MODULE.bazel` pins. There is no per-artifact pin target, so
  `maven:<group:artifact>` fails closed with the `dx update maven` hint.
- NuGet `WONT-FIX` in V1: `paket2bazel` regeneration refreshes
  `third_party/dotnet/deps` from `paket.dependencies`/`paket.lock`
  (exact pins stay). There is no per-id regeneration, so
  `nuget:<id>` fails closed with the `dx update nuget` hint.
- Go `WONT-FIX` in V1: the main workspace has no `go.mod` by design
  and the pinned `go_deps.from_file` module lock tracks Gazelle, so the
  full set is an intentional no-op success with no launch. There is no
  per-module update flag in the approved integration, so
  `go:<module-path>` (e.g. `go:github.com/google/go-cmp/cmp`) fails
  closed with `unsupported` plus the `dx bump gomod:<module> <version>`
  hint (issue #636).

A selective request for a wont-fix set never silently substitutes a full
update; execution reports `unsupported` per set and the invocation exits
`1` with `update_failed` while independent sets still continue. When a
set is both fully and package selected, the full update wins (it includes
the packages). The `dx bump` follow-up stays manual and resolver-owned:
`dx update cargo` (full) after a Cargo widen, `dx update npm:<pkg>` or
`dx update npm` (selective permitted) after an npm widen,
`dx update go` after a Go widen, `dx update maven` (whole-lock pin) after
a Maven widen, and `dx update nuget` (whole-folder regen) after a NuGet
widen. Update-only; no lock format change.

## Consequences

- `dx_update::backend` keeps exactly one selective executor (npm) and
  four `Unsupported` arms; adding a selective backend needs a successor
  record with its upstream per-package operation plus fixture evidence.
- Selector resolution keeps package-only sets as `Packages` (sorted,
  deduplicated); the backend owns the supported-versus-unsupported call.
- The command docs carry the per-set table; this record owns the rationale.

## Rejected Alternatives

- Silent full-update substitution for unsupported selective: rejected,
  it would claim an unrequested scope and hide the integration limit.
- Private per-package emulation (custom `cargo update -p`, hand-edited
  `maven_install.json`, direct `paket.lock` surgery, ad-hoc `go get`):
  rejected, backends stay resolver-owned through the approved Bazel
  integrations with no private resolver.
