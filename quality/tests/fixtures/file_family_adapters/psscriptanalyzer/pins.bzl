"""psscriptanalyzer check plus fix wiring.

"""

PSSCRIPTANALYZER_VERSION = "1.25.0"
PSSCRIPTANALYZER_ARTIFACT = "exact module plus portable pwsh runtime"
PSSCRIPTANALYZER_CHECK = "Invoke-ScriptAnalyzer text"
PSSCRIPTANALYZER_FIX = "check-only"
FILE_FAMILY_PROOF = "bazel build //quality/tests/fixtures/file_family_adapters/psscriptanalyzer:corpus_starlark"
PSSCRIPTANALYZER_REJECTED = "ambient discovery rejected; auto-supplied preset rejected"
