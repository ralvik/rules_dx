# Consumer CI example

Caller-owned starter wiring the reusable workflow at a reviewed commit pin. See `caller.yml`.
Pins `rules_dx` at a reviewed commit; bumps are reviewed, customizations preserved, no silent upgrades.

Required settings (manual, never configured by the template): approval policy
`all_external_contributors`, branch protection on stable aggregate `dx-ci` with
latest-target validation (up-to-date-branch or merge queue), least-privilege
permissions (contents read; checks + PR write for reporting jobs only).
