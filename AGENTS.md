# Repository Instructions

This repository uses Bazel.

Code rules:

- Treat warnings as errors.
- Keep READMEs short; put details in `docs/` or `examples/`.
- In-code docs: one-line purpose plus `Contract:` link in `.bzl` headers; Rust keeps only why-not-obvious notes with `See:`/`Owning contract:` link; `LCOV_EXCL_*` reasons stay short (`policy: docs/testing/README.md#coverage`). No bare `Issue #` in non-test source (see `docs/AGENTS.md`).
