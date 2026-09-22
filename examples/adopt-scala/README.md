# Adopt Scala example

Foreign sbt-layout tree adopted without upstream changes: a `greet` package
with a self-owned helper edge (`Greet.scala` + `Helper.scala`, no Bazel edge
leaves the package) plus a stdlib-only `solo` package. The tree arrived with
no `MODULE.bazel` and no `BUILD` files; the `*/BUILD.bazel` files are
generator-owned (`dx generate` output) plus
handwritten `scala_test` owners.

```sh
bazel run //cli/cli:dx -- init //examples/adopt-scala/...
bazel run //cli/cli:dx -- generate //examples/adopt-scala/...
bazel build //examples/adopt-scala/...
bazel test //examples/adopt-scala/...
```

Evidence: `dx init` writes nothing inside the tree (absent-only).
Generation emits one `scala_library` per directory (named after the directory
basename) with `srcs` as the sorted non-test sources; `*Test.scala` files
stay out of the library and are owned by handwritten `scala_test` targets
that survive regeneration. Scala/JDK imports (`scala.*`, `java.*`) never
produce edges; the pinned `rules_scala` toolchain stays authoritative at
execution time with no ambient SDK discovery. The `greet` library carries one
pinned module dep (`com.google.guava:guava:32.0.1-jre` via `build.sbt` plus
`third_party/jvm/maven_install.json` plus `@maven//:com_google_guava_guava`
through an exact `# gazelle:resolve` mapping); `Guava.join` plus the
`Guava.join` spec prove build and test over the lock. Depcheck `locks`
consistency, quality (`QualitySourcesInfo`), and coverage
(`InstrumentedFilesInfo`) flow through the shared wrappers.
Build covers 11 targets; both tests pass (`greet_test`, `pure_test`).

Scope notes: cross-package Scala imports resolve only through an exact
`# gazelle:resolve` mapping today. Regeneration is the composed
`dx generate` run.
