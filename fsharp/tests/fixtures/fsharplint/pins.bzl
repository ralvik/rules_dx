"""FSharpLint console vs library-API binding decision."""

FSHARPLINT_VERSION = "0.27.0"

FSHARPLINT_ARTIFACT = "exact official tool package over the managed .NET cohort (targets .NET 8.0)"
FSHARPLINT_RUNTIME = "shared managed .NET runtime cohort; no dotnet tool install"

FSHARPLINT_CONSOLE_WRAPPER = "FSharpLint.Console wrapper around FSharpLint.Application.Lint with ReceivedWarning callback"
FSHARPLINT_STANDARD_SHAPE = "multi-line human block: message plus start-only location plus snippet plus caret plus FLxxxx URL plus 80-dash separator; rule ID only inside URL; info lines share stdout"
FSHARPLINT_MSBUILD_SHAPE = "-f msbuild single line: path(line,col,line,col):FSharpLint warning FLxxxx: message; info lines share stdout"
FSHARPLINT_LIBRARY_API = "FSharpLint.Application.Lint (lintFile/lintFiles/lintSource/lintParsedSource/lintParsedFile/lintProject/lintSolution plus async variants) with OptionalLintParameters.ReceivedWarning callback"
FSHARPLINT_WARNING_RECORD = "LintWarning{RuleIdentifier, RuleName, FilePath, ErrorText, Details{Range{StartLine,StartColumn,EndLine,EndColumn}, Message, SuggestedFix, TypeChecks}}"

FSHARPLINT_DIAGNOSTICS = "wire via FSharpLint.Application.Lint library API with ReceivedWarning callback; console-parse rejected (both standard and msbuild shapes)"
FSHARPLINT_CONSOLE_PARSE_REJECTED = "console-parse rejected: standard shape drops end position plus rule field (URL-only) with unprovable block boundaries (console_standard.txt); msbuild shape drops ErrorText plus SuggestedFix plus TypeChecks plus RuleName with shared-stream info lines (console_msbuild.txt); fail-closed parsing unprovable"
FSHARPLINT_WIRE_REQUIRED = "custom .NET entrypoint binding FSharpLint.Application.Lint over exact-package artifacts for per-warning rule IDs plus full ranges plus messages plus fix metadata"

FSHARPLINT_PROJECT_WIRING = "target-coupled: .fsproj/.sln plus fsharplint.json from authoritative fsharp_* target context; single-file runs without required project context fail closed"
FSHARPLINT_SINGLE_FILE_NOTE = "single-file and wildcard runs warn project context detects more issues; no silent check-only fallback that would miss project-gated findings"
FSHARPLINT_FIX_FLOW = "sandbox-apply-and-diff with declared per-target outputs as unified patches; IN_PLACE mutation rejected under sandboxing; console emits diagnostics only, never patches"

FSHARPLINT_CONFIG_POLICY = "checked-in fsharplint.json required to change policy (formatting-adjacent rules off, Fantomas owns formatting); upstream built-in defaults without config, native interpretation with config"

FSHARP_FIXTURE_HELLO = "//fsharp/tests/fixtures/hello:hello_lib"

FSHARPLINT_REJECTED = "silent console parse rejected: must record; standard plus msbuild console-parse rejected as diagnostic source; IN_PLACE patching rejected; ambient project discovery rejected; auto-supplied formatting preset rejected"
