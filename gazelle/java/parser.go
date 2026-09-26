package java

import (
	"path"
	"regexp"
	"sort"
	"strings"
)

var (
	importRe  = regexp.MustCompile(`(?m)^\s*import\s+(?:static\s+)?([A-Za-z_][\w]*(?:\.[\w]+)*)(\.\*)?\s*;`)
	packageRe = regexp.MustCompile(`(?m)^\s*package\s+([A-Za-z_][\w]*(?:\.[\w]+)*)\s*;`)
	mainRe    = regexp.MustCompile(`static\s+void\s+main\s*\(`)
)

func stripNonCode(content []byte) []byte {
	s := string(content)
	out := make([]byte, len(s))
	copy(out, s)
	mask := func(from, to int) {
		for i := from; i < to; i++ {
			if out[i] != '\n' {
				out[i] = ' '
			}
		}
	}
	i := 0
	for i < len(out) {
		c := out[i]
		switch {
		case c == '/' && i+1 < len(out) && out[i+1] == '/':
			j := i + 2
			for j < len(out) && out[j] != '\n' {
				j++
			}
			mask(i, j)
			i = j
		case c == '/' && i+1 < len(out) && out[i+1] == '*':
			j := i + 2
			for j+1 < len(out) && !(out[j] == '*' && out[j+1] == '/') {
				j++
			}
			if j+1 < len(out) {
				j += 2
			} else {
				j = len(out)
			}
			mask(i, j)
			i = j
		case c == '"' && i+2 < len(out) && out[i+1] == '"' && out[i+2] == '"':
			rest := strings.Index(string(out[i+3:]), `"""`)
			if rest < 0 {
				mask(i, len(out))
				i = len(out)
			} else {
				mask(i, i+3+rest+3)
				i = i + 3 + rest + 3
			}
		case c == '"' || c == '\'':
			quote := c
			j := i + 1
			closed := false
			for j < len(out) {
				if out[j] == '\\' {
					j += 2
					continue
				}
				if out[j] == quote {
					closed = true
					j++
					break
				}
				if quote == '"' && out[j] == '\n' {
					break
				}
				j++
			}
			if closed {
				mask(i, j)
				i = j
			} else {
				k := i
				for k < len(out) && out[k] != '\n' {
					k++
				}
				mask(i, k)
				i = k
			}
		default:
			i++
		}
	}
	return out
}

func ParseImports(content []byte) []string {
	stripped := stripNonCode(content)
	set := make(map[string]struct{})
	for _, m := range importRe.FindAllSubmatch(stripped, -1) {
		dotted := string(m[1])
		onDemand := len(m[2]) > 0
		if id := normalizeImport(dotted, onDemand); id != "" {
			set[id] = struct{}{}
		}
	}
	out := make([]string, 0, len(set))
	for name := range set {
		out = append(out, name)
	}
	sort.Strings(out)
	return out
}

func ParsePackage(content []byte) (string, error) {
	stripped := stripNonCode(content)
	matches := packageRe.FindAllSubmatch(stripped, -1)
	if len(matches) > 1 {
		return "", errDuplicatePackage(string(matches[0][1]))
	}
	if len(matches) == 0 {
		return "", nil
	}
	return string(matches[0][1]), nil
}

type duplicatePackageError struct{ first string }

func (e *duplicatePackageError) Error() string {
	return "java: duplicate package clause " + e.first
}

func errDuplicatePackage(first string) error { return &duplicatePackageError{first: first} }

func normalizeImport(dotted string, onDemand bool) string {
	dotted = strings.TrimSpace(dotted)
	if dotted == "" {
		return ""
	}
	if IsStdLib(dotted) {
		return dotted
	}
	if onDemand {
		return path.Base(strings.ReplaceAll(dotted, ".", "/"))
	}
	if i := strings.LastIndexByte(dotted, '.'); i >= 0 {
		return dotted[i+1:]
	}
	return dotted
}

func DefinesMain(content []byte) bool {
	return mainRe.Match(stripNonCode(content))
}
