# Documentation

Start here to use `rules_dx`: install once, run `dx` through Bazel, and follow the
command and language guides below.

## Getting Started

- [Product overview](product/README.md): what `rules_dx` provides.
- [Product scope](product/scope.md): boundary, commands, and constraints.
- [Support matrix](product/support-matrix.md): current platform and foundation status.
- [CLI](cli/README.md): command, process, output, and target-resolution contracts.
- [Command reference](cli/commands/README.md): behavior of each `dx` command.

## Languages And Examples

- [Generation](generation/README.md): target generation and source ownership.
- [Environments](environments/README.md): environments and generated-code projections.
- [Consumer examples](../examples/README.md): starter callers and per-language adoption workspaces.

## Architecture And Contracts

- [Architecture](architecture/README.md): component boundaries and dependency flow.
- [Contradiction catalog](architecture/contradiction-catalog.md): which statement wins for each known design conflict.
- [GitHub CI](github-ci.md): consumer CI execution and reporting.
- [Quality](quality/README.md): sources, action model, integrations, protocol, tests.
- [Tools](tools/README.md): acquisition policy and first-release baseline.
- [Testing](testing/README.md): behavior, hermeticity, cache, platform, consumer evidence.
- [Documentation domain](documentation/README.md): documentation IR, site build, and CLI surface.
- [Deploy](deploy/README.md): `dx deploy` dispatch and release path.

## Internal / Maintainer-Only

Maintainer planning and evidence. Not required for normal use.

- [Promotion checklist](product/promotion-checklist.md): tag hygiene, versioning, and per-cell evidence to `Supported`.
- [Verification matrix](testing/verification-matrix.md): as-built language x layer status.
- [Native toolchains](native-toolchains.md): provisional C/C++/Rust stack and qualification plan.
- [Documentation site build](documentation/site.md) and [documentation IR](documentation/doc-ir.md): planned site design, not a working site yet.
- [Decision records](decisions/README.md): accepted, provisional, superseded, and rejected decisions.
  Authoring rules live in [decision instructions](decisions/AGENTS.md).

## Contributing

- [Contributing](../CONTRIBUTING.md): repository workflow and delivery flow.
- [Contributing guides](contributing/README.md): local workflows, devcontainer
  preview, and diagnostics/versioning preview.
- [Security policy](../SECURITY.md): vulnerability reporting and supported versions.
- [Documentation instructions](AGENTS.md): one-doc-per-fact, status labeling,
  and conflict handling for documentation edits.

### Document Authority (maintainer reference)

| Document class | Authority |
| --- | --- |
| Accepted decision records | Normative architectural and product constraints; they override conflicting provisional design prose |
| Product scope and support matrix | Authoritative product boundary, repository evidence, and current support claims, subject to accepted decisions |
| Design contracts and protocols | Authoritative component behavior within accepted product and decision constraints |
| Provisional decisions and explicitly provisional design | Validation targets, not stable commitments |
| Agent instructions | Procedural instructions only; they do not define product semantics or architecture contracts |

If authoritative documents conflict, stop affected work and resolve the documents rather than
inventing an interpretation. Update the single authoritative source in place and link to it instead
of creating parallel drafts or copied contracts.
