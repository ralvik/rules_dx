# Pester `Describe`/`It` block exercised when Pester is vendored.
# Kept as a separate file so the entry point above stays a plain
# `pwsh_test` while the Pester syntax stays pinned and reviewable.
Describe "Get-PesterGreeting" {
    It "greets powershell" {
        Get-PesterGreeting | Should -Be "hello powershell"
    }
    It "greets a name" {
        Get-PesterGreeting -Name "dx" | Should -Be "hello dx"
    }
}
