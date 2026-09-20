"""FSharpLint console vs library-API binding decision (issue #493).

Contract: `docs/quality/tool-integrations.md#initial-adapter-qualification`,
`docs/tools/tool-acquisition.md#initial-artifact-research`,
`docs/product/support-matrix.md#provisional-adapter-input-notes`.

Decides the open FSharpLint adapter-input risk with fixture evidence,
recorded explicitly here and in the owning docs, never silently dropped.
Silent console parsing is rejected per the issue alternatives.

Upstream facts (observations, not pins; recheck latest stable at
implementation):
- The console (`dotnet-fsharplint`, `FSharpLint.Console`) is a wrapper
  around the linter. `Program.fs` builds `OptionalLintParameters` with
  `ReceivedWarning = Some output.WriteWarning` plus
  `ReportLinterProgress`, then calls `Lint.asyncLintFile`,
  `Lint.asyncLintSource`, `Lint.asyncLintProject`, or
  `Lint.asyncLintSolution`; warnings flow through the `LintWarning`
  record, never through console text.
- Standard output (`Output.StandardOutput`, the default) renders each
  warning as a multi-line human block (see `console_standard.txt` modeled
  on the documented FL0036 plus FL0034 shapes): the rule message, an
  `Error in file <file> on line <line> starting at column <col>` location
  line carrying only the start position, the offending source snippet
  line, a caret line, a `See .../FLxxxx.html` rule-URL hint line, and an
  80-dash separator. The rule identifier appears only inside the URL, the
  end position is dropped, the snippet is arbitrary F# source that can
  mimic block boundaries, and progress plus info lines (`Running
  FSharpLint with ... rules`, single-file `WARNING:`, `Finished: N
  warnings`) share the same stdout stream as warnings. Message text is
  free-form (backticks, `===>` hints, rule prose), so block boundaries
  are not fail-closed.
- MSBuild output (`dotnet fsharplint -f msbuild`, `Output.MSBuildOutput`)
  renders each warning as one line (see `console_msbuild.txt`):
  `<path>(<line>,<col>,<line>,<col>):FSharpLint warning <FLxxxx>:
  <message>`. The shape carries the full range plus the rule identifier
  plus the message, but still drops `ErrorText`, `SuggestedFix`,
  `TypeChecks`, and `RuleName`; info lines (`Running FSharpLint with
  ...`, `Finished: N warnings`) share the same stdout stream as warnings,
  and file paths carrying parentheses or commas plus multi-line messages
  break naive grammars.
- The upstream-recommended integration point is the .NET library API
  `FSharpLint.Application.Lint` in the `FSharpLint.Core` NuGet package
  (`lintFile`, `lintFiles`, `lintSource`, `lintParsedSource`,
  `lintParsedFile`, `lintProject`, `lintSolution`, plus `async*`
  variants): the caller passes `OptionalLintParameters` with a
  `ReceivedWarning:(Suggestion.LintWarning -> unit)` callback plus
  `Configuration` plus `ReportLinterProgress`, and each `LintWarning`
  arrives structured as `RuleIdentifier` plus `RuleName` plus `FilePath`
  plus `ErrorText` plus `Details{Range{StartLine,StartColumn,EndLine,
  EndColumn}, Message, SuggestedFix:Lazy<SuggestedFix option> option,
  TypeChecks}` (`Framework/Suggestion.fs`; `Range` is
  `FSharp.Compiler.Text.Range`). A custom .NET entrypoint over the
  exact-package artifacts on the shared managed .NET runtime cohort is
  the wire route.
- Single-file and wildcard runs warn they are not recommended (`WARNING:
  Going to analyze single .fs file ... Using a project (slnx/sln/fsproj)
  can detect more issues`): project or solution context detects more
  issues. Type-aware rules need the project context (`.fsproj`/`.sln`
  resolved via the authoritative target, never ambient discovery).
- Source rewriting sits uneasily with immutable action outputs: the
  console has no patch-output mode, so fixes travel only as
  sandbox-apply-and-diff unified patches with declared outputs, never
  `IN_PLACE` mutation of immutable inputs.

Decision (adapter-only, no `fsharp` claim yet; cohort stays owned by #417):
- Console-parse REJECTED for diagnostics, both shapes. No fail-closed
  text parser is approved: the standard shape is lossy (end position
  dropped, rule ID only inside a URL, snippet plus caret plus separator
  plus shared-stream info lines make boundaries unprovable) and the
  MSBuild shape is lossy (fix plus typecheck context dropped,
  shared-stream info lines, path punctuation plus message grammars
  unprovable). Unrecognized, truncated, or ambiguously attributed console
  output must fail the action, never become findings or an empty
  successful result. Requalification of a console grammar on every tool
  update is not sufficient here because the gap is lossiness (missing
  structured fields), not grammar drift.
- Wire REQUIRED: structured diagnostics via a custom .NET entrypoint
  binding `FSharpLint.Application.Lint` (`lintFile`/`lintSource` for
  files, `lintProject`/`lintSolution` for project context) with the
  `ReceivedWarning` callback over the exact official tool-package
  artifacts on the shared managed .NET runtime cohort (no `dotnet tool
  install`). The entrypoint reports per-warning rule IDs plus full
  ranges plus messages plus fix metadata; the runner normalizes them to
  the result contract.
- Target-coupled project context REQUIRED where rules need it: the
  adapter resolves the `.fsproj`/`.sln` (plus `fsharplint.json` via
  `--lint-config` semantics) from the authoritative `fsharp_*` target
  context (never ambient, never inferred from the filesystem).
  Single-file runs without required project context fail closed; no
  silent check-only fallback that would miss project-gated findings.
- Fix flow is sandbox-apply-and-diff with declared per-target outputs,
  rendered as unified patches (never `IN_PLACE` mutation of immutable
  inputs). The console emits diagnostics only, never patches.
- Native config is sole policy: a checked-in `fsharplint.json` is
  required to change policy (see `example.fsharplint.json` with
  formatting-adjacent rules disabled because Fantomas owns formatting);
  without config the pinned tool uses upstream built-in defaults, with a
  config it interprets natively; adapters add only transport/hermetic
  settings.
"""

