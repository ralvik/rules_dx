"""Selective NuGet `dx update` per-package pins."""

SELECTIVE_NUGET = "wont-fix"

SELECTIVE_NUGET_FULL = "bazel run @rules_dotnet//tools/paket2bazel -- --dependencies-file third_party/dotnet/paket.dependencies --output-folder third_party/dotnet/deps"

SELECTIVE_NUGET_SELECTOR = "nuget:FSharp.Core"
SELECTIVE_NUGET_SELECTOR_LABEL = "nuget:<id>"

SELECTIVE_NUGET_HINT = "use `dx update nuget` for the set"

BUMP_FOLLOWUP_NUGET = "dx update nuget"

REJECTED_SILENT_FULL_SUBSTITUTION = "silent full-update substitution rejected"
REJECTED_PRIVATE_PAKET_LOCK_SURGERY = "private paket.lock surgery rejected"

NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #635"
