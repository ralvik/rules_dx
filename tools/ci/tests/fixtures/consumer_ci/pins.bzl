"""Consumer-CI per-gap pins (issue #509).

Contract: `docs/github-ci.md#scope-and-status`,
`docs/github-ci.md#check-selection`,
`docs/github-ci.md#execution`,
`docs/github-ci.md#events-and-revisions`,
`docs/github-ci.md#reporting`,
`docs/github-ci.md#fork-security`,
`docs/github-ci.md#merge-gating`,
`docs/github-ci.md#qualification`,
`docs/testing/github-ci.md#qualification`.
Fixture: `tools/ci/tests/fixtures/consumer_ci/` via
`bazel run //tools/ci:consumer_ci_qualification`.

Decides the consumer-CI slice beyond the reusable-workflow contract
(closed #312) plus the all-enabled self-call (closed #408) plus the
host matrix (closed #415): each gap below records its as-built static
contract with seed-only fixture evidence, without claiming live
multi-host execution, live queue/thread/fork runs, or Supported.
Platform plus release evidence stays owned gap under issues #298/#325;
no Supported claim. Backends stay provisional where pinned upstream.
"""

# Platform selection stays explicit with no implicit default.
PLATFORM_IDS = [
    "linux_x86_64",
    "linux_arm64",
    "macos_arm64",
    "macos_x86_64",
    "windows_x86_64",
]
PLATFORM_SUPPORTED_SET = 'supported = {"linux_x86_64", "linux_arm64", "macos_arm64", "macos_x86_64", "windows_x86_64"}'
PLATFORM_NO_IMPLICIT_DEFAULT = "no implicit default"
PLATFORM_NONEMPTY_JSON = "must be a nonempty JSON array"
PLATFORM_FAIL_CLOSED = "unsupported platforms"
PLATFORM_NO_SUBSTITUTION = "not skipped validation or platform substitution"

# Runner routing stays pinned per platform with no fall-through.
RUNNER_LINUX_X86_64 = "matrix.platform == 'linux_x86_64' && 'ubuntu-latest'"
RUNNER_LINUX_ARM64 = "matrix.platform == 'linux_arm64' && 'ubuntu-24.04-arm'"
RUNNER_MACOS_ARM64 = "macos_arm64' && 'macos-14'"
RUNNER_MACOS_X86_64 = "macos_x86_64' && 'macos-15-intel'"
RUNNER_WINDOWS_X86_64 = "windows_x86_64"
RUNNER_WINDOWS_LABEL = "windows-latest"
RUNNER_NO_FALLTHROUGH = "never fall through"
RUNNER_WINDOWS_SHELL = "shell: bash"

# Isolation stays per-job checkouts with least-privilege plus gate edges.
ISOLATION_CHECKOUTS = "actions/checkout"
ISOLATION_NO_CREDS = "persist-credentials: false"
ISOLATION_GATE_EDGE = "needs: [platforms-gate]"
ISOLATION_LINUX_ONCE = "Linux-once jobs never do"
ISOLATION_NO_SHARED_BASE = "do not advertise parallelism while jobs serialize on one shared Bazel output-base lock"
ISOLATION_TIMEOUTS = "timeout-minutes"
ISOLATION_FAIL_FAST_FALSE = "fail-fast: false"

# Cache stays absent in the reusable workflow; per-host scopes live in ci.yml only.
CACHE_NO_REUSABLE_WIRING = "no actions/cache in reusable-consumer"
CACHE_CI_SCOPES = [
    "bazel-seed-",
    "bazel-arm64-",
    "bazel-musl-x86_64-",
    "bazel-musl-arm64-",
    "bazel-macos-arm64-",
    "bazel-macos-x86_64-",
    "bazel-windows-x86_64-",
]
CACHE_FREE_TIER = "free-tier eligible"
CACHE_NO_PAID = "no paid services"
CACHE_NOSHOW = "--noshow_progress"

# Ordering stays parallel-only with sequential fail-closed.
ORDERING_PARALLEL_DEFAULT = 'scheduling_mode: "parallel"'
ORDERING_SEQUENTIAL_OPEN = "scheduling_mode 'sequential' is qualification-open"
ORDERING_UNKNOWN_REJECTED = "unsupported scheduling_mode"
ORDERING_PRESERVES_RESULTS = "failures preserve completed results"
ORDERING_NO_CANCEL_INDEPENDENT = "do not cancel independent checks"

