# rules_dx

Status: design only; no implementation or release exists yet.

`rules_dx` is an opinionated Bazel developer platform designed to provide a tested release stack,
lazy application foundations, Bazel-owned quality workflows, and a thin `dx` CLI. The product aims
to let consumers choose one platform version instead of independently reconciling language
rulesets, toolchains, package managers, test frameworks, and developer tools.

V1 prioritizes complete Rust, Python, JavaScript/TypeScript, and required framework workflows on all
required platforms, with a broad quality-tool baseline and additional low-cost complete foundations.
Upstream patches and packaging support the out-of-the-box experience without rebuilding stacks. See the
[scope and feasibility gate](docs/product/scope.md#first-release-admission) and
[mandatory 100% project coverage requirement](docs/testing/README.md#coverage).
Nothing is implemented or verified yet.

## Product

- [Product overview](docs/product/README.md)
- [Product scope](docs/product/scope.md)
- [Support matrix](docs/product/support-matrix.md)
- [Security policy](SECURITY.md)

## Design

- [Documentation index](docs/README.md)
- [Architecture](docs/architecture/README.md)
- [Decision records](docs/decisions/README.md)
- [Implementation plan](docs/implementation-plan.md)

## Contributing

See [Contributing](CONTRIBUTING.md) for the current workflow and delivery gates.

## License

`rules_dx` is licensed under [Apache-2.0](LICENSE). Third-party components retain their
own licenses and required notices.
