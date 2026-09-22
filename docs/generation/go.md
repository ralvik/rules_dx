# Go Generation Contract

Go follows the [common contract](common.md). Admitted to v1 by
[ADR 0019](../decisions/0019-first-release-additional-foundations.md) on the
provisional `rules_go` upstream; no switch is approved here.

Wrappers in `go/rules/defs.bzl` preserve `GoInfo`/`GoArchive` and add
`QualitySourcesInfo`. The extension lives in `gazelle/go/` with
parser/naming/lang fixtures plus focused tests. Hello builds in
`go/tests/fixtures/hello/` consume the wrappers.

## Sources And Ownership

One directory holds one Go package plus its external `*_test` package. The
extension generates at most one reusable `go_library` named after the
directory basename with sorted non-test `.go` sources, plus at most one
package-level `go_test` named `<library>_test` owning sorted `*_test.go`
files via `embed`. Exact package, `TestMain`, helper, and naming rules live
in `gazelle/go/naming.go` and `gazelle/go/lang.go`, pinned by their fixtures.

This is the narrow [common](common.md#ownership-and-naming) Go exception:
native package-level test targets are preserved instead of per-file test
targets. Thin `go_binary` entries are never inferred; a directory mixing a
library package with `package main` fails so the owner splits it first.

## Constraints And Resolution

Declared Go build constraints and platform source selection keep upstream
Go/Gazelle semantics: every source is included and the pinned toolchain
selects per platform. Generation never emits `select()` from source flow
and never interprets runtime conditions. See
[common](common.md#resolution) for the constraint boundary.

Standard-library roots filter via `IsStdLib`. Every other literal identity
uses strict common resolution over the local index, `go.mod`/`go.sum`
scope, and user-authored mappings or exact `dx_ignore_import` exceptions.
Generated rules carry `srcs` plus `importpath` only; cgo/race scope
attributes stay handwritten. Hello stays stdlib-only.

## Evidence

Accepted. Pinned by `bazel run //tools/ci:foundation_maps` with the
[test matrix](../testing/generation.md#generate-command-and-gazelle-extensions).
`go test` runner mapping is qualified seed-only via
`bazel run //tools/ci:gotest_qualification`; Go deps via
`bazel run //tools/ci:godeps_qualification`. No `Supported` claim until
platform plus consumer plus release evidence passes per the
[support matrix](../product/support-matrix.md).
