package csharp

import (
	"reflect"
	"testing"
)

func TestParseImportsSingleType(t *testing.T) {
	got := ParseImports([]byte("namespace Demo;\n\nusing Acme.Widget;\nusing System.Collections.Generic;\n"))
	// System.Collections.Generic is included (callers filter stdlib); the
	// local import contributes its simple class name.
	want := []string{"System.Collections.Generic", "Widget"}
	if !reflect.DeepEqual(got, want) {
		t.Errorf("ParseImports = %v, want %v", got, want)
	}
}

func TestParseImportsAliasAndStatic(t *testing.T) {
	got := ParseImports([]byte("namespace Demo;\n\nusing static System.Math;\nusing W = Acme.Widget;\n"))
	want := []string{"System.Math", "Widget"}
	if !reflect.DeepEqual(got, want) {
		t.Errorf("ParseImports = %v, want %v", got, want)
	}
}

func TestParseImportsCommentsAndStringsInert(t *testing.T) {
	got := ParseImports([]byte("namespace Demo;\n\n// using Acme.Commented;\n/* using Acme.Blocked; */\nusing System.Text;\n\nvar s = \"using Acme.Literal;\";\n"))
	want := []string{"System.Text"}
	if !reflect.DeepEqual(got, want) {
		t.Errorf("ParseImports = %v, want %v", got, want)
	}
}

func TestParseImportsDedupedSorted(t *testing.T) {
	got := ParseImports([]byte("namespace Demo;\n\nusing Acme.Zeta;\nusing Acme.Alpha;\nusing Acme.Zeta;\n"))
	want := []string{"Alpha", "Zeta"}
	if !reflect.DeepEqual(got, want) {
		t.Errorf("ParseImports = %v, want %v", got, want)
	}
}

func TestParsePackage(t *testing.T) {
	got, err := ParsePackage([]byte("namespace Acme.Demo;\n\nclass Demo {}\n"))
	if err != nil || got != "Acme.Demo" {
		t.Errorf("ParsePackage = %q, %v; want Acme.Demo", got, err)
	}
	if got, err := ParsePackage([]byte("namespace Demo;\n\nclass Demo {}\n")); err != nil || got != "Demo" {
		t.Errorf("ParsePackage file-scoped = %q, %v; want Demo", got, err)
	}
	if got, err := ParsePackage([]byte("class Demo {}\n")); err != nil || got != "" {
		t.Errorf("ParsePackage default = %q, %v; want empty", got, err)
	}
	if _, err := ParsePackage([]byte("namespace A {}\nnamespace B {}\n")); err == nil {
		t.Fatal("ParsePackage succeeded on duplicate namespace declarations")
	}
}

func TestDefinesMain(t *testing.T) {
	if !DefinesMain([]byte("namespace Demo;\n\nclass Main {\n  static void Main(string[] args) {}\n}\n")) {
		t.Error("DefinesMain missed a main function")
	}
	if DefinesMain([]byte("namespace Demo;\n\nclass Demo {\n  // static void Main()\n}\n")) {
		t.Error("DefinesMain matched a commented main")
	}
	if DefinesMain([]byte("namespace Demo;\n\nclass Demo {\n  void Hello() {}\n}\n")) {
		t.Error("DefinesMain matched a library source")
	}
}

func TestIsStdLib(t *testing.T) {
	for _, imp := range []string{"System.Collections.Generic", "System.Text", "Microsoft.Extensions.Logging", "System", "Microsoft"} {
		if !IsStdLib(imp) {
			t.Errorf("IsStdLib(%q) = false, want true", imp)
		}
	}
	for _, imp := range []string{"Acme.Widget", "Widget"} {
		if IsStdLib(imp) {
			t.Errorf("IsStdLib(%q) = true, want false", imp)
		}
	}
}
