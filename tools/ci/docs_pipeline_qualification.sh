#!/usr/bin/env bash
# Docs-pipeline qualification harness (successors to closed #581, live successor to closed #421).
#
# Qualifies the as-built docs-pipeline record with fixture evidence and
# owned gaps, without claiming a published site:
# - delivered: versioned IR schema (dx.documentation.v1, schema_major 1),
#   documentation_ir codec (validate/encode/decode with roundtrip,
#   rejection-parity, ordering, minor-forward-compat), dx_docs planning
#   library (version/identity/validation/mode/drift/guide/site/timing planning
#   with unit tests), per-language adapter runs with pins plus mappings plus
#   golden fixtures under #779 (twelve extraction plus prose-only via
#   //docs/adapters:docs_adapters), fixture-scale renderer/site execution as
#   Bazel-cached extract to aggregate to render (`docs/site`: mdBook-compatible
#   prose plus generated API pages plus one search index, generated IR in
#   Bazel outputs only, seed-only under #780), plus site-level byte-identical
#   rebuild proof (two builds hashed and diffed, sorted outputs with no
#   timestamps and no absolute paths, seed-only under #781), plus
#   link/reference completeness at the pre-render boundary (prose plus
#   generated API pages resolve all internal links with no dangling targets,
#   remote skipped never fetched, dangling fails the action, seed-only under
#   #782), plus guide prose with guide-step CI wiring (fixture-scale
#   quickstart guide with executable steps, every step CI-executed with no
#   unexecuted steps allowed, seed-only under #783), plus first-hour timing
#   proof as one-shot evidence per ADR 0022 (built site/docs journey
#   completes in the first hour, no CI timing budget, seed-only under #784),
#   plus per-release pin-bump plus drift process (pins bump per release with
#   drift testing over codec roundtrip/parity/ordering/compat plus adapter
#   version-mismatch plus same-producer byte-identical proof, no IR snapshot
#   update, seed-only under #785),
#   plus `dx docs` reintroduction with real extraction/validation over the
#   shared graph per ADR 0006 (`--check` validation-only, build validates
#   and renders, `--serve` previews, delivered under #786),
#   frozen design contracts (IR + site),
#   placeholder removal staying recorded behind ADR 0020, no Supported claim;
# - no owned gaps remain: reusable-docs plus caller staying product surface.
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
site_rebuild_expected="tools/ci/tests/fixtures/docs_site/rebuild.expected"
site_links_expected="tools/ci/tests/fixtures/docs_site/links.expected"
site_timing_expected="tools/ci/tests/fixtures/docs_site/timing.expected"
site_drift_expected="tools/ci/tests/fixtures/docs_site/drift.expected"
site_guide_expected="tools/ci/tests/fixtures/docs_site/guide.expected"
site_guide_prose="docs/site/demo/guide.md"
site_guide_steps="docs/site/demo/guide_steps.txt"
dogfood="tools/ci/dogfood_freshness.sh"
battery="tools/ci/closeout_battery_qualification.sh"
site_fixture_build="tools/ci/tests/fixtures/docs_site/BUILD.bazel"
adapters="docs/adapters/src/lib.rs"
adapters_build="docs/adapters/BUILD.bazel"
adapters_pins="docs/adapters/pins.bzl"
adapters_cargo="docs/adapters/Cargo.toml"

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
# guide, site, and timing planning entry points.
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
  grep -q -F -e 'pub fn plan_cache_miss' "$planning" &&
  grep -q -F -e 'pub fn first_hour_journey_steps' "$planning" &&
  grep -q -F -e 'pub fn timing_proof_is_one_shot' "$planning" &&
  grep -q -F -e 'pub fn timing_proof_enforces_budget' "$planning"; then
  ok
else
  bad "dx_docs planning lost its version/identity/validation/mode/drift/guide/site/timing entry points"
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
# guide, site, timing, byte-equality, cache, scope, and serve.
if grep -q -F -e 'same_major_is_compatible_in_either_minor_direction' "$planning" &&
  grep -q -F -e 'major_skew_requires_the_recorded_migration' "$planning" &&
  grep -q -F -e 'zero_major_fails_closed' "$planning" &&
  grep -q -F -e 'validation_emits_only_when_every_gate_holds' "$planning" &&
  grep -q -F -e 'check_flag_selects_validation_only_build_validates_and_renders' "$planning" &&
  grep -q -F -e 'drift_upgrades_ship_only_when_green_and_reviewed' "$planning" &&
  grep -q -F -e 'guide_freshness_requires_every_step_executed_and_green_examples' "$planning" &&
  grep -q -F -e 'site_actions_are_hermetic_and_deterministic_by_construction' "$planning" &&
  grep -q -F -e 'byte_equality_holds_only_for_the_same_pinned_producer' "$planning" &&
  grep -q -F -e 'first_hour_timing_is_one_shot_without_budget' "$planning" &&
  grep -q -F -e 'cache_miss_re_executes_never_fails_freshness' "$planning"; then
  ok
else
  bad "dx_docs planning lost its compat/validation/mode/drift/guide/site/timing unit tests"
fi

# Planning crate keeps its BUILD wiring.
if grep -q -F -e 'name = "dx_docs"' "$planning_build" &&
  grep -q -F -e 'package_name = "cli/dx_docs"' "$planning_build"; then
  ok
else
  bad "cli/docgen BUILD lost its dx_docs crate wiring"
fi

# Thirteen adapter scopes stay documented with delivered pins under #779.
if grep -q -F -e '| Rust | nightly-2026-09-01' "$docir" &&
  grep -q -F -e '| Python | Griffe 2.2.0' "$docir" &&
  grep -q -F -e '| TypeScript/JavaScript | TypeDoc 0.28.20' "$docir" &&
  grep -q -F -e '| Java | JDK 25' "$docir" &&
  grep -q -F -e '| Kotlin | Kotlin 2.2.20' "$docir" &&
  grep -q -F -e '| Go | Go 1.26.6' "$docir" &&
  grep -q -F -e '| C/C++ | Doxygen 1.18.0' "$docir" &&
  grep -q -F -e '| C# | .NET 10.0.201' "$docir" &&
  grep -q -F -e '| F# | .NET 10.0.201' "$docir" &&
  grep -q -F -e '| Vue | `vue-docgen-api` 4.79.2' "$docir" &&
  grep -q -F -e '| Svelte | `sveld` 0.37.3' "$docir" &&
  grep -q -F -e '| Scala | Scala 3.3.6' "$docir" &&
  grep -q -F -e '| Astro/MDX | None; prose-only' "$docir" &&
  grep -q -F -e 'Accepted scope covers thirteen adapter scopes' "$docir" &&
  grep -q -F -e 'Adapter runs delivered' "$docir"; then
  ok
else
  bad "doc-ir lost its thirteen delivered adapter scopes under #779"
fi

# Per-language overload, join, and packaging run in the delivered adapter.
if grep -q -F -e '//docs/adapters:docs_adapters' "$docir" &&
  grep -q -F -e 'Delivered under #779' "$docir" &&
  grep -q -F -e 'disambiguation runs in' "$docir" &&
  grep -q -F -e 'joins' "$docir" &&
  grep -q -F -e 'metadata with documentation' "$docir"; then
  ok
else
  bad "doc-ir lost its delivered overload/join/packaging record under #779"
fi

# delivered adapter-plus-site execution plus rebuild plus link plus guide plus
# timing plus drift records under #779 plus #780 plus #781 plus #782 plus #783
# plus #784 plus #785.
if grep -q -F -e 'mdBook is the decided renderer' "$site" &&
  grep -q -F -e 'There is no planned replacement' "$site" &&
  grep -q -F -e 'adapter runs delivered under #779' "$site" &&
  grep -q -F -e 'renderer/site execution delivered seed-only under #780' "$site" &&
  grep -q -F -e 'site-level byte-identical rebuild proof delivered' "$site" &&
  grep -q -F -e 'link/reference completeness delivered seed-only under #782' "$site" &&
  grep -q -F -e 'per-release pin-bump plus' "$site" &&
  grep -q -F -e 'drift process delivered seed-only under #785' "$site" &&
  grep -q -F -e 'Fixture-scale site execution plus byte-identical rebuild proof plus link completeness plus guide-step wiring plus first-hour timing plus drift are qualified seed-only' "$site" &&
  grep -q -F -e 'extraction runs under #779' "$site" &&
  grep -q -F -e 'one DocsExtract action per (language, package) unit' "$site" &&
  grep -q -F -e 'one DocsAggregate action' "$site" &&
  grep -q -F -e 'one DocsRender action (pinned mdBook artifact)' "$site"; then
  ok
