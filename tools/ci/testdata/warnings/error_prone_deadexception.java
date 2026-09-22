// Warning fixture: created exception not thrown (Error Prone DeadException).
package warning;

public class WarningDeadException {
  public static void check(String value) {
    new IllegalArgumentException("Missing required argument");
  }
}
