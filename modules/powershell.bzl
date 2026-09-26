"""Pinned PowerShell foundation."""

RULES_POWERSHELL_VERSION = "0.2.0"
PWSH_VERSION = "7.5.4"

GALLERY_LOCK = "//third_party/powershell:PSGallery.lock.json"
GALLERY_REQUIREMENTS = "//third_party/powershell:PSGallery.requirements.psd1"

GALLERY_MEMBERS = [
    "Pester 5.7.1",
    "PSScriptAnalyzer 1.25.0",
]
