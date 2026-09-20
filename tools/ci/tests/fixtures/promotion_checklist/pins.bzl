"""Promotion checklist pins (issue #611).
Contract: `docs/product/promotion-checklist.md`.
Fixture: `tools/ci/tests/fixtures/promotion_checklist/` via
`bazel run //tools/ci:promotion_checklist_qualification`.
Tag hygiene plus versioning plus what evidence promotes a cell; seed-only, no Supported claim.
"""

# Tag hygiene as-built: unpublishable module, no tags, ignored outputs, SHA pins.
TAG_MODULE_VERSION = 'version = "0.0.0"'
TAG_NO_TAGS = "no v* tags without explicit owner approval"
TAG_IGNORED_OUTPUTS = "/dist/ plus /release/ stay git-ignored and never committed"
TAG_CALLER_SHA_PINS = "reusable-consumer.yml@<sha> plus reusable-docs.yml@<sha> stay in sync"
TAG_HARNESSES = "bazel run //tools/ci:release_hygiene plus bazel run //tools/ci:consumer_ci_qualification"

# Versioning: SemVer bump reviewed PR, tag pre-pushed approval, never creates tags.
VERSION_BUMP = "bumps MODULE.bazel from 0.0.0 to SemVer in a reviewed PR"
VERSION_NO_RELEASE_CUT = "No release has been cut"
VERSION_TAG_PREPUSHED = "tag is pushed beforehand with owner approval"
VERSION_VERIFY_TAG = "--verify-tag everywhere, never creates or pushes tags"
VERSION_SINGLE_PIN = "Single-version dx == module pin"
VERSION_CALLERS_PIN_COMMITS = "callers pin reviewed commits, never release tags"
VERSION_NEVER_REBUILD = "never rebuilt or substituted silently, byte identity fail-closed"

# Platform evidence per cell: per-host successors plus floors/routes/coverage plus refusal.
PLATFORM_HOSTS = "Linux arm64 native issue #410 plus static-musl profiles issue #411 plus macOS arm64 native issue #412 plus macOS x86_64 best-effort native issue #413 plus Windows x86_64 MSVC-compatible native issue #414"
PLATFORM_FLOORS = "bazel run //tools/ci:deployment_floors_qualification"
PLATFORM_ROUTES = "bazel run //tools/ci:cross_routes_qualification"
PLATFORM_COVERAGE = "bazel run //tools/ci:coverage_qualification"
PLATFORM_REFUSAL = "clean unsupported_platform refusal, best-effort gaps never block required-host release"

# Consumer evidence per cell: nine checks plus explicit platforms plus self-call plus updater.
CONSUMER_NINE_CHECKS = "Nine checks through the reusable workflow with explicit platform selection"
CONSUMER_SELF_CALL = "All-enabled self-call in ci.yml (issue #408, verbatim //...)"
CONSUMER_SELF_CALL_REJECTED = "build-only self-call forever rejected"
CONSUMER_QUALIFICATION = "bazel run //tools/ci:consumer_ci_qualification"
CONSUMER_UPDATER = "sole updater (native-only, issue #461)"

# Release evidence per cell: SBOM/signing/BCR/GHCR/human-run/verifier/dry-run plus harnesses.
RELEASE_SBOM = "//deploy/release:sbom_demo with subject digest equal to artifact sha256"
RELEASE_SIGNING_FIRST = "//deploy/release:signing_demo, nothing is drafted or published unsigned"
RELEASE_BCR = '//deploy/release:bcr_demo ("submitted": False)'
RELEASE_GHCR_SEPARATE = "GHCR stays the separate .github/workflows/ghcr.yml route (issue #460)"
RELEASE_HUMAN_RUN = "deploy/release/release.sh in dry-run mode by default (issue #458)"
RELEASE_VERIFIER_REFUSAL = "//deploy/install:dx_verify refuses checksum-only inputs"
RELEASE_DRY_RUN_FIRST = "workflow_dispatch-only with a default-closed approve gate"
RELEASE_HARNESSES = "bazel run //tools/ci:release_hygiene plus bazel run //tools/ci:release_policy plus bazel run //tools/ci:publish_trust plus bazel run //tools/ci:distribution_closeout_guards plus bazel run //tools/ci:signing_distribution_qualification"

# Promotion rule plus rejected substitutes.
PROMOTION_GATE = "bazel run //tools/ci:supported_evidence_gate"
PROMOTION_ONLY_RELEASE_QUALIFICATION = "Supported promotion occurs only during release qualification"
PROMOTION_MISSING_BLOCKS = "A missing cell remains an explicit gap and blocks that support claim"
REJECTED_ADHOC = "Ad-hoc release without this checklist is rejected"
COMPAT_RELEASE_ONLY = "Compatibility: Release only"

NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #611"
OWNED_GAP = "platform plus consumer plus release evidence stays owned gap"
