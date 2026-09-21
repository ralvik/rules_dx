#!/usr/bin/env bash
# Docs-pipeline qualification harness (live successor to closed #581).
#
# Qualifies the as-built docs-pipeline record with fixture evidence and
# owned gaps, without claiming a published site:
# - delivered: versioned IR schema (dx.documentation.v1, schema_major 1),
#   documentation_ir codec (validate/encode/decode with roundtrip,
#   rejection-parity, ordering, minor-forward-compat), dx_docs planning
#   library (version/identity/validation/mode/drift/guide/site planning
#   with unit tests), frozen design contracts (IR + site), removed dx docs
#   stub behind ADR 0020, plus fixture-scale renderer/site execution as
#   Bazel-cached extract to aggregate to render (`docs/site`: mdBook-compatible
#   prose plus generated API pages plus one search index, generated IR in
#   Bazel outputs only, seed-only under #780), no Supported claim;
# - open under #779 plus #781-#785 with honest records: 13 per-language adapter runs with
#   pins/mappings (Scala TASTy proof spike first, Astro/MDX prose-only),
#   site-level byte-identical rebuild proof (codec same-producer proof is
#   delivered, site-level is not), link/reference completeness at the
#   pre-render boundary, guide prose plus guide-step CI wiring, first-hour
#   timing proof, per-release pin-bump plus drift process, dx docs
#   reintroduction per ADR 0006 build-vs-validation split, reusable-docs
#   plus caller staying product surface.
#
# Versioned here, run by CI via `bazel run //tools/ci:docs_pipeline_qualification`,
# following //tools/ci:env_codegen_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

proto="docs/ir/doc_ir.proto"
codec="docs/ir/ir/src/lib.rs"
codec_build="docs/ir/ir/BUILD.bazel"
planning="cli/docgen/src/lib.rs"
planning_build="cli/docgen/BUILD.bazel"
schema="cli/schema/src/lib.rs"
readme="docs/documentation/README.md"
docir="docs/documentation/doc-ir.md"
site="docs/documentation/site.md"
stub="docs/cli/commands/docs.md"
adr20="docs/decisions/0020-remove-dx-docs-placeholder.md"
scope="docs/product/scope.md"
roadmap="docs/roadmap.md"
matrix="docs/testing/verification-matrix.md"
support="docs/product/support-matrix.md"
cli_errors="cli/cli/src/args/error.rs"
backlog_guards="tools/ci/backlog_automation_guards.sh"
reusable=".github/workflows/reusable-docs.yml"
caller="examples/docs-ci/caller.yml"
site_bzl="docs/site/site.bzl"
site_build="docs/site/BUILD.bazel"
site_tests="docs/site/site_tests.bzl"
site_demo_symbols="docs/site/demo/demo.symbols.txt"
site_demo_prose="docs/site/demo/demo_prose.md"
site_demo_book="docs/site/demo/book.toml"
site_pins="tools/ci/tests/fixtures/docs_site/pins.bzl"
site_expected="tools/ci/tests/fixtures/docs_site/docs_site.expected"
site_fixture_build="tools/ci/tests/fixtures/docs_site/BUILD.bazel"

# IR schema identity stays dx.documentation.v1 with v1 enums and messages.
if grep -q -F -e 'package dx.documentation.v1;' "$proto" &&
  grep -q -F -e 'enum SymbolKind' "$proto" &&
  grep -q -F -e 'enum Visibility' "$proto" &&
  grep -q -F -e 'message DocIr' "$proto" &&
  grep -q -F -e 'message Symbol' "$proto" &&
  grep -q -F -e 'message Extension' "$proto"; then
  ok
else
  bad "doc_ir.proto lost its dx.documentation.v1 identity or v1 messages"
fi

# Proto keeps reserved ranges plus same-producer byte-identical ordering rule.
if grep -q -F -e 'reserved 12 to 15;' "$proto" &&
  grep -q -F -e 'reserved 6 to 15;' "$proto" &&
  grep -q -F -e 'same-producer rebuilds stay byte-identical' "$proto" &&
  grep -q -F -e 'strictly increasing key order' "$proto"; then
  ok
else
  bad "doc_ir.proto lost its reserved ranges or byte-identical ordering rule"
fi

