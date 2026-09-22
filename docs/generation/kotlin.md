# Kotlin Generation Contract

Kotlin follows the [common contract](common.md). Admitted to v1 by
[ADR 0019](../decisions/0019-first-release-additional-foundations.md) on the
provisional `rules_kotlin` upstream; no switch is approved here.

Wrappers in `kotlin/rules/defs.bzl` preserve `JavaInfo` and add
`QualitySourcesInfo`. The extension lives in `gazelle/kotlin/` with
parser/naming/lang fixtures plus focused tests. Hello builds in
`kotlin/tests/fixtures/hello/` consume the wrappers.

## Sources And Ownership

One directory holds one reusable `kotlin_library` named after the directory
basename with sorted non-test `.kt` files as `srcs`. `*Test.kt` files are
never library sources; handwritten `kotlin_test` owns them. Mixed
`.kt`/`.java` targets keep one owner per file: sources that belong in a
`java_library` stay there. Thin `kotlin_binary` entries are never inferred,
and a directory mixing a library with a `main`-defining source fails so the
owner splits it first. Exact naming, test, and main rules live in
`gazelle/kotlin/naming.go` and `gazelle/kotlin/parser.go`, pinned by their
fixtures.

## Resolution

JDK roots filter via `IsStdLib`. Every other literal identity uses strict
common resolution over the local index, the shared Maven
`third_party/jvm/maven_install.json` scope, and user-authored mappings or
exact `dx_ignore_import` exceptions. Production imports resolve from the
production scope; test imports may additionally use the assigned test
scope. The extension never selects versions or edits manifests or locks.

## Evidence

Accepted. Pinned by `bazel run //tools/ci:foundation_maps` with the
[test matrix](../testing/generation.md#generate-command-and-gazelle-extensions).
JUnit runner mapping via `bazel run //tools/ci:junit_qualification`; Maven
fail-closed lock via `bazel run //tools/ci:maven_lock_qualification`. No
`Supported` claim until platform plus consumer plus release evidence passes
per the [support matrix](../product/support-matrix.md).
