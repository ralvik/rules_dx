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
        "expected": """producer //quality/testdata:matrix_cue_format_pass
capability FORMAT
stages 1
stage cue classes=cue sources=matrix/cue_clean.cue
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_cue_format_fail
capability FORMAT
stages 1
stage cue classes=cue sources=matrix/cue_dirty.cue
completed_rounds 2
convergence STABLE
initial 1
initial WARNING cue - matrix/cue_dirty.cue 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement matrix/cue_dirty.cue 29 35 "fixed"
""",
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
        "expected": """producer //quality/testdata:matrix_jsonnet_format_pass
capability FORMAT
stages 1
stage jsonnetfmt classes=jsonnet sources=matrix/jsonnet_clean.jsonnet
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_jsonnet_format_fail
capability FORMAT
stages 1
stage jsonnetfmt classes=jsonnet sources=matrix/jsonnet_dirty.jsonnet
completed_rounds 2
convergence STABLE
initial 1
initial WARNING jsonnetfmt - matrix/jsonnet_dirty.jsonnet 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement matrix/jsonnet_dirty.jsonnet 14 20 "fixed"
""",
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
        "expected": """producer //quality/testdata:matrix_pkl_format_pass
capability FORMAT
stages 1
stage pkl classes=pkl sources=matrix/pkl_clean.pkl
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_pkl_format_fail
capability FORMAT
stages 1
stage pkl classes=pkl sources=matrix/pkl_dirty.pkl
completed_rounds 2
convergence STABLE
initial 1
initial WARNING pkl - matrix/pkl_dirty.pkl 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement matrix/pkl_dirty.pkl 13 19 "fixed"
""",
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
        "expected": """producer //quality/testdata:matrix_go_module_format_pass
capability FORMAT
stages 1
stage modfmt classes=go_module sources=matrix/modfmt_clean.mod
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_go_module_format_fail
capability FORMAT
stages 1
stage modfmt classes=go_module sources=matrix/modfmt_dirty.mod
completed_rounds 2
convergence STABLE
initial 1
initial WARNING modfmt - matrix/modfmt_dirty.mod 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement matrix/modfmt_dirty.mod 25 31 "fixed"
""",
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
        "expected": """producer //quality/testdata:matrix_terraform_format_pass
capability FORMAT
stages 1
stage terraform classes=terraform sources=matrix/terraform_clean.tf
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_terraform_format_fail
capability FORMAT
stages 1
stage terraform classes=terraform sources=matrix/terraform_dirty.tf
completed_rounds 1
convergence STABLE
initial 1
initial WARNING terraform - matrix/terraform_dirty.tf 0 0 fixable=false "file is not formatted"
terminal 1
terminal WARNING terraform - matrix/terraform_dirty.tf 0 0 fixable=false "file is not formatted"
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_yaml_format_pass
capability FORMAT
stages 1
stage yamlfmt classes=yaml sources=matrix/yamlfmt_clean.yaml
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_yaml_format_fail
capability FORMAT
stages 1
stage yamlfmt classes=yaml sources=matrix/yamlfmt_dirty.yaml
completed_rounds 1
convergence STABLE
initial 1
initial WARNING yamlfmt - matrix/yamlfmt_dirty.yaml 0 0 fixable=false "file is not formatted"
terminal 1
terminal WARNING yamlfmt - matrix/yamlfmt_dirty.yaml 0 0 fixable=false "file is not formatted"
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_shell_format_pass
capability FORMAT
stages 1
stage shfmt classes=shell sources=matrix/shfmt_clean.sh
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_shell_format_fail
capability FORMAT
stages 1
stage shfmt classes=shell sources=matrix/shfmt_dirty.sh
completed_rounds 2
convergence STABLE
initial 1
initial WARNING shfmt - matrix/shfmt_dirty.sh 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement matrix/shfmt_dirty.sh 31 37 "fixed"
""",
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
        "expected": """producer //quality/testdata:matrix_ruby_format_pass
capability FORMAT
stages 1
stage standardrb classes=ruby sources=matrix/standardrb_clean.rb
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_ruby_format_fail
capability FORMAT
stages 1
stage standardrb classes=ruby sources=matrix/standardrb_dirty.rb
completed_rounds 2
convergence STABLE
initial 1
initial WARNING standardrb - matrix/standardrb_dirty.rb 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement matrix/standardrb_dirty.rb 5 11 "fixed"
""",
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
        "expected": """producer //quality/testdata:matrix_html_template_format_pass
capability FORMAT
stages 1
stage djlint classes=html_template sources=matrix/djlint_format_clean.html
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_html_template_format_fail
capability FORMAT
stages 1
stage djlint classes=html_template sources=matrix/djlint_format_dirty.html
completed_rounds 2
convergence STABLE
initial 1
initial WARNING djlint - matrix/djlint_format_dirty.html 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement matrix/djlint_format_dirty.html 12 18 "fixed"
""",
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
        "expected": """producer //quality/testdata:matrix_css_format_pass
capability FORMAT
stages 1
stage prettier classes=css sources=matrix/prettier_css_clean.css
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_css_format_fail
capability FORMAT
stages 1
stage prettier classes=css sources=matrix/prettier_css_dirty.css
completed_rounds 2
convergence STABLE
initial 1
initial WARNING prettier - matrix/prettier_css_dirty.css 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement matrix/prettier_css_dirty.css 18 24 "fixed"
""",
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
        "expected": """producer //quality/testdata:matrix_less_format_pass
capability FORMAT
stages 1
stage prettier classes=less sources=matrix/prettier_less_clean.less
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_less_format_fail
capability FORMAT
stages 1
stage prettier classes=less sources=matrix/prettier_less_dirty.less
completed_rounds 2
convergence STABLE
initial 1
initial WARNING prettier - matrix/prettier_less_dirty.less 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement matrix/prettier_less_dirty.less 18 24 "fixed"
""",
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
        "expected": """producer //quality/testdata:matrix_scss_format_pass
capability FORMAT
stages 1
stage prettier classes=scss sources=matrix/prettier_scss_clean.scss
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_scss_format_fail
capability FORMAT
stages 1
stage prettier classes=scss sources=matrix/prettier_scss_dirty.scss
completed_rounds 2
convergence STABLE
initial 1
initial WARNING prettier - matrix/prettier_scss_dirty.scss 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement matrix/prettier_scss_dirty.scss 18 24 "fixed"
""",
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
        "expected": """producer //quality/testdata:matrix_gherkin_format_pass
capability FORMAT
stages 1
stage prettier classes=gherkin sources=matrix/prettier_gherkin_clean.feature
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_gherkin_format_fail
capability FORMAT
stages 1
stage prettier classes=gherkin sources=matrix/prettier_gherkin_dirty.feature
completed_rounds 2
convergence STABLE
initial 1
initial WARNING prettier - matrix/prettier_gherkin_dirty.feature 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement matrix/prettier_gherkin_dirty.feature 14 20 "fixed"
""",
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
        "expected": """producer //quality/testdata:matrix_sql_format_pass
capability FORMAT
stages 1
stage prettier classes=sql sources=matrix/prettier_sql_clean.sql
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_sql_format_fail
capability FORMAT
stages 1
stage prettier classes=sql sources=matrix/prettier_sql_dirty.sql
completed_rounds 2
convergence STABLE
initial 1
initial WARNING prettier - matrix/prettier_sql_dirty.sql 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement matrix/prettier_sql_dirty.sql 9 15 "fixed"
""",
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
        "expected": """producer //quality/testdata:matrix_xml_format_pass
capability FORMAT
stages 1
stage prettier classes=xml sources=matrix/prettier_xml_clean.xml
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_xml_format_fail
capability FORMAT
stages 1
stage prettier classes=xml sources=matrix/prettier_xml_dirty.xml
completed_rounds 2
convergence STABLE
initial 1
initial WARNING prettier - matrix/prettier_xml_dirty.xml 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement matrix/prettier_xml_dirty.xml 12 18 "fixed"
""",
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
        "expected": """producer //quality/testdata:matrix_html_template_lint_pass
capability LINT
stages 1
stage djlint classes=html_template sources=matrix/djlint_clean.html
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_html_template_lint_fail
capability LINT
stages 1
stage djlint classes=html_template sources=matrix/djlint_dirty.html
completed_rounds 1
convergence STABLE
initial 1
initial WARNING djlint H006 matrix/djlint_dirty.html 14 14 fixable=false "img tags require alt text"
terminal 1
terminal WARNING djlint H006 matrix/djlint_dirty.html 14 14 fixable=false "img tags require alt text"
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_css_lint_pass
capability LINT
stages 1
stage stylelint classes=css sources=matrix/stylelint_clean.css
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_css_lint_fail
capability LINT
stages 1
stage stylelint classes=css sources=matrix/stylelint_dirty.css
completed_rounds 1
convergence STABLE
initial 1
initial ERROR stylelint color-no-invalid-hex matrix/stylelint_dirty.css 14 14 fixable=false "Unexpected invalid hex"
terminal 1
terminal ERROR stylelint color-no-invalid-hex matrix/stylelint_dirty.css 14 14 fixable=false "Unexpected invalid hex"
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_less_lint_pass
capability LINT
stages 1
stage stylelint classes=less sources=matrix/stylelint_clean.less
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_less_lint_fail
capability LINT
stages 1
stage stylelint classes=less sources=matrix/stylelint_dirty.less
completed_rounds 1
convergence STABLE
initial 1
initial ERROR stylelint color-no-invalid-hex matrix/stylelint_dirty.less 14 14 fixable=false "Unexpected invalid hex"
terminal 1
terminal ERROR stylelint color-no-invalid-hex matrix/stylelint_dirty.less 14 14 fixable=false "Unexpected invalid hex"
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_scss_lint_pass
capability LINT
stages 1
stage stylelint classes=scss sources=matrix/stylelint_clean.scss
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_scss_lint_fail
capability LINT
stages 1
stage stylelint classes=scss sources=matrix/stylelint_dirty.scss
completed_rounds 1
convergence STABLE
initial 1
initial ERROR stylelint color-no-invalid-hex matrix/stylelint_dirty.scss 14 14 fixable=false "Unexpected invalid hex"
terminal 1
terminal ERROR stylelint color-no-invalid-hex matrix/stylelint_dirty.scss 14 14 fixable=false "Unexpected invalid hex"
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_ruby_lint_pass
capability LINT
stages 1
stage rubocop classes=ruby sources=matrix/rubocop_clean.rb
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_ruby_lint_fail
capability LINT
stages 1
stage rubocop classes=ruby sources=matrix/rubocop_dirty.rb
completed_rounds 1
convergence STABLE
initial 1
initial WARNING rubocop Style/StringLiterals matrix/rubocop_dirty.rb 23 23 fixable=false "Use double quotes"
terminal 1
terminal WARNING rubocop Style/StringLiterals matrix/rubocop_dirty.rb 23 23 fixable=false "Use double quotes"
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_powershell_lint_pass
capability LINT
stages 1
stage psscriptanalyzer classes=powershell sources=matrix/psscriptanalyzer_clean.ps1
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_powershell_lint_fail
capability LINT
stages 1
stage psscriptanalyzer classes=powershell sources=matrix/psscriptanalyzer_dirty.ps1
completed_rounds 1
convergence STABLE
initial 1
initial WARNING psscriptanalyzer PSAvoidUsingWriteHost matrix/psscriptanalyzer_dirty.ps1 51 51 fixable=false "Avoid using Write-Host"
terminal 1
terminal WARNING psscriptanalyzer PSAvoidUsingWriteHost matrix/psscriptanalyzer_dirty.ps1 51 51 fixable=false "Avoid using Write-Host"
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_yaml_lint_pass
capability LINT
stages 1
stage yamllint classes=yaml sources=matrix/yamllint_clean.yaml
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_yaml_lint_fail
capability LINT
stages 1
stage yamllint classes=yaml sources=matrix/yamllint_dirty.yaml
completed_rounds 1
convergence STABLE
initial 1
initial WARNING yamllint trailing-spaces matrix/yamllint_dirty.yaml 11 11 fixable=false "trailing spaces"
terminal 1
terminal WARNING yamllint trailing-spaces matrix/yamllint_dirty.yaml 11 11 fixable=false "trailing spaces"
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_shell_lint_pass
capability LINT
stages 1
stage shellcheck classes=shell sources=matrix/shellcheck_clean.sh
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_shell_lint_fail
capability LINT
stages 1
stage shellcheck classes=shell sources=matrix/shellcheck_dirty.sh
completed_rounds 1
convergence STABLE
initial 1
initial WARNING shellcheck SC2086 matrix/shellcheck_dirty.sh 33 33 fixable=false "Double quote to prevent globbing"
terminal 1
terminal WARNING shellcheck SC2086 matrix/shellcheck_dirty.sh 33 33 fixable=false "Double quote to prevent globbing"
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_text_lint_pass
capability LINT
stages 1
stage keep_sorted classes=text sources=matrix/keep_sorted_clean.txt
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
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
        "expected": """producer //quality/testdata:matrix_text_lint_fail
capability LINT
stages 1
stage keep_sorted classes=text sources=matrix/keep_sorted_dirty.txt
completed_rounds 1
convergence STABLE
initial 1
initial WARNING keep_sorted - matrix/keep_sorted_dirty.txt 6 6 fixable=false "block is not sorted"
terminal 1
terminal WARNING keep_sorted - matrix/keep_sorted_dirty.txt 6 6 fixable=false "block is not sorted"
replacements 0
""",
    },
]