# Pinned reference version (qualified seed-only under #486; living at head
# rejected; digests stay owned under issue #417).
FSHARPLINT_VERSION = "0.27.0"

# Distribution identity (exact-package plus shared-.NET-runtime route over
# the one managed .NET cohort; digests stay owned under issue #417).
FSHARPLINT_ARTIFACT = "exact official tool package over the managed .NET cohort (targets .NET 8.0)"
FSHARPLINT_RUNTIME = "shared managed .NET runtime cohort; no dotnet tool install"

# Upstream integration shapes (wrapper vs library, not rules_dx inventions).
FSHARPLINT_CONSOLE_WRAPPER = "FSharpLint.Console wrapper around FSharpLint.Application.Lint with ReceivedWarning callback"
FSHARPLINT_STANDARD_SHAPE = "multi-line human block: message plus start-only location plus snippet plus caret plus FLxxxx URL plus 80-dash separator; rule ID only inside URL; info lines share stdout"
FSHARPLINT_MSBUILD_SHAPE = "-f msbuild single line: path(line,col,line,col):FSharpLint warning FLxxxx: message; info lines share stdout"
FSHARPLINT_LIBRARY_API = "FSharpLint.Application.Lint (lintFile/lintFiles/lintSource/lintParsedSource/lintParsedFile/lintProject/lintSolution plus async variants) with OptionalLintParameters.ReceivedWarning callback"
FSHARPLINT_WARNING_RECORD = "LintWarning{RuleIdentifier, RuleName, FilePath, ErrorText, Details{Range{StartLine,StartColumn,EndLine,EndColumn}, Message, SuggestedFix, TypeChecks}}"

# Explicit parse-vs-wire record (silent console parse rejected).
FSHARPLINT_DIAGNOSTICS = "wire via FSharpLint.Application.Lint library API with ReceivedWarning callback; console-parse rejected (both standard and msbuild shapes)"
FSHARPLINT_CONSOLE_PARSE_REJECTED = "console-parse rejected: standard shape drops end position plus rule field (URL-only) with unprovable block boundaries (console_standard.txt); msbuild shape drops ErrorText plus SuggestedFix plus TypeChecks plus RuleName with shared-stream info lines (console_msbuild.txt); fail-closed parsing unprovable"
FSHARPLINT_WIRE_REQUIRED = "custom .NET entrypoint binding FSharpLint.Application.Lint over exact-package artifacts for per-warning rule IDs plus full ranges plus messages plus fix metadata"

# Explicit project-context wiring record.
FSHARPLINT_PROJECT_WIRING = "target-coupled: .fsproj/.sln plus fsharplint.json from authoritative fsharp_* target context; single-file runs without required project context fail closed"
FSHARPLINT_SINGLE_FILE_NOTE = "single-file and wildcard runs warn project context detects more issues; no silent check-only fallback that would miss project-gated findings"
FSHARPLINT_FIX_FLOW = "sandbox-apply-and-diff with declared per-target outputs as unified patches; IN_PLACE mutation rejected under sandboxing; console emits diagnostics only, never patches"

# Native-config sole policy (no hidden preset; adapters transport-only).
FSHARPLINT_CONFIG_POLICY = "checked-in fsharplint.json required to change policy (formatting-adjacent rules off, Fantomas owns formatting); upstream built-in defaults without config, native interpretation with config"

# Live proof labels (foundation consumers stay green; quality adapters claim
# nothing yet under issue #417, decision recorded under issue #493).
FSHARP_FIXTURE_HELLO = "//fsharp/tests/fixtures/hello:hello_lib"

# Rejected: silent console parse plus IN_PLACE plus ambient wiring.
FSHARPLINT_REJECTED = "silent console parse rejected: must record; standard plus msbuild console-parse rejected as diagnostic source; IN_PLACE patching rejected; ambient project discovery rejected; auto-supplied formatting preset rejected"
