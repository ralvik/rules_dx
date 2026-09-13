package mdx

import (
	"reflect"
	"strings"
	"testing"
)

// mdxWrap places one ESM body at the top of a minimal MDX document with
// a trailing heading boundary and inert prose.
func mdxWrap(esm string) string {
	return esm + "\n# Demo\n\nContent.\n"
}

func TestParseImports(t *testing.T) {
	cases := []struct {
		name   string
		source string
		want   []string
	}{
		{"empty", "", nil},
		{"proseOnly", "Hello world\n", nil},
		{"proseParagraph", "Some prose\nmore prose\n", nil},
		{"importAtStart", "import helper from \"./helper.mdx\";\n", []string{"helper"}},
		{"importAfterBlank", "Some text\n\nimport helper from \"./helper.mdx\";\n", []string{"helper"}},
		{"importParagraphContinuation", "Some text\nimport helper from \"./helper.mdx\";\n", nil},
		{"importAfterHeading", "# Demo\nimport helper from \"./helper.mdx\";\n", []string{"helper"}},
		{"importAfterThematicBreak", "---\nimport helper from \"./helper.mdx\";\n", []string{"helper"}},
		{"importAfterSetext", "Title\n===\nimport helper from \"./helper.mdx\";\n", []string{"helper"}},
		{"importAfterFence", "```js\nimport fake from \"./fake.mdx\";\n```\nimport helper from \"./helper.mdx\";\n", []string{"helper"}},
		{"fencedCodeInert", "```js\nimport fake from \"./fake.mdx\";\n```\n", nil},
		{"tildeFenceInert", "~~~js\nimport fake from \"./fake.mdx\";\n~~~\n", nil},
		{"indentedImportInert", "    import helper from \"./helper.mdx\";\n", nil},
		{"blockquoteInert", "> import helper from \"./helper.mdx\";\n", nil},
		{"listInert", "- import helper from \"./helper.mdx\";\n", nil},
		{"singleLineHtmlInert", "<div>import helper from \"./helper.mdx\";</div>\n", nil},
		{"htmlCommentInlineInert", "<!-- import helper from \"./helper.mdx\"; -->\n", nil},
		{"htmlCommentBlockInert", "<!--\nimport helper from \"./helper.mdx\";\n-->\n", nil},
		{"unterminatedHtmlCommentWholeDocInert", "import a from \"./a.mdx\";\n<!-- unterminated\nimport b from \"./b.mdx\";\n", nil},
		{"jsxInert", "<Component prop=\"import './fake.mdx'\" />\n", nil},
		{"sideEffect", "import \"./hello.mdx\";\n", []string{"hello"}},
		{"sideEffectSingle", "import './helper';\n", []string{"helper"}},
		{"default", "import hello from \"./hello.mdx\";\n", []string{"hello"}},
		{"named", "import {a, b} from './util.mjs';\n", []string{"util"}},
		{"namespace", "import * as ns from \"../pkg/demo.mdx\";\n", []string{"demo"}},
		{"bare", "import React from \"react\";\n", []string{"react"}},
		{"mdxBare", "import { compile } from \"@mdx-js/mdx\";\n", []string{"@mdx-js/mdx"}},
		{"scoped", "import x from \"@scope/pkg/sub\";\n", []string{"@scope/pkg/sub"}},
		{"scopedRoot", "import x from \"@scope/pkg\";\n", []string{"@scope/pkg"}},
		{"subpath", "import x from \"pkg/subpath\";\n", []string{"pkg/subpath"}},
		{"nodeBuiltin", "import fs from \"fs\";\n", []string{"fs"}},
		{"nodePrefix", "import fs from \"node:fs\";\n", []string{"node:fs"}},
		{"exportNamed", "export {a} from './helper.mdx';\n", []string{"helper"}},
		{"exportStar", "export * from \"./demo.mdx\";\n", []string{"demo"}},
		{"exportStarAs", "export * as ns from '../lib.mdx';\n", []string{"lib"}},
		{"exportLocal", "export const x = 1;\n", nil},
		// In MDX only column-zero `import`/`export` openers start a chunk;
		// `const ... import()`/`require()` lines are prose and stay inert.
		// Bare `import("...")` as a statement is an opener and contributes.
		{"dynamicConstInert", "const m = await import(\"./lazy.mdx\");\n", nil},
		{"dynamicSingle", "import('./other.mjs');\n", []string{"other"}},
		{"dynamicBare", "import(\"react\");\n", []string{"react"}},
		{"requireConstInert", "const x = require(\"./data.cjs\");\n", nil},
		{"requireBareLineInert", "require('fs');\n", nil},
		{"requireComputed", "require(name);\n", nil},
		{"dynamicComputed", "import(name);\n", nil},
		{"templateDynamic", "import(`./lazy.mdx`);\n", nil},
		{"importMeta", "const u = import.meta.url;\n", nil},
		{"methodRequire", "obj.require(\"./fake.mdx\");\n", nil},
		{"noImports", "const x = 1;\n", nil},
		{"lineComment", "// import foo from \"bar\";\n", nil},
		{"blockComment", "/* import \"./hidden.mdx\"; */\nconst x = 1;\n", nil},
		{"trailingComment", "import a from \"./real.mdx\"; // import \"./fake.mdx\";\n", []string{"real"}},
		// A prose line absorbs a following opener (lazy continuation), so
		// string/template/regex/division leaders keep the next import inert.
		{"doubleStringProse", "\"import './fake.mdx'\";\nimport y from './real.mdx';\n", nil},
		{"singleString", "'require(\"./fake.mdx\")';\n", nil},
		{"templateInert", "`import './fake.mdx'`;\n", nil},
		{"regexInert", "const re = /import './fake.mdx'/;\nconst x = 1;\n", nil},
		{"unterminatedProse", "\"abc\nimport y from './real.mdx';\n", nil},
		{"relativeNoExt", "import x from './helper';\n", []string{"helper"}},
		{"relativeDir", "import x from './dir/';\n", []string{"dir"}},
		// Multi-line brace bodies are outside the narrow single-line subset:
		// only the opener line is collected, yielding no edge (fail-closed).
		{"multilineNarrow", "import {\n a,\n b\n} from './multi.mdx';\n", nil},
		{"exportMultilineNarrow", "export {\na\n} from \"./shared.mdx\";\n", nil},
		{"duplicate", "import a from './same.mdx';\nimport b from './same.mdx';\n", []string{"same"}},
		{"emptySpec", "import \"\";\n", nil},
		{"unterminatedBlock", "/* import \"./hidden.mdx\";", nil},
		{"dynamicMissingParen", "import(\"a\";\n", nil},
		{"dynamicUnterminated", "import(\"abc\n", nil},
		{"dynamicOpenEOF", "import(", nil},
		{"importShortTail", "import typ", nil},
		{"importBareEOF", "import", nil},
		{"sideEffectUnterminated", "import \"abc\n", nil},
		{"staticStrayString", "import foo \"bar\";\n", nil},
		{"staticTemplate", "import x `tmpl`;\n", nil},
		{"staticSemicolon", "import foo;\n", nil},
		{"staticEOF", "import foo", nil},
		{"fromUnterminated", "import foo from \"abc\n", nil},
		{"fromNonQuote", "import foo from bar;\n", nil},
		{"exportStrayString", "export \"x\";\n", nil},
		{"exportTemplate", "export `tmpl`;\n", nil},
		{"exportNewlineFromNarrow", "export {a}\nfrom './helper.mdx';\n", nil},
		{"exportNewlineNoFrom", "export const x = 1\n", nil},
		{"exportFromUnterminated", "export {a} from \"abc\n", nil},
		{"exportFromNonQuote", "export {a} from bar;\n", nil},
		{"exportEOF", "export", nil},
		{"exportBraceEOF", "export {", nil},
		{"requireNoParen", "require;\n", nil},
		{"requireEOF", "require", nil},
		{"requireMissingParen", "require(\"a\";\n", nil},
		{"requireUnterminated", "require(\"abc\n", nil},
		{"triviaLineCommentNarrow", "import //c\n\"./a.mdx\";\n", nil},
		{"triviaBlockComment", "import /*c*/ \"./a.mdx\";\n", []string{"a"}},
		{"triviaUnterminatedBlock", "import /* unterminated", nil},
		{"importBraceComment", "import {a /*c*/} from './real.mdx';\n", []string{"real"}},
		{"exportBraceComment", "export {a /*c*/} from './real.mdx';\n", []string{"real"}},
		{"inertEscapeProse", "\"a\\nb\";\nimport y from './real.mdx';\n", nil},
		{"templateInterpObjectProse", "`outer ${ {a: 1} } inner`;\nimport y from './real.mdx';\n", nil},
		{"quotedEscape", "import \"a\\\"b\";\n", []string{"a\"b"}},
		{"quotedBackslashEOF", "import \"abc\\", nil},
		{"quotedEOF", "import \"abc", nil},
		{"templateEscapeProse", "`a\\nb`;\nimport y from './real.mdx';\n", nil},
		{"templateInterpProse", "`outer ${x} inner`;\nimport y from './real.mdx';\n", nil},
		{"templateInterpQuoteProse", "`outer ${'x'} inner`;\nimport y from './real.mdx';\n", nil},
		{"templateInterpNestedProse", "`outer ${`inner`} end`;\nimport y from './real.mdx';\n", nil},
		{"templateUnterminated", "`abc", nil},
		{"regexAtStartProse", "/abc/;\nimport y from './real.mdx';\n", nil},
		{"divisionProse", "const x = a / b;\nimport y from './real.mdx';\n", nil},
		{"divisionParenProse", "const x = (a) / b;\nimport y from './real.mdx';\n", nil},
		{"regexEscapeProse", "const re = /a\\/b/;\nimport y from './real.mdx';\n", nil},
		{"regexNewlineProse", "const re = /abc\nimport y from './real.mdx';\n", nil},
		{"regexClassProse", "const re = /[a/b]/;\nimport y from './real.mdx';\n", nil},
		{"regexEOF", "const re = /abc", nil},
	}
	// Markdown-structure cases exercise full documents directly; JS-edge
	// cases are single ESM bodies wrapped in a minimal MDX document.
	raw := map[string]bool{
		"empty": true, "proseOnly": true, "proseParagraph": true,
		"importAtStart": true, "importAfterBlank": true,
		"importParagraphContinuation": true, "importAfterHeading": true,
		"importAfterThematicBreak": true, "importAfterSetext": true,
		"importAfterFence": true, "fencedCodeInert": true,
		"tildeFenceInert": true, "indentedImportInert": true,
		"blockquoteInert": true, "listInert": true,
		"singleLineHtmlInert": true, "htmlCommentInlineInert": true,
		"htmlCommentBlockInert": true,
		"unterminatedHtmlCommentWholeDocInert": true,
		"jsxInert": true,
	}
	for _, tc := range cases {
		var content []byte
		if raw[tc.name] {
			content = []byte(tc.source)
		} else {
			content = []byte(mdxWrap(tc.source))
		}
		if got := ParseImports(content); !reflect.DeepEqual(got, tc.want) {
			t.Errorf("%s: ParseImports = %q, want %q", tc.name, got, tc.want)
		}
	}
}