# Codec crate delivers validate/encode/decode with shared schema major.
if grep -q -F -e 'pub fn validate_shard' "$codec" &&
  grep -q -F -e 'pub fn encode_shard' "$codec" &&
  grep -q -F -e 'pub fn decode_shard' "$codec" &&
  grep -q -F -e 'pub use dx_schema::SCHEMA_MAJOR' "$codec" &&
  grep -q -F -e 'pub const SCHEMA_MAJOR: u32 = 1;' "$schema"; then
  ok
else
  bad "documentation_ir codec lost validate/encode/decode or SCHEMA_MAJOR pin"
fi

# Codec tests prove roundtrip, rejection parity, ordering, minor compat.
if grep -q -F -e 'documented_example_roundtrips_byte_identical' "$codec" &&
  grep -q -F -e 'encode_and_decode_reject_the_same_invalid_shards' "$codec" &&
  grep -q -F -e 'extension_keys_must_be_strictly_increasing' "$codec" &&
  grep -q -F -e 'symbols_must_be_strictly_increasing' "$codec" &&
  grep -q -F -e 'newer_minors_decode_when_understood' "$codec" &&
  grep -q -F -e 'Same producer plus same inputs rebuild byte-identical' "$codec"; then
  ok
else
  bad "documentation_ir codec lost its roundtrip/parity/ordering/compat tests"
fi

# Codec BUILD keeps library plus unit, fmt, and clippy tests.
if grep -q -F -e 'name = "documentation_ir"' "$codec_build" &&
  grep -q -F -e 'name = "documentation_ir_test"' "$codec_build" &&
  grep -q -F -e 'name = "documentation_ir_fmt_test"' "$codec_build" &&
  grep -q -F -e 'name = "documentation_ir_clippy_test"' "$codec_build"; then
  ok
else
  bad "docs/ir/ir BUILD lost its library/test/fmt/clippy targets"
fi

# Planning library delivers version, identity, validation, mode, drift,
# guide, and site planning entry points.
if grep -q -F -e 'pub fn plan_version_compat' "$planning" &&
  grep -q -F -e 'pub fn plan_symbol_id' "$planning" &&
  grep -q -F -e 'pub fn plan_overload_id' "$planning" &&
  grep -q -F -e 'pub fn plan_validation' "$planning" &&
  grep -q -F -e 'pub fn plan_docs_mode' "$planning" &&
  grep -q -F -e 'pub fn mode_selects_render' "$planning" &&
  grep -q -F -e 'pub fn plan_mode_actions' "$planning" &&
  grep -q -F -e 'pub fn plan_drift_upgrade' "$planning" &&
  grep -q -F -e 'pub fn plan_guide_freshness' "$planning" &&
  grep -q -F -e 'pub fn is_known_guide' "$planning" &&
  grep -q -F -e 'pub fn same_producer_requires_byte_equality' "$planning" &&
  grep -q -F -e 'pub fn plan_cache_miss' "$planning"; then
  ok
else
  bad "dx_docs planning lost its version/identity/validation/mode/drift/guide/site entry points"
fi

# Planning hermeticity and lifecycle invariants stay pinned.
if grep -q -F -e 'pub fn docs_actions_use_network' "$planning" &&
  grep -q -F -e 'pub fn check_uses_separate_graph' "$planning" &&
  grep -q -F -e 'pub fn serve_is_build_action' "$planning" &&
  grep -q -F -e 'pub fn emits_partial_shards' "$planning" &&
  grep -q -F -e 'pub fn docs_build_mutates_sources' "$planning" &&
  grep -q -F -e 'pub fn commits_ir_shards' "$planning" &&
  grep -q -F -e 'pub fn drift_reaches_users_without_release' "$planning"; then
  ok
else
  bad "dx_docs planning lost its hermeticity/lifecycle invariants"
fi

# Planning unit tests cover compat, identity, validation, mode, drift,
# guide, site, byte-equality, cache, scope, and serve.
if grep -q -F -e 'same_major_is_compatible_in_either_minor_direction' "$planning" &&
  grep -q -F -e 'major_skew_requires_the_recorded_migration' "$planning" &&
  grep -q -F -e 'zero_major_fails_closed' "$planning" &&
  grep -q -F -e 'validation_emits_only_when_every_gate_holds' "$planning" &&
  grep -q -F -e 'check_flag_selects_validation_only_build_validates_and_renders' "$planning" &&
  grep -q -F -e 'drift_upgrades_ship_only_when_green_and_reviewed' "$planning" &&
  grep -q -F -e 'guide_freshness_requires_every_step_executed_and_green_examples' "$planning" &&
  grep -q -F -e 'site_actions_are_hermetic_and_deterministic_by_construction' "$planning" &&
  grep -q -F -e 'byte_equality_holds_only_for_the_same_pinned_producer' "$planning" &&
  grep -q -F -e 'cache_miss_re_executes_never_fails_freshness' "$planning"; then
  ok
