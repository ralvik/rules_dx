"""Project-owned Starlark testing facade (ADR 0009).

Contract: `docs/decisions/0009-starlark-testing.md`.
"""

DxSubjectInfo = provider(
    doc = "Analysis observations a subject rule exposes to starlark_test.",
    fields = {
        "fields": "String-keyed, string-valued observations about the subject.",
    },
)

DxAspectInfo = provider(
    doc = "Aspect observations derived without subject cooperation.",
    fields = {
        "fields": "String-keyed, string-valued observations the aspect saw.",
    },
)

def _dx_aspect_note_impl(target, ctx):
    """Derives one aspect note without subject cooperation. See: `docs/testing/starlark.md#modes`."""
    fields = {
        "aspect_seen": "True",
        "subject_label": _display_label(target.label),
    }
    if DxSubjectInfo in target:
        fields["field_count"] = str(len(target[DxSubjectInfo].fields.keys()))
        fields["has_subject"] = "True"
    else:
        fields["field_count"] = "0"
        fields["has_subject"] = "False"
    seen = []
    for dep in getattr(ctx.rule.attr, "deps", []):
        if DxAspectInfo in dep:
            label = dep[DxAspectInfo].fields.get("subject_label", "")
            if label != "":
                seen.append(label)
    if len(seen) > 0:
        fields["transitive"] = ",".join(sorted(seen))
    fields["transitive_count"] = str(len(seen))
    return [DxAspectInfo(fields = fields)]

dx_aspect_note = aspect(
    implementation = _dx_aspect_note_impl,
    attr_aspects = ["deps"],
    doc = "Observation aspect applied to every analysis subject.",
)

def expect_equal(name, actual, expected):
    """Builds one equality-check record as a JSON string.

    Runs at loading time when called from a test `.bzl` file top level or
    from a test macro body, which proves load-phase execution of the subject
    expressions computing `actual`.
    """
    return json.encode({
        "actual": actual,
        "expected": expected,
        "kind": "equal",
        "name": name,
    })

def expect_true(name, actual):
    """Builds one boolean-true record. See: `docs/testing/starlark.md#authoring`."""
    return json.encode({
        "actual": actual,
        "kind": "true",
        "name": name,
        "passed": actual == True,
    })

def expect_false(name, actual):
    """Builds one boolean-false record. See: `docs/testing/starlark.md#authoring`."""
    return json.encode({
        "actual": actual,
        "kind": "false",
        "name": name,
        "passed": actual == False,
    })

def expect_contains(name, haystack, needle):
    """Builds one membership record (string substring, list/tuple element, dict key). See: `docs/testing/starlark.md#authoring`."""
    haystack_type = type(haystack)
    if haystack_type == "string":
        if type(needle) != "string":
            fail("expect_contains: needle must be string when haystack is string, got " + type(needle))
        passed = needle in haystack
    elif haystack_type == "list" or haystack_type == "tuple":
        passed = needle in haystack
    elif haystack_type == "dict":
        passed = needle in haystack
    else:
        fail("expect_contains: haystack must be string, list, tuple, or dict, got " + haystack_type)
    return json.encode({
        "haystack": haystack,
        "kind": "contains",
        "name": name,
        "needle": needle,
        "passed": passed,
    })

def expect_match(name, value, want):
    """Builds one stringified-substring record. See: `docs/testing/starlark.md#authoring`."""
    if type(want) != "string":
        fail("expect_match: want must be string, got " + type(want))
    return json.encode({
        "kind": "match",
        "name": name,
        "passed": want in str(value),
        "value": value,
        "want": want,
    })

def _display_label(label):
    """Renders a label for observations and diagnostics.

    Strips one leading canonical-repository marker (two at-signs) from
    Bazel 9 rendering so observations stay readable; the stripped form is
 pinned to the supported Bazel and requalified on version bumps.
    """
    text = str(label)
    if text.startswith("@@"):  # buildifier: disable=canonical-repository
        return text[2:]
    return text

def display_label(label):
    """Renders a label with the canonical-repository marker stripped.

    Public alias of the observation rendering for rules that persist
    labels into outputs (for example staged management metadata).
    """
    return _display_label(label)

def _shell_quote(s):
    """Single-quote a string for embedding in the generated runner script."""
    return "'" + s.replace("'", "'\\''") + "'"

