package demo

import org.scalatest.flatspec.AnyFlatSpec

class DemoTest extends AnyFlatSpec {
  "Demo.greet" should "greet by name" in {
    assert(Demo.greet("world") == "hello world")
  }
}
