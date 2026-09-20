"""Paket lock wiring pins (issue #482).

Contract: `docs/product/support-matrix.md#provisional-default-dependency-locks`,
`docs/generation/README.md#language-mapping-qualification`.

Single shared Paket lock for the admitted .NET foundations (C#, F#) per the
support matrix: `paket.dependencies` plus `paket.lock` via `paket2bazel`
into the `paket.main` hub carrying per-package sha512. F# references this
file (same shape as Kotlin referencing `//java/tests/fixtures/junit:pins.bzl`
under issue #476); no `fsharp/tests/fixtures/paket/pins.bzl` duplicate lands.

Direct pins are FSharp.Core 10.1.201 (F# assemblies need the runtime asset
next to the app) plus xunit.v3 4.0.0 plus xunit.analyzers 2.0.0 (qualified
runner closure under issue #477 with the MTP transitive). Source is
`https://api.nuget.org/v3/index.json`, framework is net10.0. The lock files
are user-authored; generation consumes them and never writes them (common
generation contract). NuGet's native `packages.lock.json` is rejected: one
file per project instead of a central lock, hashes incompatible with Bazel's
downloader (rules_dotnet issue 444).
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