def _parse_check(raw):
    record = json.decode(raw)
    kind = record["kind"] if "kind" in record else "equal"
    if kind == "equal":
        for key in ("name", "expected", "actual"):
            if key not in record:
                fail("starlark_test: check record is missing key '" + key + "': " + raw)
        return struct(
            kind = "equal",
            name = str(record["name"]),
            expected = str(record["expected"]),
            actual = str(record["actual"]),
        )
    elif kind == "true" or kind == "false":
        for key in ("name", "actual", "passed"):
            if key not in record:
                fail("starlark_test: check record is missing key '" + key + "': " + raw)
        return struct(
            kind = kind,
            name = str(record["name"]),
            actual = str(record["actual"]),
            passed = str(record["passed"]),
        )
    elif kind == "contains":
        for key in ("name", "haystack", "needle", "passed"):
            if key not in record:
                fail("starlark_test: check record is missing key '" + key + "': " + raw)
        return struct(
            kind = "contains",
            name = str(record["name"]),
            haystack = str(record["haystack"]),
            needle = str(record["needle"]),
            passed = str(record["passed"]),
        )
    elif kind == "match":
        for key in ("name", "value", "want", "passed"):
            if key not in record:
                fail("starlark_test: check record is missing key '" + key + "': " + raw)
        return struct(
            kind = "match",
            name = str(record["name"]),
            value = str(record["value"]),
            want = str(record["want"]),
            passed = str(record["passed"]),
        )
    else:
        fail("starlark_test: unknown check kind '" + kind + "': " + raw)

def _check_lines(checks):
    lines = []
    for raw in checks:
        check = _parse_check(raw)
        if check.kind == "equal":
            lines.append(
                "check " + _shell_quote(check.name) + " " +
                _shell_quote(check.expected) + " " + _shell_quote(check.actual),
            )
        elif check.kind == "true":
            lines.append(
                "check_true " + _shell_quote(check.name) + " " +
                _shell_quote(check.actual) + " " + _shell_quote(check.passed),
            )
        elif check.kind == "false":
            lines.append(
                "check_false " + _shell_quote(check.name) + " " +
                _shell_quote(check.actual) + " " + _shell_quote(check.passed),
            )
        elif check.kind == "contains":
            lines.append(
                "check_contains " + _shell_quote(check.name) + " " +
                _shell_quote(check.haystack) + " " + _shell_quote(check.needle) + " " +
                _shell_quote(check.passed),
            )
        else:
            lines.append(
                "check_match " + _shell_quote(check.name) + " " +
                _shell_quote(check.value) + " " + _shell_quote(check.want) + " " +
                _shell_quote(check.passed),
            )
    return lines

def _file_check_lines(file_checks):
    """Renders one grep assertion per required substring.

    Each `file_checks` value lists required substrings, one per line; every
    line must be present (AND semantics). Empty lines are authoring errors.
    """
    lines = []
    for target in sorted(file_checks.keys(), key = lambda t: str(t.label)):
        want = file_checks[target]
        required = [line for line in want.split("\n") if line != ""]
        if len(required) == 0:
            fail("starlark_test: file_checks value is empty for " + str(target.label))
        for f in sorted(target.files.to_list(), key = lambda f: f.short_path):
            for i, needle in enumerate(required):
                lines.append(
                    "check_file " + _shell_quote(_display_label(target.label)) + " " +
                    _shell_quote(f.short_path) + " " +
                    _shell_quote(str(i + 1) + "/" + str(len(required))) + " " +
                    _shell_quote(needle),
                )
    return lines

