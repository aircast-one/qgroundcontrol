using System;

namespace AircastQGC.Pages;

public static class NavigationLock
{
    public static string? Reason { get; private set; }

    public static event EventHandler<string>? Refused;

    public static void Hold(string reason) => Reason = reason;

    public static void Release(string reason) => Reason = Reason == reason ? null : Reason;

    public static bool Allows() => Reason is not { } reason || Refuse(reason);

    private static bool Refuse(string reason)
    {
        Refused?.Invoke(null, reason);
        return false;
    }
}
