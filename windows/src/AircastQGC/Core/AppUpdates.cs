using System;
using System.Diagnostics;
using System.Globalization;
using System.Linq;
using System.Reflection;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using AircastQGC.Controls;
using AircastQGC.Pages.Analyze;
using Microsoft.Win32;

namespace AircastQGC.Core;

internal sealed record AppUpdate(
    string Current,
    string Channel,
    string Phase,
    string Latest,
    string Notes,
    string ReleaseUrl,
    long SizeBytes,
    double Progress,
    string File,
    string Error,
    string CheckedAt,
    bool AutoCheck,
    bool Snoozed,
    bool CanInstall)
{
    internal const string Available = "available";
    internal const string Downloading = "downloading";
    internal const string Ready = "ready";
    internal const string Failed = "failed";
    internal const string Checking = "checking";

    internal static AppUpdate Parse(JsonObject view) => new(
        MapUiJson.Text(view["current"]),
        MapUiJson.Text(view["channel"]),
        MapUiJson.Text(view["phase"]),
        MapUiJson.Text(view["latest"]),
        MapUiJson.Text(view["notes"]),
        MapUiJson.Text(view["releaseUrl"]),
        (long)(MapUiJson.Number(view["sizeBytes"]) ?? 0),
        MapUiJson.Number(view["progress"]) ?? 0,
        MapUiJson.Text(view["file"]),
        MapUiJson.Text(view["error"]),
        MapUiJson.Text(view["checkedAt"]),
        MapUiJson.Flag(view["autoCheck"], true),
        MapUiJson.Text(view["snoozed"]).Length > 0,
        MapUiJson.Flag(view["canInstall"]));

    internal bool Offered => Phase is Available or Downloading or Ready;

    internal bool Busy => Phase is Checking or Downloading;

    internal string Title => Phase switch
    {
        Downloading => $"Downloading {AppUpdates.Application} {Latest}… {Progress:P0}",
        Ready => $"{AppUpdates.Application} {Latest} is ready to install",
        _ => $"{AppUpdates.Application} {Latest} is available",
    };

    internal string ChannelName => Channel == "beta" ? "Beta" : "Stable";

    internal string Size => SizeBytes <= 0 ? "" : $"{SizeBytes / 1048576.0:0} MB";

    internal string Checked => DateTime.TryParse(CheckedAt, CultureInfo.InvariantCulture, DateTimeStyles.AdjustToUniversal | DateTimeStyles.AssumeUniversal, out var at)
        ? $"Last checked {at.ToLocalTime():g}"
        : "Not checked yet";

    internal string PlainNotes => string.Join("\n", Notes.Replace("\r", "").Split('\n')
        .Select(line => Regex.Replace(line.Trim(), @"^#+\s*|\*\*|`", ""))
        .Select(line => Regex.Replace(line, @"^[-*]\s+", "• "))
        .Where(line => line.Length > 0));
}

internal static class AppUpdates
{
    internal const string View = "view.appUpdate";
    internal const string Application = "Aircast QGC";
    internal const string DisconnectFirst = "Disconnect from the vehicle first";
    internal const string PortableNote = "This copy runs outside its installer folder, so the portable zip is updated by hand.";

    private const string UninstallKey = @"HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Uninstall\AircastQGCNative";

    private static string? _pending;

    internal static string Version =>
        Assembly.GetExecutingAssembly().GetCustomAttribute<AssemblyInformationalVersionAttribute>()?.InformationalVersion?.Split('+')[0] ?? "";

    internal static bool Installed { get; } = Registry.GetValue(UninstallKey, "InstallLocation", null) is string location
        && Environment.ProcessPath is { } exe
        && exe.StartsWith(location.TrimEnd('\\') + "\\", StringComparison.OrdinalIgnoreCase);

    internal static Task<string?> Invoke(string path, params object?[] args) =>
        Task.Run(() => Ui.Refusal(Bridge.Invoke(path, Forms.FormJson.Args(args)), Application));

    internal static void Install(string file)
    {
        _pending = file;
        App.Quit();
    }

    internal static void RunPending()
    {
        if (_pending is not { Length: > 0 } file) return;
        var script = $"Wait-Process -Id {Environment.ProcessId} -ErrorAction SilentlyContinue; Start-Process -FilePath '{file.Replace("'", "''")}' -ArgumentList '/S'";
        Process.Start(new ProcessStartInfo("powershell.exe")
        {
            ArgumentList = { "-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", script },
            UseShellExecute = false,
            CreateNoWindow = true,
        });
    }

    internal static async Task<string?> OpenRelease(string url)
    {
        if (!Uri.TryCreate(url, UriKind.Absolute, out var uri)) return "The release has no page to open.";
        try { return await Windows.System.Launcher.LaunchUriAsync(uri) ? null : "Windows could not open the release page."; }
        catch (Exception error) { return $"Windows could not open the release page: {error.Message}"; }
    }
}
