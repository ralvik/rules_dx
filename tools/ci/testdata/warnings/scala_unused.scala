// Warning fixture: unused variable (scalac -Xfatal-warnings).
object WarningUnused {
  def hello(name: String): String = {
    val unused = 1
    "hello " + name
  }
}
