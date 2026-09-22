"""Pester runner wiring pins.

Contract: `docs/product/support-matrix.md#additional-v1-foundations`,
`docs/generation/powershell.md#evidence`.
"""

# Upstream ruleset pin (MODULE.bazel plus MODULE.bazel.lock).
RULES_POWERSHELL_VERSION = "0.2.0"

# Portable runtime pin (powershell.toolchain in MODULE.bazel).
PWSH_VERSION = "7.5.4"

# Provisional Pester runner pin (Gallery lock authority in
# //third_party/powershell:PSGallery.lock.json). 5.7.1 is the widely
# deployed stable (12M Gallery downloads, Jan 2025); 6.1.0 observed Aug
# 2026 stays a currency candidate, never a floating runner.
PESTER_VERSION = "5.7.1"
PESTER_MINIMUM_PS = "5.1"
PESTER_GALLERY = "https://www.powershellgallery.com/packages/Pester"

# Live proof consumers (wrapper consumers over the pinned toolchain).
PESTER_FIXTURE_LIB = "//powershell/tests/fixtures/pester:greeter_lib"
PESTER_FIXTURE_TEST = "//powershell/tests/fixtures/pester:greeter_test"

# Rejected: unpinned or floating Pester runner; consumer Install-Module.
PESTER_REJECTED = "unpinned runner rejected: exact Gallery lock plus explicit-path import only, no Install-Module"

# Currency recheck (issue #932 pattern): runner below was verified current
# on this date (Pester 5.7.1 stable plus 6.1.0 observed per ADR 0008
# latest-stable). Refresh the date with each dependency-currency pass.
PESTER_CURRENCY_RECHECK = "2026-09-22"
