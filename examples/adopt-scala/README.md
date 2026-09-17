# Adopt Scala example

Foreign sbt-layout tree adopted without upstream changes: a `greet` package
with a self-owned helper edge (`Greet.scala` + `Helper.scala`, no Bazel edge
leaves the package) plus a stdlib-only `solo` package. The tree arrived with
no `MODULE.bazel` and no `BUILD` files; the `*/BUILD.bazel` files are
generator-owned (`//gazelle/scala:gazelle` output, see below) plus
handwritten `scala_test` owners.

```sh
bazel run //cli/cli:dx -- init //examples/adopt-scala/...
bazel run //gazelle/scala:gazelle -- update examples/adopt-scala/greet examples/adopt-scala/solo
bazel build //examples/adopt-scala/...
bazel test //examples/adopt-scala/...
```

Evidence: `dx init` writes nothing inside the tree (absent-only).
Generation emits one `scala_library` per directory (named after the directory
basename) with `srcs` as the sorted non-test sources; `*Test.scala` files
stay out of the library and are owned by handwritten `scala_test` targets
that survive regeneration. Scala/JDK imports (`scala.*`, `java.*`) never
produce edges; the pinned `rules_scala` toolchain stays authoritative at
execution time with no ambient SDK discovery. Build covers 11 targets; both
tests pass (`greet_test`, `pure_test`).

Scope notes: the tree is fully local with a `build.sbt` recording the
foreign sbt coordinates but no ecosystem lockfile in the Bazel graph
(Coursier-backed lock resolution per the support matrix stays open under
#7). Cross-package Scala imports resolve only through an exact
`# gazelle:resolve` mapping today; both packages stay self-contained for that
reason. `dx generate` runs the Rust extension only; regenerate with the
per-language command above until the dx wiring lands.
