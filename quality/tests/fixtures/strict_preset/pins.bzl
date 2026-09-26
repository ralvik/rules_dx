DEFAULT_POLICY = "default stays loose: curated defaults plus pinned upstream built-in defaults, no hidden presets"
STRICT_OPT_IN = "strict is opt-in via checked-in native configs, no workspace strict flag, no selectable preset ID"
REJECTED_FORCING_STRICT = "forcing strict by default rejected: upgrade break, requires a major release"

RUFF_LOOSE_SELECT = 'select = ["E4", "E7", "E9", "F"]'
RUFF_STRICT_SELECT = 'select = ["E", "F", "W", "I", "N", "UP", "B", "SIM"]'
RUFF_STRICT_SUPERSET = "strict E superset of loose E4 plus E7 plus E9, plus W plus I plus N plus UP plus B plus SIM"

BIOME_LOOSE = "{}"
BIOME_STRICT_RECOMMENDED = '"recommended": true'
BIOME_STRICT_NO_EXPLICIT_ANY = '"noExplicitAny": "error"'
BIOME_STRICT_USE_CONST = '"useConst": "error"'
BIOME_STRICT_NO_UNUSED_VARIABLES = '"noUnusedVariables": "error"'

TSC_STRICT = '"strict": true'

VALE_STRICT = "markers-only Dx.Markers with MinAlertLevel suggestion, prose wont-fix per issues #589, #665"

STRICT_PRESET_RUFF_LOOSE = "//quality/tests/fixtures/strict_preset:ruff_loose"
STRICT_PRESET_RUFF_STRICT = "//quality/tests/fixtures/strict_preset:ruff_strict"
STRICT_PRESET_BIOME_LOOSE = "//quality/tests/fixtures/strict_preset:biome_loose"
STRICT_PRESET_BIOME_STRICT = "//quality/tests/fixtures/strict_preset:biome_strict"

STRICT_PRESET_REJECTED = "hidden presets rejected: no selectable strict preset ID, no workspace strict flag, no forced strict default"
