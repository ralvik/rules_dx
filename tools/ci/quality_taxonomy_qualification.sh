#!/usr/bin/env bash
# Quality family taxonomy execution qualification harness.
#
# Executes the frozen family taxonomy with fixture evidence pinned in
# `quality/tests/fixtures/quality_taxonomy/pins.bzl`
# (plus `quality_taxonomy.expected`), without claiming qualified
# platforms, qualified consumers, qualified releases, or Supported:
# - taxonomy: 47 classes each with exactly one owning family, 39
#   families single-sourced in REAL_CLASS_TO_FAMILY, grouped css/json/
#   python/typescript/javascript/cc by design.
# - curated: 10 families with lazy curated defaults (java
#   google_java_format/checkstyle/pmd/spotbugs, javascript/biome,
#   json biome+prettier, kotlin ktfmt/ktlint, markdown
#   markdown_check+vale, python pydoclint+ruff+ty, rust
#   clippy+rustfmt+rustc, starlark buildifier, toml taplo, typescript
#   biome+tsc); audit stays empty with explicit disablement, Bandit
#   excluded, secrets via Gitleaks separate.
# - backed: 39 adapter-backed classes ride 53 real adapters with
#   runner-matrix pass plus fail plus parser plus native-config plus
#   aspect plus policy execution (JVM java/kotlin delivered under #796;
#   Scala/C# plus F# under #797; C/C++ plus CUDA plus Go under #798;
#   Protobuf/QML under #799; interpreted/file-family under #800).
# - deferred: 8 classes with ADR 0019 owner plus frozen route, no
#   double-claim, no undispositioned; framework regions plus
#   json5/jsonc stay owned under ADR 0019.
# - promotion (issue #802): deferred delivery linked to #796 plus #797
#   plus #798 plus #799 plus #800, digest policy with JVM digests pinned
#   in MODULE.bazel plus standalone per-host digests in quality/artifacts
#   plus rule-sets qualified under #485-489, per-platform plus consumer
#   plus release evidence linked (per-host artifacts plus coverage cells
#   plus CI matrix plus refusal, adopt workspaces plus reusable-consumer
#   workflow, promotion-checklist plus SBOM plus signing plus
#   supported_evidence_gate); support-matrix linkage; backends
#   provisional; no Supported claim.
# - applicability: provider intersect adapter intersect policy, suffix
#   rejected, cross-family union into one stage, lazy with no fetch,
#   no hidden preset; admissibility owner-contextual.
# - registry: single-sourced with versioned v1 schemas, query-only.
# - rejected: taxonomy doc only plus report-not-gate shape only.
# - owned with honest records: deferred adapters delivered under
#   796 plus 797 plus 798 plus 799 plus 800 with remaining owned under
#   ADR 0019 plus 307, digests plus rule-sets owned by cohorts with digest
#   policy plus platform plus consumer plus release evidence linked under
#   #802; backends provisional; no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:quality_taxonomy_qualification`,
# following //tools/ci:result_contract_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="quality/tests/fixtures/quality_taxonomy/pins.bzl"
pins_build="quality/tests/fixtures/quality_taxonomy/BUILD.bazel"
expected="quality/tests/fixtures/quality_taxonomy/quality_taxonomy.expected"
adapters="quality/adapters.bzl"
curated="quality/curated_defaults.bzl"
parity="quality/parity_tests.bzl"
registry="quality/registry.bzl"
sources="quality/sources.bzl"
applicability="quality/applicability.bzl"
pipeline="quality/pipeline.bzl"
native="quality/native_config.bzl"
aspects="quality/real_aspects.bzl"
policy_build="quality/BUILD.bazel"
matrix="quality/testdata/runner_matrix_cases.bzl"
subjects="quality/testdata/BUILD.bazel"
sources_doc="docs/quality/quality-sources.md"
integrations="docs/quality/tool-integrations.md"
testing_doc="docs/quality/quality-testing.md"
action_doc="docs/quality/action-model.md"
support="docs/product/support-matrix.md"
promotion="docs/product/promotion-checklist.md"
acquisition="docs/tools/tool-acquisition.md"
module="MODULE.bazel"
ci=".github/workflows/ci.yml"
build="tools/ci/BUILD.bazel"
targets="tools/ci/ci_targets_d.bzl"
dogfood="tools/ci/dogfood_freshness.sh"
verify="docs/testing/verification-matrix.md"
verify_remaining="docs/testing/verification-matrix-remaining.md"

