# Adopt Kotlin example

Foreign Maven-layout tree adopted without upstream changes: a `greet`
package with a self-owned helper edge (`Greet.kt` + `Helper.kt`, no Bazel
edge leaves the package) plus a stdlib-only `solo` package. The tree
arrived with no `MODULE.bazel` and no `BUILD` files; the `*/BUILD.bazel`
files are generator-owned (`//gazelle/kotlin:gazelle` output, see below)
plus handwritten `kotlin_test` owners.

```sh
bazel run //cli/cli:dx -- init //examples/adopt-kotlin/...
bazel run //gazelle/kotlin:gazelle -- update examples/adopt-kotlin/greet examples/adopt-kotlin/solo
bazel build //examples/adopt-kotlin/...
bazel test //examples/adopt-kotlin/...
```

Evidence: `dx init` writes nothing inside the tree (absent-only).
Generation emits one `kotlin_library` per directory (named after the directory
basename) with `srcs` as the sorted non-test sources; `*Test.kt` files stay
out of the library and are owned by handwritten `kotlin_test` targets that
survive regeneration. JDK imports (`java.*`) never produce edges; the pinned
`rules_kotlin` toolchain stays authoritative at execution time with no ambient
SDK discovery. Build covers 11 targets; both tests pass (`greet_test`,
`pure_test`).

Scope notes: the tree is fully local with a `pom.xml` recording the foreign
Maven coordinates but no ecosystem lockfile in the Bazel graph (Maven-lock
resolution per the support matrix stays open under #7). Cross-package Kotlin
imports resolve only through an exact `# gazelle:resolve` mapping today; both
packages stay self-contained for that reason. `dx generate` runs the Rust
extension only; regenerate with the per-language command above until the dx
wiring lands.
