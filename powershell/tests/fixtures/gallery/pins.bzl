"""Gallery lock wiring pins.

Contract: `docs/product/support-matrix.md#additional-foundations`,
`docs/generation/powershell.md#evidence`.
"""

# Upstream ruleset pin (MODULE.bazel plus MODULE.bazel.lock).
RULES_POWERSHELL_VERSION = "0.2.0"

# Portable runtime pin (powershell.toolchain in MODULE.bazel).
PWSH_VERSION = "7.5.4"

# Gallery lock authority (maintainer-owned, committed, never hand-edited
# beyond the documented regeneration). Exact nupkg identities live in
# //third_party/powershell:PSGallery.lock.json; this fixture pins the
# versions that lock must carry.
PESTER_VERSION = "5.7.1"
PSSCRIPTANALYZER_VERSION = "1.25.0"

# Gallery source (sole remote in the lock).
GALLERY_SOURCE = "https://www.powershellgallery.com/api/v2"

# Lock files (consumed, never written, by generation).
GALLERY_LOCK = "//third_party/powershell:PSGallery.lock.json"
GALLERY_REQUIREMENTS = "//third_party/powershell:PSGallery.requirements.psd1"

# Fail-closed contract: generation consumes the lock, never writes it;
# consumer builds never run Install-Module/Install-PSResource.
GALLERY_GENERATION = "consumes never writes"
GALLERY_FAIL_CLOSED = "fail_if_repin_required"

# Live proof consumers (wrapper consumers over the pinned toolchain).
GALLERY_FIXTURE_HELLO = "//powershell/tests/fixtures/hello:hello_lib"
GALLERY_FIXTURE_PESTER = "//powershell/tests/fixtures/pester:greeter_test"

# Rejected: consumer Install-Module/Install-PSResource; floating Gallery
# requirements; ambient module discovery.
GALLERY_REJECTED = "Install-Module rejected: exact lock plus explicit-path import only"

# Currency recheck (issue #932 pattern): modules below were verified
# current on this date (Pester 5.7.1 plus PSScriptAnalyzer 1.25.0 per ADR
# 0008 latest-stable, Pester 6.1.0 plus pwsh 7.6.5 observed). Refresh the
# date with each dependency-currency pass.
GALLERY_CURRENCY_RECHECK = "2026-09-22"
