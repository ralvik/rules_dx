package java

import "strings"

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

func IsStdLib(spec string) bool {
	for _, root := range stdlibRoots {
		if strings.HasPrefix(spec, root) {
			return true
		}
	}
	return false
}
