# M13: Repository-Corpus Quality Closure

## Outcome

Every applicable checked-in file in this repository is Bazel-owned, generated where appropriate, and quality-clean through `dx`.

## Scope

Close all repository corpus ownership, generation, quality policy, documentation, and CI gaps after the
complete Rust foundation. There are no temporary seed checks; adapters land directly.

## Contract References

- [Architecture](../architecture/), [generation](../generation/), [quality](../quality/), and [testing](../testing/).

## Deliverables

- Canonical generated Rust graph and complete repository quality ownership.
- CI that runs generation freshness, quality checks, tests, and tested-stack validation through public paths.

## Work Packages

1. Audit every repository file against ownership and semantic-class registries.
2. Close any overlap where direct-Bazel execution supersedes earlier manual checks.
3. Close deterministic generation, quality, cache, and documentation checks in CI.

## Milestone-Specific Evidence

- A clean checkout has zero unexplained corpus exclusions, stale generated metadata, or duplicate quality execution.
- Narrow source, config, and tested-stack changes invalidate only expected actions.

## Out Of Scope

- New application ecosystems, parity expansion, and release support claims.

## Completion Report Additions

- Record the file/class ownership inventory, justified exclusions, adapter introductions, and invalidation measurements.