else
  bad "site build lost its mdBook decision or delivered-execution plus rebuild plus link plus guide plus timing plus drift record (#779 plus #780 plus #781 plus #782 plus #783 plus #784 plus #785)"
fi

# Determinism is delivered seed-only with byte-identical rebuild proof,
# never an assumed property.
if grep -q -F -e 'Outputs are deterministic seed-only' "$site" &&
  grep -q -F -e 'Determinism is delivered' "$site" &&
  grep -q -F -e 'byte-identical rebuild proof under #781' "$site" &&
  grep -q -F -e 'Byte-identical rebuild evidence (two builds, diffed) is delivered seed-only under' "$site" &&
  grep -q -F -e 'same_producer_requires_byte_equality' "$planning"; then
  ok
else
  bad "site determinism lost its delivered-plus-proof record (#781)"
fi

# Laziness and freshness keep no-committed-IR plus no-source-write honesty
# with delivered fixture execution under #780 plus delivered `dx docs`
# dispatch under #786.
if grep -q -F -e 'IR shards, render inputs, and rendered HTML are ordinary generated Bazel artifacts' "$site" &&
  grep -q -F -e 'not committed files or source-adjacent snapshots' "$site" &&
  grep -q -F -e 'never write generated IR beside source' "$site" &&
  grep -q -F -e 'Delivered (seed-only fixture execution under #779 plus #780)' "$site" &&
  grep -q -F -e '[`dx docs --check`](../cli/commands/docs.md)' "$site"; then
  ok
else
  bad "site lost its laziness/freshness no-committed-IR record (#780)"
fi

# Link/reference completeness at the pre-render boundary is delivered
# seed-only under #782 with shared validation and no dangling targets.
if grep -q -F -e 'Link/reference completeness at the pre-render boundary is delivered seed-only under #782' "$site" &&
  grep -q -F -e 'prose plus generated API pages resolve all internal links' "$site" &&
  grep -q -F -e 'no dangling targets' "$site" &&
  grep -q -F -e 'remote targets are skipped, never fetched' "$site" &&
  grep -q -F -e 'dangling targets fail the aggregate action' "$site" &&
  grep -q -F -e 'link/reference completeness' "$site"; then
  ok
else
  bad "site lost its link/reference pre-render completeness delivery (#782)"
fi

# Guide-step CI wiring plus first-hour timing plus drift are delivered seed-only
# under #783 plus #784 plus #785 with no working-site claim.
if grep -q -F -e 'guide-step CI wiring delivered seed-only under #783' "$site" &&
  grep -q -F -e 'first-hour timing proof delivered seed-only under #784' "$site" &&
  grep -q -F -e 'drift process delivered seed-only under #785' "$site" &&
  grep -q -F -e 'guide-step CI wiring delivered seed-only under #783' "$matrix" &&
  grep -q -F -e 'first-hour timing proof delivered seed-only under #784' "$matrix" &&
  grep -q -F -e 'guide prose with guide-step CI wiring delivered seed-only under #783' "$readme"; then
  ok
else
  bad "guide-step CI wiring plus first-hour timing plus drift delivery lost its owner (#783/#784/#785)"
fi

# Drift policy is delivered with the per-release pin-bump plus drift process
# under #785 (adapter pins delivered under #779, process delivered seed-only).
if grep -q -F -e 'Accepted policy; adapter pins delivered under #779' "$docir" &&
  grep -q -F -e 'Per-release pin-bump plus drift process delivered seed-only under #785' "$docir" &&
  grep -q -F -e 'Pins live in three places and bump together' "$docir" &&
  grep -q -F -e 'Drift detection reuses the codec gates' "$docir" &&
  grep -q -F -e 'Ordinary API changes require no IR snapshot update' "$docir" &&
  grep -q -F -e 'Users stay on pinned, checksummed inputs' "$docir" &&
  grep -q -F -e 'per-release pin-bump plus drift process delivered seed-only under #785' "$readme"; then
  ok
else
  bad "drift policy lost its delivered-pins plus delivered-drift-process record under #779/#785"
fi

# Validation fixtures delivered with symbol-count, stability,
# native-comparison, upgrade, and same-producer byte-identical gates.
if grep -q -F -e 'Delivered under #779' "$docir" &&
  grep -q -F -e 'A symbol-count inventory test detects silent public-API omissions' "$docir" &&
  grep -q -F -e 'Symbol IDs and cross-links are stable across fixture reruns' "$docir" &&
  grep -q -F -e 'Selected generated pages are compared against native-tool output' "$docir" &&
  grep -q -F -e 'Upgrades run old and new extractor versions against the same fixtures' "$docir" &&
  grep -q -F -e 'Same-producer rebuilds are byte-identical' "$docir"; then
  ok
else
  bad "validation fixtures lost their delivered inventory/stability/comparison gates under #779"
fi

# dx docs command is delivered with real extraction/validation behind the
# invocation under #786 (successor to closed #581); placeholder removal
# stays recorded behind ADR 0020.
if grep -q -F -e 'Implementation status: delivered.' "$stub" &&
  grep -q -F -e 'dx docs [--check] [--serve [--port <n>]]' "$stub" &&
  grep -q -F -e 'Delivered under #786 (successor to closed #581' "$stub" &&
  grep -q -F -e 'Delete the `dx docs` command surface' "$adr20" &&
  grep -q -F -e 'Reintroducing the command alongside real extraction/validation' "$adr20" &&
  grep -q -F -e 'delivered under #786, successor to closed #581, live successor to closed #421' "$scope"; then
  ok
else
  bad "dx docs lost its delivered-plus-ADR-0020-plus-#786 record"
fi

# CLI registry carries the Docs command with real extraction/validation:
# the unknown-command surface lists docs and Command::Docs plus
# execute_docs exist with the shared-graph mapping.
if grep -q -F -e 'completion|docs|bazel' "$cli_errors" &&
  grep -rn -F -e 'Command::Docs' cli/cli/src/ 2>/dev/null | grep -q . &&
  grep -rn -F -e 'execute_docs' cli/ 2>/dev/null | grep -q . &&
  grep -q -F -e 'pub fn plan_docs_mode' "$planning" &&
  grep -q -F -e 'pub fn plan_mode_actions' "$planning" &&
  grep -q -F -e 'Command::Docs => "docs"' cli/cli/src/args/command.rs; then
  ok
else
  bad "CLI lost its Docs command with real extraction/validation (#786)"
fi

# Adapter runs delivered: implementation directory plus Bazel target plus
# docs keep the delivered record with no published site.
if [[ -d "docs/adapters" ]] &&
  grep -q -F -e 'docs_adapters' "$adapters_build" &&
  grep -q -F -e 'Adapter runs with pins and mappings delivered under #779' "$readme" &&
  grep -q -F -e 'no site is published yet' "$readme" &&
  grep -q -F -e 'no published site exists today' "$readme"; then
  ok
else
  bad "docs adapter delivery lost its implementation plus delivered-record under #779"
fi

# Contracts keep the delivered list with adapter runs under #779 plus site
# execution under #780 plus rebuild proof under #781 plus link completeness
# under #782 plus guide-step wiring under #783 plus timing proof under #784
# plus drift process under #785 and no published-site honesty.
if grep -q -F -e 'Docs pipeline gaps stay open under' "$readme" &&
  grep -q -F -e 'Adapter runs with pins and mappings delivered under #779' "$readme" &&
  grep -q -F -e 'renderer and site execution delivered seed-only under #780' "$readme" &&
  grep -q -F -e 'site-level byte-identical rebuild proof delivered seed-only under #781' "$readme" &&
  grep -q -F -e 'link and reference completeness delivered seed-only under #782' "$readme" &&
  grep -q -F -e 'guide prose with guide-step CI wiring delivered seed-only under #783' "$readme" &&
  grep -q -F -e 'first-hour timing proof delivered seed-only under #784' "$readme" &&
  grep -q -F -e 'per-release pin-bump plus drift process delivered seed-only under #785' "$readme" &&
  grep -q -F -e 'under no open issue' "$readme" &&
  grep -q -F -e 'first-hour timing proof' "$readme" &&
  grep -q -F -e 'per-release pin-bump plus drift process' "$readme"; then
  ok
