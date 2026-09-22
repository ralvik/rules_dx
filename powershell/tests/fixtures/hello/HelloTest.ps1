#!/usr/bin/env pwsh
# Plain pwsh_test for the hello fixture (exit code is the verdict).
# Pester mapping lives in //powershell/tests/fixtures/pester.
$ErrorActionPreference = "Stop"
$expected = "hello powershell"
if ($expected -ne "hello powershell") {
    Write-Error "unexpected greeting"
    exit 1
}
exit 0
