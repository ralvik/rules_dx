# Adopt C# example

Foreign SDK-style tree adopted without upstream changes: a `greet` package
with a self-owned helper edge (`Greet.cs` + `Helper.cs`, no Bazel edge
leaves the package) plus a stdlib-only `solo` package. The tree arrived with
no `MODULE.bazel` and no `BUILD` files; the `*/BUILD.bazel` files are
generator-owned (`dx generate` output) plus
handwritten `csharp_test` owners.

```sh
bazel run //cli/cli:dx -- init //examples/adopt-csharp/...
bazel run //cli/cli:dx -- generate //examples/adopt-csharp/...
bazel build //examples/adopt-csharp/...
bazel test //examples/adopt-csharp/...
```

Evidence: `dx init` writes nothing inside the tree (absent-only).
Generation emits one `csharp_library` per directory (named after the directory
basename) with `srcs` as the sorted non-test sources; `*Test.cs` files stay
out of the library and are owned by handwritten `csharp_test` targets that
survive regeneration. SDK imports (`System.*`) never produce edges; the
pinned `rules_dotnet` SDK stays authoritative at execution time with no
ambient compiler discovery. Each handwritten test carries one pinned module
dep (`xunit.v3.assert 4.0.0` via `adopt-csharp.csproj` plus
`third_party/dotnet/paket.lock` plus `@paket.main//xunit.v3.assert`);
`Assert.Equal` in both mains proves build and test over the lock without MTP
shims so the plain `Main` stays generation-stable. Depcheck `locks`
consistency, quality (`QualitySourcesInfo`), and coverage
(`InstrumentedFilesInfo`) flow through the shared wrappers.
Build covers 11 targets; both tests pass (`greet_test`, `pure_test`).

Scope notes: cross-package C# imports resolve only through an exact
`# gazelle:resolve` mapping today. Regeneration is the composed
`dx generate` run.