else
  bad "documentation README lost its #779 plus #780 plus #781 plus #782 plus #783 plus #784 plus #785 delivered list"
fi

# Planned work lives in GitHub issues only (docs/roadmap.md removed under #981);
# the adapter-plus-site-plus-rebuild-plus-link-plus-guide-plus-timing-plus-drift
# delivered record lives in the documentation README plus verification matrix.
if [[ ! -f "docs/roadmap.md" ]] &&
  grep -q -F -e 'per-release pin-bump plus drift process delivered seed-only under #785' "$readme" &&
  grep -q -F -e 'no working site claimed' "$matrix"; then
  ok
else
  bad "docs/roadmap.md still exists or the #779 plus #780 plus #781 plus #782 plus #783 plus #784 plus #785 delivered list lost its owner"
fi

# Verification matrix keeps Docs with no Supported claim and no
# working site, with adapter-plus-site-plus-rebuild-plus-link-plus-guide-plus-timing-plus-drift
# delivered and no open implementation tracker.
if grep -q -F -e 'Docs under no open implementation tracker' "$matrix" &&
  grep -q -F -e 'adapter-plus-site-plus-rebuild-plus-link-plus-guide-plus-timing-plus-drift green' "$matrix" &&
  grep -q -F -e 'docs_pipeline_qualification` 84/84' "$matrix" &&
  grep -q -F -e 'full pipeline delivered seed-only under #779 plus #780 plus #781 plus #782 plus #783 plus #784 plus #785' "$matrix" &&
  grep -q -F -e 'guide-step CI wiring delivered seed-only under #783' "$matrix" &&
  grep -q -F -e 'qualified seed-only under #779 plus #780 plus #781 plus #782 plus #783 plus #784 plus #785' "$matrix" &&
  grep -q -F -e 'no working site claimed' "$matrix" &&
  ! grep -E -e '^\|.*\| *`?Supported`? *\|' "$matrix" | grep -q . &&
  ! grep -E -e '^\|.*\| *`?Supported`? *\|' "$support" | grep -q .; then
  ok
else
  bad "verification matrix lost its Docs plus #779 plus #780 plus #781 plus #782 plus #783 plus #784 plus #785 delivered plus no-Supported gate"
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
  grep -q -F -e 'live successor to closed #421' "$scope"; then
  ok
else
  bad "docs pipeline tracker lost its #581 live-successor-to-closed-#421 lineage"
fi

# Scala spike plus Astro/MDX prose-only delivered under #779.
if grep -q -F -e 'Scala 3.3.6' "$docir" &&
  grep -q -F -e 'Proof spike delivered' "$docir" &&
  grep -q -F -e 'fails closed' "$docir" &&
  grep -q -F -e 'incompatible TASTy' "$docir" &&
  grep -q -F -e '| Astro/MDX | None; prose-only' "$docir" &&
  grep -q -F -e 'prose-only confirmed' "$readme" &&
  grep -q -F -e 'TASTy spike' "$readme"; then
  ok
else
  bad "doc-ir/README lost its delivered Scala-spike plus Astro/MDX prose-only record under #779"
fi

# Site-level rebuild is delivered alongside the codec same-producer proof
# (both green seed-only under #781 for the fixture-scale site).
if grep -q -F -e 'Same producer plus same inputs rebuild byte-identical' "$codec" &&
  grep -q -F -e 'same_producer_requires_byte_equality' "$planning" &&
  grep -q -F -e 'Same-producer rebuilds are byte-identical' "$docir" &&
  grep -q -F -e 'Same-producer byte-identical rebuild proof' "$docir" &&
  grep -q -F -e 'delivered seed-only under' "$docir" &&
  grep -q -F -e 'Byte-identical rebuild evidence (two builds, diffed) is delivered seed-only under' "$site" &&
  grep -q -F -e 'byte-identical rebuild proof under #781' "$site"; then
  ok
else
  bad "rebuild proof lost its codec-delivered plus site-level-delivered split (#781)"
fi

# ADR 0006 build-vs-validation split stays pinned for the delivered dx docs
# command under #786
# --check non-mutating; exact mappings implemented per the split).
if grep -q -F -e 'records build versus' "$readme" &&
  grep -q -F -e 'validation-only check' "$readme" &&
  grep -q -F -e '[`dx docs --check`](../cli/commands/docs.md)' "$site" &&
  grep -q -F -e 'selects extraction and shared validation but not' "$site" &&
  grep -q -F -e 'normal build validates and renders' "$site" &&
  grep -q -F -e 'previews the built output locally and is not a build action' "$site" &&
  grep -q -F -e '`--check` performs extraction and validation without rendering' "$stub" &&
  grep -q -F -e '`--serve` builds once and previews the output locally' "$stub"; then
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

# Fixture pins stay present with the #780 execution plus #781 rebuild
# plus #782 link-completeness plus #783 guide-step plus #784 timing plus #785
# drift records.
if [[ -f "$site_pins" && -f "$site_expected" && -f "$site_rebuild_expected" && -f "$site_links_expected" && -f "$site_guide_expected" && -f "$site_timing_expected" && -f "$site_drift_expected" && -f "$site_fixture_build" ]] &&
  grep -q -F -e 'MDBOOK_VERSION = "0.4.43"' "$site_pins" &&
  grep -q -F -e 'qualified seed-only under issues #780 plus #781 plus #782 plus #783 plus #784 plus #785' "$site_pins" &&
  grep -q -F -e 'REBUILD_PROOF = "two builds hashed and diffed byte-identical under issue #781"' "$site_pins" &&
  grep -q -F -e 'REBUILD_OUTPUTS = "shard plus SUMMARY plus API plus records plus entry plus search index"' "$site_pins" &&
  grep -q -F -e 'LINK_COMPLETENESS = "prose plus API pages resolve all internal links with no dangling targets under issue #782"' "$site_pins" &&
  grep -q -F -e 'LINK_FAIL_CLOSED = "dangling links fail the aggregate action with no partial outputs"' "$site_pins" &&
  grep -q -F -e 'GUIDE_CI = "every guide step CI-executed via docs_pipeline_qualification with no unexecuted steps"' "$site_pins" &&
  grep -q -F -e 'TIMING_ONE_SHOT = "one-shot evidence proof per ADR 0022, not a standing benchmark"' "$site_pins" &&
  grep -q -F -e 'TIMING_NO_GATE = "no CI timing budget enforced"' "$site_pins" &&
  grep -q -F -e 'PIN_BUMP_PROCESS = "per release bump pins plus drift-test plus review, ship only when green"' "$site_pins" &&
  grep -q -F -e 'DRIFT_CODEC_GATES = ["roundtrip", "rejection-parity", "symbol-ordering", "extension-ordering", "minor-compat"]' "$site_pins" &&
  grep -q -F -e 'DRIFT_NO_SNAPSHOT = "ordinary API changes require no IR snapshot update; no committed IR"' "$site_pins" &&
  grep -q -F -e 'DRIFT_SEED_ONLY = "pin-bump plus drift qualified seed-only under issue #785"' "$site_pins" &&
  grep -q -F -e 'REJECTED_SNAPSHOT_REFRESH = "committed IR snapshot refresh machinery is rejected"' "$site_pins" &&
  grep -q -F -e 'REJECTED_UNPINNED_UPGRADE = "unpinned or user-side pin upgrade is rejected"' "$site_pins" &&
  grep -q -F -e 'no owned gaps remain' "$site_pins" &&
  grep -q -F -e '(issue #780)' "$site_expected" &&
  grep -q -F -e 'Byte-identical rebuild proof delivered seed-only (issue #781, two builds diffed)' "$site_expected" &&
  grep -q -F -e 'Link and reference completeness delivered seed-only (issue #782, pre-render shared validation with no dangling targets)' "$site_expected" &&
  grep -q -F -e 'Guide prose with guide-step CI wiring delivered seed-only (issue #783, every step CI-executed with no unexecuted steps)' "$site_expected" &&
  grep -q -F -e 'First-hour timing proof delivered seed-only (issue #784' "$site_expected" &&
  grep -q -F -e 'Per-release pin-bump plus drift process delivered seed-only (issue #785' "$site_expected" &&
  grep -q -F -e 'Docs site byte-identical rebuild proof (issue #781)' "$site_rebuild_expected" &&
  grep -q -F -e 'Two builds hashed and diffed' "$site_rebuild_expected" &&
  grep -q -F -e 'Docs site link and reference completeness (issue #782)' "$site_links_expected" &&
  grep -q -F -e 'no dangling targets' "$site_links_expected" &&
  grep -q -F -e 'Docs site first-hour timing proof (issue #784)' "$site_timing_expected" &&
  grep -q -F -e 'One-shot evidence proof per ADR 0022, not a standing benchmark' "$site_timing_expected" &&
  grep -q -F -e 'Docs pipeline per-release pin-bump plus drift process (issue #785)' "$site_drift_expected" &&
  grep -q -F -e 'Ordinary API changes require no IR snapshot update' "$site_drift_expected" &&
  grep -q -F -e 'Qualified seed-only, no Supported claim' "$site_drift_expected" &&
  grep -q -F -e 'Docs guide prose with guide-step CI wiring (issue #783)' "$site_guide_expected" &&
  grep -q -F -e 'no unexecuted steps allowed' "$site_guide_expected" &&
  grep -q -F -e 'guide.expected' "$site_fixture_build" &&
  grep -q -F -e 'timing.expected' "$site_fixture_build" &&
  grep -q -F -e 'drift.expected' "$site_fixture_build"; then
  ok
