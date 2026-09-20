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
# - curated: 8 families with lazy curated defaults (javascript/biome,
#   json biome+prettier, markdown markdown_check+vale, python
#   pydoclint+ruff+ty, rust clippy+rustfmt+rustc, starlark buildifier,
#   toml taplo, typescript biome+tsc); audit stays empty with explicit
#   disablement, Bandit excluded, secrets via Gitleaks separate.
# - backed: 11 adapter-backed classes ride 16 real adapters with
#   runner-matrix pass plus fail plus parser plus native-config plus
#   aspect plus policy execution.
# - deferred: 36 classes with ADR 0019 owner plus frozen route, no
#   double-claim, no undispositioned.
# - applicability: provider intersect adapter intersect policy, suffix
#   rejected, cross-family union into one stage, lazy with no fetch,
#   no hidden preset; admissibility owner-contextual.
# - registry: single-sourced with versioned v1 schemas, query-only.
# - rejected: taxonomy doc only plus report-not-gate shape only.
# - owned with honest records: deferred adapters owned under
#   416-420 plus 307, digests plus rule-sets owned by cohorts,
#   platform plus consumer plus release evidence stays owned gap;
#   backends provisional; no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:quality_taxonomy_qualification`,
# following //tools/ci:result_contract_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

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
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
verify="docs/testing/verification-matrix.md"

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

# Pins record the curated plus backed execution.
if grep -q -F -e '8 curated families: javascript plus json plus markdown plus python plus rust plus starlark plus toml plus typescript' "$pins" &&
  grep -q -F -e 'javascript family lint biome plus format biome' "$pins" &&
  grep -q -F -e 'json family lint biome plus format prettier' "$pins" &&
  grep -q -F -e 'markdown family lint markdown_check plus vale' "$pins" &&
  grep -q -F -e 'python family lint pydoclint plus ruff plus format ruff plus typecheck ty' "$pins" &&
  grep -q -F -e 'rust family lint clippy plus format rustfmt plus typecheck rustc' "$pins" &&
  grep -q -F -e '11 adapter-backed classes:' "$pins" &&
  grep -q -F -e '16 real adapters:' "$pins"; then
  ok
else
  bad "pins.bzl lost its curated plus backed execution under issue #512"
fi

# Pins record the deferred plus audit plus rejected plus owned gaps.
if grep -q -F -e '36 deferred classes with owner plus frozen route' "$pins" &&
  grep -q -F -e 'every deferral names ADR 0019 plus frozen delivery route' "$pins" &&
  grep -q -F -e 'no class is both adapter-backed and deferred' "$pins" &&
  grep -q -F -e 'curated audit stays empty with explicit disablement' "$pins" &&
  grep -q -F -e 'Bandit excluded from v1 by ADR 0019' "$pins" &&
  grep -q -F -e 'secrets family rides Gitleaks detect with redact plus SARIF' "$pins" &&
  grep -q -F -e '"taxonomy doc only"' "$pins" &&
  grep -q -F -e 'deferred adapters stay owned under issues 416 through 420 plus 307' "$pins" &&
  grep -q -F -e 'platform plus consumer plus release evidence stays owned gap' "$pins" &&
  grep -q -F -e 'no Supported claim' "$pins" &&
  grep -q -F -e 'backends stay provisional' "$pins"; then
  ok
else
  bad "pins.bzl lost its deferred plus audit plus rejected plus owned gaps under issue #512"
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

# Curated defaults stay exact with empty audit plus frozen formatters.
if grep -q -F -e '"javascript": {' "$curated" &&
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
  grep -q -F -e '"rust": ["rustfmt"]' "$curated"; then
  ok
else
  bad "curated defaults lost their 8-family exact shape with empty audit plus frozen formatters"
fi

# Parity deferrals stay owned with fail-closed gate shape.
if grep -q -F -e 'PARITY_DEFERRED = {' "$parity" &&
  grep -q -F -e '"c": ["ADR 0019"' "$parity" &&
  grep -q -F -e '"cpp": ["ADR 0019"' "$parity" &&
  grep -q -F -e '"java": ["ADR 0019"' "$parity" &&
  grep -q -F -e '"csharp": ["ADR 0019"' "$parity" &&
  grep -q -F -e '"go": ["ADR 0019"' "$parity" &&
  grep -q -F -e '"protobuf": ["ADR 0019"' "$parity" &&
  grep -q -F -e '"qml": ["ADR 0019"' "$parity" &&
  grep -q -F -e '"shell": ["ADR 0019"' "$parity" &&
  grep -q -F -e '"text": ["ADR 0019"' "$parity" &&
  grep -q -F -e '"yaml": ["ADR 0019"' "$parity" &&
  grep -q -F -e 'no adapter class is unclassified' "$parity" &&
  grep -q -F -e 'no classified class lacks a disposition' "$parity" &&
  grep -q -F -e 'no class is both adapter-backed and deferred' "$parity"; then
  ok