_RUNNER_PRELUDE = [
    "#!/bin/sh",
    "# Generated by starlark_test. Do not edit.",
    "fail=0",
    "pass_count=0",
    "fail_count=0",
    "check() {",
    '    name="$1"; expected="$2"; actual="$3"',
    '    if [ "$expected" = "$actual" ]; then',
    '        echo "PASS: $name"',
    "        pass_count=$((pass_count + 1))",
    "    else",
    '        echo "FAIL: $name"',
    '        echo "  expected: $expected"',
    '        echo "  actual:   $actual"',
    "        fail=1",
    "        fail_count=$((fail_count + 1))",
    "    fi",
    "}",
    "check_true() {",
    '    name="$1"; actual="$2"; passed="$3"',
    '    if [ "$passed" = "True" ]; then',
    '        echo "PASS: $name"',
    "        pass_count=$((pass_count + 1))",
    "    else",
    '        echo "FAIL: $name"',
    '        echo "  expected: True"',
    '        echo "  actual:   $actual"',
    "        fail=1",
    "        fail_count=$((fail_count + 1))",
    "    fi",
    "}",
    "check_false() {",
    '    name="$1"; actual="$2"; passed="$3"',
    '    if [ "$passed" = "True" ]; then',
    '        echo "PASS: $name"',
    "        pass_count=$((pass_count + 1))",
    "    else",
    '        echo "FAIL: $name"',
    '        echo "  expected: False"',
    '        echo "  actual:   $actual"',
    "        fail=1",
    "        fail_count=$((fail_count + 1))",
    "    fi",
    "}",
    "check_contains() {",
    '    name="$1"; haystack="$2"; needle="$3"; passed="$4"',
    '    if [ "$passed" = "True" ]; then',
    '        echo "PASS: $name"',
    "        pass_count=$((pass_count + 1))",
    "    else",
    '        echo "FAIL: $name"',
    '        echo "  haystack: $haystack"',
    '        echo "  missing:  $needle"',
    "        fail=1",
    "        fail_count=$((fail_count + 1))",
    "    fi",
    "}",
    "check_match() {",
    '    name="$1"; value="$2"; want="$3"; passed="$4"',
    '    if [ "$passed" = "True" ]; then',
    '        echo "PASS: $name"',
    "        pass_count=$((pass_count + 1))",
    "    else",
    '        echo "FAIL: $name"',
    '        echo "  value: $value"',
    '        echo "  missing substring: $want"',
    "        fail=1",
    "        fail_count=$((fail_count + 1))",
    "    fi",
    "}",
    "check_file() {",
    '    label="$1"; short_path="$2"; ordinal="$3"; want="$4"',
    '    path="$TEST_SRCDIR/$TEST_WORKSPACE/$short_path"',
    '    if grep -q -F -e "$want" "$path"; then',
    '        echo "PASS: file $label contains substring $ordinal"',
    "        pass_count=$((pass_count + 1))",
    "    else",
    '        echo "FAIL: file $label is missing substring $ordinal"',
    '        echo "  substring: $want"',
    '        echo "  file: $path"',
    "        fail=1",
    "        fail_count=$((fail_count + 1))",
    "    fi",
    "}",
]

_RUNNER_EPILOGUE = [
    'echo "starlark_test: $pass_count passed, $fail_count failed"',
    "exit $fail",
]

def _write_runner(ctx, body_lines, runfiles_files):
    runner = ctx.actions.declare_file(ctx.label.name + ".sh")
    ctx.actions.write(
        runner,
        "\n".join(_RUNNER_PRELUDE + body_lines + _RUNNER_EPILOGUE) + "\n",
        is_executable = True,
    )
    return [DefaultInfo(
        executable = runner,
        runfiles = ctx.runfiles(files = runfiles_files),
    )]

def _validate_common(mode, checks, subjects, file_checks):
    if len(checks) == 0 and len(subjects) == 0 and len(file_checks) == 0:
        fail("starlark_test (" + mode + " mode): no evidence: " +
             "provide checks, subjects, or file_checks")

def _file_check_files(file_checks):
    files = []
    for target in file_checks.keys():
        target_files = target.files.to_list()
        if len(target_files) == 0:
            fail("starlark_test: file_checks target has no files: " + str(target.label))
        files.extend(target_files)
    return files

def _load_test_impl(ctx):
    _validate_common("load", ctx.attr.checks, ctx.attr.subjects, ctx.attr.file_checks)
    if len(ctx.attr.checks) == 0:
        fail("starlark_test (load mode): checks must be non-empty: " +
             "load tests assert values computed while test files load")
    if len(ctx.attr.subjects) != 0:
        fail("starlark_test (load mode): subjects must be empty: " +
             "load tests observe loading, not analysis")
    files = _file_check_files(ctx.attr.file_checks)
    body = _check_lines(ctx.attr.checks)
    body.extend(_file_check_lines(ctx.attr.file_checks))
    return _write_runner(ctx, body, files)

def _unit_test_impl(ctx):
    _validate_common("unit", ctx.attr.checks, ctx.attr.subjects, ctx.attr.file_checks)
    if len(ctx.attr.checks) == 0:
        fail("starlark_test (unit mode): checks must be non-empty: " +
             "unit tests assert pure function results")
    if len(ctx.attr.subjects) != 0:
        fail("starlark_test (unit mode): subjects must be empty: " +
             "unit tests take no analysis subjects")
    files = _file_check_files(ctx.attr.file_checks)
    body = _check_lines(ctx.attr.checks)
    body.extend(_file_check_lines(ctx.attr.file_checks))
    return _write_runner(ctx, body, files)

