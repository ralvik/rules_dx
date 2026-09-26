"""Pester runner wiring pins.

"""

RULES_POWERSHELL_VERSION = "0.2.0"

PWSH_VERSION = "7.5.4"

PESTER_VERSION = "5.7.1"
PESTER_MINIMUM_PS = "5.1"
PESTER_GALLERY = "https://www.powershellgallery.com/packages/Pester"

PESTER_FIXTURE_LIB = "//powershell/tests/fixtures/pester:greeter_lib"
PESTER_FIXTURE_TEST = "//powershell/tests/fixtures/pester:greeter_test"

PESTER_REJECTED = "unpinned runner rejected: exact Gallery lock plus explicit-path import only, no Install-Module"

PESTER_CURRENCY_RECHECK = "2026-09-22"
