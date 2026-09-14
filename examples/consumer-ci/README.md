# Consumer CI example (M27)

Caller-owned starter wiring the versioned reusable workflow. See `caller.yml`.
Pins `rules_dx v0.1.0`; bumps are reviewed, customizations preserved, no silent upgrades.

Required settings (manual, never configured by the template): approval policy
`all_external_contributors`, branch protection on stable aggregate `dx-ci` with
latest-target validation (up-to-date-branch or merge queue), least-privilege
permissions (contents read; checks + PR write for reporting jobs only).