else
  bad "dx_docs planning lost its compat/validation/mode/drift/guide/site unit tests"
fi

# Planning crate keeps its BUILD wiring.
if grep -q -F -e 'name = "dx_docs"' "$planning_build" &&
  grep -q -F -e 'package_name = "cli/dx_docs"' "$planning_build"; then
  ok
else
  bad "cli/docgen BUILD lost its dx_docs crate wiring"
fi

# Thirteen adapter scopes stay documented with provisional inputs and no
# execution claim.
if grep -q -F -e '| Rust | Pinned nightly' "$docir" &&
  grep -q -F -e '| Python | Griffe model' "$docir" &&
  grep -q -F -e '| TypeScript/JavaScript | TypeDoc JSON' "$docir" &&
  grep -q -F -e '| Java | Custom Javadoc Doclet' "$docir" &&
  grep -q -F -e '| Kotlin | Dokka model' "$docir" &&
  grep -q -F -e '| Go | `go/packages`' "$docir" &&
  grep -q -F -e '| C/C++ | Doxygen XML' "$docir" &&
  grep -q -F -e '| C# | Assembly metadata' "$docir" &&
  grep -q -F -e '| F# | Compiler-service metadata' "$docir" &&
  grep -q -F -e '| Vue | `vue-docgen-api` JSON' "$docir" &&
  grep -q -F -e '| Svelte | `sveld` JSON' "$docir" &&
  grep -q -F -e '| Scala | Scala 3 TASTy Inspector' "$docir" &&
  grep -q -F -e '| Astro/MDX | None; prose-only' "$docir" &&
  grep -q -F -e 'Accepted scope covers thirteen adapter scopes' "$docir" &&
  grep -q -F -e 'No adapter execution exists today' "$docir"; then
  ok
else
  bad "doc-ir lost its thirteen adapter scopes or no-execution honesty"
fi

# Per-language overload, join, and packaging details stay tracked under
# , never claimed as delivered.
if grep -q -F -e 'tracked under' "$docir" &&
  grep -q -F -e '#581' "$docir" &&
  grep -q -F -e 'The exact' "$docir" &&
  grep -q -F -e 'disambiguation scheme per language' "$docir" &&
  grep -q -F -e 'the adapter must' "$docir" &&
  grep -q -F -e 'join metadata with documentation' "$docir"; then
  ok
else
  bad "doc-ir lost its per-language overload/join/packaging #581 tracker"
fi

# Site build keeps the decided mdBook renderer with no replacement and the
# delivered seed-only execution record under #780.
if grep -q -F -e 'mdBook is the decided renderer' "$site" &&
  grep -q -F -e 'There is no planned replacement' "$site" &&
  grep -q -F -e 'renderer/site execution delivered seed-only under #780' "$site" &&
  grep -q -F -e 'Fixture-scale site execution is qualified seed-only; no published site is claimed' "$site" &&
  grep -q -F -e 'one DocsExtract action per (language, package) unit' "$site" &&
  grep -q -F -e 'one DocsAggregate action' "$site" &&
  grep -q -F -e 'one DocsRender action (pinned mdBook artifact)' "$site"; then
  ok
else
  bad "site build lost its mdBook decision or delivered-execution record (#780)"
fi

# Determinism stays a design requirement with byte-identical rebuild
# evidence open, never an assumed property.
if grep -q -F -e 'Outputs are designed to be deterministic' "$site" &&
  grep -q -F -e 'Determinism is a' "$site" &&
  grep -q -F -e 'design requirement; byte-identical rebuild evidence remains open' "$site" &&
  grep -q -F -e 'Byte-identical rebuild evidence (two builds, diffed) is required' "$site" &&
  grep -q -F -e 'same_producer_requires_byte_equality' "$planning"; then
  ok
else
  bad "site determinism lost its design-requirement plus open-evidence record"
fi