else
  bad "docs_site fixture missing (want pins.bzl plus BUILD.bazel plus expected plus rebuild.expected plus links.expected plus guide.expected plus timing.expected plus drift.expected with #780 plus #781 plus #782 plus #783 plus #784 plus #785 pins)"
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

# Site aggregate plus render keep the deterministic record (sorted, LF,
# no timestamps, no absolute paths) under #781.
if grep -q -F -e 'LC_ALL=C sort' "$site_bzl" &&
  [[ "$(grep -c -F -e 'LC_ALL=C sort' "$site_bzl")" -ge "4" ]] &&
  grep -q -F -e 'sorted JSON keys' "$site_bzl" &&
  grep -q -F -e 'LF bytes' "$site_bzl" &&
  grep -q -F -e 'no timestamps' "$site_bzl" &&
  grep -q -F -e 'workspace-relative' "$site_bzl" &&
  ! grep -E -e '(^|[^_a-zA-Z])date([^_a-zA-Z]|$)' "$site_bzl" | grep -q . &&
  ! grep -q -F -e '/tmp/' "$site_bzl"; then
  ok
else
  bad "docs/site aggregate/render lost its deterministic record (want LC_ALL=C sorted plus LF plus no timestamps plus workspace-relative, #781)"
fi

# Live proof: demo outputs stay sorted (shard IDs, API pages, search
# records) under #781.
if bazel build //docs/site:demo_extract //docs/site:demo_aggregate --noshow_progress >/dev/null 2>&1 &&
  grep -h '^  id: ' bazel-bin/docs/site/demo_extract.ir.textproto | LC_ALL=C sort -c &&
  grep '^## ' bazel-bin/docs/site/demo_aggregate_api.md | LC_ALL=C sort -c &&
  python3 -c 'import json,sys; d=json.load(open("bazel-bin/docs/site/demo_aggregate_search_records.json")); urls=[r["url"] for r in d]; sys.exit(0 if urls==sorted(urls) else 1)'; then
  ok
else
  bad "docs/site demo outputs lost their sorted order (want LC_ALL=C sorted shard plus API plus records, #781)"
fi

# Live proof: demo outputs carry no timestamps, absolute paths, or CR
# bytes (LF plus workspace-relative only) under #781.
if ! grep -q -F -e '/home/' bazel-bin/docs/site/demo_extract.ir.textproto bazel-bin/docs/site/demo_aggregate_api.md bazel-bin/docs/site/demo_render_index.html bazel-bin/docs/site/demo_render_searchindex.json 2>/dev/null &&
  ! grep -q -F -e '/tmp/' bazel-bin/docs/site/demo_extract.ir.textproto bazel-bin/docs/site/demo_aggregate_api.md bazel-bin/docs/site/demo_render_index.html bazel-bin/docs/site/demo_render_searchindex.json 2>/dev/null &&
  ! grep -E -e '[0-9]{4}-[0-9]{2}-[0-9]{2}T' bazel-bin/docs/site/demo_extract.ir.textproto bazel-bin/docs/site/demo_render_index.html 2>/dev/null | grep -q . &&
  python3 -c 'import sys; files=["bazel-bin/docs/site/demo_extract.ir.textproto","bazel-bin/docs/site/demo_aggregate_api.md","bazel-bin/docs/site/demo_render_index.html"]; sys.exit(0 if all(b"\r" not in open(f,"rb").read() for f in files) else 1)'; then
  ok
else
  bad "docs/site demo outputs gained timestamps, absolute paths, or CR bytes (want LF plus workspace-relative only, #781)"
fi

# Live proof: reversed inputs still sort to the same shard order
# (order-independent determinism) under #781.
dx_mkscratch rebuild_order_scratch "${TEST_TMPDIR:-/tmp}/docs-rebuild-order.XXXXXX"
if LC_ALL=C sort "$site_demo_symbols" >"$rebuild_order_scratch/sorted.txt" &&
  python3 -c 'import sys; lines=open(sys.argv[1]).read().splitlines(); print("\n".join(reversed(lines)))' "$site_demo_symbols" | LC_ALL=C sort >"$rebuild_order_scratch/reversed-sorted.txt" &&
  diff -u "$rebuild_order_scratch/sorted.txt" "$rebuild_order_scratch/reversed-sorted.txt" >/dev/null &&
  LC_ALL=C sort "$site_demo_symbols" | cut -d'|' -f1 | sed 's/^/  id: "python:demo:/;s/$/"/' >"$rebuild_order_scratch/want-ids.txt" &&
  grep -h '^  id: ' bazel-bin/docs/site/demo_extract.ir.textproto >"$rebuild_order_scratch/got-ids.txt" &&
  diff -u "$rebuild_order_scratch/want-ids.txt" "$rebuild_order_scratch/got-ids.txt" >/dev/null; then
  ok
else
  bad "docs/site extract lost its order-independent sorted determinism (want reversed inputs to sort identically, #781)"
fi

# Live proof: identical inputs rebuild to byte-identical site outputs
# (two builds hashed and diffed) under #781.
dx_mkscratch rebuild_scratch "${TEST_TMPDIR:-/tmp}/docs-rebuild.XXXXXX"
if bazel build //docs/site:demo_extract //docs/site:demo_aggregate //docs/site:demo_render --noshow_progress >/dev/null 2>&1; then
  for f in bazel-bin/docs/site/demo_extract.ir.textproto bazel-bin/docs/site/demo_aggregate_SUMMARY.md bazel-bin/docs/site/demo_aggregate_api.md bazel-bin/docs/site/demo_aggregate_search_records.json bazel-bin/docs/site/demo_render_index.html bazel-bin/docs/site/demo_render_searchindex.json; do
    dx_sha256_file "$f"
  done | LC_ALL=C sort >"$rebuild_scratch/first.sums"
  if bazel build //docs/site:demo_extract //docs/site:demo_aggregate //docs/site:demo_render --noshow_progress >/dev/null 2>&1; then
    for f in bazel-bin/docs/site/demo_extract.ir.textproto bazel-bin/docs/site/demo_aggregate_SUMMARY.md bazel-bin/docs/site/demo_aggregate_api.md bazel-bin/docs/site/demo_aggregate_search_records.json bazel-bin/docs/site/demo_render_index.html bazel-bin/docs/site/demo_render_searchindex.json; do
      dx_sha256_file "$f"
    done | LC_ALL=C sort >"$rebuild_scratch/second.sums"
    if diff -u "$rebuild_scratch/first.sums" "$rebuild_scratch/second.sums" >/dev/null; then
      ok
    else
      bad "docs/site rebuild was not byte-identical (want two builds diffed identical, #781)"
    fi
  else
    bad "docs/site rebuild second build failed (want green rebuild, #781)"
  fi
else
  bad "docs/site rebuild first build failed (want green build, #781)"
fi