# Fixture files stay present.
if [[ -f "$pins" && -f "$pins_build" && -f "$expected" ]]; then
  ok
else
  bad "quality taxonomy fixture missing (want $pins plus $pins_build plus quality_taxonomy.expected)"
fi

# Pins record the taxonomy shape with groupings.
if grep -q -F -e '47 classes each with exactly one owning family' "$pins" &&
  grep -q -F -e '39 owning families' "$pins" &&
  grep -q -F -e 'single-sourced in quality/adapters.bzl REAL_CLASS_TO_FAMILY' "$pins" &&
  grep -q -F -e 'css plus less plus scss in the css family' "$pins" &&
  grep -q -F -e 'json plus json5 plus jsonc in the json family' "$pins" &&
  grep -q -F -e 'python plus python_stub in the python family' "$pins" &&
  grep -q -F -e 'typescript plus tsx in the typescript family' "$pins" &&
  grep -q -F -e 'javascript plus jsx in the javascript family' "$pins" &&
  grep -q -F -e 'c plus cpp in the cc family' "$pins"; then
  ok
else
  bad "pins.bzl lost its taxonomy shape plus groupings under issue #512"
fi

# Pins record the curated plus backed execution (10 curated with JVM
# java/kotlin delivered under #796; 39 backed with 53 adapters covering
# #796 plus #797 plus #798 plus #799 plus #800).
if grep -q -F -e '10 curated families: java plus javascript plus json plus kotlin plus markdown plus python plus rust plus starlark plus toml plus typescript' "$pins" &&
  grep -q -F -e 'java family format google_java_format plus lint checkstyle plus pmd plus spotbugs' "$pins" &&
  grep -q -F -e 'kotlin family format ktfmt plus lint ktlint' "$pins" &&
  grep -q -F -e 'javascript family lint biome plus format biome' "$pins" &&
  grep -q -F -e 'json family lint biome plus format prettier' "$pins" &&
  grep -q -F -e 'markdown family lint markdown_check plus vale' "$pins" &&
  grep -q -F -e 'python family lint pydoclint plus ruff plus format ruff plus typecheck ty' "$pins" &&
  grep -q -F -e 'rust family lint clippy plus format rustfmt plus typecheck rustc' "$pins" &&
  grep -q -F -e '39 adapter-backed classes:' "$pins" &&
  grep -q -F -e '53 real adapters:' "$pins"; then
  ok
else
  bad "pins.bzl lost its curated plus backed execution under issue #512 plus #796 plus #802"
fi

# Pins record the deferred plus audit plus rejected plus owned gaps
# (8 deferred; delivery owned under 796-800 plus ADR 0019 plus 307 with
# promotion linked under #802).
if grep -q -F -e '8 deferred classes with owner plus frozen route' "$pins" &&
  grep -q -F -e 'every deferral names ADR 0019 plus frozen delivery route' "$pins" &&
  grep -q -F -e 'no class is both adapter-backed and deferred' "$pins" &&
  grep -q -F -e 'curated audit stays empty with explicit disablement' "$pins" &&
  grep -q -F -e 'Bandit excluded from v1 by ADR 0019' "$pins" &&
  grep -q -F -e 'secrets family rides Gitleaks detect with redact plus SARIF' "$pins" &&
  grep -q -F -e '"taxonomy doc only"' "$pins" &&
  grep -q -F -e 'deferred adapters delivered under 796 plus 797 plus 798 plus 799 plus 800 with remaining owned under ADR 0019 plus 307' "$pins" &&
  grep -q -F -e 'digests plus rule-sets stay owned by their cohort qualifications' "$pins" &&
  grep -q -F -e 'platform plus consumer plus release evidence stays owned gap with support-matrix linkage under issue 802' "$pins" &&
  grep -q -F -e 'platform plus consumer plus release evidence stays owned gap' "$pins" &&
  grep -q -F -e 'no Supported claim' "$pins" &&
  grep -q -F -e 'backends stay provisional' "$pins"; then
  ok