# Laziness and freshness keep no-committed-IR plus no-source-write honesty
# with delivered fixture execution under #780.
if grep -q -F -e 'IR shards, render inputs, and rendered HTML are ordinary generated Bazel artifacts' "$site" &&
  grep -q -F -e 'not committed files or source-adjacent snapshots' "$site" &&
  grep -q -F -e 'never write generated IR beside source' "$site" &&
  grep -q -F -e 'Delivered (seed-only fixture execution under #780)' "$site" &&
  grep -q -F -e 'The planned [`dx docs --check`]' "$site"; then
  ok
else
  bad "site lost its laziness/freshness no-committed-IR record (#780)"
fi

# Link/reference completeness at the pre-render boundary stays an owned gap.
if grep -q -F -e 'Completeness of required link/reference checks' "$site" &&
  grep -q -F -e 'at the pre-render boundary remains a gap (#782, successor to closed #581)' "$site" &&
  grep -q -F -e 'link/reference completeness' "$site"; then
  ok
else
  bad "site lost its link/reference pre-render completeness gap (#782)"
fi

# Guide-step CI wiring plus first-hour timing stay owned gaps with no
# working-site claim.
if grep -q -F -e 'guide-step CI wiring' "$site" &&
  grep -q -F -e 'first-hour timing proof' "$site" &&
  grep -q -F -e 'guide-step verification' "$matrix" &&
  grep -q -F -e 'first-hour timing proof' "$matrix" &&
  grep -q -F -e 'guide prose with' "$roadmap"; then
  ok
else
  bad "guide-step CI wiring or first-hour timing gap lost its owner"
fi

# Drift policy stays accepted with execution open and zero adapters pinned.
if grep -q -F -e 'Accepted policy; execution open (zero adapters pinned today)' "$docir" &&
  grep -q -F -e 'The exact per-release' "$docir" &&
  grep -q -F -e 'pin-bump and drift-test process is tracked under' "$docir" &&
  grep -q -F -e 'per-release pin-bump plus drift process' "$readme"; then
  ok
else
  bad "drift policy lost its accepted-but-open plus zero-pinned record"
fi

# Validation fixtures stay required-open with symbol-count, stability,
# native-comparison, upgrade, and same-producer byte-identical gates.
if grep -q -F -e 'Required (Open; no adapter execution exists today)' "$docir" &&
  grep -q -F -e 'A symbol-count inventory test detects silent public-API omissions' "$docir" &&
  grep -q -F -e 'Symbol IDs and cross-links are stable across fixture reruns' "$docir" &&
  grep -q -F -e 'Selected generated pages are compared against native-tool output' "$docir" &&
  grep -q -F -e 'Upgrades run old and new extractor versions against the same fixtures' "$docir" &&
  grep -q -F -e 'Same-producer rebuilds are byte-identical' "$docir"; then
  ok
else
  bad "validation fixtures lost their required-open inventory/stability/comparison gates"
fi

# dx docs stub stays removed behind ADR 0020 with reintroduction open
# under #786 (successor to closed #581).
if grep -q -F -e 'Removed. The `dx docs` command was deleted per' "$stub" &&
  grep -q -F -e 'open under #786 (successor to closed #581' "$stub" &&
  grep -q -F -e 'Delete the `dx docs` command surface' "$adr20" &&
  grep -q -F -e 'Reintroducing the command alongside real extraction/validation' "$adr20" &&
  grep -q -F -e 'removed; reintroduction with real extraction/validation open under #786 (successor to closed #581' "$scope"; then
  ok
else
  bad "dx docs stub lost its removed-plus-ADR-0020-plus-#786 record"
fi

# CLI registry carries no Docs command: unknown-command surface never
# lists docs and no Command::Docs implementation exists.
if ! grep -q -F -e 'docs' "$cli_errors" &&
  ! grep -rn -F -e 'Command::Docs' cli/cli/src/ 2>/dev/null | grep -q . &&
  ! grep -rn -F -e 'execute_docs' cli/ 2>/dev/null | grep -q .; then
  ok
else
  bad "CLI gained a Docs command or docs in the unknown-command surface"
fi

# No false adapter execution: no adapter implementation directory or
# Bazel target, docs keep the no-execution record.
if [[ ! -d "docs/adapters" ]] &&
  ! grep -rn -F -e 'docs/adapters' --include='BUILD.bazel' . 2>/dev/null | grep -q . &&
  grep -q -F -e 'no adapter execution exists today' "$readme" &&
  grep -q -F -e 'no site is published yet' "$readme"; then
  ok
