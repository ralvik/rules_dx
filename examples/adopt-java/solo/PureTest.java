// Foreign Java test: handwritten owner of the test-owned source.
package solo;

import static org.junit.Assert.assertEquals;
import org.junit.Test;

public class PureTest {
  @Test
  public void testPure() {
    assertEquals("hello world", Pure.pure("world"));
  }
}
