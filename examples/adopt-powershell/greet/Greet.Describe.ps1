# Pester `Describe`/`It` block exercised when Pester is vendored.
# Kept out of `Greet.Tests.ps1` so `Invoke-Pester` never discovers the
# entry point and re-enters it (See: docs/generation/powershell.md#evidence).
Describe "Get-AdoptGreeting" {
    It "greets powershell" {
        Get-AdoptGreeting | Should -Be "hello powershell"
    }
}
