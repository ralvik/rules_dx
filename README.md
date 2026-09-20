# rules_dx

Status: pre-release development on the Linux x86_64 seed host plus Linux arm64 native (issue #410) plus Linux static-musl profiles (issue #411) plus macOS arm64 native (issue #412) plus macOS x86_64 best-effort native (issue #413) plus Windows x86_64 MSVC-compatible native (issue #414); no `Supported` cells yet, no release cut. No tags, GitHub releases, or registry submissions without explicit owner approval.

`rules_dx` is an opinionated Bazel developer platform designed to provide a tested release stack,
lazy application foundations, Bazel-owned quality workflows, and a thin `dx` CLI. The product aims
to let consumers choose one platform version instead of independently reconciling language
rulesets, toolchains, package managers, test frameworks, and developer tools.

V1 prioritizes complete Rust, Python, JavaScript/TypeScript, and required framework workflows on all
required platforms, with a broad quality-tool baseline and additional low-cost complete foundations.
Upstream patches and packaging support the out-of-the-box experience without rebuilding stacks. See the
[scope and feasibility gate](docs/product/scope.md#first-release-admission) and
[mandatory 100% project coverage requirement](docs/testing/README.md#coverage).
Implementation exists on the Linux x86_64 seed host plus Linux arm64 native (issue #410) plus Linux static-musl profiles (issue #411) plus macOS arm64 native (issue #412) plus macOS x86_64 best-effort native (issue #413) plus Windows x86_64 MSVC-compatible native (issue #414); release qualification beyond those hosts is not claimed here.

## Product

- [Product overview](docs/product/README.md)
- [Product scope](docs/product/scope.md)
- [Support matrix](docs/product/support-matrix.md)
- [Security policy](SECURITY.md)

## Design

- [Documentation index](docs/README.md)
- [Architecture](docs/architecture/README.md)
- [Decision records](docs/decisions/README.md)
- [Roadmap](docs/roadmap.md)
- [Changelog](CHANGELOG.md)

## Contributing

See [Contributing](CONTRIBUTING.md) for the current workflow.

## License

`rules_dx` is licensed under [Apache-2.0](LICENSE). Third-party components retain their
own licenses and required notices.
