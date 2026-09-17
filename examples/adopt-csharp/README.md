# Adopt C# example

Foreign SDK-style tree adopted without upstream changes: a `greet` package
with a self-owned helper edge (`Greet.cs` + `Helper.cs`, no Bazel edge
leaves the package) plus a stdlib-only `solo` package. The tree arrived with
no `MODULE.bazel` and no `BUILD` files; the `*/BUILD.bazel` files are
generator-owned (`//gazelle/csharp:gazelle` output, see below) plus
handwritten `csharp_test` owners.

```sh
bazel run //cli/cli:dx -- init //examples/adopt-csharp/...
bazel run //gazelle/csharp:gazelle -- update examples/adopt-csharp/greet examples/adopt-csharp/solo
bazel build //examples/adopt-csharp/...
bazel test //examples/adopt-csharp/...
```

Evidence: `dx init` writes nothing inside the tree (absent-only).
Generation emits one `csharp_library` per directory (named after the directory
basename) with `srcs` as the sorted non-test sources; `*Test.cs` files stay
out of the library and are owned by handwritten `csharp_test` targets that
survive regeneration. SDK imports (`System.*`) never produce edges; the
pinned `rules_dotnet` SDK stays authoritative at execution time with no
ambient compiler discovery. Build covers 11 targets; both tests pass
(`greet_test`, `pure_test`).

Scope notes: the tree is fully local with an SDK-style `.csproj` recording
the foreign layout but no ecosystem lockfile in the Bazel graph (NuGet lock
wiring per the support matrix stays open under #7). Cross-package C# imports
resolve only through an exact `# gazelle:resolve` mapping today; both
packages stay self-contained for that reason. `dx generate` runs the Rust
extension only; regenerate with the per-language command above until the dx
wiring lands.
