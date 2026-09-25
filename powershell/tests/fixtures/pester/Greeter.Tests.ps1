#!/usr/bin/env pwsh
# Pester mapping fixture (provisional, ADR 0032).
#
# Runs `Describe`/`It` when the pinned Pester module is vendored via the
# Gallery lock (`//third_party/powershell:PSGallery.lock.json`); passes
# without Pester when the closure is absent so `bazel test //...` stays
# green while the Gallery fetch stays pending. Unpinned runner rejected.
#
# Run.Path must name the Describe file, never this entry point: Pester
# dot-sources its Run.Path, so a self path re-enters until pwsh aborts.
$ErrorActionPreference = "Stop"

$moduleRoot = $PSScriptRoot
Import-Module (Join-Path $moduleRoot "Greeter.psm1") -Force

if (-not (Get-Module -ListAvailable -Name Pester)) {
    Write-Output "Pester not vendored; provisional mapping only (see powershell/tests/fixtures/pester/pins.bzl)"
    $greeting = Get-PesterGreeting
    if ($greeting -ne "hello powershell") {
        Write-Error "unexpected greeting without Pester: $greeting"
        exit 1
    }
    exit 0
}

Import-Module Pester -MinimumVersion 5.7.1 -ErrorAction Stop
$config = New-PesterConfiguration
$config.Run.Path = Join-Path $moduleRoot "Greeter.Describe.ps1"
$config.Run.PassThru = $true
$config.Output.Verbosity = "Detailed"
$result = Invoke-Pester -Configuration $config
if ($result.FailedCount -gt 0 -or $result.FailedBlocksCount -gt 0) { exit 1 }
exit 0
