#!/usr/bin/env bash
# Docs build check serve harness.
#
# Machine-checks the as-built docs build/check/serve workflow with docs in
# place, using existing tools only with no custom linter:
# - delivered: `docs/documentation/build-check-serve.md` defines `bazel build
#   //docs/...`, `dx lint --check //docs/...` (existing markdown_check plus
#   Vale markers), and stdlib `python3 -m http.server` serve;
# - link and reference checks reuse the existing `//quality/markdown` checker
#   (pulldown-cmark plus slug plus deunicode) with remote URLs never fetched;
#   a custom docs linter stays rejected;
# - CI gate stays the existing `docs-ci` self-call over `//docs/...`
#   (check-only on pull requests, validated tree publish only on main);
# - docs only: no rendered mdBook site claimed here (stays open under issue
# , no `Supported` claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:docs_build_qualification`,
# following //tools/ci:docs_pipeline_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

doc="docs/documentation/build-check-serve.md"
readme="docs/documentation/README.md"
corpus="docs/BUILD.bazel"
build="tools/ci/BUILD.bazel"
reusable=".github/workflows/reusable-docs.yml"
ci=".github/workflows/ci.yml"
checker="quality/markdown/src/lib.rs"
checker_build="quality/markdown/BUILD.bazel"

# Build-check-serve doc exists with the accepted workflow.
if [[ -f "$doc" ]] &&
  grep -q -F -e '# Documentation Build Check Serve' "$doc" &&
  grep -q -F -e 'Accepted workflow (issue #620)' "$doc"; then
  ok
else
  bad "build-check-serve doc missing its H1 or issue #620 accepted record"
fi

# Build stays Bazel-owned over //docs/... with no rendered site claim.
if grep -q -F -e 'bazel build //docs/...' "$doc" &&
  grep -q -F -e 'It emits no rendered site' "$doc"; then
  ok
else
  bad "build-check-serve doc lost its bazel build //docs/... plus no-rendered-site record"
fi

# Check stays non-mutating dx lint over //docs/... with existing checker.
if grep -q -F -e 'lint --check //docs/...' "$doc" &&
  grep -q -F -e 'The check is non-mutating' "$doc"; then
  ok
else
  bad "build-check-serve doc lost its non-mutating lint check record"
fi

# Serve stays stdlib http.server local preview, never a build action.
if grep -q -F -e 'python3 -m http.server --directory docs 8000' "$doc" &&
  grep -q -F -e 'It is not a build action' "$doc"; then
  ok
else
  bad "build-check-serve doc lost its stdlib serve plus not-a-build-action record"
fi

# Existing tools only: checker plus Vale markers named, custom linter rejected.
if grep -q -F -e 'existing tools only' "$doc" &&
  grep -q -F -e 'A custom docs' "$doc" &&
  grep -q -F -e 'linter is rejected' "$doc" &&
  grep -q -F -e 'Vale' "$doc"; then
  ok
else
  bad "build-check-serve doc lost its existing-tools-only plus rejected-custom-linter record"
fi

# Remote URLs never fetched stays pinned in the doc.
if grep -q -F -e 'never fetched' "$doc"; then
  ok
else
  bad "build-check-serve doc lost its never-fetched remote record"
fi

# Rendered mdBook site stays owned, never claimed here.
if grep -q -F -e 'issue #581' "$doc" &&
  grep -q -F -e 'No rendered mdBook site is claimed here' "$doc"; then
  ok
else
  bad "build-check-serve doc lost its #581 rendered-site boundary"
fi

# Docs-only plus no Supported claim stays pinned.
if grep -q -F -e 'Docs only' "$doc" &&
  grep -q -F -e 'no `Supported` claim' "$doc"; then
  ok
else
  bad "build-check-serve doc lost its docs-only plus no-Supported record"
fi

# Documentation README links the new contract.
if grep -q -F -e '[Build check serve](build-check-serve.md)' "$readme" &&
  grep -q -F -e 'issue #620' "$readme"; then
  ok
else
  bad "documentation README lost its build-check-serve contract link (issue #620)"
fi

# Corpus owns the new doc.
if grep -q -F -e '"documentation/build-check-serve.md"' "$corpus"; then
  ok
else
  bad "docs/BUILD.bazel lost the build-check-serve corpus entry"
fi

# BUILD owns this harness target.
if grep -q -F -e 'name = "docs_build_qualification"' "$build"; then
  ok
else
  bad "tools/ci/BUILD.bazel lost the docs_build_qualification target"
fi

# No new custom docs linter: no docs-owned Rust crate or Cargo manifest.
if [[ ! -f "docs/Cargo.toml" ]] &&
  [[ ! -d "docs/checker" ]] &&
  ! grep -rn -F -e 'docs_checker' --include='BUILD.bazel' . 2>/dev/null | grep -q .; then
  ok
else
  bad "a custom docs linter appeared (want existing libs only, issue #620)"
fi

# Existing markdown checker stays the link/structure owner.
if grep -q -F -e 'pub fn check_markdown' "$checker" &&
  grep -q -F -e 'name = "quality_markdown"' "$checker_build"; then
  ok
else
  bad "existing markdown checker lost its check_markdown plus quality_markdown identity"
fi

# Existing checker deps stay existing libs only (no new docs dep).
if grep -q -F -e 'pulldown-cmark' "$checker_build" &&
  grep -q -F -e 'slug' "$checker_build" &&
  grep -q -F -e 'deunicode' "$checker_build"; then
  ok
else
  bad "markdown checker lost its pulldown-cmark plus slug plus deunicode libs"
fi

# CI gate stays the docs-ci self-call over //docs/... with publish on main.
if grep -q -F -e 'docs-ci (self-call reusable docs workflow)' "$ci" &&
  grep -q -F -e 'docs_scope:' "$ci" &&
  grep -q -F -e '//docs/...' "$ci" &&
  grep -q -F -e 'refs/heads/main' "$ci" &&
  grep -q -F -e 'lint --check' "$reusable"; then
  ok
else
  bad "docs-ci gate lost its reusable-docs plus scope plus publish-on-main wiring (issue #620)"
fi

# Reusable docs stays check-only with a clean checkout and no rendered site.
if grep -q -F -e 'is pure check-only' "$reusable" &&
  grep -q -F -e 'git status --porcelain' "$reusable" &&
  grep -q -F -e 'rendered mdBook site arrives' "$reusable"; then
  ok
else
  bad "reusable-docs lost its check-only plus clean-checkout plus no-rendered-site record"
fi

dx_test_summary "docs build qualification harness"
