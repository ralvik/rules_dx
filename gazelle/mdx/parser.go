package mdx

import (
	"path"
	"sort"
	"strings"
)

func ParseImports(content []byte) []string {
	regions := ExtractESMRegions(content)
	if len(regions) == 0 {
		return nil
	}
	set := make(map[string]struct{})
	add := func(spec string) {
		root := normalizeSpec(spec)
		if root == "" {
			return
		}
		set[root] = struct{}{}
	}
	for _, region := range regions {
		scan(region, add)
	}
	var out []string
	for name := range set {
		out = append(out, name)
	}
	sort.Strings(out)
	return out
}

func normalizeSpec(spec string) string {
	spec = strings.TrimSpace(spec)
	if spec == "" {
		return ""
	}
	if strings.HasPrefix(spec, ".") || strings.HasPrefix(spec, "/") {
		trimmed := strings.TrimSuffix(spec, "/")
		base := path.Base(trimmed)
		if base == "" || base == "." || base == "/" {
			return ""
		}
		if idx := strings.LastIndexByte(base, '.'); idx > 0 {
			base = base[:idx]
		}
		return base
	}
	return spec
}

func ExtractESMRegions(src []byte) [][]byte {
	lines := splitLines(src)
	var out [][]byte
	var chunk []string
	depth := 0
	boundary := true
	prevText := false
	inFence := false
	fenceChar := byte(0)
	fenceLen := 0
	flush := func() {
		if len(chunk) > 0 {
			out = append(out, []byte(strings.Join(chunk, "\n")+"\n"))
			chunk = nil
		}
		depth = 0
	}
	for _, raw := range lines {
		line := raw
		if !inFence {
			if idx := strings.Index(line, "<!--"); idx >= 0 {
				if end := strings.Index(line[idx+4:], "-->"); end >= 0 {
					line = line[:idx] + line[idx+4+end+3:]
				} else {
					return nil
				}
			}
		}
		if !inFence {
			if ch, ln, ok := parseFenceOpen(line); ok {
				flush()
				inFence = true
				fenceChar = ch
				fenceLen = ln
				boundary = false
				prevText = false
				continue
			}
		} else {
			if isFenceClose(line, fenceChar, fenceLen) {
				inFence = false
				boundary = true
				prevText = false
				continue
			}
			continue
		}
		if strings.TrimSpace(line) == "" {
			if depth == 0 {
				flush()
			} else {
				chunk = append(chunk, line)
			}
			boundary = true
			prevText = false
			continue
		}
		if isESMOpener(line) {
			if len(chunk) > 0 || boundary {
				chunk = append(chunk, line)
				depth += braceDelta(line)
				boundary = true
				prevText = false
				continue
			}
			boundary = false
			prevText = true
			continue
		}
		if len(chunk) > 0 && depth > 0 && isESMContinuation(line) {
			chunk = append(chunk, line)
			depth += braceDelta(line)
			boundary = false
			prevText = false
			continue
		}
		flush()
		boundary = isBoundaryLine(line, prevText)
		prevText = !boundary
	}
	flush()
	if len(out) == 0 {
		return nil
	}
	return out
}

func splitLines(src []byte) []string {
	if len(src) == 0 {
		return nil
	}
	raw := strings.Split(string(src), "\n")
	if raw[len(raw)-1] == "" {
		raw = raw[:len(raw)-1]
	}
	for i := range raw {
		raw[i] = strings.TrimSuffix(raw[i], "\r")
	}
	return raw
}

func isESMOpener(line string) bool {
	if strings.HasPrefix(line, "import") {
		if len(line) == 6 {
			return false
		}
		switch line[6] {
		case ' ', '\t', '"', '\'', '(', '{', '*':
			return true
		}
		return false
	}
	if strings.HasPrefix(line, "export") {
		if len(line) == 6 {
			return false
		}
		switch line[6] {
		case ' ', '\t', '{', '*':
			return true
		}
		return false
	}
	return false
}

