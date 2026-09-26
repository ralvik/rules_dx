BUF_VERSION = "1.72.0"
QMLFORMAT_QT_OBSERVED = "Qt 6.11.2"
QMLLINT_QT_OBSERVED = "Qt 6.11.1"

BUF_ARTIFACT = "self-contained per-platform binaries with published checksums"
QMLFORMAT_ARTIFACT = "qualified Qt distribution tool targets"
QMLLINT_ARTIFACT = "qualified Qt distribution tool targets"
QT_COUPLING = "Authoritative Qt distribution: qmlformat plus qmllint version follows the qualified Qt distribution pin, no separate acquisition"

REJECTED_BUF_HEAD = "newer than 1.72.0 observed, not pinned; living at head rejected"

NATIVE_CONFIG_POLICY = "native-configuration sole policy: no hidden presets"

BUF_POLICY = "STANDARD is the upstream built-in default lint set, not a rules_dx preset; without checked-in buf.yaml the pinned buf uses STANDARD, with buf.yaml it interprets natively"
QMLFORMAT_POLICY = "upstream built-in defaults without checked-in .qmlformat.ini, native interpretation with .qmlformat.ini; no auto-supplied ini preset"
QMLLINT_POLICY = "upstream built-in defaults without checked-in .qmllint.ini, native interpretation with .qmllint.ini; no auto-supplied ini preset"
BEYOND_DEFAULT_REJECTED = "beyond-default switches rejected: COMMENTS plus UNARY_RPC opt-in maxima, --enable=all-style opt-in maxima"

STRUCTURED_PROOF = "bazel build //quality/tests/fixtures/structured_quality:corpus_starlark"

STRUCTURED_REJECTED = "hidden presets rejected: no auto-supplied buf.yaml preset, no auto-supplied qml ini preset; unpinned versions rejected: no floating version or head"
