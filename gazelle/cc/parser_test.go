package cc

import (
	"reflect"
	"testing"
)

func TestParseQuotedIncludesBasename(t *testing.T) {
	got := ParseQuotedIncludes([]byte("#include <vector>\n#include \"cc/tests/fixtures/hello/hello.h\"\n"))
	want := []string{"hello.h"}
	if !reflect.DeepEqual(got, want) {
		t.Errorf("ParseQuotedIncludes = %v, want %v", got, want)
	}
}

func TestParseQuotedIncludesAngleInert(t *testing.T) {
	if got := ParseQuotedIncludes([]byte("#include <vector>\n#include <string>\n")); len(got) != 0 {
		t.Errorf("ParseQuotedIncludes angle-only = %v, want empty", got)
	}
}

func TestParseQuotedIncludesCommentsAndStringsInert(t *testing.T) {
	content := "// #include \"fake/comment.h\"\n/* #include \"fake/block.h\" */\n#include <string>\nconst char* s = \"#include \\\"fake/string.h\\\"\";\nchar c = '\"';\n#include \"real/kept.h\"\n"
	got := ParseQuotedIncludes([]byte(content))
	want := []string{"kept.h"}
	if !reflect.DeepEqual(got, want) {
		t.Errorf("ParseQuotedIncludes = %v, want %v", got, want)
	}
}

func TestParseQuotedIncludesDedupedSorted(t *testing.T) {
	content := "#include \"zeta/z.h\"\n#include \"alpha/a.h\"\n#include \"zeta/z.h\"\n"
	got := ParseQuotedIncludes([]byte(content))
	want := []string{"a.h", "z.h"}
	if !reflect.DeepEqual(got, want) {
		t.Errorf("ParseQuotedIncludes = %v, want %v", got, want)
	}
}

func TestDefinesMain(t *testing.T) {
	if !DefinesMain([]byte("#include <iostream>\nint main() { return 0; }\n")) {
		t.Error("DefinesMain missed a main definition")
	}
	if !DefinesMain([]byte("int\nmain (int argc, char** argv) { return 0; }\n")) {
		t.Error("DefinesMain missed a split-line main")
	}
	for _, content := range []string{
		"int domain() { return 0; }\n",
		"// int main() {}\n",
		"/* int main() {} */\n",
		"const char* s = \"int main() {}\";\n",
		"int Add(int a, int b) { return a + b; }\n",
	} {
		if DefinesMain([]byte(content)) {
			t.Errorf("DefinesMain(%q) = true, want false", content)
		}
	}
}
