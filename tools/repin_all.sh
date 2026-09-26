#!/usr/bin/env bash
set -euo pipefail

workspace="$(git rev-parse --show-toplevel)"
cd "$workspace"

step() {
  echo "repin-all: $1"
}

step "cargo (crate_universe hub)"
CARGO_BAZEL_REPIN=1 bazel build //rust/tests/fixtures/hello:hello

step "npm (authoritative pnpm graph)"
bazel run @pnpm//:pnpm -- update

step "npm (private JavaScript tool graph, lockfile only)"
bazel run @pnpm//:pnpm -- --dir quality/tools/javascript install --lockfile-only

step "maven (whole-lock pin)"
REPIN=1 bazel run @maven//:pin

step "nuget (paket2bazel regen)"
bazel run @rules_dotnet//tools/paket2bazel -- --dependencies-file "$workspace/third_party/dotnet/paket.dependencies" --output-folder "$workspace/third_party/dotnet/deps"

step "go (intentional no-op: pinned module lock tracks Gazelle; widen via dx bump)"
echo "repin-all: go stays pinned at third_party/go/go.mod plus go.sum (no launch)"

step "uv (authoritative Python graph)"
(cd python/tests/fixtures/hello && uv lock)

step "uv (private Python tool graph)"
(cd quality/tools/python && uv lock)

echo "repin-all: sequenced cargo, npm x2, maven, nuget, go, uv x2"
