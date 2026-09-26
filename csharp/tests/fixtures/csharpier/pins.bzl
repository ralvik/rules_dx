CSHARPIER_VERSION = "1.3.0"

CSHARPIER_ARTIFACT = "exact official tool package as declared DLLs (targets .NET 8.0)"
CSHARPIER_RUNTIME = "shared managed .NET runtime cohort; no dotnet tool install"

CSHARPIER_CHECK = "csharpier check plus --config-path when hinted (exit 0 clean, exit 1 with unformatted paths when dirty)"
CSHARPIER_FIX = "csharpier format in-place (re-read on exit 0, keep input otherwise)"
CSHARPIER_CONFIG_POLICY = "upstream built-in defaults without config, native interpretation with config"

CSHARP_FIXTURE_HELLO = "//csharp/tests/fixtures/hello:hello_test"

CSHARPIER_REJECTED = "dotnet tool install rejected; ambient config discovery rejected; auto-supplied preset rejected"
