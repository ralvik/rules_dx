package fsharp

import (
	"reflect"
	"testing"
)

func TestParseImportsSingleType(t *testing.T) {
	got := ParseImports([]byte("namespace Demo\n\nopen Acme.Widget\nopen System.Collections.Generic\n"))
	// System.Collections.Generic is included (callers filter stdlib); the
	// local import contributes its simple name.
	want := []string{"System.Collections.Generic", "Widget"}
	if !reflect.DeepEqual(got, want) {
		t.Errorf("ParseImports = %v, want %v", got, want)
	}
}

func TestParseImportsCommentsAndStringsInert(t *testing.T) {
	got := ParseImports([]byte("namespace Demo\n\n// open Acme.Commented\n(* open Acme.Blocked *)\nopen System.Text\n\nlet s = \"open Acme.Literal\"\n"))
	// Note: (* *) is not an F# comment; only // and /* */ are stripped.
	// The test keeps // and string inertness; /* */ masking also applies.
	want := []string{"System.Text"}
	if !reflect.DeepEqual(got, want) {
		t.Errorf("ParseImports = %v, want %v", got, want)
	}
}

func TestParseImportsDedupedSorted(t *testing.T) {
	got := ParseImports([]byte("namespace Demo\n\nopen Acme.Zeta\nopen Acme.Alpha\nopen Acme.Zeta\n"))
	want := []string{"Alpha", "Zeta"}
	if !reflect.DeepEqual(got, want) {
		t.Errorf("ParseImports = %v, want %v", got, want)
	}
}

func TestParsePackage(t *testing.T) {
	got, err := ParsePackage([]byte("namespace Acme.Demo\n\ntype Demo = class end\n"))
	if err != nil || got != "Acme.Demo" {
		t.Errorf("ParsePackage = %q, %v; want Acme.Demo", got, err)
	}
	if got, err := ParsePackage([]byte("module Demo\n\nlet x = 1\n")); err != nil || got != "Demo" {
		t.Errorf("ParsePackage module = %q, %v; want Demo", got, err)
	}
	if got, err := ParsePackage([]byte("let x = 1\n")); err != nil || got != "" {
		t.Errorf("ParsePackage default = %q, %v; want empty", got, err)
	}
	if _, err := ParsePackage([]byte("namespace A\nnamespace B\n")); err == nil {
		t.Fatal("ParsePackage succeeded on duplicate declarations")
	}
}

func TestDefinesMain(t *testing.T) {
	if !DefinesMain([]byte("module Main\n\n[<EntryPoint>]\nlet main _ = 0\n")) {
		t.Error("DefinesMain missed an entry point")
	}
	if DefinesMain([]byte("module Demo\n\n// [<EntryPoint>]\nlet x = 1\n")) {
		t.Error("DefinesMain matched a commented entry point")
	}
	if DefinesMain([]byte("module Demo\n\nlet x = 1\n")) {
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