func isESMContinuation(line string) bool {
	trimmed := strings.TrimLeft(line, " \t")
	return strings.HasPrefix(trimmed, "}") || strings.HasPrefix(trimmed, "from ") || strings.HasPrefix(trimmed, "from\t")
}

func isBoundaryLine(line string, prevText bool) bool {
	if isATXHeading(line) || isThematicBreak(line) {
		return true
	}
	if prevText && isSetextUnderline(line) {
		return true
	}
	return false
}

func isATXHeading(line string) bool {
	i := 0
	for i < len(line) && line[i] == ' ' && i < 4 {
		i++
	}
	if i >= 4 {
		return false
	}
	hashes := 0
	for i < len(line) && line[i] == '#' {
		hashes++
		i++
	}
	if hashes == 0 || hashes > 6 {
		return false
	}
	if i >= len(line) {
		return true
	}
	return line[i] == ' ' || line[i] == '\t'
}

func isThematicBreak(line string) bool {
	stripped := strings.ReplaceAll(strings.ReplaceAll(line, " ", ""), "\t", "")
	if len(stripped) < 3 {
		return false
	}
	mark := stripped[0]
	if mark != '-' && mark != '*' && mark != '_' {
		return false
	}
	for i := 1; i < len(stripped); i++ {
		if stripped[i] != mark {
			return false
		}
	}
	return true
}

func isSetextUnderline(line string) bool {
	stripped := strings.ReplaceAll(strings.ReplaceAll(line, " ", ""), "\t", "")
	if len(stripped) < 1 {
		return false
	}
	for i := 0; i < len(stripped); i++ {
		if stripped[i] != '=' {
			return false
		}
	}
	return true
}

func parseFenceOpen(line string) (byte, int, bool) {
	i := 0
	for i < len(line) && line[i] == ' ' {
		i++
	}
	if i > 3 {
		return 0, 0, false
	}
	if i >= len(line) || (line[i] != '`' && line[i] != '~') {
		return 0, 0, false
	}
	ch := line[i]
	n := 0
	for i < len(line) && line[i] == ch {
		n++
		i++
	}
	if n < 3 {
		return 0, 0, false
	}
	return ch, n, true
}

func isFenceClose(line string, ch byte, ln int) bool {
	i := 0
	for i < len(line) && line[i] == ' ' {
		i++
	}
	if i > 3 {
		return false
	}
	n := 0
	for i < len(line) && line[i] == ch {
		n++
		i++
	}
	if n < ln {
		return false
	}
	for i < len(line) {
		if line[i] != ' ' && line[i] != '\t' {
			return false
		}
		i++
	}
	return true
}

func braceDelta(line string) int {
	depth := 0
	i := 0
	for i < len(line) {
		c := line[i]
		if c == '/' && i+1 < len(line) && line[i+1] == '/' {
			break
		}
		if c == '/' && i+1 < len(line) && line[i+1] == '*' {
			j := strings.Index(line[i+2:], "*/")
			if j < 0 {
				break
			}
			i += j + 4
			continue
		}
		if c == '\'' || c == '"' {
			j := i + 1
			for j < len(line) {
				if line[j] == '\\' {
					j += 2
					continue
				}
				if line[j] == c {
					break
				}
				j++
			}
			i = j + 1
			continue
		}
		if c == '{' {
			depth++
		} else if c == '}' {
			depth--
		}
		i++
	}
	return depth
}

