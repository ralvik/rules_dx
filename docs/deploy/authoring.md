# Deploy Authoring

Two paths produce a deployable target for `dx deploy`.

Path A is a raw executable: any `*_binary` or executable target (aliases
included) is deployable with no provider. Bazel owns executability.
Use this path for a single program with no separate app identity and no
non-default profile.

Path B wraps the program in `dx_deployment`
(`deploy/rules/defs.bzl`):

```starlark
load("@rules_dx//deploy/rules:defs.bzl", "dx_deployment")

dx_deployment(
    name = "production",
    deploy = ":release_script",
    app = ":server",
    profile = "release",
)
```

`deploy` is the executable program. `app` is the deployed app target
when distinct from the program; omit it to deploy the program itself.
`profile` is the default only (`debug`, `dev`, or `release`, default
`release`); an explicit `--debug`/`--release` flag always wins.
Invalid profiles fail at analysis.

`DxDeployInfo(app, profile)` is the dispatch boundary `dx deploy`
reads. Custom rules participate by returning it directly with an
executable `DefaultInfo`; they do not depend on the wrapper. There are
no `kind`, `environment`, or veto fields: deploy identity lives in
target names (`//deploy:production`) until a concrete consumer proves
the need.

Profile vocabulary follows [ADR 0021](../decisions/0021-build-profiles.md).
Precedence (explicit flag over target `profile` over command default)
and the `DX_PROFILE` name are accepted per
[issue #179](https://github.com/ralvik/rules_dx/issues/179); the
`dx deploy` command that wires them belongs to
[issue #180](https://github.com/ralvik/rules_dx/issues/180).
