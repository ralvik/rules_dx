package javascript

import (
	"reflect"
	"testing"
)

func TestParseImports(t *testing.T) {
	cases := []struct {
		name   string
		source string
		want   []string
	}{
		{"empty", "", nil},
		{"sideEffect", "import \"./hello.js\";\n", []string{"hello"}},
		{"sideEffectSingle", "import './helper';\n", []string{"helper"}},
		{"default", "import hello from \"./hello.js\";\n", []string{"hello"}},
		{"named", "import {a, b} from './util.mjs';\n", []string{"util"}},
		{"namespace", "import * as ns from \"../pkg/demo.jsx\";\n", []string{"demo"}},
		{"bare", "import React from \"react\";\n", []string{"react"}},
		{"scoped", "import x from \"@scope/pkg/sub\";\n", []string{"@scope/pkg/sub"}},
		{"scopedRoot", "import x from \"@scope/pkg\";\n", []string{"@scope/pkg"}},
		{"subpath", "import x from \"pkg/subpath\";\n", []string{"pkg/subpath"}},
		{"nodeBuiltin", "import fs from \"fs\";\n", []string{"fs"}},
		{"nodePrefix", "import fs from \"node:fs\";\n", []string{"node:fs"}},
		{"exportNamed", "export {a} from './helper.js';\n", []string{"helper"}},
		{"exportStar", "export * from \"./demo.cjs\";\n", []string{"demo"}},
		{"exportStarAs", "export * as ns from '../lib.js';\n", []string{"lib"}},
		{"exportLocal", "export const x = 1;\n", nil},
		{"dynamic", "const m = await import(\"./lazy.js\");\n", []string{"lazy"}},
		{"dynamicSingle", "import('./other.mjs');\n", []string{"other"}},
		{"dynamicBare", "import(\"react\");\n", []string{"react"}},
		{"require", "const x = require(\"./data.cjs\");\n", []string{"data"}},
		{"requireBare", "require('fs');\n", []string{"fs"}},
		{"requireComputed", "require(name);\n", nil},
		{"dynamicComputed", "import(name);\n", nil},
		{"templateDynamic", "import(`./lazy.js`);\n", nil},
		{"importMeta", "const u = import.meta.url;\n", nil},
		{"methodRequire", "obj.require(\"./fake.js\");\n", nil},
		{"noImports", "const x = 1;\n", nil},
		{"lineComment", "// import foo from \"bar\";\n", nil},
		{"blockComment", "/* import \"./hidden.js\"; */\nconst x = 1;\n", nil},
		{"trailingComment", "import a from \"./real.js\"; // import \"./fake.js\";\n", []string{"real"}},
		{"doubleString", "\"import './fake.js'\";\nimport y from './real.js';\n", []string{"real"}},
		{"singleString", "'require(\"./fake.js\")';\n", nil},
		{"templateInert", "`import './fake.js'`;\n", nil},
		{"regexInert", "const re = /import './fake.js'/;\nconst x = 1;\n", nil},
		{"unterminated", "\"abc\nimport y from './real.js';\n", []string{"real"}},
		{"relativeNoExt", "import x from './helper';\n", []string{"helper"}},
		{"relativeDir", "import x from './dir/';\n", []string{"dir"}},
		{"multiline", "import {\n a,\n b\n} from './multi.js';\n", []string{"multi"}},
		{"exportMultiline", "export {\na\n} from \"./shared.js\";\n", []string{"shared"}},
		{"duplicate", "import a from './same.js';\nimport b from './same.js';\n", []string{"same"}},
		{"emptySpec", "import \"\";\n", nil},
	}
	for _, tc := range cases {
		if got := ParseImports([]byte(tc.source)); !reflect.DeepEqual(got, tc.want) {
			t.Errorf("%s: ParseImports = %q, want %q", tc.name, got, tc.want)
		}
	}
}

func TestIsStdLib(t *testing.T) {
	for _, name := range []string{"fs", "path", "node:fs", "node:path", "test", "node:test"} {
		if !IsStdLib(name) {
			t.Errorf("stdlib identity not recognized: %q", name)
		}
	}
	for _, name := range []string{"helper", "", "fs-extra", "react", "fs/promises/extra"} {
		if IsStdLib(name) {
			t.Errorf("non-stdlib identity recognized as stdlib: %q", name)
		}
	}
	if !IsStdLib("fs/promises") {
		t.Error("stdlib subpath fs/promises not recognized")
	}
}

func TestNormalizeSpec(t *testing.T) {
	cases := []struct{ in, want string }{
		{"./hello.js", "hello"},
		{"../pkg/demo.mjsx", "demo"},
		{"./helper", "helper"},
		{"./dir/", "dir"},
		{"/abs/path.cjs", "path"},
		{"react", "react"},
		{"@scope/pkg/sub", "@scope/pkg/sub"},
		{"node:fs", "node:fs"},
		{"", ""},
		{"./", ""},
	}
	for _, tc := range cases {
		if got := normalizeSpec(tc.in); got != tc.want {
			t.Errorf("normalizeSpec(%q) = %q, want %q", tc.in, got, tc.want)
		}
	}
}
