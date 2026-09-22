# Repository Instructions

This repository uses Bazel.

Code rules:

- Treat warnings as errors.
- Keep all documentation simple and to the point.
- Keep READMEs short; details in `docs/`, code examples in `examples/`.
- In-code docs: one-line purpose plus `Contract:` link in `.bzl` headers; Rust keeps only why-not-obvious notes with `See:`/`Owning contract:` link; `LCOV_EXCL_*` reasons stay short specific (`reason:` plus `issue:` plus `policy: docs/testing/strategy-details.md#coverage`). No bare `Issue #` in non-test source (see `docs/AGENTS.md`).