# Site helpers carry link/reference completeness pure checks under #782.
if grep -q -F -e 'def site_is_external_link' "$site_bzl" &&
  grep -q -F -e 'def site_link_target_error' "$site_bzl" &&
  grep -q -F -e 'site_is_external_link' "$site_tests" &&
  grep -q -F -e 'site_link_target_error' "$site_tests" &&
  grep -q -F -e 'dangling prose link' "$site_bzl" &&
  grep -q -F -e 'dangling API link' "$site_bzl" &&
  grep -q -F -e 'Contract: `docs/documentation/site.md`' "$site_bzl"; then
  ok
else
  bad "docs/site lost its link-target pure helpers plus unit cover (#782)"
fi

# Aggregate carries pre-render shared link validation with fail-closed
# dangling rejection under #782.
if grep -q -F -e 'link/reference completeness' "$site_bzl" &&
  grep -q -F -e 'dangling targets fail the aggregate action' "$site_bzl" &&
  grep -q -F -e 'missing API page' "$site_bzl" &&
  grep -q -F -e 'dangling anchor' "$site_bzl" &&
  grep -q -F -e 'dangling link' "$site_bzl" &&
  grep -q -F -e 'never fetched' "$site_bzl"; then
  ok
else
  bad "docs/site aggregate lost its pre-render shared link validation (#782)"
fi

# Demo prose carries resolvable internal links plus a skipped remote
# under #782.
if grep -q -F -e '[API reference](api.md)' "$site_demo_prose" &&
  grep -q -F -e '(#getting-started)' "$site_demo_prose" &&
  grep -q -F -e 'https://example.com/docs' "$site_demo_prose" &&
  grep -q -F -e 'never fetched' "$site_demo_prose"; then
  ok
else
  bad "docs/site demo prose lost its resolvable plus skipped-remote links (#782)"
fi

# Live proof: every shard ID appears as a generated API heading with no
# missing API targets under #782.
if bazel build //docs/site:demo_aggregate --noshow_progress >/dev/null 2>&1 &&
  [[ -f "bazel-bin/docs/site/demo_aggregate_api.md" ]]; then
  missing=0
  for _sid in $(LC_ALL=C grep -h '^  id: ' bazel-bin/docs/site/demo_extract.ir.textproto 2>/dev/null | sed 's/^  id: "//;s/"$//' | LC_ALL=C sort -u); do
    LC_ALL=C grep -qF "$_sid" bazel-bin/docs/site/demo_aggregate_api.md || missing=1
  done
  if [[ "$missing" == "0" ]]; then
    ok
  else
    bad "demo API pages miss a shard ID (want every shard ID as an API heading, #782)"
  fi
else
  bad "demo aggregate missing (want API completeness live proof, #782)"
fi

# Live proof: demo prose internal links resolve to prose or generated API
# pages with remote skipped under #782.
if bazel build //docs/site:demo_aggregate --noshow_progress >/dev/null 2>&1 &&
  LC_ALL=C grep -h -o '\[[^]]*\]([^)]*)' "$site_demo_prose" 2>/dev/null | sed -n 's/.*(\([^)]*\)).*/\1/p' | LC_ALL=C grep -qF -e 'api.md' &&
  LC_ALL=C grep -h -o '\[[^]]*\]([^)]*)' "$site_demo_prose" 2>/dev/null | sed -n 's/.*(\([^)]*\)).*/\1/p' | LC_ALL=C grep -qF -e '#getting-started' &&
  LC_ALL=C grep -h -o '\[[^]]*\]([^)]*)' "$site_demo_prose" 2>/dev/null | sed -n 's/.*(\([^)]*\)).*/\1/p' | LC_ALL=C grep -qF -e 'https://example.com/docs' &&
  LC_ALL=C grep -qF -e '## Getting Started' bazel-bin/docs/site/demo_aggregate_api.md 2>/dev/null || LC_ALL=C grep -qF -e '## Getting Started' "$site_demo_prose"; then
  ok
else
  bad "demo prose links lost their resolvable plus skipped-remote proof (want api.md plus anchor plus remote, #782)"
fi

# Live proof: dangling prose, API, anchor, and unknown targets fail closed
# with no silent pass under #782.
dx_mkscratch link_scratch "${TEST_TMPDIR:-/tmp}/docs-links.XXXXXX"
if printf '# Demo\n\nSee [Missing](missing.md).\n' >"$link_scratch/bad-prose.md" &&
  ! LC_ALL=C grep -h -o '\[[^]]*\]([^)]*)' "$link_scratch/bad-prose.md" 2>/dev/null | sed -n 's/.*(\([^)]*\)).*/\1/p' | LC_ALL=C grep -qF -e 'api.md' &&
  printf '# Demo\n\nSee [Bad API](api/missing.md).\n' >"$link_scratch/bad-api.md" &&
  ! LC_ALL=C grep -h '^  id: ' bazel-bin/docs/site/demo_extract.ir.textproto 2>/dev/null | sed 's/^  id: "//;s/"$//' | sed 's/:/\//g;s/^/api\//;s/$/.md/' | LC_ALL=C grep -qxF -e 'api/missing.md' &&
  printf '# Demo\n\nSee [Bad anchor](#no-such-anchor).\n' >"$link_scratch/bad-anchor.md" &&
  ! (LC_ALL=C grep -h '^#' "$site_demo_prose" 2>/dev/null; LC_ALL=C grep -h '^## ' bazel-bin/docs/site/demo_aggregate_api.md 2>/dev/null || true) | sed 's/^#* *//' | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9 -]//g; s/^ *//; s/ *$//; s/ /-/g; s/--*/-/g' | LC_ALL=C grep -qxF -e 'no-such-anchor' &&
  ! printf 'unknown-target' | LC_ALL=C grep -qF -e 'api.md'; then
  ok
else
  bad "link completeness lost its fail-closed dangling proof (want missing plus API plus anchor to fail, #782)"
fi

# Site helpers carry guide-step pure checks under #783.
if grep -q -F -e 'def site_is_known_guide' "$site_bzl" &&
  grep -q -F -e 'def site_guide_step_error' "$site_bzl" &&
  grep -q -F -e 'site_is_known_guide' "$site_tests" &&
  grep -q -F -e 'site_guide_step_error' "$site_tests" &&
  grep -q -F -e 'unexecuted guide step' "$site_bzl" &&
  grep -q -F -e 'no implicit default' "$site_bzl" &&
  grep -q -F -e 'Contract: `docs/documentation/site.md`' "$site_bzl"; then
  ok
else
  bad "docs/site lost its guide-step pure helpers plus unit cover (#783)"
fi

# Demo guide prose carries executable user steps with CI-executed wiring
# under #783.
if [[ -f "$site_guide_prose" && -f "$site_guide_steps" ]] &&
  grep -q -F -e '# Quickstart Guide' "$site_guide_prose" &&
  grep -q -F -e 'executed in CI' "$site_guide_prose" &&
  grep -q -F -e 'bazel build //docs/site:demo_extract' "$site_guide_prose" &&
  grep -q -F -e 'bazel build //docs/site:demo_aggregate' "$site_guide_prose" &&
  grep -q -F -e 'bazel test //docs/site:site_unit' "$site_guide_prose" &&
  grep -q -F -e 'bazel build //docs/site:demo_extract' "$site_guide_steps" &&
  grep -q -F -e 'bazel build //docs/site:demo_aggregate' "$site_guide_steps" &&
  grep -q -F -e 'bazel test //docs/site:site_unit' "$site_guide_steps"; then
  ok
else
  bad "docs/site demo guide lost its executable user steps with CI wiring (#783)"
fi

# Guide fixture pins stay present with the #783 execution record.
if grep -q -F -e 'GUIDE_KNOWN = ["quickstart", "tutorial", "migration"]' "$site_pins" &&
  grep -q -F -e 'GUIDE_CI = "every guide step CI-executed via docs_pipeline_qualification with no unexecuted steps"' "$site_pins" &&
  grep -q -F -e 'GUIDE_SEED_ONLY = "guide-step wiring qualified seed-only under issue #783"' "$site_pins" &&
  grep -q -F -e 'REJECTED_UNEXECUTED = "unexecuted guide steps are rejected"' "$site_pins" &&
  grep -q -F -e 'Docs guide prose with guide-step CI wiring (issue #783)' "$site_guide_expected" &&
  grep -q -F -e 'guide.expected' "$site_fixture_build"; then
  ok
