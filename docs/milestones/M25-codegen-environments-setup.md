# M25: Codegen, Environments, And Setup

## Outcome

Generated-source projection, language environments, and atomic combined setup are
complete across all v1 foundations.

## Scope

Implement codegen provider shards, codegen/environment plan collection through BEP,
immutable projections, root and exact-target selection, concurrency-safe commits,
one-request `dx setup` across all v1 foundations, and explicit `dx clean` pruning
of unselected managed generations.

## Contract References

- [CLI](../cli/), [generation](../generation/), [environments](../environments/), [quality](../quality/), [testing](../testing/), open decisions [O33, O34, O35, and O36](../open-decisions.md), and [ADR 0018](../decisions/0018-umbrella-check-fix-cleanup-clean.md).
- [Clean contract](../cli/commands/check-fix-clean.md#dx-clean) and
  [O60](../open-decisions.md): qualify cleanup mechanics before affected implementation.

## Deliverables

- Final `codegen`, `env`, `setup`, and `clean` command implementations.
- Normalized codegen plan providers, private collection of language environment plans behind public
  commands, output groups, immutable generations/projections, and atomic setup selection.

## Work Packages

1. Implement codegen shards, artifacts, exact/root selection, conflict validation, and read-only projection.
2. Consume all admitted v1 foundation and framework target plans through public repository/root/exact-target
   env planning, collection, projection, and native facades.
3. Implement independent carry-forward and one-request atomic setup with advisory locking.
4. Benchmark root strategies and certify concurrency, interruption, remote materialization, and reuse.
5. Implement `dx clean` pruning of validated unselected generations with current-pointer preservation,
   unmanaged-path refusal, dry-run listing, and explicit `--bazel` forwarding after O60 qualification.

## Milestone-Specific Evidence

- Codegen and env never invoke each other; setup shares one Bazel request and commits both or neither.
- Root candidates have equivalent semantics, and the selected strategy is justified by measured evidence.
- O46's admitted codegen pairs all have workflow evidence; the original
  Python/Node/Rust list is not a ceiling on repository workflow coverage.
- Cleanup fixtures satisfy [ADR 0018](../decisions/0018-umbrella-check-fix-cleanup-clean.md)
  and the O60-qualified contract, including in-use/concurrent preparation safety, dry-run reporting,
  current-selection preservation, unmanaged-path refusal, and explicit Bazel-clean recovery guidance.

## Out Of Scope

- Security audit, dependency update, consumer CI, release qualification, and publication.
- New languages/tools, automatic environment/projection cleanup policy (explicit `dx clean` is in scope), and shell-profile mutation.

## Completion Report Additions

- Record provider/shard schemas, root benchmark decision, projection identities/layouts, and concurrency results.
- Record O60 qualification and cleanup safety, reporting, and recovery evidence.
