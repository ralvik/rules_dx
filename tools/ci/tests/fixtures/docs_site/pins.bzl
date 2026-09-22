"""Docs site execution plus rebuild plus link pins (mdBook).

Contract: `docs/documentation/site.md`.
Fixture: `tools/ci/tests/fixtures/docs_site/` via `bazel run //tools/ci:docs_pipeline_qualification`.
"""

# Pinned renderer: mdBook is the decided renderer with no planned
# replacement; the fixture render stamps this version into site outputs.
MDBOOK_VERSION = "0.4.43"
MDBOOK_DECIDED = "mdBook is the decided renderer with no planned replacement"

# Action graph: one DocsExtract per (language, package) unit, one
# DocsAggregate with shared validation, one DocsRender with the pinned
# renderer; check selects extract plus aggregate without render.
ACTIONS = ["Extract", "Aggregate", "Render"]
CHECK_ACTIONS = ["Extract", "Aggregate"]
BUILD_ACTIONS = ["Extract", "Aggregate", "Render"]

# Execution demo: miniature python/demo unit proving the cached chain.
DEMO_LANGUAGE = "python"
DEMO_PACKAGE = "demo"
DEMO_SYMBOLS = [
    "python:demo:AccountService.create",
    "python:demo:AccountService.get",
]

# Render inputs: mdBook-compatible prose plus generated API pages.
SUMMARY_MARKER = "# Summary"
API_MARKER = "# API Reference"
PROSE_MARKER = "# Demo Guide"

# Search index: one index built directly from prose plus IR, never from
# rendered HTML; sorted keys keep rebuilds deterministic.
SEARCH_INDEX = "searchindex.json"
SEARCH_RECORD_KEYS = ["body", "title", "url"]

# Link/reference completeness at the pre-render boundary: prose plus
# generated API pages resolve all internal links/references with no
# dangling targets. Delivered seed-only under issue #782 via shared
# validation in DocsAggregate (inline plus reference-definition targets
# checked, remote URLs skipped never fetched, dangling fails the action
# with no partial outputs).
LINK_COMPLETENESS = "prose plus API pages resolve all internal links with no dangling targets under issue #782"
LINK_BOUNDARY = "pre-render boundary shared validation in DocsAggregate"
LINK_REMOTE = "remote URLs skipped never fetched"
LINK_FAIL_CLOSED = "dangling links fail the aggregate action with no partial outputs"
LINK_DEMO = "demo prose links api.md plus #getting-started plus remote skip"
LINK_SEED_ONLY = "link completeness qualified seed-only under issue #782"

# Guide prose with guide-step CI wiring: user guide prose where every step
# is verified by CI (steps actually execute). Delivered seed-only under
# issue #783 via the fixture-scale quickstart guide plus its executable
# steps file, run by docs_pipeline_qualification in CI with no unexecuted
# steps allowed and prose kept in sync with the steps file.
GUIDE_KNOWN = ["quickstart", "tutorial", "migration"]
GUIDE_PROSE = "docs/site/demo/guide.md with executable quickstart steps"
GUIDE_STEPS = "docs/site/demo/guide_steps.txt with one executable step per line"
GUIDE_CI = "every guide step CI-executed via docs_pipeline_qualification with no unexecuted steps"
GUIDE_FRESHNESS = "fresh only when every step ran and examples stayed green"
GUIDE_SEED_ONLY = "guide-step wiring qualified seed-only under issue #783"

# Determinism: sorted symbol order, sorted JSON keys, workspace-relative
# paths only, no timestamps, no absolute paths, locale-independent sort.
# Delivered seed-only under issue #781 via two-builds-diffed live proof.
DETERMINISM = "LC_ALL=C sort with no timestamps and workspace-relative paths only"
REBUILD_PROOF = "two builds hashed and diffed byte-identical under issue #781"
REBUILD_INPUTS = "demo_extract plus demo_aggregate plus demo_render outputs"
REBUILD_OUTPUTS = "shard plus SUMMARY plus API plus records plus entry plus search index"
REBUILD_DETERMINISM = "sorted symbol order plus sorted JSON keys plus LF bytes with no timestamps and no absolute paths"

