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
`dx deploy` command wiring them is delivered per
[issue #180](https://github.com/ralvik/rules_dx/issues/180).

## Path C: `archive_release` (accepted)

The first deploy macro
([issue #181](https://github.com/ralvik/rules_dx/issues/181)) packages
one executable as a tarball + sha256 checksum with host shell tools
only (`tar`, `sha256sum`/`shasum`), no new module dependencies, no
registry, no credentials:

```starlark
load("@rules_dx//deploy/rules:archive.bzl", "archive_release")

archive_release(
    name = "release",
    app = ":hello",
)
```

`bazel run //rust/hello:release` (or `dx deploy //rust/hello:release`)
verifies the checksum and copies `release.tar.gz` +
`release.tar.gz.sha256` to the output directory (first arg after `--`,
else `$BUILD_WORKSPACE_DIRECTORY`, else the cwd). Deploy targets live
next to the app they release.

## Custom deployers (accepted)

User-defined rules join `dx deploy` by returning `DxDeployInfo` with an
executable `DefaultInfo`, without depending on the wrapper:

```starlark
load("@rules_dx//deploy/rules:defs.bzl", "DxDeployInfo")

def _my_deploy_impl(ctx):
    exe = ctx.attr.deploy[DefaultInfo].files_to_run.executable
    link = ctx.actions.declare_file(ctx.label.name)
    ctx.actions.symlink(output = link, target_file = exe)
    return [
        DefaultInfo(executable = link),
        DxDeployInfo(app = ctx.attr.app.label, profile = "release"),
    ]

my_deploy = rule(
    implementation = _my_deploy_impl,
    executable = True,
    attrs = {
        "app": attr.label(mandatory = True),
        "deploy": attr.label(executable = True, cfg = "target", mandatory = True),
    },
)
```