else
  bad "pins.bzl lost its deferred plus audit plus rejected plus owned gaps under issue #512 plus #802"
fi

# Pins record the promotion evidence (digest policy plus platform plus
# consumer plus release linkage under #802).
if grep -q -F -e 'digest policy with JVM digests pinned in MODULE.bazel' "$pins" &&
  grep -q -F -e 'standalone per-host digests in quality/artifacts' "$pins" &&
  grep -q -F -e 'rule-sets qualified under 485 through 489' "$pins" &&
  grep -q -F -e 'platform evidence with per-host artifacts plus coverage cells plus CI matrix plus unsupported_platform refusal' "$pins" &&
  grep -q -F -e 'consumer evidence with adopt workspaces plus reusable-consumer workflow' "$pins" &&
  grep -q -F -e 'release evidence with promotion-checklist plus SBOM plus signing plus supported_evidence_gate linkage' "$pins"; then
  ok
else
  bad "pins.bzl lost its digest plus platform plus consumer plus release promotion pins under issue #802"
fi

# Frozen taxonomy stays single-sourced with groupings.
if grep -q -F -e 'REAL_CLASS_TO_FAMILY = {' "$adapters" &&
  grep -q -F -e 'REAL_ADAPTERS = {' "$adapters" &&
  grep -q -F -e '"css": "css"' "$adapters" &&
  grep -q -F -e '"less": "css"' "$adapters" &&
  grep -q -F -e '"scss": "css"' "$adapters" &&
  grep -q -F -e '"json": "json"' "$adapters" &&
  grep -q -F -e '"json5": "json"' "$adapters" &&
  grep -q -F -e '"jsonc": "json"' "$adapters" &&
  grep -q -F -e '"python": "python"' "$adapters" &&
  grep -q -F -e '"python_stub": "python"' "$adapters" &&
  grep -q -F -e '"typescript": "typescript"' "$adapters" &&
  grep -q -F -e '"tsx": "typescript"' "$adapters" &&
  grep -q -F -e '"javascript": "javascript"' "$adapters" &&
  grep -q -F -e '"jsx": "javascript"' "$adapters" &&
  grep -q -F -e '"c": "cc"' "$adapters" &&
  grep -q -F -e '"cpp": "cc"' "$adapters"; then
  ok
else
  bad "frozen taxonomy lost its single-sourced shape plus groupings"
fi

# Curated defaults stay exact with empty audit plus frozen formatters
# (10 families with JVM java/kotlin delivered under #796).
if grep -q -F -e '"java": {' "$curated" &&
  grep -q -F -e '"kotlin": {' "$curated" &&
  grep -q -F -e '"javascript": {' "$curated" &&
  grep -q -F -e '"json": {' "$curated" &&
  grep -q -F -e '"markdown": {' "$curated" &&
  grep -q -F -e '"python": {' "$curated" &&
  grep -q -F -e '"rust": {' "$curated" &&
  grep -q -F -e '"starlark": {' "$curated" &&
  grep -q -F -e '"toml": {' "$curated" &&
  grep -q -F -e '"typescript": {' "$curated" &&
  grep -q -F -e '"audit": []' "$curated" &&
  grep -q -F -e '"javascript": ["biome"]' "$curated" &&
  grep -q -F -e '"json": ["prettier"]' "$curated" &&
  grep -q -F -e '"python": ["ruff"]' "$curated" &&
  grep -q -F -e '"rust": ["rustfmt"]' "$curated" &&
  grep -q -F -e '"java": ["google_java_format"]' "$curated" &&
  grep -q -F -e '"kotlin": ["ktfmt"]' "$curated"; then
  ok
else
  bad "curated defaults lost their 10-family exact shape with empty audit plus frozen formatters"
fi

# Parity deferrals stay owned with fail-closed gate shape.
# Scala/.NET delivered under #797 plus JVM delivered under #796 plus native
# delivered under #798 (including CUDA via clang-format) plus Structured
# delivered under #799 plus interpreted/file-family delivered under #800,
# so the remaining deferred are framework regions plus json5/jsonc.
if grep -q -F -e 'PARITY_DEFERRED = {' "$parity" &&
  grep -q -F -e '"astro": ["ADR 0019"' "$parity" &&
  grep -q -F -e '"graphql": ["ADR 0019"' "$parity" &&
  grep -q -F -e 'no adapter class is unclassified' "$parity" &&
  grep -q -F -e 'no classified class lacks a disposition' "$parity" &&
  grep -q -F -e 'no class is both adapter-backed and deferred' "$parity"; then
  ok
