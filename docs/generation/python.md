# Python Generation Contract

Python follows the [common contract](common.md). See
[ADR 0010](../decisions/0010-python-foundation.md) and
[ADR 0015](../decisions/0015-first-party-gazelle-extensions.md) for rationale and the
[generation test matrix](../testing/generation.md#generate-command-and-gazelle-extensions) for
required evidence.

## Sources And Ownership

Runtime discovery covers `.py`. Every supported non-test source receives one reusable one-source
`python_library`, named from the source basename without `.py` using the common normalizer. Imports
between local sources become library edges, regardless of importer count; importing a module from a
test does not move or duplicate that module.

A same-directory, same-basename `.pyi` attaches to its `.py` owner as type metadata through the
stable upstream shape. It creates no target, runtime dependency owner, test, binary, entry point, or
name collision. Stub imports participate only in the supported type-analysis path. An orphan `.pyi`
is inert until a separately accepted stub-only ownership model exists.

Executable modules use the common single-library-owner and thin-binary shape. The recognized
executable entry is exactly `main.py` (non-test): the library owns the source and its
source-derived dependencies, and the thin `dx_py_binary` `<library>_bin` carries only `main`,
`imports = ["."]`, and `deps = [":<library>"]` with no `srcs`. `__main__.py`, `__main__` guards,
and manifest console scripts are not automatic recognition. Resources remain
user-owned under the [common resource boundary](common.md#resources).

## Tests

A `.py` source is a test only when its basename ends in `_test` immediately before `.py`. A `test_`
prefix, test-directory placement, pytest configuration, and test functions or classes do not create
automatic test ownership: such files receive ordinary non-test library ownership. Broader
ecosystem conventions remain O22/O25 qualification, not automatic recognition.

Each recognized source receives its own independently runnable `python_test`, named from the source
basename without `.py`. Test-only references attach only to that target; imported non-test modules
retain their ordinary library owners. `python_test` uses pytest and Bazel's standard test and coverage
protocols rather than a generic-main or alternate-driver switch.

## uv Scope And Resolution

`pyproject.toml`, `uv.lock`, and public metadata from the pinned `aspect_rules_py` integration are
authoritative for project and external-package metadata. A wheel-derived import-to-distribution index
may be a Bazel-owned generation input, but is never copied into the source tree as
`gazelle_python.yaml` or another integration sidecar.

Production targets may use only their selected production/default group. Tests may additionally use
the test or development groups assigned to that target. Optional dependencies, extras, and isolated
uv groups resolve only when authoritative target configuration already selects them. Repository
`ide_groups` are environment-only and never make an import build-admissible. Wrong-scope imports fail;
generation never enables or moves a dependency.

The extension honors the common exact `# gazelle:dx_ignore_import` exception
(`python [python] <import>`): an ignored literal contributes no edge, a mapping
plus an ignore for the same import fails, and an ignore matching no literal
reference fails as stale. The extension never generates mappings or ignores.

Ordinary imports and recognized literal runtime loads, including
`importlib.import_module("name")`, use strict common resolution. Computed module names remain the
manual kept-dependency boundary. Python files using only standard-library and local imports generate
without `pyproject.toml` or `uv.lock`.
