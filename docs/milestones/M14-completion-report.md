# M14 Completion Report: Python Application Foundation

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed. No capability-term transitions are claimed:
the foundation is adapter-tested first-party implementation under test, not a
product support claim. Public repository/root/exact-target env planning,
collection, and orchestration remain M25.

## WP1: Upstream Providers, uv Lock, Interpreter, Pytest, Coverage

Pinned `aspect_rules_py 2.0.0-alpha.6` (`MODULE.bazel`), the ADR 0010
prerelease exception. Interpreter 3.12 selected via both
`python_interpreters.toolchain(python_version = "3.12")` and the
`rules_python` 3.12 alignment; `requires-python = ">=3.12"` in
`python/hello/pyproject.toml`. Authoritative uv graph
(`python/hello/pyproject.toml` + `uv.lock`, hub `pypi`): pytest 9.1.1,
coverage 7.16.0, iniconfig 2.3.0, packaging 26.3, pluggy 1.6.0,
pygments 2.21.0 (plus colorama sdist entry).

`python/rules/defs.bzl`: narrow `dx_py_library` / `dx_py_binary` /
`dx_py_test` wrappers. Each creates one private `<name>_dx_upstream`
(`py_library` / `py_binary` / `py_pytest_test`) plus one public
forwarding rule. Libraries preserve `PyInfo` + `PyWheelsInfo` +
`DefaultInfo` + `InstrumentedFilesInfo`; binaries/tests preserve `PyInfo`
through an executable symlink forwarder plus forwarded
`InstrumentedFilesInfo`/`OutputGroupInfo`/`RunEnvironmentInfo`. Each adds
only `QualitySourcesInfo(direct_sources = {"python", "python_stub"})`
normalized from direct `srcs`. No version fields (ADR 0012: unknown
versions fail in upstream toolchain resolution). Tests tag the private
target `manual` so `bazel test //...` exercises the public wrapper only;
the test forwarder carries the `_lcov_merger` magic attribute so Bazel
coverage merges `coverage.dat` instead of exiting after an empty file.

Fixtures: `//python/hello` (library `hello_lib`, binary `hello` owning
`main.py` with `main = "main.py"`, test `hello_test` with
`dep_group = "hello"` and `@pypi//pytest` + `@pypi//coverage` deps) and
`//python/entries` (thin-entry shape: library `main` owns `main.py`,
binary `main_bin` carries only `main` + `deps = [":main"]`, no `srcs`).
`bazel run //python/hello:hello` prints `Hello, world!`;
`bazel run //python/entries:main_bin` prints `entry <x>`;
`bazel test //python/hello:hello_test //python/entries:helper_test`
pass; `bazel coverage //python/...` passes with `coverage.dat` for both
tests (standard Bazel + `py_pytest_test` protocols, no alternate driver).

## WP2: One-Source Libraries, Tests, Entries, Stubs, Strict Resolution, Merge

First-party `gazelle/python` extension (`lang.go`, `parser.go`,
`naming.go`, `stdlib.go`): one reusable `dx_py_library` per supported
non-test `.py` (basename normalizer), `dx_py_test` per `*_test.py` only
(`test_` prefix / directory placement do not create tests), thin
`dx_py_binary <library>_bin` for exactly `main.py` (non-test) carrying
only `main` + `imports = ["."]` + `deps = [":<library>"]`, paired
same-basename `.pyi` attached to its `.py` owner (no target, no edge,
no collision), orphan `.pyi` inert. Every generated rule sets
`imports = ["."]` (mergeable). `ParseImports` covers literal imports plus
`importlib.import_module("name")`; stdlib roots dropped at collection;
every other root resolves strictly or fails (`unresolved import ...
add a local one-source library or an exact # gazelle:resolve mapping`);
computed names are the manual kept boundary. `uv.lock`/`pyproject.toml`
are authoritative for externals; source-only stdlib/local graphs generate
with no ecosystem metadata and no `gazelle_python.yaml` or other sidecar
(anywhere: `find` shows none; generation contract forbids it).

