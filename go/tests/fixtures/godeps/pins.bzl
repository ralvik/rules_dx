"""Go from_file lock wiring pins.

Contract: `docs/product/support-matrix.md#provisional-default-dependency-locks`.
"""

# Upstream ruleset pin (MODULE.bazel plus MODULE.bazel.lock).
RULES_GO_VERSION = "0.63.0"

# Toolchain pin (go_sdk.download in MODULE.bazel, owned by
# //modules:toolchains.bzl GO_SDK_VERSION).
GO_SDK_VERSION = "1.26.6"

# Language floor (go directive in third_party/go/go.mod, owned by
# //modules:toolchains.bzl GO_LANGUAGE_FLOOR). Tracks gazelle 0.52.2's
# go 1.24.12 so the shared go_deps extension sees no version conflict;
# the SDK minor stays >= this floor (issue #1003).
GO_LANGUAGE_FLOOR = "1.24.12"

# Gazelle ruleset pin (go_deps extension owner).
GAZELLE_VERSION = "0.52.2"

# Lock wiring: single-module go.mod via from_file (go.work only for
# multi-module layouts). Versions match gazelle 0.52.2 go.mod so the
# shared go_deps extension sees no version conflict.
GODEPS_FROM_FILE = "go_deps.from_file"
GODEPS_GO_MOD = "//third_party/go:go.mod"
GODEPS_GO_SUM = "//third_party/go:go.sum"
GODEPS_GO_WORK_ABSENT = "go.work only for multi-module"

# Direct pins (require entries in third_party/go/go.mod).
BUILDTOOLS_VERSION = "v0.0.0-20250930140053-2eb4fccefb52"
GO_CMP_VERSION = "v0.6.0"
DIFLIB_VERSION = "v1.0.0"

# External repo labels (use_repo from the go_deps extension).
GODEPS_REPO_BUILDTOOLS = "@com_github_bazelbuild_buildtools"
GODEPS_REPO_GO_CMP = "@com_github_google_go_cmp"
GODEPS_REPO_DIFLIB = "@com_github_pmezard_go_difflib"

# Fixture labels (wrapper consumers over the pinned hub).
GODEPS_FIXTURE_LIB = "//go/tests/fixtures/godeps:godeps"
GODEPS_FIXTURE_TEST = "//go/tests/fixtures/godeps:godeps_test"

# Fixture import plus label shape (handwritten deps name external labels
# directly; exact resolve mapping in the fixture BUILD).
GODEPS_IMPORT = "github.com/google/go-cmp/cmp"
GODEPS_LABEL = "@com_github_google_go_cmp//cmp:cmp"

# Generation contract: consumes the lock, never writes it.
GODEPS_GENERATION = "consumes never writes"

# Rejected: hand-written go_deps.module tags (Gazelle documents from_file
# as preferred); go.work for single-module layouts.
GODEPS_REJECTED = "hand module tags rejected: from_file preferred; go.work only for multi-module"

# Currency recheck (issue #932): directives below were verified current on
# this date (go language floor plus SDK plus buildtools per ADR 0008
# latest-stable). Refresh the date with each dependency-currency pass;
# pin_consistency.sh fails when absent or stale.
GODEPS_CURRENCY_RECHECK = "2026-09-22"
