# Documentation

The root [README](../README.md) is the concise product landing page. This index groups the
authoritative product, design, delivery, and governance documents.

## Product

- [Product overview](product/README.md): entry point.
- [Product scope](product/scope.md): audit, boundary, commands, risks.
- [Support matrix](product/support-matrix.md): foundation and quality claims.
- [Promotion checklist](product/promotion-checklist.md): tag hygiene, versioning, and per-cell evidence to `Supported` (issue #611).

## Architecture And Contracts

- [Architecture](architecture/README.md): component boundaries and dependency flow.
- [CLI](cli/README.md): command, process, output, report, and target-resolution contracts.
- [GitHub CI](github-ci.md): consumer CI execution, reporting, and qualification.
- [Generation](generation/README.md): target generation and source ownership.
- [Environments](environments/README.md): environments and generated-code projections.
- [Documentation domain](documentation/README.md): planned unified documentation
  IR, site build, and CLI surface.
- [Native toolchains](native-toolchains.md): provisional C/C++/Rust stack and
  qualification questions.

## Quality And Tools

- [Quality](quality/README.md): sources, action model, integrations, protocol, tests.
- [Tools](tools/README.md): acquisition policy and first-release baseline.
- [Testing](testing/README.md): behavior, hermeticity, cache, platform, consumer evidence.
- [Verification matrix](testing/verification-matrix.md): as-built language x
  layer status for the close-out battery (no `Supported` claims).

## Delivery And Governance

- [Roadmap](roadmap.md): current tracks and priorities.
- [Changelog](../CHANGELOG.md): release history.
- [Decision records](decisions/README.md): accepted, provisional, superseded, and rejected decisions.
  Authoring rules live in [decision instructions](decisions/AGENTS.md).

## Contributing

- [Contributing](../CONTRIBUTING.md): repository workflow and delivery flow.
- [Contributing guides](contributing/README.md): local workflows, devcontainer
  preview, and diagnostics/versioning preview.
- [Security policy](../SECURITY.md): vulnerability reporting and supported versions.
- [Documentation instructions](AGENTS.md): one-doc-per-fact, status labeling,
  and conflict handling for documentation edits.

## Document Authority

| Document class | Authority |
| --- | --- |
| Accepted decision records | Normative architectural and product constraints; they override conflicting provisional design prose |
| Product scope and support matrix | Authoritative product boundary, repository evidence, and current support claims, subject to accepted decisions |
| Design contracts and protocols | Authoritative component behavior within accepted product and decision constraints |
| Provisional decisions and explicitly provisional design | Validation targets, not stable commitments |
| Roadmap | Planned work with status and next steps; not normative behavior |
| Agent instructions | Procedural instructions only; they do not define product semantics or architecture contracts |

If authoritative documents conflict, stop affected work and resolve the documents rather than
inventing an interpretation. Update the single authoritative source in place and link to it instead
of creating parallel drafts or copied contracts.
