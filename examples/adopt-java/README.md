# Adopt Java example

Foreign Maven-layout tree adopted without upstream changes: a `greet`
package with a self-owned helper edge (`Greet.java` + `Helper.java`, no
Bazel edge leaves the package) plus a stdlib-only `solo` package. The tree
arrived with no `MODULE.bazel` and no `BUILD` files; the `*/BUILD.bazel`
files are generator-owned (`dx generate` output)
plus handwritten `java_test` owners.

```sh
bazel run //cli/cli:dx -- init //examples/adopt-java/...
bazel run //cli/cli:dx -- generate //examples/adopt-java/...
bazel build //examples/adopt-java/...
bazel test //examples/adopt-java/...
```

Evidence: `dx init` writes nothing inside the tree (absent-only).
Generation emits one `java_library` per directory (named after the directory
basename) with `srcs` as the sorted non-test sources; `*Test.java` files
stay out of the library and are owned by handwritten `java_test` targets
that survive regeneration. JDK imports (`java.*`) never produce edges; the
pinned `rules_java` toolchain stays authoritative at execution time with no
ambient JDK discovery. The `greet` library carries one pinned module dep
(`com.google.guava:guava:32.0.1-jre` via `pom.xml` plus
`third_party/jvm/maven_install.json` plus `@maven//:com_google_guava_guava`
through an exact `# gazelle:resolve` mapping); `Guava.join` plus
`testGuavaJoin` prove build and test over the lock. Depcheck `locks`
consistency, quality (`QualitySourcesInfo`), and coverage
(`InstrumentedFilesInfo`) flow through the shared wrappers.
Build covers 11 targets; both tests pass (`greet_test`, `pure_test`).

Scope notes: cross-package Java imports resolve only through an exact
`# gazelle:resolve` mapping today. Regeneration is the composed
`dx generate` run.
