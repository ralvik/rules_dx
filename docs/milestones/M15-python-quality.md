# M15: Python Quality

## Outcome

Python quality is adapter-tested with curated Ruff, Ty, and pydoclint behavior and optional flake8/pylint parity.

## Scope

Implement Ruff lint/format, Ty typecheck, pydoclint and optional flake8/pylint lint,
Python semantic providers, Ruff native-config generation, and standalone/private-wheel acquisition proofs.

## Contract References

- [Quality](../quality/), [tools](../tools/), [generation](../generation/), [testing](../testing/), and open decisions [O5 and O26](../open-decisions.md).
- [Python baseline](../tools/tool-baseline.md#curated-differences),
  [private ecosystem graphs](../tools/tool-acquisition.md#ruleset-owned-ecosystem-graphs), and
  [shared-runtime requirements](../tools/tool-acquisition.md#shared-runtimes).

## Deliverables

- Python adapter manifests, configs, diagnostic/fix normalization, and curated defaults.
- Standalone Ruff/Ty artifacts and one private wheel-only graph with a shared managed Python runtime.

## Work Packages

1. Prove Ruff/Ty artifacts and the private Python tool graph with pydoclint and another eligible
   baseline Python tool in a compatible runtime cohort under O26. Fixture membership selects no
   new tool or product default and does not depend on M26 audit qualification.
2. Implement adapters, Ruff config closure/binding, and Ty provider propagation.
3. Add flake8 and pylint opt-ins through the proven graph and run cross-tool convergence suites.

## Milestone-Specific Evidence

- Consumers use no tool lock, installer, ambient Python, sdist, or wheel compilation.
- Ruff config locality, Ty target context, VCS isolation, fixability, audit separation, and runtime sharing pass.

## Out Of Scope

- Non-Python parity tools, dependency-audit CLI behavior, and supported status.

## Completion Report Additions

- Record Python tool versions, defaults/opt-ins, lock members, runtime identity, config behavior, and adapter-test matrix.