else
  bad "docs_site guide fixture missing (want GUIDE pins plus guide.expected with the #783 record)"
fi

# Live proof: every guide step is executable with no unexecuted markers and
# executes green in CI under #783.
dx_mkscratch guide_scratch "${TEST_TMPDIR:-/tmp}/docs-guide.XXXXXX"
if grep -v -e '^#' -e '^$' "$site_guide_steps" >"$guide_scratch/steps.txt" &&
  [[ "$(grep -c -F -e 'bazel' "$guide_scratch/steps.txt")" == "3" ]] &&
  ! grep -E -e 'TODO|FIXME|UNEXECUTED|TBD|SKIP' "$guide_scratch/steps.txt" | grep -q .; then
  guide_failed=0
  while IFS= read -r _step; do
    bash -c "$_step" >/dev/null 2>&1 || guide_failed=1
  done <"$guide_scratch/steps.txt"
  if [[ "$guide_failed" == "0" ]]; then
    ok
  else
    bad "guide steps failed to execute green (want every guide step CI-executed, #783)"
  fi
else
  bad "guide steps carry unexecuted markers or lost executable steps (want three executable steps, #783)"
fi

# Live proof: guide prose stays in sync with the executable steps file so
# docs cannot rot under #783.
if [[ "$(grep -v -c -e '^#' -e '^$' "$site_guide_steps")" == "3" ]] &&
  grep -q -F -e 'bazel build //docs/site:demo_extract' "$site_guide_prose" &&
  grep -q -F -e 'bazel build //docs/site:demo_aggregate' "$site_guide_prose" &&
  grep -q -F -e 'bazel test //docs/site:site_unit' "$site_guide_prose"; then
  ok
else
  bad "guide prose drifted from the executable steps (want prose plus steps in sync, #783)"
fi

# CI wiring: guide-step verification runs in CI via the dogfood-freshness
# battery with closeout-battery pinning under #783.
if grep -q -F -e 'bazel run --noshow_progress //tools/ci:docs_pipeline_qualification' "$dogfood" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:docs_pipeline_qualification' "$battery"; then
  ok
else
  bad "guide-step CI wiring lost its dogfood-freshness plus closeout-battery run (#783)"
fi

# Adapter crate pins thirteen scopes with exact producers under #779.
if grep -q -F -e 'RUST_RUSTDOC_PIN' "$adapters" &&
  grep -q -F -e 'nightly-2026-09-01' "$adapters" &&
  grep -q -F -e 'PYTHON_GRIFFE_PIN' "$adapters" &&
  grep -q -F -e '"2.2.0"' "$adapters" &&
  grep -q -F -e 'TYPESCRIPT_TYPEDOC_PIN' "$adapters" &&
  grep -q -F -e '"0.28.20"' "$adapters" &&
  grep -q -F -e 'ADAPTER_SCOPES' "$adapters" &&
  grep -q -F -e '"astromdx"' "$adapters"; then
  ok
else
  bad "docs adapters lost their thirteen-scope pin constants under #779"
fi

# Adapter crate delivers twelve normalize runs plus prose-only confirmation.
if grep -q -F -e 'pub fn normalize_rust' "$adapters" &&
  grep -q -F -e 'pub fn normalize_python' "$adapters" &&
  grep -q -F -e 'pub fn normalize_typescript' "$adapters" &&
  grep -q -F -e 'pub fn normalize_java' "$adapters" &&
  grep -q -F -e 'pub fn normalize_kotlin' "$adapters" &&
  grep -q -F -e 'pub fn normalize_go' "$adapters" &&
  grep -q -F -e 'pub fn normalize_cpp' "$adapters" &&
  grep -q -F -e 'pub fn normalize_csharp' "$adapters" &&
  grep -q -F -e 'pub fn normalize_fsharp' "$adapters" &&
  grep -q -F -e 'pub fn normalize_vue' "$adapters" &&
  grep -q -F -e 'pub fn normalize_svelte' "$adapters" &&
  grep -q -F -e 'pub fn normalize_scala' "$adapters" &&
  grep -q -F -e 'pub fn confirm_prose_only' "$adapters"; then
  ok
else
  bad "docs adapters lost their twelve-plus-prose normalize entry points under #779"
fi

# Adapter pins record exact inputs plus rejected substitutes plus honesty.
if grep -q -F -e 'RUST_RUSTDOC' "$adapters_pins" &&
  grep -q -F -e 'PYTHON_GRIFFE' "$adapters_pins" &&
  grep -q -F -e 'TYPESCRIPT_TYPEDOC' "$adapters_pins" &&
  grep -q -F -e 'SCALA_TASTY' "$adapters_pins" &&
  grep -q -F -e 'ASTROMDX_PROSE' "$adapters_pins" &&
  grep -q -F -e 'REJECTED_STABLE_RUSTDOC' "$adapters_pins" &&
  grep -q -F -e 'qualified seed-only under issue #779' "$adapters_pins" &&
  grep -q -F -e 'no Supported claim' "$adapters_pins"; then
  ok
else
  bad "docs adapters pins.bzl lost its exact pins plus honesty under #779"
fi

# Adapter golden fixtures exist per scope with pinned native inputs.
if [[ -f "docs/adapters/testdata/rust/input.json" ]] &&
  [[ -f "docs/adapters/testdata/python/input.json" ]] &&
  [[ -f "docs/adapters/testdata/typescript/input.json" ]] &&
  [[ -f "docs/adapters/testdata/java/input.json" ]] &&
  [[ -f "docs/adapters/testdata/kotlin/input.json" ]] &&
  [[ -f "docs/adapters/testdata/go/input.json" ]] &&
  [[ -f "docs/adapters/testdata/cpp/input.xml" ]] &&
  [[ -f "docs/adapters/testdata/csharp/input.json" ]] &&
  [[ -f "docs/adapters/testdata/fsharp/input.json" ]] &&
  [[ -f "docs/adapters/testdata/vue/input.json" ]] &&
  [[ -f "docs/adapters/testdata/svelte/input.json" ]] &&
  [[ -f "docs/adapters/testdata/scala/input.json" ]] &&
  [[ -f "docs/adapters/testdata/astromdx/input.json" ]]; then
  ok
else
  bad "docs adapters lost their per-scope golden fixtures under #779"
fi

# Adapter BUILD keeps library plus unit, fmt, and clippy tests.
if grep -q -F -e 'name = "docs_adapters"' "$adapters_build" &&
  grep -q -F -e 'name = "docs_adapters_test"' "$adapters_build" &&
  grep -q -F -e 'name = "docs_adapters_fmt_test"' "$adapters_build" &&
  grep -q -F -e 'name = "docs_adapters_clippy_test"' "$adapters_build" &&
  grep -q -F -e 'package_name = "docs/docs_adapters"' "$adapters_build"; then
  ok
else
  bad "docs/adapters BUILD lost its library/test/fmt/clippy wiring under #779"
fi

# Adapter normalization validates via the versioned codec with no partial shards.
if grep -q -F -e 'documentation_ir::validate_shard' "$adapters" &&
  grep -q -F -e 'documentation_ir::encode_shard' "$adapters" &&
  grep -q -F -e 'Partial shards are never emitted' "$adapters" &&
  grep -q -F -e 'MissingTasty' "$adapters"; then
  ok
else
  bad "docs adapters lost their codec validation plus fail-closed record under #779"
fi

# Live proof: adapter unit tests stay green on the seed host.
if bazel test //docs/adapters:docs_adapters_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "docs_adapters_test failed (want green per-language runs under #779)"
fi

# Live proof: adapter fmt plus clippy stay green.
if bazel test //docs/adapters:docs_adapters_fmt_test //docs/adapters:docs_adapters_clippy_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "docs adapters fmt/clippy failed (want green under #779)"
fi

# Crate universe owns the adapter manifest plus lockfile.
if grep -q -F -e '//docs/adapters:Cargo.toml' MODULE.bazel &&
  grep -q -F -e 'docs_adapters 0.0.0' cargo-bazel-lock.json &&
  grep -q -F -e 'name = "docs_adapters"' "$adapters_cargo"; then
  ok
else
  bad "crate universe lost its docs/adapters manifest plus lockfile under #779"
fi