func TestExtractESMRegions(t *testing.T) {
	cases := []struct {
		name string
		source string
		want []string
	}{
		{"empty", "", nil},
		{"proseOnly", "Hello\n", nil},
		{"singleImport", "import a from \"./a.mdx\";\n", []string{"import a from \"./a.mdx\";\n"}},
		{"blankSeparated", "import a from \"./a.mdx\";\n\nimport b from \"./b.mdx\";\n", []string{"import a from \"./a.mdx\";\n", "import b from \"./b.mdx\";\n"}},
		{"paragraphContinuationInert", "text\nimport a from \"./a.mdx\";\n", nil},
		{"headingBoundary", "# T\nimport a from \"./a.mdx\";\n", []string{"import a from \"./a.mdx\";\n"}},
		{"thematicBoundary", "---\nimport a from \"./a.mdx\";\n", []string{"import a from \"./a.mdx\";\n"}},
		{"fenceInert", "```js\nimport a from \"./a.mdx\";\n```\n", nil},
		// Narrow subset: only the opener line is kept when braces span lines.
		{"multilineBracesNarrow", "import {\n a\n} from \"./a.mdx\";\n", []string{"import {\n"}},
		{"commentWholeDocInert", "import a from \"./a.mdx\";\n<!-- oops\nimport b from \"./b.mdx\";\n", nil},
	}
	for _, tc := range cases {
		got := ExtractESMRegions([]byte(tc.source))
		if tc.want == nil {
			if got != nil {
				t.Errorf("%s: ExtractESMRegions = %q, want nil", tc.name, got)
			}
			continue
		}
		if len(got) != len(tc.want) {
			t.Errorf("%s: ExtractESMRegions = %q, want %q", tc.name, got, tc.want)
			continue
		}
		for i := range got {
			if strings.TrimSpace(string(got[i])) != strings.TrimSpace(tc.want[i]) {
				t.Errorf("%s: region[%d] = %q, want %q", tc.name, i, got[i], tc.want[i])
			}
		}
	}
}

