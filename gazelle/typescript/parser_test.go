package typescript

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
		{"sideEffect", "import \"./hello.ts\";\n", []string{"hello"}},
		{"default", "import hello from \"./hello.ts\";\n", []string{"hello"}},
		{"named", "import {a} from './util.tsx';\n", []string{"util"}},
		{"typeImport", "import type {A} from './types.ts';\n", []string{"types"}},
		{"bare", "import React from \"react\";\n", []string{"react"}},
		{"nodeBuiltin", "import fs from \"fs\";\n", []string{"fs"}},
		{"exportNamed", "export {a} from './helper.ts';\n", []string{"helper"}},
		{"exportType", "export type {A} from \"./types.mts\";\n", []string{"types"}},
		{"exportStar", "export * from \"./demo.cts\";\n", []string{"demo"}},
		{"exportLocal", "export interface A { x: number }\n", nil},
		{"exportDecl", "export declare const x: number;\n", nil},
		{"dynamic", "const m = await import(\"./lazy.ts\");\n", []string{"lazy"}},
		{"require", "const x = require(\"./data.cjs\");\n", []string{"data"}},
		{"importMeta", "const u = import.meta.url;\n", nil},
		{"lineComment", "// import foo from \"bar\";\n", nil},
		{"relativeNoExt", "import x from './helper';\n", []string{"helper"}},
		{"duplicate", "import a from './same.ts';\nimport b from './same.ts';\n", []string{"same"}},
		{"multilineExport", "export {\na\n} from \"./shared.ts\";\n", []string{"shared"}},
	}
	for _, tc := range cases {
		if got := ParseImports([]byte(tc.source)); !reflect.DeepEqual(got, tc.want) {
			t.Errorf("%s: ParseImports = %q, want %q", tc.name, got, tc.want)
		}
	}
}

func TestIsStdLib(t *testing.T) {
	for _, name := range []string{"fs", "path", "node:fs", "test", "node:test"} {
		if !IsStdLib(name) {
			t.Errorf("stdlib identity not recognized: %q", name)
		}
	}
	for _, name := range []string{"helper", "", "react", "fs-extra"} {
		if IsStdLib(name) {
			t.Errorf("non-stdlib recognized as stdlib: %q", name)
		}
	}
	if !IsStdLib("fs/promises") {
		t.Error("stdlib subpath fs/promises not recognized")
	}
}

func TestNormalizeSpec(t *testing.T) {
	cases := []struct{ in, want string }{
		{"./hello.ts", "hello"},
		{"../pkg/demo.tsx", "demo"},
		{"react", "react"},
		{"node:fs", "node:fs"},
		{"", ""},
	}
	for _, tc := range cases {
		if got := normalizeSpec(tc.in); got != tc.want {
			t.Errorf("normalizeSpec(%q) = %q, want %q", tc.in, got, tc.want)
		}
	}
}