else
  bad "a docs adapter implementation appeared or the no-execution record drifted"
fi

# Contracts keep the gap list with delivered site execution under #780 and
# no published-site honesty.
if grep -q -F -e 'Docs pipeline gaps stay open under' "$readme" &&
  grep -q -F -e 'per-language adapter runs' "$readme" &&
  grep -q -F -e 'renderer and site execution delivered seed-only' "$readme" &&
  grep -q -F -e 'under #780' "$readme" &&
  grep -q -F -e 'byte-identical rebuild proof' "$readme" &&
  grep -q -F -e 'link and reference completeness' "$readme" &&
  grep -q -F -e 'guide-step CI wiring' "$readme" &&
  grep -q -F -e 'first-hour timing proof' "$readme" &&
  grep -q -F -e 'per-release pin-bump plus drift process' "$readme"; then
  ok
else
  bad "documentation README lost its #780 delivered plus remaining-gap list"
fi

# Roadmap keeps the execution-gap list with delivered site execution.
if grep -q -F -e 'Docs-pipeline execution gaps stay open under #779 plus #781-#785' "$roadmap" &&
  grep -q -F -e 'adapter runs with pins' "$roadmap" &&
  grep -q -F -e 'renderer and site execution delivered seed-only' "$roadmap" &&
  grep -q -F -e 'no working site claimed' "$roadmap"; then
  ok
else
  bad "roadmap lost its #780 delivered plus remaining-gap list"
fi

# Verification matrix keeps Docs Open with no Supported claim and no
# working site, with site execution delivered.
if grep -q -F -e 'stay open under #779 plus #781-#785' "$matrix" &&
  grep -q -F -e 'renderer/site execution delivered' "$matrix" &&
  grep -q -F -e 'no working site claimed' "$matrix" &&
  ! grep -E -e '^\|.*\| *`?Supported`? *\|' "$matrix" | grep -q . &&
  ! grep -E -e '^\|.*\| *`?Supported`? *\|' "$support" | grep -q .; then
  ok
else
  bad "verification matrix lost its Docs Open plus #780 delivered plus no-Supported gate"
fi

# Functional: schema major pins agree (proto v1, codec example, shared helper).
if grep -q -F -e 'uint32 schema_major = 1;' "$proto" &&
  grep -q -F -e 'schema_major: 1,' "$codec" &&
  grep -q -F -e 'IrVersion { major: 1' "$planning" &&
  [[ "$(grep -F -e 'pub const SCHEMA_MAJOR' "$schema" | sed 's/.*= //; s/;.*//')" == "1" ]]; then
  ok
else
  bad "schema-major pins drifted across proto/codec/planning"
fi

# Backlog guards still track the docs-pipeline gap.
if grep -q -F -e 'docs-pipeline gaps stay tracked' "$backlog_guards" &&
  grep -q -F -e "documentation README lost its #581 docs-pipeline tracker record" "$backlog_guards"; then
  ok
else
  bad "backlog automation guards lost their #581 tracker"
fi

# Tracker lineage: the closed- citations now own as the live
# successor to closed 
# owns).
if grep -q -F -e 'live successor to closed #421' "$readme" &&
  grep -q -F -e 'live successor to closed #421' "$docir" &&
  grep -q -F -e 'live successor to closed #421' "$site" &&
  grep -q -F -e 'live successor to closed #421' "$stub" &&
  grep -q -F -e 'live successor to closed #421' "$scope" &&
  grep -q -F -e 'live successor to closed #421' "$roadmap"; then
  ok
else
  bad "docs pipeline tracker lost its #581 live-successor-to-closed-#421 lineage"
fi

# Scala spike plus Astro/MDX prose-only stay explicit with no execution
# claim
# missing; Astro/MDX prose-only confirmation).
if grep -q -F -e 'Scala 3 TASTy Inspector' "$docir" &&
  grep -q -F -e 'proof spike required' "$docir" &&
  grep -q -F -e 'Missing or' "$docir" &&
  grep -q -F -e 'incompatible TASTy' "$docir" &&
  grep -q -F -e '| Astro/MDX | None; prose-only' "$docir" &&
  grep -q -F -e 'Astro/MDX are prose-only' "$readme" &&
  grep -q -F -e 'Scaladoc/TASTy proof spike' "$readme"; then
  ok