func scan(src []byte, add func(string)) {
	n := len(src)
	i := 0
	for i < n {
		c := src[i]
		if c == '/' && i+1 < n && src[i+1] == '/' {
			j := i + 2
			for j < n && src[j] != '\n' {
				j++
			}
			i = j
			continue
		}
		if c == '/' && i+1 < n && src[i+1] == '*' {
			j := i + 2
			for j+1 < n && !(src[j] == '*' && src[j+1] == '/') {
				j++
			}
			if j+1 < n {
				i = j + 2
			} else {
				return
			}
			continue
		}
		if c == '\'' || c == '"' {
			i = skipQuoted(src, i)
			continue
		}
		if c == '`' {
			i = skipTemplate(src, i)
			continue
		}
		if c == '/' && isRegexStart(src, i) {
			i = skipRegex(src, i)
			continue
		}
		if isIdentStart(c) {
			j := i + 1
			for j < n && isIdentChar(src[j]) {
				j++
			}
			word := string(src[i:j])
			switch word {
			case "import":
				i = parseImport(src, i, j, add)
				continue
			case "export":
				i = parseExport(src, j, add)
				continue
			case "require":
				if isPrecededByDot(src, i) {
					i = j
					continue
				}
				i = parseRequire(src, j, add)
				continue
			}
			i = j
			continue
		}
		i++
	}
}

func isIdentStart(c byte) bool {
	return c == '_' || c == '$' ||
		(c >= 'a' && c <= 'z') ||
		(c >= 'A' && c <= 'Z')
}

func isIdentChar(c byte) bool {
	return isIdentStart(c) || (c >= '0' && c <= '9')
}

func isPrecededByDot(src []byte, i int) bool {
	j := i - 1
	for j >= 0 && (src[j] == ' ' || src[j] == '\t' || src[j] == '\n' || src[j] == '\r') {
		j--
	}
	return j >= 0 && src[j] == '.'
}

func isRegexStart(src []byte, i int) bool {
	j := i - 1
	for j >= 0 && (src[j] == ' ' || src[j] == '\t' || src[j] == '\n' || src[j] == '\r') {
		j--
	}
	if j < 0 {
		return true
	}
	p := src[j]
	if p == ')' || p == ']' || p == '}' {
		return false
	}
	if isIdentChar(p) || p == '$' || p == '"' || p == '\'' || p == '`' {
		return false
	}
	return true
}

func skipQuoted(src []byte, i int) int {
	quote := src[i]
	j := i + 1
	for j < len(src) {
		if src[j] == '\\' {
			j += 2
			continue
		}
		if src[j] == quote {
			return j + 1
		}
		if src[j] == '\n' {
			return j
		}
		j++
	}
	return len(src)
}

func skipTemplate(src []byte, i int) int {
	j := i + 1
	for j < len(src) {
		if src[j] == '\\' {
			j += 2
			continue
		}
		if src[j] == '`' {
			return j + 1
		}
		if src[j] == '$' && j+1 < len(src) && src[j+1] == '{' {
			depth := 1
			j += 2
			for j < len(src) && depth > 0 {
				if src[j] == '{' {
					depth++
				} else if src[j] == '}' {
					depth--
				} else if src[j] == '\'' || src[j] == '"' {
					j = skipQuoted(src, j)
					continue
				} else if src[j] == '`' {
					j = skipTemplate(src, j)
					continue
				}
				j++
			}
			continue
		}
		j++
	}
	return len(src)
}

func skipRegex(src []byte, i int) int {
	j := i + 1
	inClass := false
	for j < len(src) {
		c := src[j]
		if c == '\\' {
			j += 2
			continue
		}
		if c == '\n' {
			return j
		}
		if c == '[' {
			inClass = true
		} else if c == ']' {
			inClass = false
		} else if c == '/' && !inClass {
			j++
			for j < len(src) && isIdentChar(src[j]) {
				j++
			}
			return j
		}
		j++
	}
	return len(src)
}

func skipTrivia(src []byte, pos int) int {
	n := len(src)
	p := pos
	for p < n {
		c := src[p]
		if c == ' ' || c == '\t' || c == '\r' || c == '\n' {
			p++
			continue
		}
		if c == '/' && p+1 < n && src[p+1] == '/' {
			j := p + 2
			for j < n && src[j] != '\n' {
				j++
			}
			p = j
			continue
		}
		if c == '/' && p+1 < n && src[p+1] == '*' {
			j := p + 2
			for j+1 < n && !(src[j] == '*' && src[j+1] == '/') {
				j++
			}
			if j+1 < n {
				p = j + 2
				continue
			}
			return n
		}
		return p
	}
	return p
}