# Merge validation stays proposed-merge with same snapshot plus blocked path.
MERGE_PROPOSED = "Validate the proposed merge, not the contributor branch alone"
MERGE_SAME_SNAPSHOT = "All selected checks in a run use the same test-merge snapshot"
MERGE_HEAD_BASE = "PR-head and target-branch revisions recorded"
MERGE_BLOCKED = "report validation as blocked without head-only fallback or aggregate success"
MERGE_NO_FORK_EXEC = "without executing fork code with reporting privileges"

# Diff mapping stays scope-preserving with no invented locations.
DIFF_NO_NARROW = "diff-based review placement does not narrow analysis scope"
DIFF_UNMAPPABLE_STAYS = "Unmappable findings stay in reports and summary counts, never on invented lines"
DIFF_NO_INVENTED = "no invented review locations"

# Queue stays native with queue-revision binding plus no PR writes.
QUEUE_NATIVE = "Consumers enable and configure their queue"
QUEUE_SAME_CHECKS = "Queue checks use the same selected checks and stable aggregate identity"
QUEUE_BOUND_REVISION = "bound to the queue revision rather than an individual PR head"
QUEUE_NO_PR_WRITES = "Queue runs never create, update, or clean up PR comments/threads"
QUEUE_STALE_NEVER = "obsolete queue results cannot satisfy a newer queue revision"
QUEUE_NO_SECRETS = "Queue membership does not grant fork code secrets or write credentials"

# Cancellation stays concurrency-scoped with history plus no-overwrite.
CANCEL_GROUP = "group: dx-ci-${{ github.workflow }}-${{ github.event.pull_request.number || github.ref }}"
CANCEL_IN_PROGRESS = "cancel-in-progress: true"
CANCEL_SCOPED = "Scope PR supersession to that PR's integration runs"
CANCEL_RETAINS_HISTORY = "Retain prior run history under normal retention"
CANCEL_NOT_PASSED = "unfinished or cancelled checks are not passed"
CANCEL_NO_OVERWRITE = "Late callbacks must not overwrite current summaries"

# Aggregate stays stable dx-ci with always plus skipped-green plus no false success.
AGGREGATE_NEEDS = "needs: [platforms-gate, lint, typecheck, format, generate, security-audit, license-audit, test, build, coverage]"
AGGREGATE_ALWAYS = "if: ${{ always() }}"
AGGREGATE_SKIPPED_GREEN = "Disabled checks report skipped and stay green"
AGGREGATE_FAILING_NAMES = "failing checks"
AGGREGATE_NO_FALSE_SUCCESS = "missing, blocked, unexpectedly skipped, cancelled, or incomplete selected results cannot produce success"
AGGREGATE_BRANCH_PROTECTION = "require in branch protection"

# Thread lifecycle stays replyable with dedup plus human preservation.
THREAD_REPLYABLE = "replyable/resolvable PR review threads"
THREAD_NO_OPTOUT = "There is no review-comment opt-out or annotation-only mode"
THREAD_PRESERVE_HUMAN = "Preserve human comments, replies, and unrelated threads"
THREAD_DEDUP = "Deduplicate integration-owned findings across reruns"
THREAD_KEEP_WHILE_PRESENT = "Keep a thread while its finding remains"
THREAD_BOT_ONLY_DELETE = "delete bot-only threads without replies"
THREAD_WITH_REPLIES_RESOLVE = "resolve threads with human replies instead"

# Limits stay fixed with deterministic priority plus no fresh allowance.
LIMIT_FIXED = "one fixed, documented per-PR review-thread limit"
LIMIT_NO_CONSUMER_SETTING = "without a consumer setting"
LIMIT_DETERMINISTIC = "deterministic ordering, not job completion order"
LIMIT_NO_FRESH_ALLOWANCE = "no fresh allowance per job/rerun"
LIMIT_FULL_REPORTS = "Full reports retain every finding"
LIMIT_SUMMARY_DISTINGUISHES = "summary distinguishes findings omitted due to the limit"
LIMIT_NOT_FROZEN = "The numeric limit and thread-accounting mechanics are not yet frozen"

# Fork security stays native approval with fork-safe comments.
FORK_POLICY = "all_external_contributors"
FORK_NO_SELF_APPROVAL = "outside authors cannot self-approve"
FORK_NO_SECRETS = "does not give fork code secrets or write credentials"
FORK_SKIP_MARKER = "dx-coverage-summary: coverage"
FORK_SKIP_NOTE = "Fork PRs skip so fork code never gets write"
FORK_DETECTED = "Fork PR detected"
FORK_PRIVILEGED_SEPARATE = "Privileged reporting stays separate and never executes fork-controlled code"