else
  bad "parity deferrals lost their owned routes plus fail-closed gate shape"
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
if grep -q -F -e 'matrix_rust_format_pass' "$matrix" &&
  grep -q -F -e 'matrix_python_lint_pass' "$matrix" &&
  grep -q -F -e 'matrix_javascript_lint_pass' "$matrix" &&
  grep -q -F -e 'matrix_typescript_lint_pass' "$matrix" &&
  grep -q -F -e 'matrix_json_lint_pass' "$matrix" &&
  grep -q -F -e 'matrix_starlark_lint_pass' "$matrix" &&
  grep -q -F -e 'matrix_toml_lint_pass' "$matrix" &&
  grep -q -F -e 'matrix_markdown_lint_pass' "$matrix" &&
  grep -q -F -e '_fail' "$matrix" &&
  grep -q -F -e '_pass' "$matrix" &&
  grep -q -F -e 'replacement ' "$matrix"; then
  ok
else
  bad "runner matrix lost its backed-class pass plus fail execution"
fi

# Parsers plus native plus aspects plus policy execute the taxonomy.
if [[ -f "quality/adapter/src/parsers/biome.rs" ]] &&
  [[ -f "quality/adapter/src/parsers/ruff.rs" ]] &&
  [[ -f "quality/adapter/src/parsers/prettier.rs" ]] &&
  grep -q -F -e 'parse_clippy' quality/adapter/src/parsers/rust.rs &&
  grep -q -F -e 'parse_rustc' quality/adapter/src/parsers/rust.rs &&
  grep -q -F -e 'biome_config' "$native" &&
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

# Support matrix owns the qualified taxonomy record with honest gaps.
if grep -q -F -e 'qualified seed-only under issue #512' "$support" &&
  grep -q -F -e 'quality_taxonomy_qualification' "$support" &&
  grep -q -F -e 'quality/tests/fixtures/quality_taxonomy/pins.bzl' "$support" &&
  grep -q -F -e 'taxonomy doc only rejected' "$support" &&
  ! grep -q -E -e '^\| .* \| Supported' "$support"; then
  ok
else
  bad "docs/product/support-matrix.md lost its qualified taxonomy record with doc-only rejection under issue #512"
fi

# Quality docs own the qualified record; audit stays explicit.
if grep -q -F -e 'qualified seed-only under issue #512' "$sources_doc" &&
  grep -q -F -e 'quality_taxonomy_qualification' "$sources_doc" &&
  grep -q -F -e 'qualified seed-only under issue #512' "$integrations" &&
  grep -q -F -e 'quality_taxonomy_qualification' "$integrations" &&
  grep -q -F -e 'qualified seed-only under issue #512' "$action_doc" &&
  grep -q -F -e 'quality_taxonomy_qualification' "$action_doc" &&
  grep -q -F -e 'Bandit excluded from v1' "$action_doc"; then
  ok
else
  bad "quality docs lost their qualified taxonomy record plus Bandit honesty under issue #512"
fi

# Verification matrix owns the qualified seed-only record.
if grep -q -F -e 'quality_taxonomy_qualification' "$verify" &&
  grep -q -F -e 'qualified seed-only under #512' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:quality_taxonomy_qualification' "$verify" &&
  grep -q -F -e '`quality_taxonomy_qualification` 17/17' "$verify" &&
  ! grep -E -e '^\|.*\| *`?Supported`? *\|' "$verify" | grep -q .; then
  ok
else
  bad "verification-matrix lost its #512 qualified taxonomy record or gained Supported"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "quality_taxonomy_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:quality_taxonomy_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the quality_taxonomy_qualification wiring (want target plus dogfood-freshness)"
fi

# Fixture expected covers the taxonomy with owned gaps.
if grep -q -F -e 'qualified seed-only under issue #512' "$expected" &&
  grep -q -F -e 'taxonomy doc only' "$expected" &&
  grep -q -F -e '47 classes' "$expected" &&
  grep -q -F -e '36 deferred' "$expected" &&
  grep -q -F -e 'Bandit excluded' "$expected" &&
  grep -q -F -e 'owned gap' "$expected"; then
  ok
else
  bad "quality_taxonomy.expected lost taxonomy coverage (want qualified plus doc-only plus 47 plus 36 plus Bandit plus owned gap, issue #512)"
fi

# Live proof: quality suites plus the fixture build stay green.
if bazel test //quality:all --noshow_progress >/dev/null 2>&1 &&
  bazel build //quality/tests/fixtures/quality_taxonomy/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "quality taxonomy live shapes failed (want //quality:all plus fixture green, issue #512)"
fi

dx_test_summary "quality taxonomy qualification harness"
