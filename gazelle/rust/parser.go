// Parser extracts the narrow recognized source facts the Rust Gazelle
// extension needs for crate ownership and strict dependency resolution.
//
// Recognized syntax (M09 narrow scope; additional forms require parser
// fixtures before they become recognized):
//
//   - `mod name;` and `pub mod name;` file modules, including explicit
//     `#[path = "..."]` modules. Inline `mod name { ... }` nests inside
//     the same crate owner.
//   - `use` declarations with literal paths, including `{a, b}` groups,
//     `as` aliases, and `self`/`super`/`crate` segments. Glob `*` imports
//     are recorded without expansion.
//   - `extern crate name;` with optional `as` alias.
//   - `#[test]` functions and `#[cfg(test)]` regions, which mark the
//     crate unit-test target and test-only dependency scope.
//
// Comments, string and character literals, and unexpanded `macro_rules!`
// bodies are inert: tokens that look like markers inside them never
// trigger a unit-test target or an edge. A `#[cfg]` predicate that
// mentions `test` without being exactly `test` (including `cfg_attr`
// forms) is an ambiguous parser context and fails closed instead of
// widening production scope or dropping an edge.
package rust

import (
	"fmt"
	"strings"
)

// ModuleDecl is one recognized `mod` item.
type ModuleDecl struct {
	// Name is the declared module name.
	Name string
	// Path is the explicit `#[path]` override, empty when absent.
	Path string
	// Inline is true for `mod name { ... }`, which nests in the same
	// crate owner instead of loading a file module.
	Inline bool
	// CfgTest is true when the module is test-only.
	CfgTest bool
}

// UseDecl is one expanded literal `use` identity.
type UseDecl struct {
	// Path is the fully expanded literal path (`a::b::Trait`).
	Path string
	// Glob is true for `prefix::*` imports, recorded without expansion.
	Glob bool
	// CfgTest is true when the import is test-only.
	CfgTest bool
}

// ExternCrate is one recognized `extern crate` item.
type ExternCrate struct {
	// Name is the declared crate name.
	Name string
	// As is the `as` alias, empty when absent.
	As string
	// CfgTest is true when the item is test-only.
	CfgTest bool
}

// FileFacts are the recognized facts for one parsed Rust source file.
type FileFacts struct {
	Modules []ModuleDecl
	Uses    []UseDecl
	Externs []ExternCrate
	// HasTestAttr is true when a top-level `#[test]` function was parsed.
	HasTestAttr bool
	// HasCfgTest is true when a top-level `#[cfg(test)]` region was parsed.
	HasCfgTest bool
	// InnerCfgTest is true for a leading `#![cfg(test)]` file attribute.
	InnerCfgTest bool
}

// AmbiguousError reports a parser context that mentions `test` without
// being exactly `#[cfg(test)]`. Generation fails instead of guessing
// which target owns the item or its edges.
type AmbiguousError struct {
	What string
	Attr string
}

func (e *AmbiguousError) Error() string {
	return fmt.Sprintf("rust: ambiguous test context on %s: %s is not exactly #[cfg(test)]; disambiguate with #[cfg(test)] or move the item", e.What, e.Attr)
}

// ParseFacts parses one Rust source file. The path is used for diagnostics
// only; content is the file bytes.
func ParseFacts(path string, content []byte) (*FileFacts, error) {
	p := &parser{path: path, src: cleanSource(string(content))}
	if err := p.parseItems(0, false, &p.facts); err != nil {
		return nil, err
	}
	return &p.facts, nil
}

type parser struct {
	path  string
	src   string
	pos   int
	facts FileFacts
}

