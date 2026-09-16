# Consumer CI example

Caller-owned starter wiring the reusable workflow at a reviewed commit pin.
Copy `caller.yml` to `.github/workflows/ci.yml` in the consumer repo; do not
copy per-check logic. The pin is already a reviewed commit and both example
callers share it (drift fails `//tools/ci:examples_pins_test`); bumps are
deliberate and reviewed, customizations preserved, no silent upgrades.

The consumer module pins the same version the caller passes as
`rules_dx_version` (`0.0.0` today):

```python
bazel_dep(name = "rules_dx", version = "0.0.0")
```

Required settings (manual, never configured by the template): approval policy
`all_external_contributors`, branch protection on stable aggregate `dx-ci` with
latest-target validation (up-to-date-branch or merge queue), least-privilege
permissions (contents read; checks + PR write for reporting jobs only).