# Lifecycle: generated IR stays in Bazel outputs, never beside sources or
# in Git; builds write bazel-out plus cache entries only.
GENERATED_IR = "demo_extract.ir.textproto in bazel-bin, never beside sources"
NO_SOURCE_WRITES = "no source-tree writes"
NO_COMMITTED_IR = "no committed IR shards"

# Rejected substitutes.
REJECTED_COMMITTED_IR = "committed IR snapshots are rejected"
REJECTED_SNAPSHOT_REFRESH = "committed IR snapshot refresh machinery is rejected"
REJECTED_UNPINNED_UPGRADE = "unpinned or user-side pin upgrade is rejected"
REJECTED_HTML_PARSED_INDEX = "search index parsing rendered HTML is rejected"
REJECTED_WATCHER = "custom watcher or refresh engine is rejected; Bazel incrementality only"
REJECTED_MDBOOK_TEST = "mdbook test wrapper is rejected"
REJECTED_DANGLING_SILENT = "silent dangling link pass is rejected"
REJECTED_UNEXECUTED = "unexecuted guide steps are rejected"
REJECTED_TIMING_GATE = "CI timing budget enforcement is rejected per ADR 0022"
REJECTED_STANDING_BENCHMARK = "standing benchmark with baseline or comparison machinery is rejected per ADR 0022"

# First-hour timing proof at the built site/docs flow: the fresh-checkout
# journey completes in the first hour. Delivered seed-only under issue #784
# as one-shot evidence proof per ADR 0022, never a standing benchmark with
# no CI timing budget enforced.
FIRST_HOUR_JOURNEY = "demo_site cold plus warm plus docs corpus plus site tests"
TIMING_STEPS = ["demo_site", "docs corpus", "site tests"]
TIMING_ONE_SHOT = "one-shot evidence proof per ADR 0022, not a standing benchmark"
TIMING_NO_GATE = "no CI timing budget enforced"
TIMING_METHOD = "wall plus Elapsed plus actions on seed host, historical reference only"
TIMING_SEED_ONLY = "first-hour timing proof qualified seed-only under issue #784"

# Per-release pin-bump plus drift process: maintainers bump every pinned
# native input per release, run drift testing, and ship only when green plus
# reviewed. Delivered seed-only under issue #785 via the codec
# roundtrip/parity/ordering/compat gates plus adapter version-mismatch plus
# same-producer byte-identical proof; ordinary API changes require no IR
# snapshot update and adapter goldens stay inputs, not snapshots.
PIN_BUMP_PROCESS = "per release bump pins plus drift-test plus review, ship only when green"
DRIFT_GATES = ["contract suite", "golden fixtures", "determinism evidence", "review"]
DRIFT_CODEC_GATES = ["roundtrip", "rejection-parity", "symbol-ordering", "extension-ordering", "minor-compat"]
DRIFT_SAME_PRODUCER = "same pinned producer plus inputs rebuild byte-identical"
DRIFT_CROSS_VERSION = "cross-version compares decoded semantics never bytes"
DRIFT_NO_SNAPSHOT = "ordinary API changes require no IR snapshot update; no committed IR"
DRIFT_USER_PINNED = "users stay on pinned checksummed inputs; upstream change reds release prep never user build"
DRIFT_SEED_ONLY = "pin-bump plus drift qualified seed-only under issue #785"

COMPAT_SEED_ONLY = "Compatibility: seed Linux x86_64 only"
NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issues #780 plus #781 plus #782 plus #783 plus #784 plus #785"
REBUILD_SEED_ONLY = "rebuild proof qualified seed-only under issue #781"
LINK_SEED_ONLY_QUAL = "link completeness qualified seed-only under issue #782"
OWNED_GAP = "no owned gaps remain; full pipeline delivered seed-only under #779 plus #780 plus #781 plus #782 plus #783 plus #784 plus #785"
