# rules_dx v0.1.0 (local candidate, Linux x86_64)

Candidate module + CLI bytes qualified on local seed host only (Linux x86_64 glibc, local execution, no remote).
Public registry submission and credentialed publication execute in CI (OIDC); this tree carries the exact bytes + digests.

Contents: `dist/dx-linux_x86_64` (+ `.sha256`), `dist/MANIFEST.json`, `dist/SHA256SUMS`, `dist/verify.sh`.
Consumer CI: `.github/workflows/reusable-consumer.yml` pinned `@v0.1.0`; caller template via `dx init` and `examples/consumer-ci/caller.yml`.
Adoption: `dx init/hooks/status/version/docs/watch/owners/deps/why/completion` (see `dx --help` usage).

Gaps (not claimed): non-Linux hosts, external-consumer matrix runs, Cosign signing + SLSA provenance (CI OIDC), registry entry.