else
  bad "doc-ir/README lost its Scala-spike plus Astro/MDX prose-only record"
fi

# Site-level rebuild stays distinct from the delivered codec
# same-producer proof
# site-level does not).
if grep -q -F -e 'Same producer plus same inputs rebuild byte-identical' "$codec" &&
  grep -q -F -e 'same_producer_requires_byte_equality' "$planning" &&
  grep -q -F -e 'Same-producer rebuilds are byte-identical' "$docir" &&
  grep -q -F -e 'Byte-identical rebuild evidence (two builds, diffed) is required' "$site" &&
  grep -q -F -e 'design requirement; byte-identical rebuild evidence remains open' "$site"; then
  ok
else
  bad "rebuild proof lost its codec-delivered versus site-level-open split"
fi

# ADR 0006 build-vs-validation split stays pinned for dx docs
# reintroduction
# --check non-mutating; exact mappings live in the issue, not the stub).
if grep -q -F -e 'records build versus' "$readme" &&
  grep -q -F -e 'validation-only check' "$readme" &&
  grep -q -F -e 'The planned [`dx docs --check`]' "$site" &&
  grep -q -F -e 'selects extraction and shared validation but not' "$site" &&
  grep -q -F -e 'normal build validates and renders' "$site" &&
  grep -q -F -e 'previews the built output locally and is not a build action' "$site"; then
  ok
else
  bad "dx docs lost its ADR 0006 build-vs-validation split record"
fi

# Generated-IR lifecycle stays Bazel-owned: shards live in Bazel outputs,
# never beside sources or in Git (scope item 2).
if grep -q -F -e 'Generated IR stays in Bazel outputs' "$readme" &&
  grep -q -F -e 'not beside source files or in Git' "$readme" &&
  grep -q -F -e 'never write generated IR beside source' "$site" &&
  grep -q -F -e 'not committed files or source-adjacent snapshots' "$site"; then
  ok
else
  bad "generated-IR lifecycle lost its Bazel-outputs-only record"
fi

# Reusable-docs plus caller stay product surface: check-only lint over the
# caller scope, validated tree (not rendered site), reviewed SHA pin,
# publish only on main (scope item 5).
if grep -q -F -e 'runs `dx lint --check`' "$reusable" &&
  grep -q -F -e 'validated docs tree' "$reusable" &&
  grep -q -F -e 'rendered mdBook site arrives' "$reusable" &&
  grep -q -F -e 'is pure check-only' "$reusable" &&
  grep -q -F -e 'issue #581' "$reusable" &&
  grep -q -F -e 'reusable-docs.yml@' "$caller" &&
  grep -q -F -e 'docs_scope' "$caller" &&
  grep -q -F -e "github.event_name == 'push'" "$caller"; then
  ok
else
  bad "reusable-docs/caller lost its check-only product-surface record"
fi

# No Supported docs claim until platform plus consumer plus release
# evidence passes (scope item 6, via supported_evidence_gate).
if grep -q -F -e 'No cell below is `Supported`' "$support" &&
  grep -q -F -e 'supported_evidence_gate' "$support" &&
  grep -q -F -e 'No cell is `Supported`' "$matrix" &&
  grep -q -F -e 'supported_evidence_gate' "$matrix"; then
  ok
else
  bad "docs lost its no-Supported-until-evidence-gate record"
fi

# Site execution rules exist with the pinned renderer plus the three
# actions and deterministic helpers (issue #780).
if [[ -f "$site_bzl" && -f "$site_build" && -f "$site_tests" ]] &&
  grep -q -F -e 'MDBOOK_VERSION = "0.4.43"' "$site_bzl" &&
  grep -q -F -e 'def docs_extract' "$site_bzl" &&
  grep -q -F -e 'def docs_aggregate' "$site_bzl" &&
  grep -q -F -e 'def docs_render' "$site_bzl" &&
  grep -q -F -e 'def docs_site' "$site_bzl" &&
  grep -q -F -e 'def site_symbol_id' "$site_bzl" &&
  grep -q -F -e 'def site_api_path' "$site_bzl" &&
  grep -q -F -e 'Contract: `docs/documentation/site.md`' "$site_bzl"; then
  ok
else
  bad "docs/site lost its pinned-renderer plus extract/aggregate/render rules (#780)"
fi