def _observe_subjects(subjects):
    lines = []
    for target in sorted(subjects, key = lambda t: str(t.label)):
        lines.append("subject " + _display_label(target.label))
        info = target[DefaultInfo]
        for f in sorted(info.files.to_list(), key = lambda f: f.basename):
            lines.append("file " + f.basename)
        if DxSubjectInfo in target:
            fields = target[DxSubjectInfo].fields
            for key in sorted(fields.keys()):
                lines.append("field " + key + "=" + fields[key])
        if DxAspectInfo in target:
            aspect_fields = target[DxAspectInfo].fields
            for key in sorted(aspect_fields.keys()):
                lines.append("aspect_field " + key + "=" + aspect_fields[key])
    return lines

def _analysis_test_impl(ctx):
    _validate_common("analysis", ctx.attr.checks, ctx.attr.subjects, ctx.attr.file_checks)
    if len(ctx.attr.subjects) == 0:
        fail("starlark_test (analysis mode): subjects must be non-empty: " +
             "analysis tests observe subject targets")
    observed = _observe_subjects(ctx.attr.subjects)
    observations = ctx.actions.declare_file(ctx.label.name + "_observations.txt")
    ctx.actions.write(observations, "\n".join(observed) + "\n")
    body = _check_lines(ctx.attr.checks)
    body.extend(_file_check_lines(ctx.attr.file_checks))
    if ctx.attr.expected_observations != "":
        body.append("check " + _shell_quote("observations") + " " +
                    _shell_quote(ctx.attr.expected_observations.strip()) + " " +
                    _shell_quote("\n".join(observed)))
    body.append("echo '--- observations ---'")
    body.append("cat \"$TEST_SRCDIR/$TEST_WORKSPACE/" + observations.short_path + "\"")
    files = _file_check_files(ctx.attr.file_checks)
    files.append(observations)
    return _write_runner(ctx, body, files)

def _execution_test_impl(ctx):
    _validate_common("execution", ctx.attr.checks, ctx.attr.subjects, ctx.attr.file_checks)
    if len(ctx.attr.file_checks) == 0:
        fail("starlark_test (execution mode): file_checks must be non-empty: " +
             "execution tests assert on files read while the test runs")
    if len(ctx.attr.subjects) != 0:
        fail("starlark_test (execution mode): subjects must be empty: " +
             "execution tests read runfiles, not analysis subjects")
    files = _file_check_files(ctx.attr.file_checks)
    body = _check_lines(ctx.attr.checks)
    body.extend(_file_check_lines(ctx.attr.file_checks))
    return _write_runner(ctx, body, files)

_common_attrs = {
    "checks": attr.string_list(
        doc = "Assertion records from expect_equal, expect_true, expect_false, expect_contains, expect_match, evaluated at execution.",
    ),
    "expected_observations": attr.string(
        default = "",
        doc = "Analysis-mode expected observation rendering, one line per entry.",
    ),
    "file_checks": attr.label_keyed_string_dict(
        allow_files = True,
        doc = "Maps file targets to required substrings, one per line; " +
              "every line must be present in the file at execution time.",
    ),
    "subjects": attr.label_list(
        aspects = [dx_aspect_note],
        doc = "Analysis-mode subject targets observed for providers, outputs, and aspect notes.",
    ),
}

_starlark_load_test = rule(
    implementation = _load_test_impl,
    test = True,
    attrs = _common_attrs,
)

_starlark_unit_test = rule(
    implementation = _unit_test_impl,
    test = True,
    attrs = _common_attrs,
)

_starlark_analysis_test = rule(
    implementation = _analysis_test_impl,
    test = True,
    attrs = _common_attrs,
)

_starlark_execution_test = rule(
    implementation = _execution_test_impl,
    test = True,
    attrs = _common_attrs,
)

_MODES = {
    "analysis": _starlark_analysis_test,
    "execution": _starlark_execution_test,
    "load": _starlark_load_test,
    "unit": _starlark_unit_test,
}

def starlark_test(name, mode, checks = [], subjects = [], expected_observations = "", file_checks = {}, **kwargs):
    """Instantiates one test target in the given mode.

    One macro call is one addressable Bazel test target with one Bazel
    result; mismatches accumulate and report together in declaration order.
    `size` defaults to `small`; pass `tags = ["manual"]` for negative
    demonstrations that must fail without breaking `//...` suites."""
    if mode not in _MODES:
        fail("starlark_test: unknown mode '" + mode + "': want one of " +
             ", ".join(sorted(_MODES.keys())))
    kwargs.setdefault("size", "small")
    _MODES[mode](
        name = name,
        checks = checks,
        subjects = subjects,
        expected_observations = expected_observations,
        file_checks = file_checks,
        **kwargs
    )
