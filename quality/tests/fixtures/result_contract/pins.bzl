SEVERITY_INFO = "proto INFO maps to NDJSON info"
SEVERITY_WARNING = "proto WARNING maps to NDJSON warning"
SEVERITY_ERROR = "proto ERROR maps to NDJSON error"
SEVERITY_UNSPECIFIED_REJECTED = "SEVERITY_UNSPECIFIED rejected"
SEVERITY_RANK_PARITY = "evaluator rank mirrors CLI meets_threshold rank"

CAPABILITY_LINT = "CAPABILITY LINT maps to lint pipeline"
CAPABILITY_TYPECHECK = "CAPABILITY TYPECHECK maps to typecheck pipeline"
CAPABILITY_FORMAT = "CAPABILITY FORMAT maps to format pipeline"
CAPABILITY_AUDIT = "CAPABILITY AUDIT maps to audit action"
CAPABILITY_UNSPECIFIED_REJECTED = "CAPABILITY_UNSPECIFIED rejected"

DIAGNOSTIC_TOOL = "tool_id maps to tool"
DIAGNOSTIC_MESSAGE = "message maps to message"
DIAGNOSTIC_RULE = "rule_id maps to rule"
DIAGNOSTIC_PATH = "path maps to path"
DIAGNOSTIC_RANGE = "start_byte plus end_byte map to range"
DIAGNOSTIC_FIXABLE = "fixable maps to fixable"
DIAGNOSTIC_SNAPSHOT = "snapshot maps to initial plus terminal"

PATH_NORMALIZED = "normalized slash-separated workspace-relative path"
PATH_ABSOLUTE_REJECTED = "absolute paths rejected"
PATH_BACKSLASH_REJECTED = "backslash paths rejected"
PATH_DOT_REJECTED = "dot components rejected"
PATH_DOTDOT_REJECTED = "dotdot components rejected"

DIGEST_ALGORITHM = "BLAKE3-256 over exact file bytes"
DIGEST_PROTO_LEN = "exactly 32 raw bytes in proto"
DIGEST_NDJSON_HEX = "exactly 64 lowercase hex in source_digest"
DIGEST_MISMATCH = "digest mismatch fails as stale_source"

CHANGE_PATH = "FileEdits path maps to path"
CHANGE_DIGEST = "original_digest maps to source_digest"
CHANGE_EDITS = "edits map to edits with exact UTF-8 replacement"
CHANGE_KIND_MODIFY = "quality kind is modify"
CHANGE_KIND_CREATE_GENERATE_ONLY = "create stays generate-only"

EDIT_SORTED = "strictly start-byte-sorted non-overlapping edits"
EDIT_ADJACENT_VALID = "adjacent edits valid"
EDIT_SAME_OFFSET_REJECTED = "same-offset edits rejected"
EDIT_INSERTION_INSIDE_REJECTED = "insertion inside replaced range rejected"
EDIT_NOOP_REJECTED = "no-op edits plus candidates rejected"
EDIT_UTF8_REQUIRED = "invalid UTF-8 source plus replacement rejected"

FIXABLE_GUARANTEE = "fixable true only with exact same-file candidate plus absent terminal"
FIXABLE_TERMINAL_FALSE = "terminal diagnostics always fixable false"
FIXABLE_DEDUP_CONSERVATIVE = "conservative duplicate merging with one false makes false"
FIXABLE_NEVER_INFERRED = "fixability never inferred from another change"

RESOLUTION_CHECK_OMITS = "check mode omits resolution"
RESOLUTION_FIXED = "fixed means absent terminal plus applied candidate"
RESOLUTION_REMAINING = "remaining means present in terminal snapshot"
RESOLUTION_NOT_APPLIED = "not_applied means absent terminal plus unapplied candidate"
RESOLUTION_TERMINAL_OMITS = "terminal diagnostics omit resolution"

CONVERGENCE_STABLE = "STABLE may carry replacements"
CONVERGENCE_OSCILLATION = "OSCILLATION completes with no replacement"
CONVERGENCE_ITERATION_LIMIT = "ITERATION_LIMIT completes with no replacement"
CONVERGENCE_UNSPECIFIED_REJECTED = "CONVERGENCE_UNSPECIFIED rejected"
CONVERGENCE_MAX_ROUNDS = "at most ten complete cross-tool rounds"

THRESHOLD_INFO = "fail_on info maps to Threshold Info"
THRESHOLD_WARNING = "fail_on warning maps to Threshold Warning"
THRESHOLD_ERROR = "fail_on error maps to Threshold Error"
THRESHOLD_NEVER_REJECTED = "fail_on never rejected"
THRESHOLD_REPLACEMENT_FAILS = "replacements fail at every threshold"
THRESHOLD_NON_STABLE_FAILS = "non-stable convergence fails"
THRESHOLD_FIXABLE_FAILS = "fixable still fails without apply"

COLLECTION_GROUP = "dx_results output group via BEP named sets"
COLLECTION_DECODE_VALIDATED = "decode_validated per artifact"
COLLECTION_INCOMPLETE_NO_MUTATION = "incomplete collection emits no change plus no mutation"
COLLECTION_DETERMINISTIC_ORDER = "deterministic order independent of BEP order"
COLLECTION_NO_AGGREGATE = "no invocation-level aggregate action"

VERSION_UNKNOWN_MAJOR_REJECTED = "unknown major rejected"
VERSION_NEWER_MINOR_ACCEPTED = "newer minors accepted when bytes satisfy rules"
VERSION_UNKNOWN_FIELD_IGNORED = "unknown fields ignored"
VERSION_DETERMINISTIC_BYTES = "deterministic bytes for identical inputs"
VERSION_SEMANTIC_COMPARE = "semantic compare not envelope equality"

EMISSION_CHECK_NO_MUTATION = "check mode emits change with no mutation"
EMISSION_CHANGE_BEFORE_MUTATION = "default emits change before terminal mutation"
EMISSION_CHECK_CHANGE_FAILS = "any check-mode change fails independently of severity"

REJECTED_ALTERNATIVES = [
    "bare contract",
    "unmapped Protobuf",
    "unmapped NDJSON",
]

OWNED_SPDP_PARSE = "SPDX parsing plus policy-table loading stays owned"
OWNED_SPDP_EMISSION = "live SPDX emission stays owned"
OWNED_UPDATE_AGGREGATE = "update aggregate exit codes stay owned"
OWNED_GAPS_NOTE = "platform plus consumer plus release evidence stays owned gap"
NO_SUPPORTED_CLAIM = "no Supported claim"

LIVE_RESULT = "//quality/result:quality_result"
LIVE_EVALUATOR = "//quality/evaluator:quality_evaluator"
LIVE_OUTPUT = "//cli/output:dx_output"
LIVE_FIXTURE_CORPUS = "//quality/tests/fixtures/result_contract:corpus_starlark"
