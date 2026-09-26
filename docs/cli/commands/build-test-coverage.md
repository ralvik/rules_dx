# Build, Test, And Coverage Commands

```sh
bazel run //cli/cli:dx -- build //...
bazel run //cli/cli:dx -- test //...
bazel run //cli/cli:dx -- coverage //...
```

No scope means `//...`. Pass a label, pattern, file, or dir to narrow it. Use `--here` for the current dir.

## `dx build` And `dx test`

```text
dx build [scope...]
dx test [scope...]
```

Builds or tests the scope. File scopes build the owning targets. Test file scopes run the owning tests.

## `dx run`

```text
dx run <label...> [-- args...]
```

Runs binaries. Args after `--` go to the app.

## `dx deploy`

```text
dx deploy //pkg:target [-- args...]
```

Builds and runs one deployable target. Takes exactly one label.

## `dx coverage`

```text
dx coverage [--min-coverage <percent>] [scope...]
```

Runs coverage. `--min-coverage` fails below that percent. LCOV is the report format.

Coverage ignores use `LCOV_EXCL_LINE` or `LCOV_EXCL_START` / `LCOV_EXCL_STOP` with a short `reason:` and `issue:`. Bare ignores without a reason fail the gate.

## Build Profiles

`--debug` uses `dx_debug`. `--release` uses `dx_release`. No flag uses `dx_dev`, except `dx deploy` which uses `dx_release`.
