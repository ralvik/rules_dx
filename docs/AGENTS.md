# Documentation Instructions

- Put cross-domain docs in `docs/`. Use the existing domain folders.
- Edit in place. One doc per fact; link, don't copy.
- Keep all documentation simple and to the point.
- Keep READMEs short; details in `docs/`, code examples in `examples/`.
- In-code docs: `.bzl` headers carry one-line purpose plus `Contract:` link; Rust keeps only why-not-obvious notes with `See:`/`Owning contract:` link; `LCOV_EXCL_*` reasons stay short specific (`reason:` plus `issue:` plus `policy: docs/testing/strategy-details.md#coverage`). No bare `Issue #` in non-test source. Link-don't-copy applies to code comments and docstrings too.
