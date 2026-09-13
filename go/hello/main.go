// M22 seed Go binary; consumer of dx_go_binary.
package main

import (
	"fmt"

	"rules_dx/go/hello"
)

func main() {
	fmt.Println(hello.Hello("world"))
}
