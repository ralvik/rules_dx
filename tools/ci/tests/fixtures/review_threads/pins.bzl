"""Review-thread limit pins.
Contract: `docs/github-ci.md#thread-lifecycle-and-limits`.
Fixture: `tools/ci/tests/fixtures/review_threads/` via
`bazel run //tools/ci:review_threads_qualification`.
Frozen 50 open threads; seed-only, no Supported claim.
"""

# Frozen numeric limit: exactly 50 open integration-owned threads per PR.
REVIEW_THREAD_LIMIT = 50
LIMIT_FROZEN = "frozen at 50 open integration-owned threads under issue #592"
LIMIT_SCOPE = "one fixed per-PR limit across checks and platforms"
LIMIT_NO_CONSUMER_SETTING = "without a consumer setting"
LIMIT_NO_FRESH_ALLOWANCE = "no fresh allowance per job/rerun"

# Deterministic priority: failure-contributing first, never job completion order.
LIMIT_FAILURE_FIRST = "failure-contributing findings first"
LIMIT_ORDERING_KEY = "check/file/line/rule ordering"
LIMIT_NOT_COMPLETION_ORDER = "not job completion order"

# Existing threads preserved: no rotation of still-present findings.
LIMIT_NO_ROTATION = "do not delete or resolve still-present findings to rotate"
LIMIT_KEEP_WHILE_PRESENT = "Keep a thread while its finding remains"

# Slot accounting: bot-only deletion frees, resolved history stays outside open count.
LIMIT_BOT_ONLY_FREES = "delete bot-only threads without replies frees a slot"
LIMIT_RESOLVED_RETAINED = "resolve threads with human replies instead, retained outside the open count"
LIMIT_NEVER_PROVES_GONE = "skipped, disabled, cancelled, or incomplete checks never prove a finding is gone"
LIMIT_OUTSIDE_DIFF_NEVER_PROVES_GONE = "locations moving outside the diff do not prove a finding is gone"

# Full reports plus summary distinction plus truncation honesty.
LIMIT_FULL_REPORTS = "Full reports retain every finding"
LIMIT_SUMMARY_DISTINGUISHES = "summary distinguishes findings omitted due to the limit"
LIMIT_SUMMARY_UNMAPPABLE = "those without a valid diff location"
LIMIT_TRUNCATION_NOT_FAILURE = "Intentional truncation is not incomplete analysis or publication failure"
LIMIT_TRUNCATION_NEVER_CHANGES_OUTCOMES = "never changes CI outcomes"

# Concurrency: late callbacks never overwrite current threads or summaries.
LIMIT_NO_OVERWRITE = "late callbacks never overwrite current threads/summaries"

# Rejected substitutes per the issue alternatives.
REJECTED_UNFROZEN = "leaving unfrozen rejected"
REJECTED_CONSUMER_SETTING = "consumer limit setting rejected"
REJECTED_FRESH_ALLOWANCE = "fresh allowance per job/rerun rejected"
REJECTED_COMPLETION_ORDER = "job completion order rejected"
REJECTED_ROTATION = "rotation of still-present findings rejected"

NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #592"