# Extract declares hermetic deterministic shards with no network and no
# timestamps or absolute paths.
if grep -q -F -e 'LC_ALL=C sort' "$site_bzl" &&
  grep -q -F -e 'No network access' "$site_bzl" &&
  ! grep -E -e '(^|[^_a-zA-Z])date([^_a-zA-Z]|$)' "$site_bzl" | grep -q . &&
  ! grep -q -F -e '/tmp/' "$site_bzl" &&
  grep -q -F -e 'workspace-relative' "$site_bzl"; then
  ok
else
  bad "docs/site extract lost its hermetic deterministic record (#780)"
fi

# Aggregate builds mdBook-compatible render inputs from shards plus prose
# plus config, with the search index never parsing rendered HTML.
if grep -q -F -e '# Summary' "$site_bzl" &&
  grep -q -F -e '# API Reference' "$site_bzl" &&
  grep -q -F -e 'never parse' "$site_bzl" &&
  grep -q -F -e 'never parses rendered HTML' "$site"; then
  ok
else
  bad "docs/site aggregate lost its mdBook-compatible plus no-HTML-parse record (#780)"
fi

# Render emits the static entry plus the single search index from aggregate
# records with the pinned version stamp.
if grep -q -F -e 'searchindex.json' "$site_bzl" &&
  grep -q -F -e 'rendered by mdBook' "$site_bzl" &&
  grep -q -F -e 'single search index' "$site" &&
  grep -q -F -e 'one search index' "$readme"; then
  ok
else
  bad "docs/site render lost its single-search-index plus pinned-stamp record (#780)"
fi

# Demo inputs stay miniature and mdBook-compatible.
if [[ -f "$site_demo_symbols" && -f "$site_demo_prose" && -f "$site_demo_book" ]] &&
  grep -q -F -e 'AccountService.create' "$site_demo_symbols" &&
  grep -q -F -e '# Demo Guide' "$site_demo_prose" &&
  grep -q -F -e 'title = "demo"' "$site_demo_book"; then
  ok
else
  bad "docs/site demo lost its miniature symbols plus prose plus book inputs (#780)"
fi

# Fixture pins stay present with the #780 execution record.
if [[ -f "$site_pins" && -f "$site_expected" && -f "$site_fixture_build" ]] &&
  grep -q -F -e 'MDBOOK_VERSION = "0.4.43"' "$site_pins" &&
  grep -q -F -e 'qualified seed-only under issue #780' "$site_pins" &&
  grep -q -F -e '(issue #780)' "$site_expected"; then
  ok
else
  bad "docs_site fixture missing (want pins.bzl plus BUILD.bazel plus expected with #780 pins)"
fi

# Live proof: the site package builds green on the seed host.
if bazel build //docs/site/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "docs/site fixture failed to build (want green on the seed host, #780)"
fi

# Live proof: the site unit plus file tests pass on the seed host.
if bazel test //docs/site/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "docs/site tests failed (want site_unit plus site_files green, #780)"
fi

# Live proof: generated IR lives in Bazel outputs, never beside sources.
if bazel build //docs/site:demo_extract --noshow_progress >/dev/null 2>&1 &&
  [[ -f "bazel-bin/docs/site/demo_extract.ir.textproto" ]] &&
  ! git ls-files -- 'docs/site/*.ir.textproto' 'docs/site/**/*.ir.textproto' | grep -q . &&
  ! git ls-files -- 'tools/ci/tests/fixtures/docs_site/*.ir.textproto' | grep -q .; then
  ok
else
  bad "generated IR escaped Bazel outputs (want bazel-bin only, never committed, #780)"
fi

# Live proof: the rendered entry plus search index carry the pinned
# renderer and the demo symbols.
if grep -q -F -e '<!-- rendered by mdBook 0.4.43 fixture -->' bazel-bin/docs/site/demo_render_index.html 2>/dev/null &&
  grep -q -F -e 'python:demo:AccountService.create' bazel-bin/docs/site/demo_render_index.html 2>/dev/null &&
  grep -q -F -e '"docs"' bazel-bin/docs/site/demo_render_searchindex.json 2>/dev/null &&
  grep -q -F -e 'python:demo:AccountService.get' bazel-bin/docs/site/demo_render_searchindex.json 2>/dev/null; then
  ok
else
  bad "rendered site lost its pinned stamp plus demo symbols (want mdBook 0.4.43 plus demo IDs, #780)"
fi

dx_test_summary "docs pipeline qualification harness"
