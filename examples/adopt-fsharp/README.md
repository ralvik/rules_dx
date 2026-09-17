# Adopt F# example

Foreign SDK-style tree adopted without upstream changes: a `greet` package
with a self-owned helper edge (`Greet.fs` + `Helper.fs`, no Bazel edge
leaves the package) plus a stdlib-only `solo` package. The tree arrived with
no `MODULE.bazel` and no `BUILD` files; the `*/BUILD.bazel` files are
generator-owned (`//gazelle/fsharp:gazelle` output, see below) plus
handwritten `fsharp_test` owners.

```sh
bazel run //cli/cli:dx -- init //examples/adopt-fsharp/...
bazel run //gazelle/fsharp:gazelle -- update examples/adopt-fsharp/greet examples/adopt-fsharp/solo
bazel build //examples/adopt-fsharp/...
bazel test //examples/adopt-fsharp/...
```

Evidence: `dx init` writes nothing inside the tree (absent-only).
Generation emits one `fsharp_library` per directory (named after the directory
basename) with `srcs` as the sorted non-test sources; `*Test.fs` files stay
out of the library and are owned by handwritten `fsharp_test` targets that
survive regeneration. SDK imports (`System.*`) never produce edges; the
pinned `rules_dotnet` SDK stays authoritative at execution time with no
ambient compiler discovery. Build covers 11 targets; both tests pass
(`greet_test`, `pure_test`).

Scope notes: the tree is fully local with an SDK-style `.fsproj` recording
the foreign layout but no ecosystem lockfile in the Bazel graph (NuGet lock
wiring per the support matrix stays open under #7). Cross-package F# imports
resolve only through an exact `# gazelle:resolve` mapping today; both
packages stay self-contained for that reason. `dx generate` runs the Rust
extension only; regenerate with the per-language command above until the dx
wiring lands.
