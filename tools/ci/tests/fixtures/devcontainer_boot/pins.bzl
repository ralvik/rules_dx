"""Devcontainer boot manual plus wont-fix pins.

Contract: `docs/contributing/devcontainer.md#container-boot-manual`.
Fixture: `tools/ci/tests/fixtures/devcontainer_boot/` via
`bazel run //tools/ci:devcontainer_boot_qualification`.
"""

# Seed-host manual boot scope (linux/amd64 only, CI stays check-only).
SEED_HOST = "linux/amd64 seed-host boot only"
BOOT_SHAPE = "devcontainer CLI build plus postCreateCommand execution asserting the managed environment materializes"
POST_CREATE = "bazel run //dx:env then dx setup"
NO_FULL_BUILD = "no full build on create"
CI_SHAPE_ONLY = "devcontainer-check stays parity plus definition shape"

# Manual verify procedure (local docker plus devcontainer up, evidence in PR).
LOCAL_BUILD = "docker build -f .devcontainer/Dockerfile.prebuilt -t dx-devcontainer:local ."
LOCAL_UP = "devcontainer up --workspace-folder ."
TRIGGERS = "on every change to .devcontainer/Dockerfile.prebuilt or .devcontainer/devcontainer.json plus before any gated GHCR push"
EVIDENCE_PR = "with evidence in the same reviewed PR"
OWNER = "sole maintainer owns every row until delegation"

# Wont-fix scope (no multi-platform container support claimed).
WONTFIX_NON_LINUX = "non-Linux runs wont-fix"
WONTFIX_ARM64_BOOT = "linux/arm64 boot wont-fix"
WONTFIX_ARM64_VARIANT = "arm64 prebuilt variant wont-fix"
SCAFFOLD_ARCH = "scaffold devcontainer.json arch-independent"
SEED_SLICE = "amd64-only seed slice pins the amd64 Bazelisk launcher"
NATIVE_ONLY = "natively qualified per #410 plus #411 plus #412 plus #414 not container boot"

# Customer-flows-only qualification (static harness plus on-demand local boot).
LOCAL_ON_DEMAND = "qualified locally or on demand with customer flows only"
CUSTOMER_HARNESS = "bazel run //tools/ci:devcontainer_boot_qualification"
STATIC_ONLY = "static pins builds nothing boots nothing"
ON_DEMAND_BOOT = "plus on-demand local docker build plus devcontainer up"

# Rejected substitutes per the issue alternatives.
REJECTED_BOOT_JOB = "boot job in CI rejected non-customer"
REJECTED_NO_NEW_JOB = "no extra CI job"
REJECTED_CUSTOMER_ONLY = "keep CI customer-only"

# Honesty lines (never pinned as supported here).
NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #648"
INFRA_ONLY = "infra only"