// cleanSource blanks comments, string and character literals, and
// `macro_rules!`/`macro` bodies with spaces (newlines preserved) so the
// item scanner only sees real code.
func cleanSource(s string) string {
	out := []byte(s)
	n := len(out)
	i := 0
	attrDepth := 0
	blank := func(from, to int) {
		for k := from; k < to; k++ {
			if out[k] != '\n' {
				out[k] = ' '
			}
		}
	}
	for i < n {
		c := out[i]
		switch {
		case c == '#' && i+1 < n && (out[i+1] == '[' || (out[i+1] == '!' && i+2 < n && out[i+2] == '[')):
			attrDepth = 1
			if out[i+1] == '!' {
				i += 3
			} else {
				i += 2
			}
		case attrDepth > 0 && c == '[':
			attrDepth++
			i++
		case attrDepth > 0 && c == ']':
			attrDepth--
			i++
		case attrDepth > 0:
			// Attribute contents are consumed by scanAttr and may contain
			// semantic string values such as #[path = "..."] .
			i++
		case c == '/' && i+1 < n && out[i+1] == '/':
			j := i + 2
			for j < n && out[j] != '\n' {
				j++
			}
			blank(i, j)
			i = j
		case c == '/' && i+1 < n && out[i+1] == '*':
			depth := 1
			j := i + 2
			for j < n && depth > 0 {
				if out[j] == '/' && j+1 < n && out[j+1] == '*' {
					depth++
					j += 2
				} else if out[j] == '*' && j+1 < n && out[j+1] == '/' {
					depth--
					j += 2
				} else {
					j++
				}
			}
			blank(i, j)
			i = j
		case c == '"' || (c == 'r' && i+1 < n && (out[i+1] == '"' || out[i+1] == '#')) || (c == 'b' && i+1 < n && (out[i+1] == '"' || out[i+1] == '\'' || out[i+1] == 'r')):
			j := scanString(out, i)
			blank(i, j)
			i = j
		case c == '\'':
			if j, ok := scanChar(out, i); ok {
				blank(i, j)
				i = j
			} else {
				i++
			}
		case isIdentStart(c):
			j := i + 1
			for j < n && isIdentChar(out[j]) {
				j++
			}
			word := string(out[i:j])
			if word == "macro_rules" || word == "macro" {
				k := j
				for k < n && (out[k] == ' ' || out[k] == '\t' || out[k] == '\n' || out[k] == '\r' || out[k] == '!') {
					k++
				}
				// macro_rules! name { ... } or macro name { ... }: skip
				// the name when present, then the balanced body.
				if k < n && isIdentStart(out[k]) {
					for k < n && isIdentChar(out[k]) {
						k++
					}
					for k < n && (out[k] == ' ' || out[k] == '\t' || out[k] == '\n' || out[k] == '\r') {
						k++
					}
				}
				if k < n && (out[k] == '{' || out[k] == '[' || out[k] == '(') {
					if end, ok := matchBalanced(out, k); ok {
						blank(i, end)
						i = end
						continue
					}
				}
				i = j
			} else {
				i = j
			}
		default:
			i++
		}
	}
	return string(out)
}

// scanString returns the end offset of the string literal at i.
func scanString(out []byte, i int) int {
	n := len(out)
	j := i
	if out[j] == 'b' {
		j++
	}
	if j < n && out[j] == 'r' {
		j++
		hashes := 0
		for j < n && out[j] == '#' {
			hashes++
			j++
		}
		if j < n && out[j] == '"' {
			j++
			for j < n {
				if out[j] == '"' {
					k := j + 1
					h := 0
					for h < hashes && k < n && out[k] == '#' {
						h++
						k++
					}
					if h == hashes {
						return k
					}
				}
				j++
			}
			return n
		}
		return i + 1
	}
	// Ordinary or byte string.
	if j < n && (out[j] == '"' || out[j] == '\'') {
		q := out[j]
		j++
		for j < n {
			if out[j] == '\\' {
				j += 2
				continue
			}
			if out[j] == q {
				return j + 1
			}
			if out[j] == '\n' && q == '\'' {
				return j
			}
			j++
		}
		return n
	}
	return i + 1
}

