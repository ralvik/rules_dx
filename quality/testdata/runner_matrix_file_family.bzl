"""Interpreted/file-family cohort matrix cells."""

DJLINT_LINT = """matrix/djlint_dirty.html:3:1: H006 img tags require alt text"""

STYLELINT_LINT_CSS = """[{"source": "matrix/stylelint_dirty.css", "warnings": [{"line": 2, "column": 5, "rule": "color-no-invalid-hex", "text": "Unexpected invalid hex", "severity": "error"}]}]"""
STYLELINT_LINT_LESS = """[{"source": "matrix/stylelint_dirty.less", "warnings": [{"line": 2, "column": 5, "rule": "color-no-invalid-hex", "text": "Unexpected invalid hex", "severity": "error"}]}]"""
STYLELINT_LINT_SCSS = """[{"source": "matrix/stylelint_dirty.scss", "warnings": [{"line": 2, "column": 5, "rule": "color-no-invalid-hex", "text": "Unexpected invalid hex", "severity": "error"}]}]"""

RUBOCOP_LINT = """{"files": [{"path": "matrix/rubocop_dirty.rb", "offenses": [{"severity": "convention", "message": "Use double quotes", "cop_name": "Style/StringLiterals", "location": {"line": 3, "column": 1}}]}]}"""

PSSCRIPTANALYZER_LINT = """matrix/psscriptanalyzer_dirty.ps1:4:1: [PSAvoidUsingWriteHost] Avoid using Write-Host"""

YAMLLINT_LINT = """matrix/yamllint_dirty.yaml:2:1: [trailing-spaces] trailing spaces"""

SHELLCHECK_LINT = """matrix/shellcheck_dirty.sh:3:1: warning: Double quote to prevent globbing [SC2086]"""

KEEP_SORTED_LINT = """matrix/keep_sorted_dirty.txt:4: block is not sorted"""

