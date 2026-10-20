using System;
using System.IO;
using System.Runtime.InteropServices;
using System.Security;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using Microsoft.Win32;
using Microsoft.Windows.AppLifecycle;
using Windows.ApplicationModel.Activation;

namespace AircastQGC.Core;

public static partial class DeepLinks
{
    public const string Scheme = "aircast-qgc";
    private const string InstanceKey = "AircastQGC.main";
    private const string ClassesKey = @"Software\Classes\" + Scheme;

    [GeneratedRegex(@"aircast-qgc://[^\s""]+")]
    private static partial Regex LinkPattern();

    public static string? In(string commandLine) =>
        LinkPattern().Match(commandLine) is { Success: true } found ? found.Value : null;

    public static void Register(string exe)
    {
        var command = $"\"{exe}\" \"%1\"";
        try
        {
            using var scheme = Registry.CurrentUser.CreateSubKey(ClassesKey);
            using var open = scheme.CreateSubKey(@"shell\open\command");
            if (open.GetValue("") as string == command) return;
            using var icon = scheme.CreateSubKey("DefaultIcon");
            scheme.SetValue("", "URL:Aircast QGC");
            scheme.SetValue("URL Protocol", "");
            icon.SetValue("", $"\"{exe}\",0");
            open.SetValue("", command);
        }
        catch (Exception error) when (error is UnauthorizedAccessException or SecurityException or IOException)
        {
        }
    }

    public static async Task<bool> Forwarded(string? link, Action<string> arrived)
    {
        var main = AppInstance.FindOrRegisterForKey(InstanceKey);
        if (main.IsCurrent)
        {
            main.Activated += (_, activation) =>
            {
                if (activation.Data is ILaunchActivatedEventArgs launch && In(launch.Arguments ?? "") is { } next) arrived(next);
            };
            return false;
        }
        if (link is null) return false;
        await main.RedirectActivationToAsync(AppInstance.GetCurrent().GetActivatedEventArgs());
        return true;
    }

    public static void Handle(string link) => qgc_handle_deep_link(link);

    [LibraryImport("groundstation", StringMarshalling = StringMarshalling.Utf8)]
    private static partial void qgc_handle_deep_link(string link);
}
