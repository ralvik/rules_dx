# Quality Commands

```sh
bazel run //cli/cli:dx -- lint //...
bazel run //cli/cli:dx -- typecheck //...
bazel run //cli/cli:dx -- format //...
```

No scope means `//...`. Use `--here` for the current dir.

## `dx lint`

Fixes lint findings by default. `dx lint --check` only reports them.

## `dx typecheck`

Fixes type findings by default. `dx typecheck --check` only reports them.

## `dx format`

Formats files by default. `dx format --check` only reports drift.

All three take `--fail-on info|warning|error`. Default is `warning`.
