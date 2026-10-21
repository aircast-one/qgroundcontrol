using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.Linq;
using System.Security.Principal;
using Microsoft.UI.Xaml;

namespace AircastQGC.Core;

public static class Elevation
{
    public const string AfterExitOf = "--after-exit-of";
    private const int UserCancelled = 1223;
    private static readonly TimeSpan PreviousExit = TimeSpan.FromSeconds(30);

    public static bool Elevated()
    {
        try { return new WindowsPrincipal(WindowsIdentity.GetCurrent()).IsInRole(WindowsBuiltInRole.Administrator); }
        catch (Exception) { return false; }
    }

    public static string? Restart()
    {
        if (Environment.ProcessPath is not { } exe) return "Aircast could not find its own program file to restart.";
        try { Process.Start(new ProcessStartInfo(exe, $"{AfterExitOf} {Environment.ProcessId}") { UseShellExecute = true, Verb = "runas" }); }
        catch (Win32Exception error) when (error.NativeErrorCode == UserCancelled) { return "Aircast was not restarted. Windows did not get permission to run it as administrator."; }
        catch (Win32Exception error) { return $"Aircast could not restart as administrator: {error.Message}"; }
        Application.Current.Exit();
        return null;
    }

    public static void AwaitPrevious(IReadOnlyList<string> args) =>
        args.SkipWhile(arg => arg != AfterExitOf).Skip(1).Take(1)
            .Select(text => int.TryParse(text, out var pid) ? pid : 0)
            .Where(pid => pid > 0 && pid != Environment.ProcessId)
            .ToList()
            .ForEach(Exited);

    private static void Exited(int pid)
    {
        try
        {
            using var previous = Process.GetProcessById(pid);
            previous.WaitForExit(PreviousExit);
        }
        catch (Exception error) when (error is ArgumentException or InvalidOperationException or Win32Exception)
        {
        }
    }
}