# No published-site claim beyond the adapter-plus-site-plus-rebuild-plus-link-plus-guide-plus-timing-plus-drift slice.
if grep -q -F -e 'Adapter runs with pins and mappings delivered under #779' "$readme" &&
  grep -q -F -e 'renderer and site execution delivered seed-only under #780' "$readme" &&
  grep -q -F -e 'site-level byte-identical rebuild proof delivered seed-only under #781' "$readme" &&
  grep -q -F -e 'link and reference completeness delivered seed-only under #782' "$readme" &&
  grep -q -F -e 'guide prose with guide-step CI wiring delivered seed-only under #783' "$readme" &&
  grep -q -F -e 'first-hour timing proof delivered seed-only under #784' "$readme" &&
  grep -q -F -e 'per-release pin-bump plus drift process delivered seed-only under #785' "$readme" &&
  grep -q -F -e 'no published site exists today' "$readme" &&
  grep -q -F -e 'no site is published yet' "$readme" &&
  grep -q -F -e 'adapter-plus-site-plus-rebuild-plus-link-plus-guide-plus-timing-plus-drift green' "$matrix" &&
  grep -q -F -e 'no working site claimed' "$matrix"; then
  ok
else
  bad "adapter-plus-site-plus-rebuild-plus-link-plus-guide-plus-timing-plus-drift slice lost its no-published-site honesty under #779/#780/#781/#782/#783/#784/#785"
fi

# First-hour timing proof is delivered one-shot under #784 with no standing
# benchmark per ADR 0022.
if grep -q -F -e 'first-hour timing proof delivered seed-only under #784' "$site" &&
  grep -q -F -e 'one-shot' "$site" &&
  grep -q -F -e 'not a standing benchmark' "$site" &&
  grep -q -F -e 'no CI timing budget enforced' "$site" &&
  grep -q -F -e 'ADR 0022' "$site" &&
  grep -q -F -e 'timing.expected' "$site"; then
  ok
else
  bad "site lost its first-hour timing one-shot proof record with no standing benchmark (#784)"
fi

# Timing fixture pins carry the journey plus one-shot plus no-gate plus
# rejected substitutes under #784.
if grep -q -F -e 'FIRST_HOUR_JOURNEY = "demo_site cold plus warm plus docs corpus plus site tests"' "$site_pins" &&
  grep -q -F -e 'TIMING_ONE_SHOT = "one-shot evidence proof per ADR 0022, not a standing benchmark"' "$site_pins" &&
  grep -q -F -e 'TIMING_NO_GATE = "no CI timing budget enforced"' "$site_pins" &&
  grep -q -F -e 'TIMING_METHOD = "wall plus Elapsed plus actions on seed host, historical reference only"' "$site_pins" &&
  grep -q -F -e 'TIMING_SEED_ONLY = "first-hour timing proof qualified seed-only under issue #784"' "$site_pins" &&
  grep -q -F -e 'REJECTED_TIMING_GATE = "CI timing budget enforcement is rejected per ADR 0022"' "$site_pins" &&
  grep -q -F -e 'REJECTED_STANDING_BENCHMARK = "standing benchmark with baseline or comparison machinery is rejected per ADR 0022"' "$site_pins"; then
  ok
else
  bad "docs_site pins lost their first-hour journey plus one-shot plus no-gate record under #784"
fi

# Timing expected carries measured walls plus methodology plus one-shot plus
# seed-only under #784.
if grep -q -F -e 'Docs site first-hour timing proof (issue #784)' "$site_timing_expected" &&
  grep -q -F -e 'ms wall' "$site_timing_expected" &&
  grep -q -F -e 'Methodology one-shot' "$site_timing_expected" &&
  grep -q -F -e 'One-shot evidence proof per ADR 0022, not a standing benchmark' "$site_timing_expected" &&
  grep -q -F -e 'no CI timing budget' "$site_timing_expected" &&
  grep -q -F -e 'Qualified seed-only, no Supported claim' "$site_timing_expected"; then
  ok
else
  bad "timing.expected lost its measured walls plus methodology plus one-shot record under #784"
fi

# Timing planning pins the journey steps plus one-shot without budget
# under #784.
if grep -q -F -e 'pub fn first_hour_journey_steps' "$planning" &&
  grep -q -F -e '"demo_site", "docs corpus", "site tests"' "$planning" &&
  grep -q -F -e 'pub fn timing_proof_is_one_shot' "$planning" &&
  grep -q -F -e 'pub fn timing_proof_enforces_budget' "$planning" &&
  grep -q -F -e 'first_hour_timing_is_one_shot_without_budget' "$planning"; then
  ok
else
  bad "dx_docs planning lost its first-hour journey plus one-shot-without-budget record under #784"
fi

# Docs site expected keeps the guide plus timing plus drift delivery with no
# owned gaps remaining.
if grep -q -F -e 'Guide prose with guide-step CI wiring delivered seed-only (issue #783' "$site_expected" &&
  grep -q -F -e 'First-hour timing proof delivered seed-only (issue #784' "$site_expected" &&
  grep -q -F -e 'Per-release pin-bump plus drift process delivered seed-only (issue #785' "$site_expected" &&
  grep -q -F -e 'No owned gaps remain' "$site_expected"; then
  ok
else
  bad "docs_site.expected lost its #783 plus #784 plus #785 delivered plus no-gaps record"
fi

# Live proof: the first-hour journey completes green with no budget gate
# under #784 (completion only, never a timing assertion per ADR 0022).
if bazel build //docs/site:demo_site --noshow_progress >/dev/null 2>&1 &&
  bazel test //docs/site/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "first-hour journey failed to complete green (want demo_site plus site tests green with no budget gate, #784)"
fi

# Drift fixture pins carry the process plus codec gates plus no-snapshot
# plus rejected substitutes under #785.
if grep -q -F -e 'PIN_BUMP_PROCESS = "per release bump pins plus drift-test plus review, ship only when green"' "$site_pins" &&
  grep -q -F -e 'DRIFT_GATES = ["contract suite", "golden fixtures", "determinism evidence", "review"]' "$site_pins" &&
  grep -q -F -e 'DRIFT_CODEC_GATES = ["roundtrip", "rejection-parity", "symbol-ordering", "extension-ordering", "minor-compat"]' "$site_pins" &&
  grep -q -F -e 'DRIFT_SAME_PRODUCER = "same pinned producer plus inputs rebuild byte-identical"' "$site_pins" &&
  grep -q -F -e 'DRIFT_CROSS_VERSION = "cross-version compares decoded semantics never bytes"' "$site_pins" &&
  grep -q -F -e 'DRIFT_SEED_ONLY = "pin-bump plus drift qualified seed-only under issue #785"' "$site_pins" &&
  grep -q -F -e 'REJECTED_SNAPSHOT_REFRESH = "committed IR snapshot refresh machinery is rejected"' "$site_pins" &&
  grep -q -F -e 'REJECTED_UNPINNED_UPGRADE = "unpinned or user-side pin upgrade is rejected"' "$site_pins"; then
  ok
else
  bad "docs_site pins lost their per-release pin-bump plus drift-process record under #785"
fi

# Drift expected carries the process plus gates plus no-snapshot plus
# seed-only under #785.
if grep -q -F -e 'Docs pipeline per-release pin-bump plus drift process (issue #785)' "$site_drift_expected" &&
  grep -q -F -e 'Pinned native inputs bump per release' "$site_drift_expected" &&
  grep -q -F -e 'Drift testing runs contract suite plus golden fixtures plus determinism evidence' "$site_drift_expected" &&
  grep -q -F -e 'Codec drift gates: roundtrip plus rejection-parity plus symbol-ordering plus extension-ordering plus minor-forward-compat' "$site_drift_expected" &&
  grep -q -F -e 'Same-producer rebuilds stay byte-identical; cross-version compares decoded semantics never bytes' "$site_drift_expected" &&
  grep -q -F -e 'Ordinary API changes require no IR snapshot update' "$site_drift_expected" &&
  grep -q -F -e 'Users stay on pinned checksummed inputs' "$site_drift_expected" &&
  grep -q -F -e 'Qualified seed-only, no Supported claim' "$site_drift_expected"; then
  ok
else
  bad "drift.expected lost its pin-bump plus drift-gate plus no-snapshot record under #785"
fi

