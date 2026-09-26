# GitHub CI

Run `dx` in GitHub Actions with the reusable workflow. Copy `examples/consumer-ci/` into your repo and pin a version.

Checks run: `lint`, `typecheck`, `format`, `generate`, `security`, `license`, `test`, `build`, `coverage`. Each runs `dx <check> --check` on your platforms.

Docs use `examples/docs-ci/`. It runs `dx docs --check`. Turn on `publish` to deploy to Pages.

Codecov is optional. Coverage comes from `dx coverage`.
