# rules_dx

Pre-release.

`rules_dx` is an opinionated Bazel developer platform designed to provide a tested release stack,
lazy application foundations, Bazel-owned quality workflows, and a thin `dx` CLI. The product aims
to let consumers choose one platform version instead of independently reconciling language
rulesets, toolchains, package managers, test frameworks, and developer tools.

## Installation

Prerequisites: Bazel via Bazelisk (see `.bazelversion`); language toolchains
come from Bazel automatically.

```sh
# Build everything on the seed host.
bazel build //...
```

## Quickstart

`dx` runs through Bazel. No installation step publishes `dx` outside the repository yet.

```sh
# Show help.
bazel run //cli/cli:dx -- --help

# Build and test the current workspace.
bazel run //cli/cli:dx -- build //...
bazel run //cli/cli:dx -- test //...
```

Next: [CLI reference](docs/cli/README.md), [Command reference](docs/cli/commands/README.md),
[consumer examples](examples/README.md), [CI](docs/github-ci.md), and the
[docs site](https://ralvik.github.io/rules_dx/).

## Contributing

See [Contributing](CONTRIBUTING.md) for the current workflow.

## License

`rules_dx` is licensed under [Apache-2.0](LICENSE). Third-party components retain their
own licenses and required notices.
