package cgo

import "C"

func Hello(name string) string {
	return "hello " + name
}