# Untrusted handling stays validate-plus-handoff.
UNTRUSTED_ARTIFACTS = "Treat artifacts and PR metadata as untrusted"
UNTRUSTED_HANDOFF = "trusted reporting handoff"

# Sensitive handling stays least-privilege with no private-channel claim.
SENSITIVE_LEAST_PRIVILEGE = "contents: read"
SENSITIVE_CHECKS_WRITE = "checks: write"
SENSITIVE_PR_WRITE = "pull-requests: write"
SENSITIVE_NO_EXPOSING = "does not authorize exposing credentials, secret values"
SENSITIVE_NOT_PRIVATE_CHANNEL = "public CI reporting is not confidential"
SENSITIVE_NO_GENERIC_REDACTION = "without claiming an unqualified generic redaction guarantee"

# Retries stay native rerun with bounded transport retries only.
RETRY_NATIVE_RERUN = "Use GitHub's native rerun controls, not bot comment commands"
RETRY_NO_BOT_COMMANDS = "without bot comment commands"
RETRY_BOUNDED_TRANSPORT = "Bounded transient reporting-transport retries may reuse the same identified results"
RETRY_EXHAUSTED_FAILS = "exhausted retries retain reporting failure"
RETRY_NO_RETRY_UNTIL_GREEN = "without retrying analyzer failures until green"

# Code Scanning stays opt-in off by default with no upload wiring.
CODESCAN_OPT_IN = "code_scanning_opt_in"
CODESCAN_OFF_DEFAULT = "Off by default"
CODESCAN_NO_UPLOAD = "upload-sarif"
CODESCAN_SARIF_COMPLETE = "Preserve the [SARIF completeness contract]"
CODESCAN_DISABLED_NOT_FAILURE = "Disabled publication is not a reporting failure"

# Sequential stays fail-closed with the mode input declared.
SEQUENTIAL_QUAL_OPEN = "Sequential ordering is [qualification-open]"
SEQUENTIAL_FAILS_CLOSED = "fails closed on `scheduling_mode: sequential`"
SEQUENTIAL_INPUT_DECLARED = "The mode input stays declared"

# Tag hygiene stays as-built with no releases cut.
TAG_VERSION = 'version = "0.0.0"'
TAG_NO_V_TAGS = "no v* tags"
TAG_IGNORED_DIST = "/dist/"
TAG_IGNORED_RELEASE = "/release/"
TAG_HARNESSES = [
    "tools/ci/release_hygiene.sh",
    "tools/ci/publish_trust.sh",
]
TAG_NO_PUBLICATION_PRESSURE = "no publication pressure"

# Release inputs stay reviewed pins with no silent upgrades.
RELEASE_CALLER_SHA = "uses: rules_dx/.github/workflows/reusable-consumer.yml@[0-9a-f]{40}"
RELEASE_VERSION_MATCH = "Must match the consumer MODULE.bazel pin"
RELEASE_NO_SILENT_UPGRADE = "no silent upgrades"
RELEASE_HYGIENE_GAP = "Tag hygiene and release-input gaps"

# Native-bot follow-ups stay sole-updater with no third-party updater.
BOT_SOLE_UPDATER = "sole updater"
BOT_NATIVE_ONLY = "native-only"
BOT_NO_THIRD_PARTY = "no third-party updater is retained"

# Rejected substitutes per the issue alternatives.
REJECTED_ALTERNATIVES = [
    "build-only self-call forever",
    "copy-pasted per host",
    "Waiting for all hosts before touching CI",
    "silent upgrades",
    "reviewdog",
    "bot commits",
]

# Fixture corpus plus live proofs plus owned gaps.
CONSUMER_CI_FIXTURE_CORPUS = "//tools/ci/tests/fixtures/consumer_ci:corpus_starlark"
CONSUMER_CI_SEED_PROOFS = [
    "//tools/ci:consumer_scheduling_test",
    "//tools/ci:consumer_aggregate_test",
    "//tools/ci:consumer_guards_test",
    "//tools/ci:consumer_pins_test",
]
OWNED_GAPS_NOTE = "platform plus consumer plus release evidence stays owned gap"
NO_SUPPORTED_CLAIM = "no Supported claim"
