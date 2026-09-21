# ADR 0026: Rust Product Code Boundary And Migration Umbrella

## Status

Accepted. The boundary below plus the `//tools/ci:product_runtime_guards`
fail-closed gate are decided constraints. Archiver/hasher is delivered Rust;
the five remaining migration slices stay provisional phase indexes, each
needing its own accepted successor record before its rule-kind change lands.

## Context

Repo-owned product logic is split across Python (`py_binary`) and bash
(`sh_binary`/`sh_test`) where Rust would be cross-platform and faster.
Shell, not Python, is the platform blocker: the managed 3.12 toolchain runs
everywhere while bash harnesses stay Linux-only with Windows `shell: bash`
(see [Tool And Platform Test Matrix](../testing/tools.md#shell-and-host-tool-contract)).

The as-built product inventory is the hermetic tools
(`deploy/rules:archiver`/`hasher` as Rust `rust_binary` plus
`npm_packer` as `py_binary`,
`deploy/release:sbom_spdx_gen`/`sbom_prov_gen`/`bcr_source_gen`,
`tools/bazelrc:preset.update`, `tools/depcheck` checker,
`quality/artifacts:update`), the per-instance deploy launcher `py_binary`
programs expanded by `archive`/`github`/`pypi`/`crates`/`npm`/`nuget`/`maven`/`oci`/`octopus`
(see [Deploy Authoring](../deploy/authoring.md)), the `bcr`/`signing`
`sh_binary` launchers, the `dx_verify` installer verifier, and the POSIX
fixtures (`env/tool.sh`, `env/doctor.sh`, deploy fixtures) plus the CI
`tools/ci` drivers and `tools/sh` helpers. The starting audit is tracked
under the umbrella issue; counts moved since (depcheck split, archive/github
hermetic `py_binary` per the deploy launcher direction).

Some Python and shell must stay permanently. The toolchain
(`MODULE.bazel` `aspect_rules_py` plus `rules_python`, managed 3.12) follows
[ADR 0010](./0010-python-foundation.md). Fixtures, examples, and testdata
(`python/tests/fixtures/hello/`, `tools/depcheck/testdata/python/`,
`examples/adopt-python/`) plus upstream linters (flake8, pylint, pydoclint,
yamllint, djlint) plus the `flake8_main.py`/`pylint_main.py` shims (they call
into Python libraries Rust cannot import) stay. Bazel-imposed shell
(`rules_shell`, genrule `cmd`, Windows `shell: bash`) plus Starlark (`.bzl`)
stay regardless.

Without an umbrella, per-phase work collides with existing trackers or gets
re-litigated per PR: shell-harness elimination under #667, archive/github
launcher hermetic direction under #748, deploy macros no-shell hermetic
programs under #723-#730, helper/guard/dedupe slices under
#651/#653/#654/#749/#750, platform qualification under #410-#414.

## Decision

Product logic migrates to Rust; the must-stay list above never migrates.
Concretely:

- New product `py_binary`/`sh_binary` is rejected without a decision: the
  `//tools/ci:product_runtime_guards` gate pins the current product
  `py_binary`/`sh_binary` allowlist (direct BUILD targets plus deploy-macro
  `.bzl` rule kinds) plus the must-stay pins. A new product interpreter
  target or a new macro rule kind fails the gate until this record (or its
  accepted successor) is updated. `rust_binary` remains the migration
  direction; the gate allows it and only pins the Python/shell ceiling.
- The phase index is: archiver/hasher tools (delivered Rust under #760),
  plus provisional `preset.py`,
  depcheck checker, SBOM/BCR generators, deploy launcher programs, and the
  deferred CI-drivers plus `quality/artifacts/update.py` stance (defers to
  #667; no harness-wide Rust-ify). Each phase links its owning contract
  (`deploy/rules` plus [Deploy Authoring](../deploy/authoring.md),
  `tools/bazelrc` preset, `tools/depcheck`, `deploy/release` SBOM/BCR,
  deploy macros) and lands only with an accepted successor record plus its
  guard update. No consumer API changes until a launcher rule kind changes.
- Collision scope stays narrow: #667 owns harness strategy, #748 owns the
  archive/github `py_binary` direction (phases extend it to `rust_binary`
  without closing it), #723-#730 own the no-shell macro direction,
  #651/#653/#654/#749/#750 own helpers, #410-#414 own qualification. This
  work feeds qualification (native `rust_test` runs everywhere) but claims
  none itself.

## Consequences

- `docs/testing/tools.md` links this boundary; the guard owns the pin.
- Phases shrink the allowlist slice by slice; the gate stays green by
  updating the pinned names plus counts together with each accepted phase.
- CI drivers plus `quality/artifacts/update.py` stay Python/shell until #667
  decides otherwise; the guard pins that deferral instead of migrating it.

## Rejected Alternatives

- Keep `py_binary`/`sh_binary` product code and fix portability within shell
  (POSIX plus `shell: bash` on Windows): rejected, preserves
  interpreter/startup cost and Linux-only pins where Rust is cross-platform.
- Harness-wide Rust-ify of `tools/ci` drivers in this umbrella: rejected,
  owned by #667; this umbrella scopes the Rust-product-code slice only.