else
  bad "parity deferrals lost their owned routes plus fail-closed gate shape"
fi

# Deferred delivery: JVM java/kotlin ride six adapters delivered under
# #796 with curated plus native plus matrix evidence; remaining deferred
# classes stay owned under #800 plus #307 (issue #802 promotion).
if grep -q -F -e '"google_java_format": {"format": ["java"]}' "$adapters" &&
  grep -q -F -e '"checkstyle": {"lint": ["java"]}' "$adapters" &&
  grep -q -F -e '"pmd": {"lint": ["java"]}' "$adapters" &&
  grep -q -F -e '"spotbugs": {"lint": ["java"]}' "$adapters" &&
  grep -q -F -e '"ktfmt": {"format": ["kotlin"]}' "$adapters" &&
  grep -q -F -e '"ktlint": {"lint": ["kotlin"]}' "$adapters" &&
  grep -q -F -e '"java": "java"' "$adapters" &&
  grep -q -F -e '"kotlin": "kotlin"' "$adapters" &&
  ! grep -q -F -e '"java":' "$parity" &&
  ! grep -q -F -e '"kotlin":' "$parity"; then
  ok
else
  bad "taxonomy lost its JVM java/kotlin deferred-delivery record (want six adapters plus no parity deferral under #796 plus #802)"
fi

# Deferred delivery linkage: cohort harnesses own #796 plus #797 plus #798
# plus #799 plus #800 (issue #802 promotion, still seed-only). Verify text
# is split across lines in the matrix, so match line-safe fragments.
if [[ -f "tools/ci/jvm_cohort_qualification.sh" && -f "tools/ci/scala_dotnet_adapters_qualification.sh" && -f "tools/ci/native_adapters_qualification.sh" && -f "tools/ci/structured_adapters_qualification.sh" && -f "tools/ci/file_family_adapters_qualification.sh" ]] &&
  grep -q -F -e 'JVM delivered under #796' "$verify" &&
  grep -q -F -e 'Scala/.NET Layer-2 delivered under #797' "$verify" &&
  grep -q -F -e 'Native Layer-2 delivered' "$verify" &&
  grep -q -F -e 'under #798 plus Structured Protobuf/QML delivered under #799' "$verify"; then
  ok
else
  bad "taxonomy lost its #796 plus #797 plus #798 plus #799 plus #800 deferred-delivery linkage under issue #802"
fi

# Registry stays single-sourced with versioned v1 schemas.
if grep -q -F -e 'Single-sourced versioned registry queries' "$registry" &&
  grep -q -F -e 'REGISTRY_SCHEMA_VERSION = 1' "$registry" &&
  grep -q -F -e 'SOURCES_REGISTRY_SCHEMA_VERSION' "$registry" &&
  grep -q -F -e 'ADAPTER_REGISTRY_SCHEMA_VERSION' "$registry" &&
  grep -q -F -e 'CURATED_SCHEMA_VERSION' "$registry" &&
  grep -q -F -e 'PARITY_SCHEMA_VERSION' "$registry" &&
  grep -q -F -e 'registry_schema_error' "$registry" &&
  grep -q -F -e 'ADAPTER_REGISTRY_SCHEMA_VERSION = 1' "$adapters" &&
  grep -q -F -e 'SOURCES_REGISTRY_SCHEMA_VERSION = 1' "$sources"; then
  ok
else
  bad "registry lost its single-sourced plus versioned v1 shape"
fi

