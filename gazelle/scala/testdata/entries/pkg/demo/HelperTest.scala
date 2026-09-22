package demo

import org.scalatest.flatspec.AnyFlatSpec

class HelperTest extends AnyFlatSpec {
  "Main.greet" should "greet by name" in {
    assert(Main.greet("world") == "entry hello world")
  }
}
