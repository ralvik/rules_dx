# M26: Audit And Update

## Outcome

Security/license audit and dependency update are complete across the admitted v1 ecosystems.

## Scope

Implement source/dependency security audit and dependency license-policy analysis through qualified
integrations, plus authoritative resolver updates. Bare `dx audit` runs both audit families;
update refreshes standard locks within declared requirements through approved Bazel integrations.
The update-bot scope remains unresolved and blocked under O53; this milestone does not approve
new bot deliverables.

## Contract References

- [Audit and update contracts](../cli/commands/audit-update-bazel.md), [CLI](../cli/), [quality](../quality/), [testing](../testing/), and open decisions [O11, O12, and O58](../open-decisions.md).
- [License-family contract](../cli/commands/audit-update-bazel.md#license-family-dx-audit-license):
  O58 shared-lock attribution, identity/approval/report mappings, and proofs gate implementation.
  [O53](../open-decisions.md) remains an
  unresolved M26/M27 bot scope gate, not delivery approval.

## Deliverables

- Final `audit` and `update` command implementations.
- Qualified auditor integrations, advisory acquisition/snapshot inputs, risk-exception
  configuration, and ecosystem updater mappings for every admitted dependency set.
- License-family policy, identity mappings, and SPDX reporting under the O58-qualified audit contract.

## Work Packages

1. Add the security audit family with qualified source/dependency tools, advisory snapshots,
   severity/report mappings, and risk-exception lifecycle.
2. Add resolver-owned dependency updates with dependency-set/package selection,
   non-registry handling, and upstream operation/report mappings.
3. Qualify O58 and implement the license family and default-both audit selection through the
   owning audit contract, including policy, exception, SPDX, and notice-text evidence.

## Milestone-Specific Evidence

- Audit is non-mutating; update changes only authoritative standard manifests/locks through approved resolvers.
- The [CLI matrix](../testing/cli.md#command-registry-and-behavior) proves independent-update
  continuation, blocked dependents, preserved successes, and O12-qualified exit/report mappings.
- O46's admitted audit/update ecosystems all have workflow evidence; the original
  Python/Node/Rust list is not a ceiling on repository workflow coverage.
- Security-only, license-only, and default-both fixtures satisfy the audit contract; license
  evidence covers the qualified ecosystem identities, tier policy, SPDX expressions/reports,
  exceptions, and notice-text collection without approving deferred NOTICE aggregation. Include
  explicit-approval and in-range exception-upgrade cases from the CLI matrix.

## Out Of Scope

- Codegen, environments, setup, consumer CI, release qualification, and publication.
- Update-bot implementation pending resolution of O53's scope gate.

## Completion Report Additions

- Record audit sources, updater ecosystems, advisory identities, and exception-lifecycle results.
- Record O58 qualification, license-family evidence, and the unresolved O53 bot boundary.
