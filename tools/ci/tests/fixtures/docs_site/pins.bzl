"""Docs site execution plus rebuild pins (renderer plus site execution, mdBook).

Contract: `docs/documentation/site.md`.
Fixture: `tools/ci/tests/fixtures/docs_site/` via
`bazel run //tools/ci:docs_pipeline_qualification`.
Execution: `//docs/site:demo_site` extract to aggregate to render over
miniature inputs; generated IR stays in Bazel outputs, never beside
sources; rebuild proof hashes two builds and diffs them; seed-only,
no Supported claim.
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
REJECTED_HTML_PARSED_INDEX = "search index parsing rendered HTML is rejected"
REJECTED_WATCHER = "custom watcher or refresh engine is rejected; Bazel incrementality only"
REJECTED_MDBOOK_TEST = "mdbook test wrapper is rejected"

COMPAT_SEED_ONLY = "Compatibility: seed Linux x86_64 only"
NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issues #780 plus #781"
REBUILD_SEED_ONLY = "rebuild proof qualified seed-only under issue #781"
OWNED_GAP = "link completeness plus guide-step wiring plus timing proof plus pin-bump/drift stay owned gaps under #782-#785"
