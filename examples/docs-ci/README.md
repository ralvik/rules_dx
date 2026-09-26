# Docs CI Example

This directory templates third-party reuse of the docs workflow at
`../../.github/workflows/reusable-docs.yml`. Copy `caller.yml` to
`.github/workflows/docs.yml` in the consumer repo; never copy the check
or deploy steps. `caller.yml` is the single source for the caller shape:
its pin is already a reviewed commit, shared with the consumer-ci caller
(drift fails `//tools/ci:examples_pins_test`); bumps are deliberate and
reviewed, customizations preserved, no silent upgrades.

The consumer module pins the same version the caller passes as
`rules_dx_version` (`0.0.0` today):

```python
bazel_dep(name = "rules_dx", version = "0.0.0")
```

`docs-check` runs `dx lint --check` over
`docs_scope` (default `//...`, the entire repo) plus `dx docs --check`
site validation; the lint pass is the Markdown link/structure audit, so
broken relative targets fail before deploy. `publish: true` builds the
rendered site via `dx docs` and deploys the Bazel-owned bundle to the
`github-pages` environment. Enable
Pages with source GitHub Actions in repository settings before the
first publishing run. This repository self-calls the workflow in
`../../.github/workflows/ci.yml` (check-only on pull requests,
publishing on `main`).

The reusable publish job needs `pages:write` + `id-token:write`, so the
calling job grants them (see `permissions` in `caller.yml`). No
`secrets: inherit`: the workflow defines no secrets.
