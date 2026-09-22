# C/C++ Generation Contract

C/C++ follows the [common contract](common.md). Admitted to v1 by
[ADR 0019](../decisions/0019-first-release-additional-foundations.md) on the
provisional `rules_cc` plus hermetic-llvm backend; no switch is approved
here.

Wrappers in `cc/rules/defs.bzl` preserve `CcInfo` and add
`QualitySourcesInfo`. The extension lives in `gazelle/cc/` with
parser/naming/lang fixtures plus focused tests. Hello builds in
`cc/tests/fixtures/hello/` consume the wrappers.

## Sources And Ownership

One directory holds one reusable `cc_library` named after the directory
basename with sorted non-test sources in `srcs` and sorted non-test headers
in `hdrs`. `*_test.*` files are never library sources; handwritten
`cc_test` owns them. Thin `cc_binary` entries are never inferred, and a
directory mixing a library with a `main`-defining source fails so the owner
splits it first. Exact extensions, test, and main rules live in
`gazelle/cc/naming.go` and `gazelle/cc/parser.go`, pinned by their fixtures.

## Resolution

Only quoted includes contribute literal identities; they resolve strictly
through the local index and user-authored mappings or exact
`dx_ignore_import` exceptions. There is no lockfile: every `http_archive`
carries `sha256`/`integrity`. The extension never guesses an include, emits
an unresolved marker, or infers resources.

## Evidence

Accepted. Pinned by `bazel run //tools/ci:foundation_maps` with the
[test matrix](../testing/generation.md#generate-command-and-gazelle-extensions).
GoogleTest mapping via `bazel run //tools/ci:googletest_qualification`;
hermetic hash wiring via
`bazel run //tools/ci:cc_hermetic_qualification`. No `Supported` claim until
platform plus consumer plus release evidence passes per the
[support matrix](../product/support-matrix.md).
