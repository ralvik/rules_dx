// Foreign Kotlin greeting implementation: adopted without upstream changes.
package greet

import java.util.Objects

object Greet {
  fun greet(name: String): String {
    return "hello " + Objects.requireNonNull(name)
  }
}
