# Adopt C++ example

Foreign C++ tree adopted without upstream changes: a `greet` package with
a self-owned quoted-include edge (`greet.cc` -> `helper.h`, no Bazel edge
leaves the package) plus a stdlib-only `solo` package. The tree arrived
with no `MODULE.bazel` and no `BUILD` files; the `*/BUILD.bazel` files are
generator-owned (`//gazelle/cc:gazelle` output, see below) plus handwritten
`cc_test` owners.

```sh
bazel run //cli/cli:dx -- init //examples/adopt-cpp/...
bazel run //gazelle/cc:gazelle -- update examples/adopt-cpp/greet examples/adopt-cpp/solo
bazel build //examples/adopt-cpp/...
bazel test //examples/adopt-cpp/...
```

Evidence: `dx init` writes nothing inside the tree (absent-only).
Generation emits one `cc_library` per directory (named after the directory
basename) with `srcs` as the sorted non-test sources and `hdrs` as the
sorted non-test headers; `*_test.cc` files stay out of the library and are
owned by handwritten `cc_test` targets that survive regeneration.
Angle includes (`<string>`, `<gtest/gtest.h>`) never produce edges; the pinned
`rules_cc` toolchain stays authoritative at execution time with no ambient
compiler discovery. Each handwritten test carries one pinned dep
(GoogleTest v1.18.0 plus explicit `-std=c++17` floor via
`@googletest//:gtest_main`); `TEST`/`EXPECT_EQ` plus the `__cplusplus`
floor assert prove build and test over the hash authority
(`MODULE.bazel.lock` integrity plus per-archive `sha256`; C/C++ has no
ecosystem lockfile). Depcheck `cc_deps` sha256, quality
(`QualitySourcesInfo`), and coverage (`InstrumentedFilesInfo`) flow through
the shared wrappers. Build covers 11 targets; both tests pass (`greet_test`,
`pure_test`).

Scope notes: cross-package quoted includes resolve only through an exact
`# gazelle:resolve` mapping today. `dx generate` runs the Rust extension only; regenerate with
the per-language command above until the dx wiring lands.
