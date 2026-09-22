// Foreign Java guava helper: one pinned module dep (guava 32.0.1-jre).
// Generation resolves `ImmutableList` through an exact `# gazelle:resolve`
// mapping to the shared @maven hub; the pom above records the coordinates.
package greet;

import com.google.common.collect.ImmutableList;

public class Guava {
  public static String join() {
    return String.join(",", ImmutableList.of("hello", "world"));
  }
}