func TestSkipQuotedEOF(t *testing.T) {
	if got := skipQuoted([]byte("\"abc"), 0); got != 4 {
		t.Errorf("skipQuoted EOF = %d, want 4", got)
	}
}

func TestSkipTemplateEOF(t *testing.T) {
	if got := skipTemplate([]byte("`abc"), 0); got != 4 {
		t.Errorf("skipTemplate EOF = %d, want 4", got)
	}
}

func TestIsStdLib(t *testing.T) {
	for _, name := range []string{"fs", "path", "node:fs", "node:path", "test", "node:test"} {
		if !IsStdLib(name) {
			t.Errorf("stdlib identity not recognized: %q", name)
		}
	}
	for _, name := range []string{"helper", "", "@mdx-js/mdx", "fs-extra", "react", "fs/promises/extra"} {
		if IsStdLib(name) {
			t.Errorf("non-stdlib identity recognized as stdlib: %q", name)
		}
	}
	if !IsStdLib("fs/promises") {
		t.Error("stdlib subpath fs/promises not recognized")
	}
	if IsStdLib("node:") {
		t.Error("empty node: identity recognized as stdlib")
	}
}

func TestNormalizeSpec(t *testing.T) {
	cases := []struct{ in, want string }{
		{"./hello.mdx", "hello"},
		{"../pkg/demo.mjsx", "demo"},
		{"./helper", "helper"},
		{"./dir/", "dir"},
		{"/abs/path.mdx", "path"},
		{"@mdx-js/mdx", "@mdx-js/mdx"},
		{"@scope/pkg/sub", "@scope/pkg/sub"},
		{"node:fs", "node:fs"},
		{"", ""},
		{"./", ""},
		{"   ", ""},
	}
	for _, tc := range cases {
		if got := normalizeSpec(tc.in); got != tc.want {
			t.Errorf("normalizeSpec(%q) = %q, want %q", tc.in, got, tc.want)
		}
	}
}
