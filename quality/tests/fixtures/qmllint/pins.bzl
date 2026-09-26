"""qmllint check wiring (QML lint).

"""

QMLLINT_QT_OBSERVED = "Qt 6.11.1"

QMLLINT_ARTIFACT = "qualified Qt distribution tool targets"

QMLLINT_CHECK = "qmllint --json - (exit 0 clean, exit 1 with JSON diagnostics when dirty)"
QMLLINT_CONFIG_POLICY = "upstream built-in defaults without .qmllint.ini, native interpretation with .qmllint.ini; //qmllint enable/disable scoping native"

STRUCTURED_FIXTURE_QMLLINT = "//quality/tests/fixtures/qmllint:corpus_starlark"

QMLLINT_REJECTED = "console-parse rejected; IN_PLACE patching outside sandbox rejected; ambient .qmllint.ini discovery rejected; auto-supplied preset rejected"
