# Warnings as Errors

Per-toolchain warnings-as-errors pins (issue #455).

Rust uses `--deny=warnings` via
`--@rules_rust//rust/settings:extra_rustc_flags` in `.bazelrc`.
C/C++ uses `copts = ["-Werror"]` in `cc/rules/defs.bzl` (no global
`--copt`: a global flag would turn third-party external compiles red).
Java uses `--javacopt=-Werror --javacopt=-Xlint:all` in `.bazelrc`
and `javacopts` in `java/rules/defs.bzl`.
Kotlin uses `warn = "error"` plus `language_version = "2.2"` plus
`api_version = "2.2"` in `kotlin/rules:warnings_as_errors`
(`kt_kotlinc_options`) with wrapper default `kotlinc_opts`.
Scala uses `scalacopts = ["-Xfatal-warnings"]` in `scala/rules/defs.bzl`.
C# and F# use `treat_warnings_as_errors = True` in
`csharp/rules/defs.bzl` and `fsharp/rules/defs.bzl`.
Go has no compiler warnings; `go vet` is the warning gate and
`go_test` runs its vet subset, so a vet finding fails the test.
Python uses `PYTHONWARNINGS=error` in `.bazelrc` for runtime warnings;
lint warnings are errors via `ruff.toml` and `--fail_on warning`.
JavaScript has no compiler warnings; Biome findings are errors via
`--fail_on warning`.
TypeScript uses `strict = true` in `tsconfig.json`; `ts_project`
fails on type errors and Biome findings are errors via
`--fail_on warning`.

The gate is `bazel run //tools/ci:warnings_as_errors`.
Violation fixtures live in `tools/ci/testdata/warnings/` as
uncompiled data; the gate proves each fixture carries its warning
pattern and each pin is present.
