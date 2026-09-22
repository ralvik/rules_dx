# rules_dx

Pre-release. See the [support matrix](docs/product/support-matrix.md#unqualified-platforms) for current platform status.

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
[consumer examples](examples/README.md), [support matrix](docs/product/support-matrix.md),
and [product scope](docs/product/scope.md).

## Product

- [Product overview](docs/product/README.md)
- [Product scope](docs/product/scope.md)
- [Support matrix](docs/product/support-matrix.md)
- [Security policy](SECURITY.md)

## Design

- [Documentation index](docs/README.md)
- [Architecture](docs/architecture/README.md)
- [Decision records](docs/decisions/README.md)

## Contributing

See [Contributing](CONTRIBUTING.md) for the current workflow.

## License

`rules_dx` is licensed under [Apache-2.0](LICENSE). Third-party components retain their
own licenses and required notices.
