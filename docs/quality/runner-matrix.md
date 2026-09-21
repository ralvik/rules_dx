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
matrix cell; `spotbugs` lints `java` but is target-coupled (needs
`JavaInfo` classes, dropped for provider-less fixtures), so it has no
matrix cell. Scala plus C# plus F# cells below are opt-in adapters
delivered under #797 (successor to closed #417), Java plus Kotlin cells
are opt-in adapters delivered under #796 (successor to closed #416),
C plus C++ plus Go cells are opt-in adapters delivered under #798
(successor to closed #418), Protobuf plus QML cells below are
opt-in adapters delivered under #799 (successor to closed #419), and
Interpreted/file-family cells below are opt-in adapters delivered
under #800 (successor to closed #420).

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

## Python (`python` + `python_stub`: pydoclint+ruff lint, ruff format, ty typecheck, ruff S audit opt-in under #801)

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

## Java (`java`: google-java-format format, checkstyle plus pmd lint; spotbugs target-coupled)

| Case | Capability | Input |
| --- | --- | --- |
| `matrix_java_format_pass` / `matrix_java_format_fail` | format | `real_clean.java` / `real_dirty.java` |
| `matrix_java_checkstyle_pass` / `matrix_java_checkstyle_fail` | lint (checkstyle) | `real_clean.java` / `real_dirty.java` + `checkstyle_cfg` closure |
| `matrix_java_pmd_pass` / `matrix_java_pmd_fail` | lint (pmd) | `real_clean.java` / `real_dirty.java` (upstream quickstart default) |

## Kotlin (`kotlin`: ktfmt format, ktlint lint)

| Case | Capability | Input |
| --- | --- | --- |
| `matrix_kotlin_format_pass` / `matrix_kotlin_format_fail` | format | `real_clean.kt` / `real_dirty.kt` |
| `matrix_kotlin_lint_pass` | lint | generated `matrix/Hello.kt` (filename plus expression-body clean) |
| `matrix_kotlin_lint_fail` | lint | `real_dirty.kt` (filename plus indent plus expression-body plus spacing) |

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

## C (`c`: clang-format format, clang-tidy lint)

| Case | Capability | Input |
| --- | --- | --- |
| `matrix_c_format_pass` / `matrix_c_format_fail` | format (clang-format) | generated `matrix/clang_format_clean.c` / `matrix/clang_format_dirty.c` |
| `matrix_c_lint_pass` / `matrix_c_lint_fail` | lint (clang-tidy) | generated `matrix/clang_tidy_clean.c` / recorded text diagnostics |

## C++ (`cpp`: clang-format format, cppcheck lint)

| Case | Capability | Input |
| --- | --- | --- |
| `matrix_cpp_format_pass` / `matrix_cpp_format_fail` | format (clang-format) | generated `matrix/clang_format_clean.cpp` / `matrix/clang_format_dirty.cpp` |
| `matrix_cpp_lint_pass` / `matrix_cpp_lint_fail` | lint (cppcheck) | generated `matrix/cppcheck_clean.c` / recorded XML diagnostics |

## Go (`go`: gofumpt format, staticcheck/govet/errcheck lint)

| Case | Capability | Input |
| --- | --- | --- |
| `matrix_go_format_pass` / `matrix_go_format_fail` | format (gofumpt) | generated `matrix/gofumpt_clean.go` / `matrix/gofumpt_dirty.go` |
| `matrix_go_staticcheck_pass` / `matrix_go_staticcheck_fail` | lint (staticcheck) | generated `matrix/staticcheck_clean.go` / recorded JSON diagnostics |
| `matrix_go_govet_pass` / `matrix_go_govet_fail` | lint (govet) | generated `matrix/govet_clean.go` / recorded text diagnostics |
| `matrix_go_errcheck_pass` / `matrix_go_errcheck_fail` | lint (errcheck) | generated `matrix/errcheck_clean.go` / recorded text diagnostics |

## Protobuf (`protobuf`: buf format plus lint)

| Case | Capability | Input |
| --- | --- | --- |
| `matrix_protobuf_format_pass` / `matrix_protobuf_format_fail` | format (buf) | generated `matrix/buf_clean.proto` / `matrix/buf_dirty.proto` |
| `matrix_protobuf_lint_pass` / `matrix_protobuf_lint_fail` | lint (buf) | generated `matrix/buf_lint_clean.proto` / recorded buf JSONL |

## QML (`qml`: qmlformat format, qmllint lint)

| Case | Capability | Input |
| --- | --- | --- |
| `matrix_qml_format_pass` / `matrix_qml_format_fail` | format (qmlformat) | generated `matrix/qmlformat_clean.qml` / `matrix/qmlformat_dirty.qml` |
| `matrix_qml_lint_pass` / `matrix_qml_lint_fail` | lint (qmllint) | generated `matrix/qmllint_clean.qml` / recorded qmllint JSON |

## Interpreted/file-family (`cue`, `jsonnet`, `pkl`, `go_module`, `terraform`, `html_template`, `css`/`less`/`scss`, `gherkin`, `sql`, `xml`, `yaml`, `text`, `shell`, `ruby`, `powershell`: opt-in adapters delivered under #800, successor to closed #420)

| Case | Capability | Input |
| --- | --- | --- |
| `matrix_cue_format_pass` / `matrix_cue_format_fail` | format (cue) | generated `matrix/cue_clean.cue` / `matrix/cue_dirty.cue` |
| `matrix_jsonnet_format_pass` / `matrix_jsonnet_format_fail` | format (jsonnetfmt) | generated `matrix/jsonnet_clean.jsonnet` / `matrix/jsonnet_dirty.jsonnet` |
| `matrix_pkl_format_pass` / `matrix_pkl_format_fail` | format (pkl) | generated `matrix/pkl_clean.pkl` / `matrix/pkl_dirty.pkl` |
| `matrix_go_module_format_pass` / `matrix_go_module_format_fail` | format (modfmt) | generated `matrix/modfmt_clean.mod` / `matrix/modfmt_dirty.mod` |
| `matrix_terraform_format_pass` / `matrix_terraform_format_fail` | format (terraform) | generated `matrix/terraform_clean.tf` / `matrix/terraform_dirty.tf` |
| `matrix_yaml_format_pass` / `matrix_yaml_format_fail` | format (yamlfmt) | generated `matrix/yamlfmt_clean.yaml` / `matrix/yamlfmt_dirty.yaml` |
| `matrix_shell_format_pass` / `matrix_shell_format_fail` | format (shfmt) | generated `matrix/shfmt_clean.sh` / `matrix/shfmt_dirty.sh` |
| `matrix_ruby_format_pass` / `matrix_ruby_format_fail` | format (standardrb) | generated `matrix/standardrb_clean.rb` / `matrix/standardrb_dirty.rb` |
| `matrix_html_template_format_pass` / `matrix_html_template_format_fail` | format (djlint) | generated `matrix/djlint_format_clean.html` / `matrix/djlint_format_dirty.html` |
| `matrix_css_format_pass` / `matrix_css_format_fail` | format (prettier) | generated `matrix/prettier_css_clean.css` / `matrix/prettier_css_dirty.css` |
| `matrix_less_format_pass` / `matrix_less_format_fail` | format (prettier) | generated `matrix/prettier_less_clean.less` / `matrix/prettier_less_dirty.less` |
| `matrix_scss_format_pass` / `matrix_scss_format_fail` | format (prettier) | generated `matrix/prettier_scss_clean.scss` / `matrix/prettier_scss_dirty.scss` |
| `matrix_gherkin_format_pass` / `matrix_gherkin_format_fail` | format (prettier) | generated `matrix/prettier_gherkin_clean.feature` / `matrix/prettier_gherkin_dirty.feature` |
| `matrix_sql_format_pass` / `matrix_sql_format_fail` | format (prettier) | generated `matrix/prettier_sql_clean.sql` / `matrix/prettier_sql_dirty.sql` |
| `matrix_xml_format_pass` / `matrix_xml_format_fail` | format (prettier) | generated `matrix/prettier_xml_clean.xml` / `matrix/prettier_xml_dirty.xml` |
| `matrix_html_template_lint_pass` / `matrix_html_template_lint_fail` | lint (djlint) | generated `matrix/djlint_clean.html` / recorded text diagnostics |
| `matrix_css_lint_pass` / `matrix_css_lint_fail` | lint (stylelint) | generated `matrix/stylelint_clean.css` / recorded JSON diagnostics |
| `matrix_less_lint_pass` / `matrix_less_lint_fail` | lint (stylelint) | generated `matrix/stylelint_clean.less` / recorded JSON diagnostics |
| `matrix_scss_lint_pass` / `matrix_scss_lint_fail` | lint (stylelint) | generated `matrix/stylelint_clean.scss` / recorded JSON diagnostics |
| `matrix_ruby_lint_pass` / `matrix_ruby_lint_fail` | lint (rubocop) | generated `matrix/rubocop_clean.rb` / recorded JSON diagnostics |
| `matrix_powershell_lint_pass` / `matrix_powershell_lint_fail` | lint (psscriptanalyzer) | generated `matrix/psscriptanalyzer_clean.ps1` / recorded text diagnostics |
| `matrix_yaml_lint_pass` / `matrix_yaml_lint_fail` | lint (yamllint) | generated `matrix/yamllint_clean.yaml` / recorded text diagnostics |
| `matrix_shell_lint_pass` / `matrix_shell_lint_fail` | lint (shellcheck) | generated `matrix/shellcheck_clean.sh` / recorded gcc diagnostics |
| `matrix_text_lint_pass` / `matrix_text_lint_fail` | lint (keep_sorted) | generated `matrix/keep_sorted_clean.txt` / recorded text diagnostics |

## Parser samples (issue #465)

Every adapter-backed tool keeps a parser with pass (clean) plus fail (dirty)
samples exercised as unit tests under `quality/adapter/src/parsers/` and pinned by
`bazel run //tools/ci:parser_sample_qualification`: biome lint plus format,
buf lint plus format, buildifier, checkstyle, clang_format, clang_tidy,
clippy plus rustc via the shared rust diagnostics, cppcheck, csharpier,
cue, djlint lint plus format, errcheck, eslint, fantomas, flake8, fsharplint,
gofumpt, google-java-format, govet, jsonnetfmt, keep_sorted, ktfmt, ktlint,
markdown_check, modfmt, pkl, pmd, prettier, psscriptanalyzer, pydoclint, pylint,
qmlformat, qmllint, roslyn, rubocop,
ruff lint plus format, rustfmt, scalafix, scalafmt, shared SARIF for the
JVM lint cohort plus spotbugs, shellcheck, shfmt, standardrb, staticcheck, stylelint,
taplo lint plus format, terraform, tsc, ty, vale, yamlfmt, yamllint. Recorded Clippy/rustc diagnostics stay
byte-identical to the parser unit samples (`quality/adapter/src/parsers/rust.rs`);
`tsc` keeps its adapter parser with pass plus fail samples but stays
pipeline-only by design (target-coupled, no runner dispatch, no matrix cell).
Roslyn keeps its SARIF parser with pass plus fail samples and runs delegated
(per-pivot SARIF inputs, no spawn), like Clippy/rustc. Buf lint plus qmllint
keep their JSON parsers with pass plus fail samples and run delegated
(recorded upstream diagnostics, no spawn) in the matrix, like Clippy/rustc;
in production they spawn the pinned binaries.

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
