// Warning fixture: unused variable (Roslyn /warnaserror+).
namespace Warning;

public static class WarningUnused
{
    public static string Greet(string name)
    {
        int unused = 1;
        return "hello " + name;
    }
}