# Sources plus admissibility stay owner-contextual.
if grep -q -F -e 'KNOWN_SEMANTIC_FILE_CLASSES = [' "$sources" &&
  grep -q -F -e '"rust",' "$sources" &&
  grep -q -F -e '"python",' "$sources" &&
  grep -q -F -e '"typescript",' "$sources" &&
  grep -q -F -e 'def effective_classes' "$applicability" &&
  grep -q -F -e 'Returns the sorted three-way class intersection' "$applicability" &&
  grep -q -F -e 'Raw extension matching is registry validation' "$integrations" &&
  grep -q -F -e 'never infer custom-rule sources' "$sources_doc" &&
  grep -q -F -e '`BUILD` files admissible by basename' "$sources_doc" &&
  grep -q -F -e 'alone cannot decide `c` versus `cpp`' "$sources_doc" &&
  grep -q -F -e 'Extensionless requires shell rule/provider evidence' "$sources_doc" &&
  grep -q -F -e 'Requires template provider; bare `.html` without it is `html`' "$sources_doc" &&
  grep -q -F -e 'never inferred as fallback' "$sources_doc"; then
  ok
else
  bad "sources plus admissibility lost their owner-contextual pins"
fi

# Runner matrix executes the backed classes with pass plus fail.
# Per-cohort cells live in their split files; the aggregator carries the
# case-list symbols. JVM java/kotlin cells delivered under #796 ride the
# taxonomy promotion in #802.
if grep -q -F -e 'matrix_rust_format_pass' quality/testdata/runner_matrix_rust.bzl &&
  grep -q -F -e 'matrix_python_lint_pass' quality/testdata/runner_matrix_python.bzl &&
  grep -q -F -e 'matrix_javascript_lint_pass' quality/testdata/runner_matrix_js.bzl &&
  grep -q -F -e 'matrix_typescript_lint_pass' quality/testdata/runner_matrix_ts.bzl &&
  grep -q -F -e 'matrix_json_lint_pass' quality/testdata/runner_matrix_json.bzl &&
  grep -q -F -e 'matrix_starlark_lint_pass' quality/testdata/runner_matrix_data.bzl &&
  grep -q -F -e 'matrix_toml_lint_pass' quality/testdata/runner_matrix_data.bzl &&
  grep -q -F -e 'matrix_markdown_lint_pass' quality/testdata/runner_matrix_markdown.bzl &&
  grep -q -F -e 'matrix_java_format_pass' quality/testdata/runner_matrix_jvm.bzl &&
  grep -q -F -e 'matrix_kotlin_format_pass' quality/testdata/runner_matrix_jvm.bzl &&
  grep -q -F -e 'matrix_scala_format_pass' quality/testdata/runner_matrix_scala_dotnet.bzl &&
  grep -q -F -e 'matrix_c_format_pass' quality/testdata/runner_matrix_native.bzl &&
  grep -q -F -e 'matrix_go_format_pass' quality/testdata/runner_matrix_native.bzl &&
  grep -q -F -e 'matrix_protobuf_format_pass' quality/testdata/runner_matrix_structured.bzl &&
  grep -q -F -e 'matrix_qml_format_pass' quality/testdata/runner_matrix_structured.bzl &&
  grep -q -F -e 'matrix_cue_format_pass' quality/testdata/runner_matrix_file_family.bzl &&
  grep -q -F -e 'matrix_shell_lint_pass' quality/testdata/runner_matrix_file_family.bzl &&
  grep -q -F -e 'NATIVE_CASES' "$matrix" &&
  grep -q -F -e 'SCALA_DOTNET_CASES' "$matrix" &&
  grep -q -F -e 'STRUCTURED_CASES' "$matrix" &&
  grep -q -F -e 'FILE_FAMILY_CASES' "$matrix" &&
  grep -q -F -e '_fail' quality/testdata/runner_matrix_native.bzl &&
  grep -q -F -e '_pass' quality/testdata/runner_matrix_native.bzl &&
  grep -q -F -e 'replacement ' quality/testdata/runner_matrix_native.bzl; then
  ok
else
  bad "runner matrix lost its backed-class pass plus fail execution"
fi