FILE_FAMILY_CASES = [
    {
        "name": "matrix_cue_format_pass",
        "generated": {
            "matrix/cue_clean.cue": "package sample\n\nvalue: \"hello\"\n",
        },
        "capability": "format",
        "stages": ["cue;cue;matrix/cue_clean.cue"],
        "tool_names": ["cue"],
        "tool_binaries": ["//quality/testdata:fake_cue"],
        "expected": """producer //quality/testdata:matrix_cue_format_pass""",
    },
    {
        "name": "matrix_cue_format_fail",
        "generated": {
            "matrix/cue_dirty.cue": "package sample\n\nvalue:\"hello\"BADFMT\n",
        },
        "capability": "format",
        "stages": ["cue;cue;matrix/cue_dirty.cue"],
        "tool_names": ["cue"],
        "tool_binaries": ["//quality/testdata:fake_cue"],
        "expected": """producer //quality/testdata:matrix_cue_format_fail""",
    },
    {
        "name": "matrix_jsonnet_format_pass",
        "generated": {
            "matrix/jsonnet_clean.jsonnet": "{\n  value: \"hello\"\n}\n",
        },
        "capability": "format",
        "stages": ["jsonnetfmt;jsonnet;matrix/jsonnet_clean.jsonnet"],
        "tool_names": ["jsonnetfmt"],
        "tool_binaries": ["//quality/testdata:fake_jsonnetfmt"],
        "expected": """producer //quality/testdata:matrix_jsonnet_format_pass""",
    },
    {
        "name": "matrix_jsonnet_format_fail",
        "generated": {
            "matrix/jsonnet_dirty.jsonnet": "{value:\"hello\"BADFMT}\n",
        },
        "capability": "format",
        "stages": ["jsonnetfmt;jsonnet;matrix/jsonnet_dirty.jsonnet"],
        "tool_names": ["jsonnetfmt"],
        "tool_binaries": ["//quality/testdata:fake_jsonnetfmt"],
        "expected": """producer //quality/testdata:matrix_jsonnet_format_fail""",
    },
    {
        "name": "matrix_pkl_format_pass",
        "generated": {
            "matrix/pkl_clean.pkl": "value = \"hello\"\n",
        },
        "capability": "format",
        "stages": ["pkl;pkl;matrix/pkl_clean.pkl"],
        "tool_names": ["pkl"],
        "tool_binaries": ["//quality/testdata:fake_pkl"],
        "expected": """producer //quality/testdata:matrix_pkl_format_pass""",
    },
    {
        "name": "matrix_pkl_format_fail",
        "generated": {
            "matrix/pkl_dirty.pkl": "value=\"hello\"BADFMT\n",
        },
        "capability": "format",
        "stages": ["pkl;pkl;matrix/pkl_dirty.pkl"],
        "tool_names": ["pkl"],
        "tool_binaries": ["//quality/testdata:fake_pkl"],
        "expected": """producer //quality/testdata:matrix_pkl_format_fail""",
    },
    {
        "name": "matrix_go_module_format_pass",
        "generated": {
            "matrix/modfmt_clean.mod": "module example.com/sample\n\ngo 1.24\n",
        },
        "capability": "format",
        "stages": ["modfmt;go_module;matrix/modfmt_clean.mod"],
        "tool_names": ["modfmt"],
        "tool_binaries": ["//quality/testdata:fake_modfmt"],
        "expected": """producer //quality/testdata:matrix_go_module_format_pass""",
    },
    {
        "name": "matrix_go_module_format_fail",
        "generated": {
            "matrix/modfmt_dirty.mod": "module example.com/sampleBADFMT\n",
        },
        "capability": "format",
        "stages": ["modfmt;go_module;matrix/modfmt_dirty.mod"],
        "tool_names": ["modfmt"],
        "tool_binaries": ["//quality/testdata:fake_modfmt"],
        "expected": """producer //quality/testdata:matrix_go_module_format_fail""",
    },
    {
        "name": "matrix_terraform_format_pass",
        "generated": {
            "matrix/terraform_clean.tf": "resource \"null_resource\" \"sample\" {}\n",
        },
        "capability": "format",
        "stages": ["terraform;terraform;matrix/terraform_clean.tf"],
        "tool_names": ["terraform"],
        "tool_binaries": ["//quality/testdata:fake_terraform"],
        "expected": """producer //quality/testdata:matrix_terraform_format_pass""",
    },
    {
        "name": "matrix_terraform_format_fail",
        "generated": {
            "matrix/terraform_dirty.tf": "resourceBADFMT\n",
        },
        "capability": "format",
        "stages": ["terraform;terraform;matrix/terraform_dirty.tf"],
        "tool_names": ["terraform"],
        "tool_binaries": ["//quality/testdata:fake_terraform"],
        "expected": """producer //quality/testdata:matrix_terraform_format_fail""",
    },
    {
        "name": "matrix_yaml_format_pass",
        "generated": {
            "matrix/yamlfmt_clean.yaml": "key: value\n",
        },
        "capability": "format",
        "stages": ["yamlfmt;yaml;matrix/yamlfmt_clean.yaml"],
        "tool_names": ["yamlfmt"],
        "tool_binaries": ["//quality/testdata:fake_yamlfmt"],
        "expected": """producer //quality/testdata:matrix_yaml_format_pass""",
    },
    {
        "name": "matrix_yaml_format_fail",
        "generated": {
            "matrix/yamlfmt_dirty.yaml": "key:\"value\"BADFMT\n",
        },
        "capability": "format",
        "stages": ["yamlfmt;yaml;matrix/yamlfmt_dirty.yaml"],
        "tool_names": ["yamlfmt"],
        "tool_binaries": ["//quality/testdata:fake_yamlfmt"],
        "expected": """producer //quality/testdata:matrix_yaml_format_fail""",
    },
    {
        "name": "matrix_shell_format_pass",
        "generated": {
            "matrix/shfmt_clean.sh": "#!/usr/bin/env bash\necho \"hello\"\n",
        },
        "capability": "format",
        "stages": ["shfmt;shell;matrix/shfmt_clean.sh"],
        "tool_names": ["shfmt"],
        "tool_binaries": ["//quality/testdata:fake_shfmt"],
        "expected": """producer //quality/testdata:matrix_shell_format_pass""",
    },
    {
        "name": "matrix_shell_format_fail",
        "generated": {
            "matrix/shfmt_dirty.sh": "#!/usr/bin/env bash\necho\"hello\"BADFMT\n",
        },
        "capability": "format",
        "stages": ["shfmt;shell;matrix/shfmt_dirty.sh"],
        "tool_names": ["shfmt"],
        "tool_binaries": ["//quality/testdata:fake_shfmt"],
        "expected": """producer //quality/testdata:matrix_shell_format_fail""",
    },
    {
        "name": "matrix_ruby_format_pass",
        "generated": {
            "matrix/standardrb_clean.rb": "puts \"hello\"\n",
        },
        "capability": "format",
        "stages": ["standardrb;ruby;matrix/standardrb_clean.rb"],
        "tool_names": ["standardrb"],
        "tool_binaries": ["//quality/testdata:fake_standardrb"],
        "expected": """producer //quality/testdata:matrix_ruby_format_pass""",
    },
    {
        "name": "matrix_ruby_format_fail",
        "generated": {
            "matrix/standardrb_dirty.rb": "puts BADFMT\n",
        },
        "capability": "format",
        "stages": ["standardrb;ruby;matrix/standardrb_dirty.rb"],
        "tool_names": ["standardrb"],
        "tool_binaries": ["//quality/testdata:fake_standardrb"],
        "expected": """producer //quality/testdata:matrix_ruby_format_fail""",
    },
    {
        "name": "matrix_html_template_format_pass",
        "generated": {
            "matrix/djlint_format_clean.html": "<html><body><p>hello</p></body></html>\n",
        },
        "capability": "format",
        "stages": ["djlint;html_template;matrix/djlint_format_clean.html"],
        "tool_names": ["djlint"],
        "tool_binaries": ["//quality/testdata:fake_djlint_format"],
        "expected": """producer //quality/testdata:matrix_html_template_format_pass""",
    },
    {
        "name": "matrix_html_template_format_fail",
        "generated": {
            "matrix/djlint_format_dirty.html": "<html><body>BADFMT</body></html>\n",
        },
        "capability": "format",
        "stages": ["djlint;html_template;matrix/djlint_format_dirty.html"],
        "tool_names": ["djlint"],
        "tool_binaries": ["//quality/testdata:fake_djlint_format"],
        "expected": """producer //quality/testdata:matrix_html_template_format_fail""",
    },
    {
        "name": "matrix_css_format_pass",
        "generated": {
            "matrix/prettier_css_clean.css": ".sample {\n  color: #fff;\n}\n",
        },
        "capability": "format",
        "stages": ["prettier;css;matrix/prettier_css_clean.css"],
        "tool_names": ["prettier"],
        "tool_binaries": ["//quality/testdata:fake_prettier_file_family"],
        "expected": """producer //quality/testdata:matrix_css_format_pass""",
    },
    {
        "name": "matrix_css_format_fail",
        "generated": {
            "matrix/prettier_css_dirty.css": ".sample{color:#fffBADFMT}\n",
        },
        "capability": "format",
        "stages": ["prettier;css;matrix/prettier_css_dirty.css"],
        "tool_names": ["prettier"],
        "tool_binaries": ["//quality/testdata:fake_prettier_file_family"],
        "expected": """producer //quality/testdata:matrix_css_format_fail""",
    },
    {
        "name": "matrix_less_format_pass",
        "generated": {
            "matrix/prettier_less_clean.less": ".sample {\n  color: #fff;\n}\n",
        },
        "capability": "format",
        "stages": ["prettier;less;matrix/prettier_less_clean.less"],
        "tool_names": ["prettier"],
        "tool_binaries": ["//quality/testdata:fake_prettier_file_family"],
        "expected": """producer //quality/testdata:matrix_less_format_pass""",
    },
    {
        "name": "matrix_less_format_fail",
        "generated": {
            "matrix/prettier_less_dirty.less": ".sample{color:#fffBADFMT}\n",
        },
        "capability": "format",
        "stages": ["prettier;less;matrix/prettier_less_dirty.less"],
        "tool_names": ["prettier"],
        "tool_binaries": ["//quality/testdata:fake_prettier_file_family"],
        "expected": """producer //quality/testdata:matrix_less_format_fail""",
    },
    {
        "name": "matrix_scss_format_pass",
        "generated": {
            "matrix/prettier_scss_clean.scss": ".sample {\n  color: #fff;\n}\n",
        },
        "capability": "format",
        "stages": ["prettier;scss;matrix/prettier_scss_clean.scss"],
        "tool_names": ["prettier"],
        "tool_binaries": ["//quality/testdata:fake_prettier_file_family"],
        "expected": """producer //quality/testdata:matrix_scss_format_pass""",
    },
    {
        "name": "matrix_scss_format_fail",
        "generated": {
            "matrix/prettier_scss_dirty.scss": ".sample{color:#fffBADFMT}\n",
        },
        "capability": "format",
        "stages": ["prettier;scss;matrix/prettier_scss_dirty.scss"],
        "tool_names": ["prettier"],
        "tool_binaries": ["//quality/testdata:fake_prettier_file_family"],
        "expected": """producer //quality/testdata:matrix_scss_format_fail""",
    },
    {
        "name": "matrix_gherkin_format_pass",
        "generated": {
            "matrix/prettier_gherkin_clean.feature": "Feature: sample\n  Scenario: hello\n",
        },
        "capability": "format",
        "stages": ["prettier;gherkin;matrix/prettier_gherkin_clean.feature"],
        "tool_names": ["prettier"],
        "tool_binaries": ["//quality/testdata:fake_prettier_file_family"],
        "expected": """producer //quality/testdata:matrix_gherkin_format_pass""",
    },
    {
        "name": "matrix_gherkin_format_fail",
        "generated": {
            "matrix/prettier_gherkin_dirty.feature": "Feature:sampleBADFMT\n",
        },
        "capability": "format",
        "stages": ["prettier;gherkin;matrix/prettier_gherkin_dirty.feature"],
        "tool_names": ["prettier"],
        "tool_binaries": ["//quality/testdata:fake_prettier_file_family"],
        "expected": """producer //quality/testdata:matrix_gherkin_format_fail""",
    },
    {
        "name": "matrix_sql_format_pass",
        "generated": {
            "matrix/prettier_sql_clean.sql": "SELECT 1;\n",
        },
        "capability": "format",
        "stages": ["prettier;sql;matrix/prettier_sql_clean.sql"],
        "tool_names": ["prettier"],
        "tool_binaries": ["//quality/testdata:fake_prettier_file_family"],
        "expected": """producer //quality/testdata:matrix_sql_format_pass""",
    },
    {
        "name": "matrix_sql_format_fail",
        "generated": {
            "matrix/prettier_sql_dirty.sql": "SELECT  1BADFMT\n",
        },
        "capability": "format",
        "stages": ["prettier;sql;matrix/prettier_sql_dirty.sql"],
        "tool_names": ["prettier"],
        "tool_binaries": ["//quality/testdata:fake_prettier_file_family"],
        "expected": """producer //quality/testdata:matrix_sql_format_fail""",
    },
    {
        "name": "matrix_xml_format_pass",
        "generated": {
            "matrix/prettier_xml_clean.xml": "<a>hello</a>\n",
        },
        "capability": "format",
        "stages": ["prettier;xml;matrix/prettier_xml_clean.xml"],
        "tool_names": ["prettier"],
        "tool_binaries": ["//quality/testdata:fake_prettier_file_family"],
        "expected": """producer //quality/testdata:matrix_xml_format_pass""",
    },
    {
        "name": "matrix_xml_format_fail",
        "generated": {
            "matrix/prettier_xml_dirty.xml": "<a>hello</a>BADFMT\n",
        },
        "capability": "format",
        "stages": ["prettier;xml;matrix/prettier_xml_dirty.xml"],
        "tool_names": ["prettier"],
        "tool_binaries": ["//quality/testdata:fake_prettier_file_family"],
        "expected": """producer //quality/testdata:matrix_xml_format_fail""",
    },
    {
        "name": "matrix_html_template_lint_pass",
        "generated": {
            "matrix/djlint_clean.html": "<html><body><p>hello</p></body></html>\n",
        },
        "capability": "lint",
        "stages": ["djlint;html_template;matrix/djlint_clean.html"],
        "upstream_tools": ["djlint"],
        "upstream_generated": {"djlint": ""},
        "expected": """producer //quality/testdata:matrix_html_template_lint_pass""",
    },
    {
        "name": "matrix_html_template_lint_fail",
        "generated": {
            "matrix/djlint_dirty.html": "<html>\n<body>\n<img src=\"x\">\n</body>\n</html>\n",
        },
        "capability": "lint",
        "stages": ["djlint;html_template;matrix/djlint_dirty.html"],
        "upstream_tools": ["djlint"],
        "upstream_generated": {"djlint": DJLINT_LINT},
        "expected": """producer //quality/testdata:matrix_html_template_lint_fail""",
    },
    {
        "name": "matrix_css_lint_pass",
        "generated": {
            "matrix/stylelint_clean.css": ".sample {\n  color: #fff;\n}\n",
        },
        "capability": "lint",
        "stages": ["stylelint;css;matrix/stylelint_clean.css"],
        "upstream_tools": ["stylelint"],
        "upstream_generated": {"stylelint": "[{\"source\": \"matrix/stylelint_clean.css\", \"warnings\": []}]"},
        "expected": """producer //quality/testdata:matrix_css_lint_pass""",
    },
    {
        "name": "matrix_css_lint_fail",
        "generated": {
            "matrix/stylelint_dirty.css": ".sample {\n  color: #zzz;\n}\n",
        },
        "capability": "lint",
        "stages": ["stylelint;css;matrix/stylelint_dirty.css"],
        "upstream_tools": ["stylelint"],
        "upstream_generated": {"stylelint": STYLELINT_LINT_CSS},
        "expected": """producer //quality/testdata:matrix_css_lint_fail""",
    },
    {
        "name": "matrix_less_lint_pass",
        "generated": {
            "matrix/stylelint_clean.less": ".sample {\n  color: #fff;\n}\n",
        },
        "capability": "lint",
        "stages": ["stylelint;less;matrix/stylelint_clean.less"],
        "upstream_tools": ["stylelint"],
        "upstream_generated": {"stylelint": "[{\"source\": \"matrix/stylelint_clean.less\", \"warnings\": []}]"},
        "expected": """producer //quality/testdata:matrix_less_lint_pass""",
    },
    {
        "name": "matrix_less_lint_fail",
        "generated": {
            "matrix/stylelint_dirty.less": ".sample {\n  color: #zzz;\n}\n",
        },
        "capability": "lint",
        "stages": ["stylelint;less;matrix/stylelint_dirty.less"],
        "upstream_tools": ["stylelint"],
        "upstream_generated": {"stylelint": STYLELINT_LINT_LESS},
        "expected": """producer //quality/testdata:matrix_less_lint_fail""",
    },
    {
        "name": "matrix_scss_lint_pass",
        "generated": {
            "matrix/stylelint_clean.scss": ".sample {\n  color: #fff;\n}\n",
        },
        "capability": "lint",
        "stages": ["stylelint;scss;matrix/stylelint_clean.scss"],
        "upstream_tools": ["stylelint"],
        "upstream_generated": {"stylelint": "[{\"source\": \"matrix/stylelint_clean.scss\", \"warnings\": []}]"},
        "expected": """producer //quality/testdata:matrix_scss_lint_pass""",
    },
    {
        "name": "matrix_scss_lint_fail",
        "generated": {
            "matrix/stylelint_dirty.scss": ".sample {\n  color: #zzz;\n}\n",
        },
        "capability": "lint",
        "stages": ["stylelint;scss;matrix/stylelint_dirty.scss"],
        "upstream_tools": ["stylelint"],
        "upstream_generated": {"stylelint": STYLELINT_LINT_SCSS},
        "expected": """producer //quality/testdata:matrix_scss_lint_fail""",
    },
    {
        "name": "matrix_ruby_lint_pass",
        "generated": {
            "matrix/rubocop_clean.rb": "puts \"hello\"\n",
        },
        "capability": "lint",
        "stages": ["rubocop;ruby;matrix/rubocop_clean.rb"],
        "upstream_tools": ["rubocop"],
        "upstream_generated": {"rubocop": "{\"files\": [{\"path\": \"matrix/rubocop_clean.rb\", \"offenses\": []}]}"},
        "expected": """producer //quality/testdata:matrix_ruby_lint_pass""",
    },
    {
        "name": "matrix_ruby_lint_fail",
        "generated": {
            "matrix/rubocop_dirty.rb": "puts \"hello\"\n# comment\nputs hello\n",
        },
        "capability": "lint",
        "stages": ["rubocop;ruby;matrix/rubocop_dirty.rb"],
        "upstream_tools": ["rubocop"],
        "upstream_generated": {"rubocop": RUBOCOP_LINT},
        "expected": """producer //quality/testdata:matrix_ruby_lint_fail""",
    },
    {
        "name": "matrix_powershell_lint_pass",
        "generated": {
            "matrix/psscriptanalyzer_clean.ps1": "Write-Output \"hello\"\n",
        },
        "capability": "lint",
        "stages": ["psscriptanalyzer;powershell;matrix/psscriptanalyzer_clean.ps1"],
        "upstream_tools": ["psscriptanalyzer"],
        "upstream_generated": {"psscriptanalyzer": ""},
        "expected": """producer //quality/testdata:matrix_powershell_lint_pass""",
    },
    {
        "name": "matrix_powershell_lint_fail",
        "generated": {
            "matrix/psscriptanalyzer_dirty.ps1": "Write-Output \"a\"\nWrite-Output \"b\"\nWrite-Output \"c\"\nWrite-Host \"hello\"\n",
        },
        "capability": "lint",
        "stages": ["psscriptanalyzer;powershell;matrix/psscriptanalyzer_dirty.ps1"],
        "upstream_tools": ["psscriptanalyzer"],
        "upstream_generated": {"psscriptanalyzer": PSSCRIPTANALYZER_LINT},
        "expected": """producer //quality/testdata:matrix_powershell_lint_fail""",
    },
    {
        "name": "matrix_yaml_lint_pass",
        "generated": {
            "matrix/yamllint_clean.yaml": "key: value\n",
        },
        "capability": "lint",
        "stages": ["yamllint;yaml;matrix/yamllint_clean.yaml"],
        "upstream_tools": ["yamllint"],
        "upstream_generated": {"yamllint": ""},
        "expected": """producer //quality/testdata:matrix_yaml_lint_pass""",
    },
    {
        "name": "matrix_yaml_lint_fail",
        "generated": {
            "matrix/yamllint_dirty.yaml": "key: value\nkey2: value  \n",
        },
        "capability": "lint",
        "stages": ["yamllint;yaml;matrix/yamllint_dirty.yaml"],
        "upstream_tools": ["yamllint"],
        "upstream_generated": {"yamllint": YAMLLINT_LINT},
        "expected": """producer //quality/testdata:matrix_yaml_lint_fail""",
    },
    {
        "name": "matrix_shell_lint_pass",
        "generated": {
            "matrix/shellcheck_clean.sh": "#!/usr/bin/env bash\necho \"hello\"\n",
        },
        "capability": "lint",
        "stages": ["shellcheck;shell;matrix/shellcheck_clean.sh"],
        "upstream_tools": ["shellcheck"],
        "upstream_generated": {"shellcheck": ""},
        "expected": """producer //quality/testdata:matrix_shell_lint_pass""",
    },
    {
        "name": "matrix_shell_lint_fail",
        "generated": {
            "matrix/shellcheck_dirty.sh": "#!/usr/bin/env bash\necho \"start\"\necho $hello\n",
        },
        "capability": "lint",
        "stages": ["shellcheck;shell;matrix/shellcheck_dirty.sh"],
        "upstream_tools": ["shellcheck"],
        "upstream_generated": {"shellcheck": SHELLCHECK_LINT},
        "expected": """producer //quality/testdata:matrix_shell_lint_fail""",
    },
    {
        "name": "matrix_text_lint_pass",
        "generated": {
            "matrix/keep_sorted_clean.txt": "a\nb\nc\n",
        },
        "capability": "lint",
        "stages": ["keep_sorted;text;matrix/keep_sorted_clean.txt"],
        "upstream_tools": ["keep_sorted"],
        "upstream_generated": {"keep_sorted": ""},
        "expected": """producer //quality/testdata:matrix_text_lint_pass""",
    },
    {
        "name": "matrix_text_lint_fail",
        "generated": {
            "matrix/keep_sorted_dirty.txt": "c\nb\na\nd\n",
        },
        "capability": "lint",
        "stages": ["keep_sorted;text;matrix/keep_sorted_dirty.txt"],
        "upstream_tools": ["keep_sorted"],
        "upstream_generated": {"keep_sorted": KEEP_SORTED_LINT},
        "expected": """producer //quality/testdata:matrix_text_lint_fail""",
    },
]
