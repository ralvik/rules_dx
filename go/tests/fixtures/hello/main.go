// M22 seed Go binary; consumer of go_binary.
package main

import (
	"fmt"

	"rules_dx/go/tests/fixtures/hello"
)

func main() {
	fmt.Println(hello.Hello("world"))
}