# Parsers plus native plus aspects plus policy execute the taxonomy
# (JVM checkstyle native binding delivered under #796 rides #802).
if [[ -f "quality/adapter/src/parsers/biome.rs" ]] &&
  [[ -f "quality/adapter/src/parsers/ruff.rs" ]] &&
  [[ -f "quality/adapter/src/parsers/prettier.rs" ]] &&
  [[ -f "quality/adapter/src/parsers/checkstyle.rs" ]] &&
  grep -q -F -e 'parse_clippy' quality/adapter/src/parsers/rust.rs &&
  grep -q -F -e 'parse_rustc' quality/adapter/src/parsers/rust.rs &&
  grep -q -F -e 'biome_config' "$native" &&
  grep -q -F -e 'checkstyle_config' "$native" &&
  grep -q -F -e 'ruff_config' "$native" &&
  grep -q -F -e 'vale_config' "$native" &&
  grep -q -F -e 'taplo_config' "$native" &&
  grep -q -F -e 'buildifier_config' "$native" &&
  grep -q -F -e 'eslint_config' "$native" &&
  grep -q -F -e 'upstream-delegated' "$adapters" &&
  grep -q -F -e 'target-coupled' "$adapters" &&
  grep -q -F -e 'real_fixture_policy' "$policy_build" &&
  grep -q -F -e 'union' "$pipeline" &&
  grep -q -F -e 'cross-family union' "$pins"; then
  ok
else
  bad "parsers plus native plus aspects plus policy lost their taxonomy execution"
fi

# Digest policy: JVM digests pinned in MODULE.bazel plus standalone
# per-host digests in quality/artifacts plus rule-sets qualified under
# #485-489 with digest self-reference policy owned by tool-acquisition
# (issue #802 promotion, still seed-only).
if grep -q -F -e 'name = "jvm_google_java_format"' "$module" &&
  grep -q -F -e 'name = "jvm_checkstyle"' "$module" &&
  grep -q -F -e 'name = "jvm_ktfmt"' "$module" &&
  grep -q -F -e 'name = "jvm_ktlint"' "$module" &&
  grep -q -F -e 'sha256' "$module" &&
  [[ -f "quality/artifacts/ruff.linux_x86_64.bzl" && -f "quality/artifacts/biome.linux_x86_64.bzl" && -f "quality/artifacts/buildifier.linux_x86_64.bzl" ]] &&
  grep -q -F -e 'sha256' quality/artifacts/ruff.linux_x86_64.bzl &&
  grep -q -F -e 'digests pinned in `MODULE.bazel`' "$integrations" &&
  grep -q -F -e 'avoids digest self-reference' "$acquisition"; then
  ok
else
  bad "taxonomy lost its digest policy (want JVM MODULE.bazel plus standalone per-host digests plus self-reference policy under issue #802)"
fi

# Platform evidence linkage: per-host artifacts plus coverage cells plus CI
# matrix plus clean refusal (issue #802 promotion, still seed-executed).
if grep -q -F -e 'unsupported_platform' cli/cli/src/platform.rs &&
  grep -q -F -e 'build-arm64' "$ci" &&
  grep -q -F -e 'build-windows-x86_64' "$ci" &&
  grep -q -F -e 'no cross-cell union' docs/testing/strategy-details.md; then
  ok
else
  bad "taxonomy lost its platform evidence linkage (want refusal plus per-host CI plus no-union under issue #802)"
fi

# Consumer evidence linkage: adopt workspaces plus reusable workflow
# (issue #802 promotion, still seed-only).
if [[ -d "examples/adopt-rust" && -d "examples/adopt-python" ]] &&
  [[ -f ".github/workflows/reusable-consumer.yml" ]] &&
  grep -q -F -e 'adopt-rust' "$verify"; then
  ok
else
  bad "taxonomy lost its consumer evidence linkage (want adopt plus reusable-consumer under issue #802)"
fi

# Release evidence linkage: promotion checklist plus SBOM plus signing plus
# supported gate with no Supported claim (issue #802 promotion).
if grep -q -F -e 'Promotion Checklist' "$promotion" &&
  grep -q -F -e 'bazel run //tools/ci:supported_evidence_gate' "$promotion" &&
  [[ -f "tools/ci/supported_evidence_gate.sh" ]] &&
  grep -q -F -e 'sbom-provenance' "$ci" &&
  ! grep -q -E -e '^\| .* \| Supported' "$support"; then
  ok
else
  bad "taxonomy lost its release evidence linkage (want checklist plus SBOM plus gate with no Supported under issue #802)"
fi

