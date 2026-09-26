# rules_dx

Pre-release.

## Install

Prerequisites: Bazel via Bazelisk.

```sh
bazel build //...
```

## Quickstart

```sh
bazel run //cli/cli:dx -- --help
bazel run //cli/cli:dx -- build //...
bazel run //cli/cli:dx -- test //...
```

See [Command reference](docs/cli/commands/README.md),
[examples](examples/README.md), and [CI](docs/github-ci.md).

## License

`rules_dx` is licensed under [Apache-2.0](LICENSE).
