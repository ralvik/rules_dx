# C# Generation Contract

C# follows the [common contract](common.md). Admitted to v1 by
[ADR 0019](../decisions/0019-first-release-additional-foundations.md) on the
provisional `rules_dotnet` upstream; no switch is approved here.

Wrappers in `csharp/rules/defs.bzl` preserve the upstream .NET assembly
providers and add `QualitySourcesInfo`. The extension lives in
`gazelle/csharp/` with parser/naming/lang fixtures plus focused tests.
Hello builds in `csharp/tests/fixtures/hello/` consume the wrappers.

Wrappers default to single-TFM `net10.0`; multi-pivot consumers declare
one `csharp_library` per TFM and aggregate per-pivot Roslyn SARIFs in
deterministic pivot order, proven by `csharp/tests/fixtures/roslyn/`.

## Sources And Ownership

One directory holds one reusable `csharp_library` named after the directory
basename with sorted non-test `.cs` sources. `*Test.cs` files are never
library sources; handwritten `csharp_test` owns them. Thin `csharp_binary`
entries are never inferred, and a directory mixing a library with a
`main`-defining source fails so the owner splits it first. Exact naming,
test, and main rules live in `gazelle/csharp/naming.go` and
`gazelle/csharp/parser.go`, pinned by their fixtures.

## Resolution

Framework roots filter via `IsStdLib`. Every other literal identity uses
strict common resolution over the local index, the Paket
`third_party/dotnet/paket.lock` scope, and user-authored mappings or exact
`dx_ignore_import` exceptions. The extension never selects versions or
edits manifests or locks.

## Evidence

Accepted. Pinned by `bazel run //tools/ci:foundation_maps` with the
[test matrix](../testing/generation.md#generate-command-and-gazelle-extensions).
xUnit runner mapping via `bazel run //tools/ci:xunit_qualification`; Paket
lock via `bazel run //tools/ci:paket_qualification`. No `Supported` claim
until platform plus consumer plus release evidence passes per the
[support matrix](../product/support-matrix.md).
