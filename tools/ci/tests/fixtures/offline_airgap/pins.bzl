"""Offline/airgap bootstrap pins.
Contract: `docs/deploy/offline-bootstrap.md`.
Fixture: `tools/ci/tests/fixtures/offline_airgap/` via
`bazel run //tools/ci:offline_airgap_qualification`.
Vendored launcher plus advisory mirror with checksum manifests, verified
before install with no network; seed-only, no Supported claim.
"""

# Vendored bundle: pinned launcher bytes plus advisory mirror bytes,
# each bound by a SHA256SUMS manifest in `sha256sum` line format.
BUNDLE_LAUNCHER = "bundle/bazelisk/bazelisk-linux-amd64 with bundle/bazelisk/SHA256SUMS"
BUNDLE_ADVISORY = "bundle/advisory/cargo.json with bundle/advisory/SHA256SUMS"
BUNDLE_NO_NETWORK = "bootstrap installs plus populates with curl/wget shadowed by failing stubs"

# Advisory mirror: `file://` provenance validates like upstream
# `https://` with the same sha256 plus same-day freshness gates.
MIRROR_URL = "file:// vendored provenance via dx_audit::advisory::is_local_mirror"
MIRROR_FRESHNESS = "same-day retrieved_at gate, stale fails with advisory_refresh_failed"
MIRROR_FAIL_CLOSED = "missing, invalid, tampered, or stale mirror never clean, never a stale fallback"

# Setup/env/codegen: unchanged selection plus atomic commit from the
# bundle; `--dry-run` plans with no launch and no fetch.
SETUP_OFFLINE = "dx setup --dry-run plans with no Bazel launch and no network"
SETUP_FIRST_FETCH = "first Bazel module/toolchain fetch still needs network once"

# Hermetic bundle manifests via //deploy/offline:offline_demo.
STARLARK_MANIFEST = "offline_bundle manifest plus sidecar via hermetic //deploy/rules:hasher"

# Rejected substitutes.
REJECTED_CURL_BOOTSTRAP = "curl-on-first-run is rejected on the offline path"
REJECTED_CHECKSUM_ONLY = "checksum without manifest binding is not bundle proof"
REJECTED_LIVE_MIRROR = "no live mirror service is operated; dry-run/local-mirror fixtures only"
COMPAT_SEED_ONLY = "Compatibility: seed Linux x86_64 only"

NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #774"
OWNED_GAP = "platform plus consumer plus release evidence stays owned gap"