# Support matrix owns the qualified taxonomy record with honest gaps
# plus the #802 promotion linkage.
if grep -q -F -e 'qualified seed-only under closed #512' "$support" &&
  grep -q -F -e 'quality_taxonomy_qualification' "$support" &&
  grep -q -F -e 'quality/tests/fixtures/quality_taxonomy/pins.bzl' "$support" &&
  grep -q -F -e 'taxonomy doc only rejected' "$support" &&
  grep -q -F -e 'promotion gaps linked under #802' "$support" &&
  ! grep -q -E -e '^\| .* \| Supported' "$support"; then
  ok
else
  bad "docs/product/support-matrix.md lost its qualified taxonomy record with doc-only rejection plus #802 linkage under closed #512"
fi

# Quality docs own the qualified record; audit stays explicit.
# Promotion gaps stay linked under #802.
if grep -q -F -e 'qualified seed-only under issue #512' "$sources_doc" &&
  grep -q -F -e 'quality_taxonomy_qualification' "$sources_doc" &&
  grep -q -F -e 'qualified seed-only under issue #512' "$integrations" &&
  grep -q -F -e 'quality_taxonomy_qualification' "$integrations" &&
  grep -q -F -e 'qualified seed-only under issue #512' "$action_doc" &&
  grep -q -F -e 'quality_taxonomy_qualification' "$action_doc" &&
  grep -q -F -e 'Bandit excluded from v1' "$action_doc" &&
  grep -q -F -e '#802' "$sources_doc" &&
  grep -q -F -e '#802' "$integrations"; then
  ok
else
  bad "quality docs lost their qualified taxonomy record plus Bandit honesty plus #802 linkage under issue #512"
fi

# Verification matrix owns the qualified seed-only record plus #802
# promotion linkage.
if grep -q -F -e 'quality_taxonomy_qualification' "$verify" &&
  grep -q -F -e 'qualified seed-only under closed #512' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:quality_taxonomy_qualification' "$verify" &&
  grep -q -F -e '`quality_taxonomy_qualification` 25/25' "$verify" &&
  grep -q -F -e 'promotion gaps #802' "$verify" &&
  ! grep -E -e '^\|.*\| *`?Supported`? *\|' "$verify" | grep -q .; then
  ok
else
  bad "verification-matrix lost its #512 qualified taxonomy record plus #802 linkage or gained Supported"
fi

# Targets own the harness plus dogfood wires it.
if grep -q -F -e 'name = "quality_taxonomy_qualification"' "$targets" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:quality_taxonomy_qualification' "$dogfood"; then
  ok
else
  bad "ci_targets or dogfood lost the quality_taxonomy_qualification wiring (want target plus dogfood-freshness)"
fi

# Fixture expected covers the taxonomy with owned gaps plus #802
# promotion linkage.
if grep -q -F -e 'qualified seed-only under issue #512' "$expected" &&
  grep -q -F -e 'taxonomy doc only' "$expected" &&
  grep -q -F -e '47 classes' "$expected" &&
  grep -q -F -e '8 deferred' "$expected" &&
  grep -q -F -e '39 adapter-backed classes' "$expected" &&
  grep -q -F -e '53 real adapters' "$expected" &&
  grep -q -F -e 'Bandit excluded' "$expected" &&
  grep -q -F -e 'owned gap' "$expected" &&
  grep -q -F -e 'under issue #802' "$expected"; then
  ok
else
  bad "quality_taxonomy.expected lost taxonomy coverage (want qualified plus doc-only plus 47 plus 8 plus 39 plus 53 plus Bandit plus owned gap plus #802, issue #512)"
fi

# Verification-remaining mirrors the #802 promotion record.
if grep -q -F -e 'quality_taxonomy_qualification' "$verify_remaining" &&
  grep -q -F -e '#802' "$verify_remaining"; then
  ok
else
  bad "verification-matrix-remaining lost its #802 taxonomy promotion record"
fi

# Live proof: quality suites plus the fixture build stay green.
if bazel test //quality:all --noshow_progress >/dev/null 2>&1 &&
  bazel build //quality/tests/fixtures/quality_taxonomy/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "quality taxonomy live shapes failed (want //quality:all plus fixture green, issue #512)"
fi

dx_test_summary "quality taxonomy qualification harness"
