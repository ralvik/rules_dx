# Runner matrix (Layer-2, #59)

Real `//quality/runner:quality_runner` over provider-less data, one case per
supported language x capability cell x {pass, fail}. Cases live in
`quality/testdata/runner_matrix_cases.bzl` (suite `//quality/testdata:runner_matrix`,
harness `quality/testdata/runner_matrix_tests.bzl`); goldens pin the decoded
`print_result` text byte-for-byte.

Dirty inputs reuse the existing `real_dirty.*` files by direct file label (no
`QualitySourcesInfo`, so the real aspects yield no actions for them) or use
analysis-time-written `generated` bytes. Delegated Clippy/rustc cells inject
recorded upstream diagnostics byte-identical to the parser unit samples
(`quality/adapter/src/parsers/rust.rs`); dx never spawns those tools (#47/#48).

Out of scope by design: `tsc` typechecks `typescript`/`tsx` but is
target-coupled and never runs as a bare backend invocation, so it has no
matrix cell; adapter-less classes (`cc`, `go`, `java`, `kotlin`, …) have
no backing tool and no cells. Scala plus C# plus F# cells below are
opt-in adapters delivered under #797 (successor to closed #417).

Crate edition (issue #468): the two `edition_*` cells above pin the
`--tool-edition` flow over the real toolchain rustfmt. The 2015 cell stays
clean only because the aspect edition reaches `--edition`; the mismatch cell
proves the converse (same bytes under edition 2021 surface syntax errors),
so a single-edition rustfmt stays rejected.

## Rust (`rust`: rustfmt format, clippy lint, rustc typecheck)

| Case | Capability | Input |
| --- | --- | --- |
| `matrix_rust_format_pass` | format | `clean.rs` (toolchain rustfmt) |
| `matrix_rust_format_fail` | format | generated `matrix/rustfmt_dirty.rs` |
| `matrix_rust_format_edition_2015` | format | generated `matrix/rustfmt_edition_2015.rs` (2015-only `async` identifier, edition 2015) |
| `matrix_rust_format_edition_mismatch` | format | generated `matrix/rustfmt_edition_mismatch.rs` (same bytes, edition 2021: syntax errors) |
| `matrix_rust_lint_pass` | lint | generated `matrix/clippy_clean.rs`, empty upstream |
| `matrix_rust_lint_fail` | lint | generated `matrix/clippy_len.rs`, recorded `clippy::len_zero` |
| `matrix_rust_typecheck_pass` | typecheck | generated `matrix/rustc_type_clean.rs`, empty upstream |
| `matrix_rust_typecheck_fail` | typecheck | generated `matrix/rustc_type.rs`, recorded `E0308` |

## Python (`python` + `python_stub`: pydoclint+ruff lint, ruff format, ty typecheck)

| Case | Capability | Input |
| --- | --- | --- |
| `matrix_python_lint_pass` / `matrix_python_lint_fail` | lint | `real_clean.py` / `real_dirty.py` |
| `matrix_python_format_pass` / `matrix_python_format_fail` | format | `real_clean.py` / `real_dirty.py` |
| `matrix_python_typecheck_pass` / `matrix_python_typecheck_fail` | typecheck | `real_clean.py` / `real_dirty.py` |
| `matrix_python_stub_lint_pass` / `matrix_python_stub_lint_fail` | lint | generated `matrix/stub_clean.pyi` / `matrix/stub_dirty.pyi` |
| `matrix_python_stub_format_pass` / `matrix_python_stub_format_fail` | format | generated `matrix/stub_format_clean.pyi` / `matrix/stub_format_dirty.pyi` |
| `matrix_python_stub_typecheck_pass` / `matrix_python_stub_typecheck_fail` | typecheck | generated `matrix/stub_typecheck_clean.pyi` / `matrix/stub_typecheck_dirty.pyi` |
| `matrix_python_ruff_hinted` | lint | `real_clean.py` + `ruff.toml` closure (mirrors `fixture_real_python_hinted`) |
| `matrix_python_flake8_pass` / `matrix_python_flake8_fail` | lint | `real_clean.py` / `real_dirty.py` (flake8 opt-in) |
| `matrix_python_pylint_pass` / `matrix_python_pylint_fail` | lint | `real_clean.py` / `real_dirty.py` (pylint opt-in) |

## JavaScript (`javascript`: biome defaults, eslint opt-in)

| Case | Capability | Input |
| --- | --- | --- |
| `matrix_javascript_lint_pass` / `matrix_javascript_lint_fail` | lint (biome) | `real_clean.js` / `real_dirty.js` |
| `matrix_javascript_format_pass` / `matrix_javascript_format_fail` | format (biome) | `real_clean.js` / `real_dirty.js` |
| `matrix_javascript_biome_hinted` | lint (biome) | `real_clean.js` + `biome_cfg/biome.json` closure |
| `matrix_javascript_eslint_pass` / `matrix_javascript_eslint_fail` | lint (eslint) | `real_clean.js` / `real_dirty.js` + `eslint_cfg` closure |

## TypeScript / JSX / TSX (biome)

| Case | Capability | Input |
| --- | --- | --- |
| `matrix_typescript_lint_pass` / `matrix_typescript_lint_fail` | lint | `real_clean.ts` / `real_dirty.ts` |
| `matrix_typescript_format_pass` / `matrix_typescript_format_fail` | format | `real_clean.ts` / `real_dirty.ts` |
| `matrix_jsx_lint_pass` / `matrix_jsx_lint_fail` | lint | `real_clean.jsx` / generated `matrix/jsx_dirty.jsx` |
| `matrix_jsx_format_pass` / `matrix_jsx_format_fail` | format | `real_clean.jsx` / generated `matrix/jsx_dirty.jsx` |
| `matrix_tsx_lint_pass` / `matrix_tsx_lint_fail` | lint | `real_clean.tsx` / generated `matrix/tsx_dirty.tsx` |
| `matrix_tsx_format_pass` / `matrix_tsx_format_fail` | format | `real_clean.tsx` / generated `matrix/tsx_dirty.tsx` |

## JSON (`json`: biome lint, prettier format)

| Case | Capability | Input |
| --- | --- | --- |
| `matrix_json_lint_pass` | lint | `real_clean.json` |
| `matrix_json_lint_fail` | lint | generated `matrix/json_dup.json` (duplicate key; the compact dirty document is lint-clean by design) |
| `matrix_json_format_pass` / `matrix_json_format_fail` | format | `real_clean.json` / `real_dirty.json` |

## Starlark / TOML (buildifier / taplo lint+format)

| Case | Capability | Input |
| --- | --- | --- |
| `matrix_starlark_lint_pass` / `matrix_starlark_lint_fail` | lint | `real_clean.bzl` / generated `matrix/starlark_dirty.bzl` (`module-docstring` + format convergence) |
| `matrix_starlark_format_pass` / `matrix_starlark_format_fail` | format | `real_clean.bzl` / generated `matrix/starlark_dirty.bzl` |
| `matrix_toml_lint_pass` / `matrix_toml_lint_fail` | lint | `real_clean.toml` / generated `matrix/toml_dirty.toml` (converges via format) |
| `matrix_toml_format_pass` / `matrix_toml_format_fail` | format | `real_clean.toml` / generated `matrix/toml_dirty.toml` |

## Markdown (`markdown`: markdown_check + vale lint)

| Case | Capability | Input |
| --- | --- | --- |
| `matrix_markdown_lint_pass` | lint | `real_clean.md` + `vale_test.ini` closure |
| `matrix_markdown_lint_fail` | lint | generated `matrix/markdown_dirty.md` (unresolved link) + `vale_test.ini` closure |
| `matrix_markdown_sibling_pass` | lint | `sibling_clean.md` + `sibling_license.txt` sibling + `vale_test.ini` closure |

## Scala (`scala`: scalafmt format, scalafix lint)

| Case | Capability | Input |
| --- | --- | --- |
| `matrix_scala_format_pass` / `matrix_scala_format_fail` | format (scalafmt) | generated `matrix/scalafmt_clean.scala` / `matrix/scalafmt_dirty.scala` |
| `matrix_scala_lint_pass` / `matrix_scala_lint_fail` | lint (scalafix) | generated `matrix/scalafix_clean.scala` / recorded Scalafix callback NDJSON |

## C# (`csharp`: csharpier format, roslyn lint)

| Case | Capability | Input |
| --- | --- | --- |
| `matrix_csharp_format_pass` / `matrix_csharp_format_fail` | format (csharpier) | generated `matrix/csharpier_clean.cs` / `matrix/csharpier_dirty.cs` |
| `matrix_csharp_lint_pass` / `matrix_csharp_lint_fail` | lint (roslyn) | generated `matrix/roslyn_clean.cs`, empty SARIF / recorded per-pivot SARIF union |

## F# (`fsharp`: fantomas format, fsharplint lint)

| Case | Capability | Input |
| --- | --- | --- |
| `matrix_fsharp_format_pass` / `matrix_fsharp_format_fail` | format (fantomas) | generated `matrix/fantomas_clean.fs` / `matrix/fantomas_dirty.fs` |
| `matrix_fsharp_lint_pass` / `matrix_fsharp_lint_fail` | lint (fsharplint) | generated `matrix/fsharplint_clean.fs` / recorded library NDJSON |

## Parser samples (issue #465)

Every adapter-backed tool keeps a parser with pass (clean) plus fail (dirty)
samples exercised as unit tests under `quality/adapter/src/parsers/` and pinned by
`bazel run //tools/ci:parser_sample_qualification`: biome lint plus format,
buildifier, clippy plus rustc via the shared rust diagnostics, csharpier,
eslint, fantomas, flake8, fsharplint, markdown_check, prettier, pydoclint,
pylint, roslyn, ruff lint plus format, rustfmt, scalafix, scalafmt,
taplo lint plus format, tsc, ty, vale. Recorded Clippy/rustc diagnostics stay
byte-identical to the parser unit samples (`quality/adapter/src/parsers/rust.rs`);
`tsc` keeps its adapter parser with pass plus fail samples but stays
pipeline-only by design (target-coupled, no runner dispatch, no matrix cell).
Roslyn keeps its SARIF parser with pass plus fail samples and runs delegated
(per-pivot SARIF inputs, no spawn), like Clippy/rustc.

## Adding a cell

Append to the per-class list in `runner_matrix_cases.bzl` with a placeholder
`expected`, then run with `UPDATE_EXPECT=1` to stage the fresh actual
(`bazel test //quality/testdata:<case> --test_env=UPDATE_EXPECT`): the
harness schema-validates the decoded `print_result` first, then writes the
replacement `expected` block plus `$TEST_UNDECLARED_OUTPUTS_DIR/*.update`.
Review the staged shape (producer, capability, stage/count consistency)
before pinning. Prefer reusing `real_clean.*` /
`real_dirty.*` by file label; use `generated` bytes only when no such file
exists. Never add an aspect-visible dirty subject: matrix inputs must stay
provider-less so `dx lint/format/typecheck --check //...` stays fixture-free.