func parseImport(src []byte, start, j int, add func(string)) int {
	n := len(src)
	k := skipTrivia(src, j)
	if k < n && (src[k] == '\'' || src[k] == '"') {
		if spec, next, ok := readQuoted(src, k); ok {
			add(spec)
			return next
		}
		return k + 1
	}
	if k < n && src[k] == '(' {
		m := skipTrivia(src, k+1)
		if m < n && (src[m] == '\'' || src[m] == '"') {
			if spec, next, ok := readQuoted(src, m); ok {
				after := skipTrivia(src, next)
				if after < n && src[after] == ')' {
					add(spec)
					return after + 1
				}
				return next
			}
		}
		return k + 1
	}
	depth := 0
	p := k
	for p < n {
		if p+1 < n && src[p] == '/' && (src[p+1] == '/' || src[p+1] == '*') {
			p = skipTrivia(src, p)
			continue
		}
		c := src[p]
		if c == '{' {
			depth++
		} else if c == '}' {
			if depth > 0 {
				depth--
			}
		} else if c == '\'' || c == '"' {
			p = skipQuoted(src, p)
			continue
		} else if c == '`' {
			p = skipTemplate(src, p)
			continue
		} else if c == ';' {
			return p + 1
		} else if depth == 0 && isIdentStart(c) {
			q := p + 1
			for q < n && isIdentChar(src[q]) {
				q++
			}
			if string(src[p:q]) == "from" {
				m := skipTrivia(src, q)
				if m < n && (src[m] == '\'' || src[m] == '"') {
					if spec, next, ok := readQuoted(src, m); ok {
						add(spec)
						return next
					}
				}
				return q
			}
			p = q
			continue
		}
		p++
	}
	return p
}

func parseExport(src []byte, j int, add func(string)) int {
	n := len(src)
	depth := 0
	p := skipTrivia(src, j)
	for p < n {
		if p+1 < n && src[p] == '/' && (src[p+1] == '/' || src[p+1] == '*') {
			p = skipTrivia(src, p)
			continue
		}
		c := src[p]
		if c == '{' {
			depth++
		} else if c == '}' {
			if depth > 0 {
				depth--
			}
		} else if c == '\'' || c == '"' {
			p = skipQuoted(src, p)
			continue
		} else if c == '`' {
			p = skipTemplate(src, p)
			continue
		} else if c == ';' {
			return p + 1
		} else if depth == 0 && c == '*' {
		} else if depth == 0 && isIdentStart(c) {
			q := p + 1
			for q < n && isIdentChar(src[q]) {
				q++
			}
			if string(src[p:q]) == "from" {
				m := skipTrivia(src, q)
				if m < n && (src[m] == '\'' || src[m] == '"') {
					if spec, next, ok := readQuoted(src, m); ok {
						add(spec)
						return next
					}
				}
				return q
			}
			p = q
			continue
		}
		p++
	}
	return p
}

func parseRequire(src []byte, j int, add func(string)) int {
	n := len(src)
	k := skipTrivia(src, j)
	if k >= n || src[k] != '(' {
		return j
	}
	m := skipTrivia(src, k+1)
	if m < n && (src[m] == '\'' || src[m] == '"') {
		if spec, next, ok := readQuoted(src, m); ok {
			after := skipTrivia(src, next)
			if after < n && src[after] == ')' {
				add(spec)
				return after + 1
			}
			return next
		}
	}
	return k + 1
}

func readQuoted(src []byte, i int) (string, int, bool) {
	quote := src[i]
	var b strings.Builder
	j := i + 1
	for j < len(src) {
		if src[j] == '\\' && j+1 < len(src) {
			b.WriteByte(src[j+1])
			j += 2
			continue
		}
		if src[j] == quote {
			return b.String(), j + 1, true
		}
		if src[j] == '\n' {
			return "", j, false
		}
		b.WriteByte(src[j])
		j++
	}
	return "", len(src), false
}
