# Greeter module for the Pester fixture.
function Get-PesterGreeting {
    [OutputType([string])]
    param(
        [string]$Name = "powershell"
    )
    return "hello $Name"
}