// scanChar matches a character literal at i and reports its end offset.
// A `'` that does not form `'x'` or `'\..'` is a lifetime tick, not a
// literal.
func scanChar(out []byte, i int) (int, bool) {
	n := len(out)
	j := i + 1
	if j >= n {
		return 0, false
	}
	if out[j] == '\\' {
		j++
		if j < n && out[j] == 'u' {
			j++
			if j < n && out[j] == '{' {
				for j < n && out[j] != '}' {
					j++
				}
				if j < n {
					j++
				}
			} else {
				for k := 0; k < 4 && j < n; k++ {
					j++
				}
			}
		} else if j < n {
			j++
		}
	} else if out[j] == '\'' || out[j] == '\n' {
		return 0, false
	} else {
		// Multibyte chars occupy several bytes; find the closing tick on
		// the same line within a short window.
		k := j
		for k < n && k-j < 8 && out[k] != '\'' && out[k] != '\n' {
			k++
		}
		if k < n && out[k] == '\'' && k-j >= 1 {
			// Reject lifetimes: 'a followed by an identifier character
			// is a tick, not a literal, unless it closed immediately.
			if k == j+1 || (k == j+2 && out[j] == '\\') {
				return k + 1, true
			}
			// 'ab' cannot be a char literal; treat as lifetimes.
			return 0, false
		}
		return 0, false
	}
	if j < n && out[j] == '\'' {
		return j + 1, true
	}
	return 0, false
}

// matchBalanced returns the offset just past the bracket group opening at
// i, which must be one of `{[(`.
func matchBalanced(out []byte, i int) (int, bool) {
	pairs := map[byte]byte{'{': '}', '[': ']', '(': ')'}
	open := out[i]
	want := pairs[open]
	depth := 0
	for j := i; j < len(out); j++ {
		switch out[j] {
		case open:
			depth++
		case want:
			depth--
			if depth == 0 {
				return j + 1, true
			}
		}
	}
	return 0, false
}

