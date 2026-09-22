# Testing Strategy

Tests prove graph ownership, action construction, hermeticity, deterministic
selection, cache invalidation, diagnostics, and CLI transparency. Passing tool
output alone is insufficient. Open work is tracked in GitHub issues.

## Goals

Focused test matrices define the domain-specific evidence:

- [CLI](cli.md) covers argument handling, output, target resolution, command behavior,
  Bazel forwarding, and end-to-end CLI operation.
- [GitHub CI](github-ci.md) covers consumer setup, check selection, execution, events
  and revisions, reporting, fork security, merge gating, and qualification.
- [Generation](generation.md) covers first-party Gazelle extensions, language wrappers,
  dependency resolution, naming, merge behavior, and generation transport.
- [Environments](environments.md) covers environment, codegen, setup, BEP collection,
  projections, installation, and concurrency.
- [Tools](tools.md) covers acquisition, runtimes, toolchains, platform execution,
  release pins, performance, and unused-foundation laziness.
- [Quality Workflow Testing](../quality/quality-testing.md) covers quality result and
  diagnostic protocols, configured policy, native config binding, source selection,
  evaluators, cache correctness, determinism, apply safety, and tool parity.

Environment-specific contracts are defined in
[Developer Environments](../environments/environment.md) and
[Python Environment](../environments/python-environment.md). Each focused matrix links
to the authoritative product contracts that its tests exercise.

## Coverage

Coverage gate and short `policy:` ignore rule live in [Strategy Details](strategy-details.md#coverage).

## Details

Layer, coverage, infrastructure, and evidence contracts live in [Strategy Details](strategy-details.md).

- [CLI](cli.md), [GitHub CI](github-ci.md), [Generation](generation.md), [Environments](environments.md), [Tools](tools.md), [Quality Workflow](../quality/quality-testing.md)
- [Starlark Testing](starlark.md)

```sh
bazel test //...
```
