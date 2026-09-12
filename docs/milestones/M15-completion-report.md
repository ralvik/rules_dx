# M15 Completion Report: Python Quality

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed. No capability-term transitions are claimed:
Python quality is adapter-tested first-party implementation under test, not a
product support claim. Dependency-audit CLI behavior and supported status
remain out of scope.

## WP1: Ruff/Ty Artifacts And Private Wheel-Only Graph

Standalone artifacts (generated, `//quality/artifacts:update`): Ruff 0.16.7,
Ty 0.0.80 (`ruff.linux_x86_64.bzl`, `ty.linux_x86_64.bzl`).

Private graph (`//quality/tools/python`, ruleset-owned `pyproject.toml` +
`uv.lock`, single `pypi` hub, shared managed CPython 3.12.13, no ambient
Python, no consumer lock/installer/sdist/compile):

- pydoclint 0.9.1 + click 8.5.0 + docstring-parser-fork 0.0.16 via
  `py_console_script_binary` (SystemExit-preserving).
- flake8 7.3.0 + mccabe 0.7.0 + pycodestyle 2.14.0 + pyflakes 3.4.0 via
  `py_binary` + `sys.exit` wrapper (`flake8_main.py`).
- pylint 4.0.8 + astroid 4.0.4 + dill 0.4.1 + isort 9.0.1 + platformdirs
  4.11.8 + tomlkit 0.15.1 via `py_binary` + `sys.exit` wrapper
  (`pylint_main.py`).

Lock members are pure wheels (`py3-none-any` / `py2.py3-none-any`).
`bazel run //quality/tools/python:{pydoclint,flake8,pylint} -- --version`
reports the pinned versions on CPython 3.12.13. Exit-code contract verified:
flake8 dirty=1 clean=0, pylint dirty=4 (warning mask) clean=0.
`//quality/tools/python:corpus` owns `BUILD.bazel` + `pyproject.toml`;
corpus audit prints nothing.

## WP2: Adapters, Ruff Config Closure, Ty Provider Propagation

Registry (`quality/adapters.bzl`, `REAL_ADAPTERS` authoritative):

- Curated defaults per tool-baseline: `real_python_family` format `[ruff]`,
  lint `[pydoclint, ruff]`, typecheck `[ty]` over `python`/`python_stub`.
- `ruff_config` native-config rule (`.toml` transport, `ruff.toml` /
  `.ruff.toml` basenames, never `pyproject.toml`); hinted fixture proves
  `--config` reaches the tool and `--isolated` is dropped only when hinted.

Backend (`quality/adapter`, `quality/runner:real`):

- Ruff lint (`check --isolated`) / format (`format --check --isolated`),
  fix converges via reread (lint rereads on 0/1, format on 0 only).
- Ty `check --output-format concise --no-respect-ignore-files`,
  workspace-relative paths re-anchored to scratch-absolute.
- pydoclint `--quiet` stderr, `RUNFILES_DIR` runfiles forest, check-only.
- Hermeticity: scratch tree, empty `PATH`, `TMPDIR` set, no VCS observation.

Fixtures (`quality/testdata`): `real_clean.py` (passes all), `real_dirty.py`
(F401, unformatted line, DOC101/DOC103, invalid-argument-type), hinted clean,
`no-typecheck` opt-out. `real_aspect_presence` and `real_typecheck_presence`
pin capability presence.

## WP3: Flake8/Pylint Opt-Ins And Convergence

Opt-ins (selectable, absent from curated defaults, pinned upstream defaults,
no native-config rule, check-only, never rewriting):

- flake8 `--isolated --color=never --jobs=1 --format
  %(path)s:%(row)s:%(col)s:%(code)s:%(text)s`, points, family severity
  (E/F error, W/C warning).
- pylint `--persistent=n --reports=n --score=n --output-format=json
  --jobs=1`, 0-based columns placed 1-based, null end is a point,
  bit-encoded exit mask (findings on nonzero with parsable JSON).
- `REAL_ADAPTERS` lint `["python", "python_stub"]`; lexical stage order
  flake8, pydoclint, pylint, ruff (`real_pipeline_unit` pins).
- Aspect wiring stages tool binaries + runfiles forests with per-tool
  `RUNFILES_DIR`.

Convergence evidence (same dirty fixture):

- flake8: `F401, E231, E225` on dirty, clean empty on exit 0.
- pylint: `W0611` on dirty, `[]` on exit 0.
- Curated lint `.pb` contains ruff + pydoclint only; format `.pb` contains
  ruff `unformatted` only (opt-ins absent unless selected).
- Rust suites: `flake8_reports_and_is_check_only`,
  `pylint_reports_and_is_check_only`, `python_grammar_mismatches_are_fail_closed`,
  `ruff_format_clean_and_unterminated_fix`, `ruff_fix_failure_keeps_input`,
  `ty_output_failure_aborts_diagnose`.

Milestone-specific evidence: no consumer lock/installer/ambient/sdist/compile;
Ruff config locality, Ty target context, VCS isolation, fixability, audit
separation, and runtime sharing pass via the suites and binary probes above.

## Evidence

Exact commands on this host, committed tree:

- `bazel build //...`: success (314 targets).
- `bazel test //...`: 87/87 pass.
- `bazel coverage //...`: 80/80 pass (coverage-instrumented subset).
- Coverage gate (`//tools/coverage:check` over LCOV vs inventory vs
  Bazel-declared `*.rs`/`*.go`): **PASS 24478/24478** executable lines.
- `bazel run //dx:generate_check`: `EXIT=0` (clean).
- `bazel build //quality/testdata:fixture_real_python_dirty_subject
  --output_groups=dx_results`: lint `.pb` has ruff F401 + pydoclint
  DOC101/DOC103, format `.pb` has ruff `unformatted`; no flake8/pylint
  under curated defaults.
- Direct binaries: flake8 7.3.0 / pydoclint 0.9.1 / pylint 4.0.8 /
  astroid 4.0.4 on CPython 3.12.13; Ruff 0.16.7; Ty 0.0.80.

## Changed Components

- `quality/artifacts/` (Ruff/Ty standalone metadata).
- `quality/tools/python/` (private graph, wrappers, corpus).
- `quality/adapter/` (commands/parsers for ruff, ty, pydoclint, flake8,
  pylint).
- `quality/runner/` (real backend dispatch, fixability, hermetic doubles).
- `quality/` (`adapters.bzl`, `real_aspects.bzl`, `real_pipeline_tests.bzl`,
  `BUILD.bazel` curated family, `fixtures.bzl`, `native_config.bzl`).
- `quality/testdata/` (fixtures, presence suites, `ruff.toml`).
- This report.

## Open Items

- Non-Linux hosts, remote execution, and clean external-consumer evidence
  are unproven (same gap class as M00/M12/M14).
- Flake8/pylint remain opt-ins, never defaults; no native-config rules.
- Dependency-audit CLI behavior and supported status are out of scope.
