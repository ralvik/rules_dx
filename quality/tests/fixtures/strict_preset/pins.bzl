"""Strict preset decision pins (issue #615).

Contract: `docs/quality/strict-preset.md`,
`docs/quality/native-configuration.md#authority`,
`docs/tools/tool-baseline.md#curated-differences`.

Decides default-loose vs strict-opt-in for users: the default stays
loose (curated defaults plus pinned upstream built-in defaults, no
hidden presets), strict is opt-in via checked-in native configs copied
from the documented examples. Forcing strict by default is rejected
(upgrade break, needs a major release). Quality only; no behavior
change for existing consumers.
"""

# Decision: default stays loose, strict is opt-in.
DEFAULT_POLICY = "default stays loose: curated defaults plus pinned upstream built-in defaults, no hidden presets"
STRICT_OPT_IN = "strict is opt-in via checked-in native configs, no workspace strict flag, no selectable preset ID"
REJECTED_FORCING_STRICT = "forcing strict by default rejected: upgrade break, requires a major release"

# Ruff loose vs strict (strict is a strict superset of loose).
RUFF_LOOSE_SELECT = 'select = ["E4", "E7", "E9", "F"]'
RUFF_STRICT_SELECT = 'select = ["E", "F", "W", "I", "N", "UP", "B", "SIM"]'
RUFF_STRICT_SUPERSET = "strict E superset of loose E4 plus E7 plus E9, plus W plus I plus N plus UP plus B plus SIM"

# Biome loose vs strict (strict enables recommended plus strict errors).
BIOME_LOOSE = "{}"
BIOME_STRICT_RECOMMENDED = '"recommended": true'
BIOME_STRICT_NO_EXPLICIT_ANY = '"noExplicitAny": "error"'
BIOME_STRICT_USE_CONST = '"useConst": "error"'
BIOME_STRICT_NO_UNUSED_VARIABLES = '"noUnusedVariables": "error"'

# TypeScript strict (tsc reads the user tsconfig; strict true is the opt-in).
TSC_STRICT = '"strict": true'

# Vale strict stays markers-only (prose wont-fix per issue #589).
VALE_STRICT = "markers-only Dx.Markers with MinAlertLevel suggestion, prose wont-fix per issue #589"

# Live proof labels (defaults unchanged; no adapter or default change).
STRICT_PRESET_RUFF_LOOSE = "//quality/tests/fixtures/strict_preset:ruff_loose"
STRICT_PRESET_RUFF_STRICT = "//quality/tests/fixtures/strict_preset:ruff_strict"
STRICT_PRESET_BIOME_LOOSE = "//quality/tests/fixtures/strict_preset:biome_loose"
STRICT_PRESET_BIOME_STRICT = "//quality/tests/fixtures/strict_preset:biome_strict"

# Rejected: hidden presets plus forced strict.
STRICT_PRESET_REJECTED = "hidden presets rejected: no selectable strict preset ID, no workspace strict flag, no forced strict default"
