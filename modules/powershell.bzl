"""Pinned PowerShell foundation."""

# Pinned PowerShell foundation (see MODULE.bazel; per-split pin_consistency in tools/ci/pin_consistency.sh).
RULES_POWERSHELL_VERSION = "0.2.0"
PWSH_VERSION = "7.5.4"

# Gallery lock for the admitted PowerShell foundation (see MODULE.bazel).
GALLERY_LOCK = "//third_party/powershell:PSGallery.lock.json"
GALLERY_REQUIREMENTS = "//third_party/powershell:PSGallery.requirements.psd1"

# Gallery members (Pester runner plus PSScriptAnalyzer tool module).
GALLERY_MEMBERS = [
    "Pester 5.7.1",
    "PSScriptAnalyzer 1.25.0",
]
