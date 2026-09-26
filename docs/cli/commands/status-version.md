# `dx status` And `dx version`

```sh
bazel run //cli/cli:dx -- status
bazel run //cli/cli:dx -- version
```

`dx status` reports toolchain, platform, tools, and pin. `dx version` prints the version.

```sh
bazel run //cli/cli:dx -- version --check
bazel run //cli/cli:dx -- version --pin 0.0.0
bazel run //cli/cli:dx -- version --rollback
```

`--check` verifies the pin. `--pin` sets it. `--rollback` restores the last one. There is no `dx doctor`. Use `dx status` instead.
