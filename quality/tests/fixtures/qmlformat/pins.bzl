"""qmlformat check plus fix wiring (QML format).

"""

QMLFORMAT_QT_OBSERVED = "Qt 6.11.2"

QMLFORMAT_ARTIFACT = "qualified Qt distribution tool targets"
QMLFORMAT_RUNTIME = "authoritative Qt distribution; no separate acquisition"

QMLFORMAT_CHECK = "qmlformat --check (exit 0 clean, exit 1 with unformatted paths when dirty)"
QMLFORMAT_FIX = "qmlformat -i in-place (re-read on exit 0, keep input otherwise)"
QMLFORMAT_CONFIG_POLICY = "upstream built-in defaults without .qmlformat.ini, native interpretation with .qmlformat.ini; no auto-supplied preset"

STRUCTURED_FIXTURE_QMLFORMAT = "//quality/tests/fixtures/qmlformat:corpus_starlark"

QMLFORMAT_REJECTED = "ambient .qmlformat.ini discovery rejected; auto-supplied preset rejected"
