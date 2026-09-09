# M27: Consumer CI

## Outcome

The reusable consumer GitHub CI workflow, caller template, and first-party reporting are
delivered and proven through clean external consumers.

## Scope

Deliver the accepted consumer GitHub CI contract through a versioned reusable workflow,
consumer-owned caller template, and first-party reporting. Explicit platform choice and merge-queue
support are accepted scope. This planning assignment authorizes no implementation: exact workflow
APIs/pins, runner/platform mappings, event/ref bindings, security/permission boundaries, and review-thread
mechanics/limit mappings must be qualified and frozen before affected implementation, not deferred to M28.

## Contract References

- [Consumer GitHub CI contract](../github-ci.md), [CI evidence matrix](../testing/github-ci.md), and
  [consumer CI qualification](../open-decisions.md#consumer-ci-qualification).

## Deliverables

- Versioned reusable consumer workflow, caller template, first-party reporting implementation,
  onboarding examples, and repository-settings guidance under the qualified CI contract.

## Work Packages

1. Qualify and freeze the consumer CI mappings identified in Scope against the CI evidence matrix;
   record exact identities and feasibility evidence before implementing the affected integration.
2. Deliver caller onboarding and reusable execution for all nine accepted checks, check selection,
   explicit platform selection, concurrent/sequential scheduling, and non-mutating command modes.
3. Wire qualified PR/draft, default-branch, manual, and merge-queue revision handling, supersession,
   fork isolation, and stable aggregate merge gating without changing consumer repository settings.
4. Implement first-party checks, review-thread lifecycle/limit handling, compact PR summaries, and
   optional Code Scanning publication with truthful command, collection, and reporting failures.
5. Exercise the CI matrix through clean external consumers and document setup, reviewed pin updates,
   native reruns, and required repository settings; hand qualified mappings and fixtures to M28.

## Milestone-Specific Evidence

- The [CI matrix](../testing/github-ci.md) has fixture/run evidence for onboarding, all nine checks
  individually and together, defaults/opt-outs/no-ops, explicit platform selections, scheduling,
  isolation, and non-mutation; repository dogfood alone is insufficient.
- Event/revision and merge-gating fixtures cover test-merge failures, base advancement, stale runs,
  merge queues, and fork approval/credential isolation without false aggregate success.
- Reporting fixtures cover deduplication, human-reply preservation, thread cleanup and limit races,
  security-audit rendering, optional Code Scanning, and injected collection/publication failures.

## Out Of Scope

- New languages/tools, release qualification, and publication.

## Completion Report Additions

- Record frozen CI mappings and identities, external-consumer matrix evidence, reporting/security
  race results, onboarding/settings documentation, and the M28 release-requalification handoff.
  Distinguish integration evidence from release-qualified support.
