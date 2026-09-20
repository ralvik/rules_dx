"""GHCR rebuild plus signing rotation pins.

Contract: `docs/contributing/devcontainer.md#ghcr-rebuild-plus-signing-rotation`.
Fixture: `tools/ci/tests/fixtures/ghcr_rebuild_rotation/` via
`bazel run //tools/ci:ghcr_rebuild_rotation_qualification`.
"""

# Base-image pin (digest-pinned FROM, never latest; re-pin deliberately).
BASE_IMAGE = "ubuntu:24.04@sha256:69cecf4bbf72d2d44a9eef1b71fb98c7fb973d78af11399deccef19beb008ad9"
BASE_RESOLVED = "resolved 2026-09-17 from tag ubuntu:24.04"
BASE_POLICY = "re-pin deliberately with evidence, never latest"

# Bazelisk plus Bazel pins (canonical source is setup-bazelisk/action.yml).
BAZELISK_VERSION = "v1.29.0 pinned Bazelisk launcher"
BAZELISK_SHA_LINUX_AMD64 = "5a408715e932c0250d28bd84555f12edbf70117de42f9181691c736eacc4a992"
BAZEL_VERSION = "9.2.0 via USE_BAZEL_VERSION"
BAZELISK_CANONICAL = "canonical source .github/actions/setup-bazelisk/action.yml"

# Cosign pin (single-sourced across signing stack plus GHCR fetch plus dry-run).
COSIGN_VERSION = "v2.4.1 checksum-verified fetch"
COSIGN_SINGLE_SOURCE = "single-sourced SIGNING_COSIGN_VERSION plus ghcr.yml plus sign_deploy.sh"

# TUF trust pins (documented, not self-hosted).
TUF_ROOT = "https://tuf-repo-cdn.sigstore.dev"
TUF_ISSUER = "https://token.actions.githubusercontent.com"
TUF_BUNDLE_MEDIA = "application/vnd.dev.sigstore.bundle.v0.3+json"

# Manual on-demand cadence plus owner plus triggers.
CADENCE = "manual on-demand rebuild plus rotation, recorded"
TRIGGERS = "on upstream release notice plus on base-image refresh or CVE plus before any gated push"
OWNER = "sole maintainer owns every row until delegation"

# Manual rebuild procedure (local build, one reviewed PR, gated push signs).
LOCAL_BUILD = "docker build -f .devcontainer/Dockerfile.prebuilt -t dx-devcontainer:local ."
REVIEWED_PR = "re-pin deliberately with evidence in one reviewed PR"
GATED_PUSH = "workflow_dispatch plus approve: true with cosign sign plus verify"
LOCAL_ON_DEMAND = "qualified locally or on demand with customer flows only"
CUSTOMER_HARNESS = "bazel run //tools/ci:ghcr_rebuild_rotation_qualification"

# Rejected substitutes per the issue alternatives.
REJECTED_SCHEDULED = "scheduled CI rebuild rejected"
REJECTED_NO_PUSH_SCHEDULE = "no push or schedule trigger"
REJECTED_NO_NEW_JOB = "no extra CI job"
REJECTED_CUSTOMER_ONLY = "keep CI customer-only"

# Honesty lines (never pinned as supported here).
NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #647"
INFRA_ONLY = "infra only"
