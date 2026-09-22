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

Adapter families use check-mode plus `--fail_on warning` (issue #1056).

Ruby uses `rubocop --format json` check-only with no `--fail-level`
weakening; `convention`/`refactor` map to warning so any offense fails
via `--fail_on warning`. Format diffs fail via `standardrb --check`.
PowerShell uses `Invoke-ScriptAnalyzer` console text with no `-Severity`
filter; every finding is a warning so any finding fails via
`--fail_on warning` (`psscriptanalyzer`).
Shell uses `shellcheck --format=gcc` with no `-S` filter; warnings fail
via `--fail_on warning` (`shellcheck`). Format diffs fail via
`shfmt -d` (`shfmt`).
CUE uses `cue fmt --check --diff`; any diff fails (`cue`).
QML uses `qmlformat --check`; any unformatted path fails (`qmlformat`).
`qmllint --json -` warnings fail via `--fail_on warning` (`qmllint`).
Protobuf uses `buf lint --error-format=json`; every record is an error
(`buf`). Format diffs fail via `buf format --diff --exit-code`.
`keep_sorted` is check-only with no severity filter; every diagnostic
is a warning so any finding fails via `--fail_on warning`.
Error Prone uses `javac -Xplugin:ErrorProne` with default severities and
no `-Werror` (preserves error/warning distinction); warnings fail via
`--fail_on warning` (`error_prone`).
`detekt` has no dispatched adapter yet so no strict pin; pending under
the JVM qualification with no hidden preset.

The gate is `bazel run //tools/ci:warnings_as_errors`.
Violation fixtures live in `tools/ci/testdata/warnings/` as
uncompiled data; the gate proves each fixture carries its warning
pattern and each pin is present.