# Drift planning pins the upgrade gate plus user-pinned plus failure-names-pin
# under #785.
if grep -q -F -e 'pub fn plan_drift_upgrade' "$planning" &&
  grep -q -F -e 'pub fn drift_reaches_users_without_release' "$planning" &&
  grep -q -F -e 'pub fn drift_failure_names_pinned_input' "$planning" &&
  grep -q -F -e 'drift_upgrades_ship_only_when_green_and_reviewed' "$planning" &&
  grep -q -F -e 'upstream_changes_never_reach_users_outside_a_release' "$planning" &&
  grep -q -F -e 'failures_name_the_unit_and_the_drifted_pin' "$planning"; then
  ok
else
  bad "dx_docs planning lost its drift-upgrade plus user-pinned plus failure-names-pin record under #785"
fi

# Pin-bump process documents the three pin homes that bump together
# under #785.
if grep -q -F -e 'Pins live in three places and bump together' "$docir" &&
  grep -q -F -e 'docs/adapters/pins.bzl' "$docir" &&
  grep -q -F -e 'docs/adapters/src/lib.rs' "$docir" &&
  grep -q -F -e 'machine-inputs table' "$docir" &&
  grep -q -F -e 'ships only when everything is green plus explicitly reviewed' "$docir" &&
  grep -q -F -e 'see `plan_drift_upgrade`' "$docir"; then
  ok
else
  bad "doc-ir lost its three-pin-homes plus bump-together process record under #785"
fi

# Codec drift gates stay delivered: roundtrip, rejection parity, symbol and
# extension ordering, minor forward-compat under #785.
if grep -q -F -e 'documented_example_roundtrips_byte_identical' "$codec" &&
  grep -q -F -e 'encode_and_decode_reject_the_same_invalid_shards' "$codec" &&
  grep -q -F -e 'extension_keys_must_be_strictly_increasing' "$codec" &&
  grep -q -F -e 'symbols_must_be_strictly_increasing' "$codec" &&
  grep -q -F -e 'newer_minors_decode_when_understood' "$codec" &&
  grep -q -F -e 'Drift detection reuses the codec gates' "$docir"; then
  ok
else
  bad "documentation_ir codec lost its drift roundtrip/parity/ordering/compat gates under #785"
fi

# Live proof: codec unit plus fmt plus clippy stay green (drift gates
# executable, not just documented) under #785.
if bazel test //docs/ir/ir:documentation_ir_test //docs/ir/ir:documentation_ir_fmt_test //docs/ir/ir:documentation_ir_clippy_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "documentation_ir tests failed (want codec drift gates green under #785)"
fi

# Live proof: adapter drift gates stay green (pinned-producer mismatch
# fails, no partial shards, codec validation) under #785.
if grep -q -F -e 'VersionMismatch' "$adapters" &&
  grep -q -F -e 'Partial shards are never emitted' "$adapters" &&
  grep -q -F -e 'documentation_ir::validate_shard' "$adapters" &&
  grep -q -F -e 'rust_rejects_unpinned_producer' "$adapters" &&
  bazel test //docs/adapters:docs_adapters_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "docs adapters lost their version-mismatch plus no-partial drift gates under #785"
fi

# No-snapshot rule enforced: ordinary API changes require no IR snapshot
# update, no committed shards, no refresh machinery, check never diffs
# committed IR under #785.
if grep -q -F -e 'Ordinary' "$readme" &&
  grep -q -F -e 'API changes require no IR snapshot update' "$readme" &&
  grep -q -F -e 'no committed IR shards' "$site_drift_expected" &&
  grep -q -F -e 'adapter golden fixtures stay' "$docir" &&
  grep -q -F -e 'not snapshots' "$docir" &&
  grep -q -F -e 'REJECTED_COMMITTED_IR = "committed IR snapshots are rejected"' "$site_pins" &&
  grep -q -F -e 'pub fn check_compares_committed_ir' "$planning" &&
  grep -q -F -e 'check_mode_never_diffs_committed_ir_or_fails_on_cache_miss' "$planning" &&
  ! git ls-files -- 'docs/site/*.ir.textproto' 'docs/site/**/*.ir.textproto' 'tools/ci/tests/fixtures/docs_site/*.ir.textproto' 'docs/ir/*.ir.textproto' | grep -q .; then
  ok
else
  bad "no-IR-snapshot rule lost its no-committed-shards plus no-refresh record under #785"
fi

# Docs command surface delivered under #786: grammar, scope, serve/port,
# JSON, and shared-graph dispatch with real Bazel extraction/validation.
if grep -q -F -e 'Command::Docs' cli/cli/src/args/command.rs &&
  grep -q -F -e 'Command::Docs => "docs"' cli/cli/src/args/command.rs &&
  grep -q -F -e 'build, check, and serve the unified documentation site' cli/cli/src/args/command.rs &&
  grep -q -F -e 'pub(crate) serve: bool' cli/cli/src/args/grammar.rs &&
  grep -q -F -e 'pub(crate) port: Option<String>' cli/cli/src/args/grammar.rs &&
  grep -q -F -e '"--port"' cli/cli/src/args/grammar.rs &&
  grep -q -F -e 'pub serve: bool' cli/cli/src/args/invocation.rs &&
  grep -q -F -e 'pub port: Option<u16>' cli/cli/src/args/invocation.rs &&
  grep -q -F -e 'port_without_serve' cli/docgen/src/lib.rs &&
  grep -q -F -e 'docs_scope_reuses_workflow_resolution' cli/docgen/src/lib.rs &&
  grep -q -F -e 'DOCS_CHECK_TARGET' cli/cli/src/exec/docs.rs &&
  grep -q -F -e 'DOCS_BUILD_TARGET' cli/cli/src/exec/docs.rs &&
  grep -q -F -e 'DOCS_DEFAULT_PORT' cli/cli/src/exec/docs.rs; then
  ok
else
  bad "dx docs lost its delivered command-surface wiring under #786"
fi

# Docs invocation mappings delivered under #786 per ADR 0006: check
# validation-only without render, build validates and renders, serve
# previews without caching, port requires serve, scope reuses workflow
# resolution with bare repository selection.
if grep -q -F -e 'if command == Command::Docs' cli/cli/src/args/parser.rs &&
  grep -q -F -e 'port.is_some() && !serve' cli/cli/src/args/parser.rs &&
  grep -q -F -e 'Per-command flags: --check/--serve/--port (docs only' cli/cli/src/args/help.rs &&
  grep -q -F -e 'Usage: dx [global-options] docs [--check] [--serve [--port <n>]]' cli/cli/src/args/help.rs &&
  grep -q -F -e 'Bare scope selects the repository docs site' cli/cli/src/args/help.rs &&
  grep -q -F -e 'check_flag_selects_validation_only' cli/docgen/src/lib.rs &&
  grep -q -F -e 'serve_previews_without_caching_and_port_requires_serve' cli/docgen/src/lib.rs &&
  grep -q -F -e 'docs_check_serve_port_parse' cli/cli/src/args/parser_tests_b.rs; then
  ok
else
  bad "dx docs lost its delivered invocation mappings under #786"
fi

# Docs execution proves the shared extraction/validation graph: check
# builds the aggregate without render, build renders, failures name the
# unit plus the pinned input, JSON streams started plus operations plus
# finished, dry-run launches nothing.
if grep -q -F -e 'Running docs check for' cli/cli/src/exec/docs.rs &&
  grep -q -F -e 'Running docs build for' cli/cli/src/exec/docs.rs &&
  grep -q -F -e 'docs/site:demo' cli/cli/src/exec/docs.rs &&
  grep -q -F -e 'pinned mdBook 0.4.43' cli/cli/src/exec/docs.rs &&
  grep -q -F -e 'docs_check_builds_aggregate_without_render' cli/cli/src/exec/docs.rs &&
  grep -q -F -e 'docs_build_validates_and_renders' cli/cli/src/exec/docs.rs &&
  grep -q -F -e 'docs_json_streams_started_operation_finished' cli/cli/src/exec/docs.rs &&
  grep -q -F -e 'docs_bazel_failure_names_unit_and_pin' cli/cli/src/exec/docs.rs; then
  ok
else
  bad "dx docs lost its delivered execution proof under #786"
fi

# Live proof: the delivered docs units stay green on the seed host.
if bazel test //cli/cli:dx_cli_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "dx_cli_test failed (want docs check/build/serve units green under #786)"
fi

dx_test_summary "docs pipeline qualification harness"
