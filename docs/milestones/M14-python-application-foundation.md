# M14: Python Application Foundation

## Outcome

Python applications generate, build, test, cover, and produce provider-derived IDE environment plans
and focused projections from authoritative Bazel and uv metadata.

## Scope

Add pinned `aspect_rules_py` 2.x wrappers, pytest/coverage, the first-party Python Gazelle extension,
strict imports and groups, `.py`/paired `.pyi` ownership, entries, toolchain versions, and provider-derived
`.venv` plans/projections for focused language targets. Public repository/root/exact-target env planning,
collection, and orchestration remain in M25.

## Contract References

- [Python decision](../decisions/0010-python-foundation.md), [toolchain versions](../decisions/0012-language-toolchain-versions.md), [generation](../generation/), [environments](../environments/), [testing](../testing/), and open decision [O25](../open-decisions.md).

## Deliverables

- Python library, binary, and pytest wrappers with provider preservation.
- Strict Python Gazelle extension, Bazel-owned wheel/import metadata, and symlink-only `.venv` projection.

## Work Packages

1. Prove upstream providers, uv lock, interpreter selection, pytest, and coverage.
2. Implement one-source libraries, tests, entries, paired stubs, strict import/group resolution, and merge.
3. Project interpreter, wheel, source, and generated roots into immutable Python environments.

## Milestone-Specific Evidence

- Source-only local graphs and authoritative uv graphs generate without guessed dependencies or sidecars.
- Required-platform focused-target fixtures prove pytest/coverage, version selection, group isolation,
  provider-derived `.venv` imports/projections, and laziness without public env orchestration.

## Out Of Scope

- Python quality tools and broad repository env/codegen/setup orchestration.

## Completion Report Additions

- Record Python versions, provider mappings, generated shapes, uv/group behavior, stub behavior, and `.venv` layout.
