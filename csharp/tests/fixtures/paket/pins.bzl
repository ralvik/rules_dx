"""Paket lock wiring pins.

Contract: `docs/product/support-matrix.md#provisional-default-dependency-locks`,
`docs/generation/README.md#language-mapping-qualification`.
"""

# Upstream ruleset pin (MODULE.bazel plus MODULE.bazel.lock).
RULES_DOTNET_VERSION = "0.22.1"

# SDK pin (dotnet.toolchain in MODULE.bazel).
DOTNET_SDK_VERSION = "10.0.201"

# Target framework (wrapper default in csharp/fsharp rules).
PAKET_FRAMEWORK = "net10.0"

# NuGet source (sole remote in paket.dependencies).
PAKET_SOURCE = "https://api.nuget.org/v3/index.json"

# Direct pins (nuget entries in third_party/dotnet/paket.dependencies).
FSHARP_CORE_VERSION = "10.1.201"
XUNIT_V3_VERSION = "4.0.0"
XUNIT_ANALYZERS_VERSION = "2.0.0"

# Lock authority (maintainer-owned, committed, never hand-edited beyond
# the documented paket2bazel regeneration).
PAKET_DEPENDENCIES = "//third_party/dotnet:paket.dependencies"
PAKET_LOCK = "//third_party/dotnet:paket.lock"
PAKET_HUB = "//third_party/dotnet/deps:paket.main.bzl"
PAKET_EXTENSION = "//third_party/dotnet/deps:paket.main_extension.bzl"
PAKET_HUB_REPO = "@paket.main"

# Hub wiring: every entry carries its sha512 so the Bazel downloader
# verifies each artifact (fail-closed on stale lock).
PAKET_HASH_ATTR = "sha512"

# Generation contract: consumes the Paket files, never writes them.
PAKET_GENERATION = "consumes never writes"

# Live proof labels (wrapper consumers over the pinned hub).
PAKET_FIXTURE_FSHARP_HELLO = "//fsharp/tests/fixtures/hello:hello_lib"
PAKET_FIXTURE_CSHARP_XUNIT = "//csharp/tests/fixtures/xunit:greeter_test"
PAKET_FIXTURE_FSHARP_XUNIT = "//fsharp/tests/fixtures/xunit:greeter_test"

# Rejected: NuGet native packages.lock.json (per-project file, hashes
# incompatible with Bazel's downloader per rules_dotnet issue 444);
# unpinned or floating Paket entries.
PAKET_REJECTED = "packages.lock.json rejected: central paket.lock plus sha512 only"

# Currency recheck (issue #932): directives below were verified current on
# this date (dotnet SDK 10.0.201 with FSharp.Core 10.1.201 plus xUnit v3
# 4.0.0 per ADR 0008 latest-stable). Refresh the date with each
# dependency-currency pass; pin_consistency.sh fails when absent or stale.
PAKET_CURRENCY_RECHECK = "2026-09-22"
