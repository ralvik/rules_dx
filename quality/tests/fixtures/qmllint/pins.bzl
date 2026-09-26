"""qmllint check wiring (QML lint).

Contract: `docs/quality/tool-integrations.md#initial-adapter-qualification`,
`docs/tools/tool-acquisition.md#initial-artifact-research`.
"""

# Pinned reference version (follows the qualified Qt distribution pin;
# Qt 6.11.1 observed; living at head rejected; digests plus distribution
# identity plus licensing plus platform artifacts stay owned under
QMLLINT_QT_OBSERVED = "Qt 6.11.1"

# Distribution identity (authoritative-toolchain route from the qualified
# Qt distribution tool targets, Qt-last ordering decided; no separate
QMLLINT_ARTIFACT = "qualified Qt distribution tool targets"

# Invocation shapes (check-only with provisional sandbox-apply-and-diff
# fix flow).
QMLLINT_CHECK = "qmllint --json - (exit 0 clean, exit 1 with JSON diagnostics when dirty)"
QMLLINT_CONFIG_POLICY = "upstream built-in defaults without .qmllint.ini, native interpretation with .qmllint.ini; //qmllint enable/disable scoping native"

# foundations are not admitted as build/test targets, so the fixture
# pair plus matrix cells are the live proof).
STRUCTURED_FIXTURE_QMLLINT = "//quality/tests/fixtures/qmllint:corpus_starlark"

# Rejected: console-parse (JSON is the faithful shape), IN_PLACE
# mutation outside sandbox, ambient config discovery, auto-supplied
# preset.
QMLLINT_REJECTED = "console-parse rejected; IN_PLACE patching outside sandbox rejected; ambient .qmllint.ini discovery rejected; auto-supplied preset rejected"