Group/extra semantics per contract: production targets use only the
selected production/default group; tests may additionally use assigned
test/dev groups; optional/extras/isolated groups resolve only when
authoritative config selects them; `ide_groups` are environment-only.
Fixtures use a single `pypi` hub and `dep_group = "hello"`; wrong-scope
imports fail rather than enabling a group (strict resolver, no
group-mutating generation). Exact `# gazelle:dx_ignore_import`
(`python [python] <import>`): ignored literal contributes no edge,
mapping+ignore for the same import fails, stale ignore fails.

Pinned by `lang_test.go` (source-only, entries, strict, ignore,
collision, merge/stale suites) plus golden `testdata/source_only` and
`testdata/entries` (`BUILD.in`/`BUILD.out`); `_test.pyi` paired-stub
coverage included. `//gazelle/python:generation_test` and
`//gazelle/python:python_test` pass.

## WP3: Focused Provider-Derived Environment Plans

`python/env/plan.bzl` (`python_env_plan`) + `aspect.bzl`
(`dx_python_env_wheels_aspect`): for one `dx_py_*` wrapper, reads
analyzed `PyInfo.imports`, `PyInfo.transitive_sources` (first-party
`.py` basenames), `QualitySourcesInfo` direct basenames, and the
aspect-merged `PyWheelsInfo.wheels` postorder closure (wrapper
`upstream` -> private target `venv` -> sibling venv lib `deps` ->
library closure; exec targets carry no `PyWheelsInfo` themselves).
Materializes deterministic JSON + `PythonEnvPlanInfo` +
`DxSubjectInfo`. No checkout scan, no uv re-resolution, no mutation,
no repository/root/exact-target orchestration (M25).

`//python/env:env_plan_tests` (analysis `starlark_test`) pins five
plans: `hello_lib` (direct `hello.py`, 0 wheels), `hello` (direct
`main.py`, transitive `hello.py,main.py`, 0 wheels), `hello_test`
(direct `hello_test.py`, 6 wheels: coverage, iniconfig, packaging,
pluggy, pygments, pytest; transitive includes `pytest_main.py`,
`pytest_shard.py`, launcher env), `main` and thin `main_bin` (direct
empty for the thin binary, transitive `helper.py,main.py`, 0 wheels).
Passes. Laziness: source-only closures project zero wheels and no venv
payload; wheel records appear only through the test's `venv` edge.

## Evidence

Exact commands on this host (2026-09-12), committed tree:

- `bazel build //...`: success (296 targets).
- `bazel test //...`: 87/87 pass, including
  `//gazelle/python:generation_test`, `//gazelle/python:python_test`,
  `//python/hello:hello_test`, `//python/entries:helper_test`,
  `//python/env:env_plan_tests`.
- `bazel coverage //python/... //gazelle/python/...`: 4/4 pass with
  `coverage.dat` for both pytest wrappers.
- `bazel coverage //...`: 80/80 pass.
- Coverage gate (`//tools/coverage:check` over LCOV vs inventory vs
  Bazel-declared `*.rs`/`*.go`): **PASS 23173/23173** executable lines.
- `bazel run //dx:generate_check`: `EXIT=0` (clean).
- `bazel run //python/hello:hello`: `Hello, world!`.
  `bazel run //python/entries:main_bin`: `entry <x>`.
- `bazel build //python/env:hello_lib_plan` JSON matches the pinned
  `plan_tests.bzl` observations verbatim.

## Changed Components

- `MODULE.bazel` (aspect_rules_py pin, 3.12 toolchains, `pypi` hub).
- `python/rules/defs.bzl`, `python/hello/`, `python/entries/`,
  `python/env/` (`plan.bzl`, `aspect.bzl`, `plan_tests.bzl`, BUILD files).
- `gazelle/python/` (extension + tests + golden testdata).
- `docs/generation/python.md` (ignore-exception wording), `tools/coverage/inventory.txt`
  (new Go eligible sources), corpus `BUILD.bazel` ownership for new packages.
- This report.

## Open Items

- O25 mappings remain provisional pending required-platform and
  consumer evidence (prerelease exception per ADR 0010); no supported claim.
- Non-Linux hosts, remote execution, and clean external-consumer evidence
  are unproven (same gap class as M00/M12).
- Python quality tools (M15), broad env/codegen/setup orchestration (M25),
  and audit/update (M26) are out of scope and untouched.
