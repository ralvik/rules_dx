# ADR 0002: Targets, Not Files

## Status

Accepted.

## Context

Developer input often starts as a file path, while Bazel owns source and
dependency relationships. Constructing checker file lists by parsing BUILD files
or recursively scanning directories would duplicate graph semantics and omit
generated or configured relationships.

## Decision

Lint, typecheck, format, and source-audit operations execute over Bazel targets. When a user
supplies a path, `dx` asks Bazel to resolve its owning target and then selects the
configured aspects.

Language wrappers and supported upstream rule kinds expose authoritative direct-source facts.
Applicability comes from those facts, workspace policy, and adapter metadata. Tags do not
supply file lists or authorize source discovery.

Custom source-owning rules use the public `QualitySourcesInfo` provider to expose direct
main-workspace non-generated source artifacts grouped by stable semantic file-class ID. The provider is both the source-ownership
and classification boundary for custom rules, not a generic dependency or configuration
provider. Project wrappers emit it automatically. A target may expose any number of classes;
each adapter receives only the classes it supports under workspace policy.

The CLI does not parse BUILD files and does not recursively scan the repository to
create lint inputs. Directory scopes use Bazel packages, target patterns, or query.

## Consequences

- Source ownership must be explicit in the Bazel graph.
- Zero and multiple owners require documented diagnostics and policy.
- Generated-source ownership may require `cquery` or an aspect after validation.
- Checker actions consume declared target inputs rather than client-generated
  lists.
- Custom rules integrate through `QualitySourcesInfo`; generic aspects do not inspect arbitrary
  attributes, infer classes from extensions, or consume `DefaultInfo` outputs.
- Semantic file classes determine adapter applicability but do not create per-class actions;
  lint, typecheck, and format remain target/capability pipelines.
- Classification belongs to an owner context. Multiple direct owners may classify one path
  differently, but mutating owner pipelines must still agree on identical final bytes.
- Target resolution becomes a tested CLI/Bazel contract.

## Rejected Alternatives

- Filesystem globbing or walking in `dx`.
- BUILD/Starlark parsing in `dx`.
- Treating an arbitrary raw file list as the authoritative CI lint boundary.
