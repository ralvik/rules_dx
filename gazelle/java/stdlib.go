// Stdlib lists the JDK import roots treated as standard library by the
// Java Gazelle extension (M23, O31). Generation treats these imports as
// JDK-provided without an edge, a manifest, or a lockfile. The set covers
// the stable `java.*`, `javax.*`, `javafx.*`, `jdk.*`, plus the JDK-bundled
// `org.w3c.*`/`org.xml.*` packages; it is revisited when O31 freezes the
// JDK baseline.
package java

import "strings"

// stdlibRoots are the dotted-path roots treated as JDK-provided.
var stdlibRoots = []string{
	"java.",
	"javax.",
	"javafx.",
	"jdk.",
	"org.w3c.",
	"org.xml.",
	"com.sun.",
	"sun.",
}

// IsStdLib reports whether a dotted import path is JDK-provided.
func IsStdLib(spec string) bool {
	for _, root := range stdlibRoots {
		if strings.HasPrefix(spec, root) {
			return true
		}
	}
	return false
}
