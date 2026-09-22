# F# Generation Contract

F# follows the [common contract](common.md). Admitted to v1 by
[ADR 0019](../decisions/0019-first-release-additional-foundations.md) on the
provisional `rules_dotnet` upstream; no switch is approved here.

Wrappers in `fsharp/rules/defs.bzl` preserve the upstream .NET assembly
providers and add `QualitySourcesInfo`. The extension lives in
`gazelle/fsharp/` with parser/naming/lang fixtures plus focused tests.
Hello builds in `fsharp/tests/fixtures/hello/` consume the wrappers.

## Sources And Ownership

One directory holds one reusable `fsharp_library` named after the directory
basename with dependency-ordered non-test `.fs` sources. `srcs` order is
significant: F# compiles topologically, so dependencies come first with an
alphabetical tie-break (unlike C#, where order is irrelevant). Intra-package
`open` edges matching sibling basenames drive the order; same-namespace uses
without an `open` still need handwritten dependency-first order. `*Test.fs`
files are never library sources; handwritten `fsharp_test` owns them. Thin
`fsharp_binary` entries are never inferred, and a directory mixing a library
with a `main`-defining source fails so the owner splits it first. Exact
naming, test, main, and ordering rules live in `gazelle/fsharp/naming.go` and
`gazelle/fsharp/parser.go`, pinned by their fixtures. Wrapper `srcs` keep the
same contract in `fsharp/rules/defs.bzl`.

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