func isIdentStart(c byte) bool {
	return c == '_' || (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z')
}

func isIdentChar(c byte) bool {
	return isIdentStart(c) || (c >= '0' && c <= '9')
}

func isSpace(c byte) bool {
	return c == ' ' || c == '\t' || c == '\n' || c == '\r'
}

// parseItems scans items until end of input or a closing `}` at the
// current nesting level. testScope is true inside a `#[cfg(test)]`
// inline module.
func (p *parser) parseItems(depth int, testScope bool, facts *FileFacts) error {
	attrs := make([]string, 0, 2)
	for {
		p.skipSpace()
		if p.pos >= len(p.src) {
			return nil
		}
		if p.src[p.pos] == '}' {
			if depth == 0 {
				p.pos++
				continue
			}
			return nil
		}
		if p.src[p.pos] == '#' {
			attr, inner, ok := p.scanAttr()
			if !ok {
				p.pos++
				attrs = attrs[:0]
				continue
			}
			if inner {
				if isCfgTest(attr) {
					facts.InnerCfgTest = true
				}
				continue
			}
			attrs = append(attrs, attr)
			continue
		}
		word, ok := p.scanWord()
		if !ok {
			p.pos++
			attrs = attrs[:0]
			continue
		}
		switch word {
		case "mod":
			if err := p.parseMod(attrs, testScope, depth, facts); err != nil {
				return err
			}
		case "use":
			if err := p.parseUse(attrs, testScope, facts); err != nil {
				return err
			}
		case "extern":
			if err := p.parseExtern(attrs, testScope, facts); err != nil {
				return err
			}
		case "fn":
			if err := p.parseFn(attrs, testScope, facts); err != nil {
				return err
			}
		case "pub", "unsafe", "const", "static", "struct", "enum", "trait", "impl", "type", "where":
			// Qualifiers and unrelated items: keep pending attributes
			// only across `pub` (and `pub(...)`); anything else ends
			// the attribute run.
			if word != "pub" {
				attrs = attrs[:0]
			} else {
				p.skipPubRestrict()
			}
			continue
		default:
			attrs = attrs[:0]
			continue
		}
		attrs = attrs[:0]
	}
}

// parseMod parses after the `mod` keyword: `name;` or `name { ... }`.
func (p *parser) parseMod(attrs []string, testScope bool, depth int, facts *FileFacts) error {
	p.skipSpace()
	name, ok := p.scanWord()
	if !ok {
		return fmt.Errorf("rust: %s: expected module name after `mod`", p.path)
	}
	cfgTest := testScope
	pathOverride := ""
	for _, a := range attrs {
		kind, err := classifyTestAttr("module "+name, a)
		if err != nil {
			return err
		}
		switch kind {
		case attrTest:
			cfgTest = true
		}
		if v, ok := parsePathAttr(a); ok {
			pathOverride = v
		}
	}
	p.skipSpace()
	if p.pos >= len(p.src) {
		return fmt.Errorf("rust: %s: unexpected end after `mod %s`", p.path, name)
	}
	switch p.src[p.pos] {
	case ';':
		p.pos++
		facts.Modules = append(facts.Modules, ModuleDecl{Name: name, Path: pathOverride, CfgTest: cfgTest})
		if cfgTest {
			facts.HasCfgTest = true
		}
		return nil
	case '{':
		p.pos++
		facts.Modules = append(facts.Modules, ModuleDecl{Name: name, Inline: true, CfgTest: cfgTest})
		if cfgTest {
			facts.HasCfgTest = true
		}
		sub := &FileFacts{}
		if err := p.parseItems(depth+1, cfgTest, sub); err != nil {
			return err
		}
		p.skipSpace()
		if p.pos < len(p.src) && p.src[p.pos] == '}' {
			p.pos++
		}
		facts.Modules = append(facts.Modules, sub.Modules...)
		facts.Uses = append(facts.Uses, sub.Uses...)
		facts.Externs = append(facts.Externs, sub.Externs...)
		if sub.HasTestAttr {
			facts.HasTestAttr = true
		}
		if sub.HasCfgTest {
			facts.HasCfgTest = true
		}
		return nil
	default:
		return fmt.Errorf("rust: %s: expected `;` or `{` after `mod %s`", p.path, name)
	}
}

// parseUse parses after the `use` keyword up to the terminating `;` and
// expands `{a, b}` groups into individual literal identities.
func (p *parser) parseUse(attrs []string, testScope bool, facts *FileFacts) error {
	cfgTest := testScope
	for _, a := range attrs {
		kind, err := classifyTestAttr("use declaration", a)
		if err != nil {
			return err
		}
		if kind == attrTest {
			cfgTest = true
		}
	}
	start := p.pos
	semi := strings.IndexByte(p.src[start:], ';')
	if semi < 0 {
		return fmt.Errorf("rust: %s: unterminated `use` declaration", p.path)
	}
	raw := strings.TrimSpace(p.src[start : start+semi])
	p.pos = start + semi + 1
	if raw == "" {
		return fmt.Errorf("rust: %s: empty `use` declaration", p.path)
	}
	for _, path := range expandUse(raw) {
		decl := UseDecl{Path: path, CfgTest: cfgTest}
		if strings.HasSuffix(path, "::*") {
			decl.Glob = true
		}
		facts.Uses = append(facts.Uses, decl)
	}
	if cfgTest {
		facts.HasCfgTest = true
	}
	return nil
}

// parseExtern parses after the `extern` keyword: `crate name [as alias];`.
func (p *parser) parseExtern(attrs []string, testScope bool, facts *FileFacts) error {
	cfgTest := testScope
	for _, a := range attrs {
		kind, err := classifyTestAttr("extern crate declaration", a)
		if err != nil {
			return err
		}
		if kind == attrTest {
			cfgTest = true
		}
	}
	p.skipSpace()
	word, ok := p.scanWord()
	if !ok || word != "crate" {
		// `extern "C" { ... }` blocks and other extern forms carry no
		// dependency identity; skip to the next item boundary.
		for p.pos < len(p.src) && p.src[p.pos] != ';' && p.src[p.pos] != '{' && p.src[p.pos] != '}' {
			p.pos++
		}
		return nil
	}
	p.skipSpace()
	name, ok := p.scanWord()
	if !ok {
		return fmt.Errorf("rust: %s: expected crate name after `extern crate`", p.path)
	}
	alias := ""
	p.skipSpace()
	if w, ok := p.scanWord(); ok && w == "as" {
		p.skipSpace()
		a, ok := p.scanWord()
		if !ok {
			return fmt.Errorf("rust: %s: expected alias after `as`", p.path)
		}
		alias = a
	} else if ok {
		p.rewindWord(w)
	}
	facts.Externs = append(facts.Externs, ExternCrate{Name: name, As: alias, CfgTest: cfgTest})
	if cfgTest {
		facts.HasCfgTest = true
	}
	return nil
}

// parseFn records `#[test]` markers on functions. Bodies are skipped by
// brace matching so nested items never leak into crate scope.
func (p *parser) parseFn(attrs []string, testScope bool, facts *FileFacts) error {
	isTest := testScope
	for _, a := range attrs {
		kind, err := classifyTestAttr("function", a)
		if err != nil {
			return err
		}
		if kind == attrTest {
			isTest = true
		}
	}
	// Skip the signature up to the body or `;`.
	for p.pos < len(p.src) && p.src[p.pos] != '{' && p.src[p.pos] != ';' {
		p.pos++
	}
	if p.pos < len(p.src) && p.src[p.pos] == '{' {
		if end, ok := matchBalanced([]byte(p.src), p.pos); ok {
			p.pos = end
		} else {
			p.pos = len(p.src)
		}
	} else if p.pos < len(p.src) {
		p.pos++
	}
	if isTest {
		facts.HasTestAttr = true
	}
	return nil
}

type attrKind int

const (
	attrNone attrKind = iota
	attrTest
)

// classifyTestAttr maps one `#[...]` body to its test scope. Exactly
// `test` and `cfg(test)` are recognized; any other predicate mentioning
// `test` (including `cfg_attr`) fails closed as ambiguous.
func classifyTestAttr(what, attr string) (attrKind, error) {
	body := strings.TrimSpace(attr)
	if body == "test" {
		return attrTest, nil
	}
	if isCfgTest(body) {
		return attrTest, nil
	}
	if mentionsTest(body) {
		return attrNone, &AmbiguousError{What: what, Attr: "#[" + body + "]"}
	}
	return attrNone, nil
}

func isTestAttr(attr string) bool {
	kind, err := classifyTestAttr("item", attr)
	return err == nil && kind == attrTest
}

// isCfgTest reports whether the attribute body is exactly `cfg(test)`
// modulo whitespace.
func isCfgTest(body string) bool {
	rest := strings.TrimSpace(body)
	if !strings.HasPrefix(rest, "cfg") {
		return false
	}
	rest = strings.TrimSpace(rest[len("cfg"):])
	if !strings.HasPrefix(rest, "(") || !strings.HasSuffix(rest, ")") {
		return false
	}
	return strings.TrimSpace(rest[1:len(rest)-1]) == "test"
}

// mentionsTest reports whether the attribute body contains a `test`
// token outside string literals.
func mentionsTest(body string) bool {
	for i := 0; i < len(body); {
		if body[i] == '"' {
			i++
			for i < len(body) && body[i] != '"' {
				if body[i] == '\\' {
					i++
				}
				i++
			}
			if i < len(body) {
				i++
			}
			continue
		}
		if isIdentStart(body[i]) {
			j := i + 1
			for j < len(body) && (isIdentChar(body[j])) {
				j++
			}
			if body[i:j] == "test" {
				return true
			}
			i = j
			continue
		}
		i++
	}
	return false
}

// parsePathAttr extracts the value of `path = "..."` from an attribute
// body, used for explicit `#[path]` module overrides.
func parsePathAttr(attr string) (string, bool) {
	i := 0
	for i < len(attr) {
		for i < len(attr) && (isSpace(attr[i]) || attr[i] == ',') {
			i++
		}
		if !strings.HasPrefix(attr[i:], "path") {
			for i < len(attr) && attr[i] != ',' {
				i++
			}
			continue
		}
		j := i + len("path")
		if j < len(attr) && (isIdentChar(attr[j])) {
			i = j
			continue
		}
		for j < len(attr) && isSpace(attr[j]) {
			j++
		}
		if j >= len(attr) || attr[j] != '=' {
			i = j
			continue
		}
		j++
		for j < len(attr) && isSpace(attr[j]) {
			j++
		}
		if j >= len(attr) || attr[j] != '"' {
			return "", false
		}
		j++
		var b strings.Builder
		for j < len(attr) && attr[j] != '"' {
			if attr[j] == '\\' && j+1 < len(attr) {
				j++
			}
			b.WriteByte(attr[j])
			j++
		}
		return b.String(), true
	}
	return "", false
}

// expandUse expands one `use` tree into literal path identities:
// `a::{b, c}` becomes `a::b` and `a::c`. `self` inside a group resolves
// to the prefix; leading `::`, `crate`, `self`, and `super` segments are
// preserved verbatim for the resolver.
func expandUse(raw string) []string {
	raw = strings.TrimSpace(raw)
	if raw == "" {
		return nil
	}
	if idx := strings.IndexByte(raw, '{'); idx >= 0 {
		prefix := strings.TrimSpace(raw[:idx])
		prefix = strings.TrimSuffix(prefix, "::")
		end := strings.LastIndexByte(raw, '}')
		if end < 0 {
			return []string{raw}
		}
		inner := raw[idx+1 : end]
		var out []string
		for _, part := range splitTopLevel(inner, ',') {
			part = strings.TrimSpace(part)
			if part == "" {
				continue
			}
			if part == "self" {
				out = append(out, prefix)
				continue
			}
			expanded := expandUse(part)
			for _, e := range expanded {
				if prefix == "" {
					out = append(out, e)
				} else {
					out = append(out, prefix+"::"+e)
				}
			}
		}
		// A trailing `as` alias applies to the whole tree only for a
		// single path; groups with aliases are left verbatim per part.
		rest := strings.TrimSpace(raw[end+1:])
		_ = rest
		return out
	}
	// Strip a trailing `as alias`: the alias is a local binding, not an
	// identity the resolver needs.
	if alias := stripAlias(raw); alias != "" {
		return []string{alias}
	}
	return []string{raw}
}

// stripAlias removes a top-level ` as alias` suffix and reports the path.
func stripAlias(raw string) string {
	if i := strings.LastIndex(raw, " as "); i >= 0 {
		return strings.TrimSpace(raw[:i])
	}
	return ""
}

// splitTopLevel splits s on sep at brace depth zero.
func splitTopLevel(s string, sep byte) []string {
	var parts []string
	depth := 0
	start := 0
	for i := 0; i < len(s); i++ {
		switch s[i] {
		case '{', '(', '[':
			depth++
		case '}', ')', ']':
			depth--
		default:
			if s[i] == sep && depth == 0 {
				parts = append(parts, s[start:i])
				start = i + 1
			}
		}
	}
	return append(parts, s[start:])
}

// scanAttr scans an attribute at the current position. It reports the
// attribute body, whether it is an inner `#![...]` attribute, and whether
// a well-formed attribute was found.
func (p *parser) scanAttr() (body string, inner bool, ok bool) {
	if p.pos >= len(p.src) || p.src[p.pos] != '#' {
		return "", false, false
	}
	i := p.pos + 1
	if i < len(p.src) && p.src[i] == '!' {
		inner = true
		i++
	}
	for i < len(p.src) && isSpace(p.src[i]) {
		i++
	}
	if i >= len(p.src) || p.src[i] != '[' {
		return "", false, false
	}
	depth := 0
	for j := i; j < len(p.src); j++ {
		switch p.src[j] {
		case '[':
			depth++
		case ']':
			depth--
			if depth == 0 {
				p.pos = j + 1
				return strings.TrimSpace(p.src[i+1 : j]), inner, true
			}
		}
	}
	return "", false, false
}

// scanWord scans one identifier at the current position.
func (p *parser) scanWord() (string, bool) {
	p.skipSpace()
	if p.pos >= len(p.src) || !isIdentStart(p.src[p.pos]) {
		return "", false
	}
	i := p.pos + 1
	for i < len(p.src) && isIdentChar(p.src[i]) {
		i++
	}
	w := p.src[p.pos:i]
	p.pos = i
	return w, true
}

// rewindWord moves the cursor back over a just-scanned word.
func (p *parser) rewindWord(w string) {
	p.pos -= len(w)
}

// skipSpace advances past whitespace.
func (p *parser) skipSpace() {
	for p.pos < len(p.src) && isSpace(p.src[p.pos]) {
		p.pos++
	}
}

// skipPubRestrict skips a `(...)` restriction after `pub`.
func (p *parser) skipPubRestrict() {
	p.skipSpace()
	if p.pos < len(p.src) && p.src[p.pos] == '(' {
		if end, ok := matchBalanced([]byte(p.src), p.pos); ok {
			p.pos = end
		}
	}
}
