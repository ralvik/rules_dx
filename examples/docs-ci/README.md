# Docs CI Example

This directory templates third-party reuse of the docs workflow at
`../../.github/workflows/reusable-docs.yml`. Copy `caller.yml` to
`.github/workflows/docs.yml` in the consumer repo and set the pin to a
reviewed commit; never copy the check or deploy steps.

```yaml
jobs:
  docs:
    uses: rules_dx/.github/workflows/reusable-docs.yml@<reviewed-commit>
    with:
      rules_dx_version: "0.0.0"
      docs_scope: "//docs/..."
      docs_dir: "docs"
      publish: false # true deploys the validated tree to GitHub Pages
    secrets: inherit
```

`docs-check` runs `dx docs --check` plus `dx lint --check` over
`docs_scope`; the lint pass is the Markdown link/structure audit, so
broken relative targets fail before deploy. `publish: true` uploads
`docs_dir` and deploys it to the `github-pages` environment. Enable
Pages with source GitHub Actions in repository settings before the
first publishing run. This repository self-calls the workflow in
`../../.github/workflows/ci.yml` (check-only on pull requests,
publishing on `main`).
