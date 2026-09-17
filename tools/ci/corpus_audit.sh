#!/usr/bin/env bash
# Corpus ownership audit (issue #80): every applicable checked-in file must
# have a corpus owner (`real_source_target`), so the dx quality commands
# actually cover the repository they claim to dogfood.
#
# Hermetic promotion of the former inline ci.yml pipeline: versioned here,
# run by CI via `bazel run //tools/ci:corpus_audit`, so the audit logic
# itself is reviewed, pinned, and reproducible instead of drifting inside
# workflow YAML. Exits 0 when fully covered, 1 listing orphan files.
set -euo pipefail

if [[ -n "${BUILD_WORKSPACE_DIRECTORY:-}" ]]; then
  workspace="$BUILD_WORKSPACE_DIRECTORY"
else
  workspace="$(git rev-parse --show-toplevel)"
fi
cd "$workspace"

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

# The paket2bazel hub output below is generator-owned (regenerate via
# `bazel run @rules_dotnet//tools/paket2bazel`, see
# third_party/dotnet/deps/BUILD.bazel), never corpus sources. The
# marker check keeps this exclusion honest: hand edits lose it.
head -1 third_party/dotnet/deps/paket.main.bzl | grep -q GENERATED
head -1 third_party/dotnet/deps/paket.main_extension.bzl | grep -qi GENERATED

# Stage 4 E2E carve-out (issue #55): `integration/` scenario workspaces
# are .bazelignore'd out of the parent universe and staged to scratch
# by shell copy, so they can never carry corpus owners. The marker
# check keeps this exclusion honest: dropping the ignore re-lists
# every scenario file below as uncovered.
grep -q -F -e 'integration/' .bazelignore

git ls-files \
  | grep -E '(^|/)(BUILD\.bazel|MODULE\.bazel)$|\.(bzl|toml|md)$' \
  | grep -v -E '\.lock$' \
  | grep -v -E '^third_party/dotnet/deps/paket\.main(_extension)?\.bzl$' \
  | grep -v -E '^integration/' \
  | LC_ALL=C sort -u > "$scratch/corpus_applicable.txt"

bazel query "kind('source file', deps(kind(real_source_target, //...)))" 2>/dev/null \
  | grep -E '^(@@)?//' \
  | sed 's/^@@//; s|^//||; s|:|/|; s|^/||' \
  | LC_ALL=C sort -u > "$scratch/corpus_closure.txt"

uncovered="$(comm -23 "$scratch/corpus_applicable.txt" "$scratch/corpus_closure.txt" | wc -l | tr -d ' ')"
if [ "$uncovered" -ne 0 ]; then
  echo "corpus ownership audit failed: $uncovered applicable files have no corpus owner:"
  comm -23 "$scratch/corpus_applicable.txt" "$scratch/corpus_closure.txt"
  exit 1
fi
echo "corpus ownership audit: all applicable files have a corpus owner"
