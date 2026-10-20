using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;

namespace AircastQGC.Controls;

internal sealed record AppMessage(string Title, string Text, string Action)
{
    internal const string RebootVehicle = "rebootVehicle";
    internal const string OpenSetup = "openSetup";
    internal const string UseRadio = "useRadio";

    internal string Confirm => Action switch
    {
        OpenSetup => "Open Setup",
        UseRadio => "Use",
        _ => "OK",
    };

    internal string? Dismiss => Action switch
    {
        RebootVehicle => "Cancel",
        OpenSetup => "Later",
        UseRadio => "Not now",
        _ => null,
    };

    internal string? InvokePath => Action switch
    {
        RebootVehicle => "vehicle.rebootVehicle",
        UseRadio => "usbRadio.use",
        _ => null,
    };
}

internal sealed record NoticeBatch(
    long Through,
    string? Destination,
    IReadOnlyList<string> Banners,
    IReadOnlyList<string> ErrorBanners,
    IReadOnlyList<AppMessage> Dialogs)
{
    internal const string Acknowledge = "host.acknowledgeThrough";
    internal const string ResetErrors = "vehicle.resetErrorLevelMessages";
    private const string VehicleError = "vehicleError";
    private const string AdditionalErrors = "Additional errors received";
    private static readonly TimeSpan RepeatQuiet = TimeSpan.FromSeconds(30);

    internal static string Path(long acknowledgedThrough) => $"view.hostNotices({acknowledgedThrough})";

    internal static NoticeBatch? Parse(JsonObject view)
    {
        var notices = MapUiJson.Objects(view["unseen"]).Where(notice => MapUiJson.Number(notice["id"]) >= 0).ToList();
        return notices.Count == 0
            ? null
            : new NoticeBatch(
                notices.Max(notice => (long)(MapUiJson.Number(notice["id"]) ?? 0)),
                MapUiJson.Text(view["destination"]).Trim() is { Length: > 0 } destination ? destination : null,
                MapUiJson.Texts(view["banners"]).Where(banner => banner.Trim().Length > 0).ToList(),
                notices.Where(notice => MapUiJson.Text(notice["kind"]) == VehicleError)
                    .Select(notice => MapUiJson.Text(notice["banner"]))
                    .Where(banner => banner.Trim().Length > 0)
                    .Distinct()
                    .ToList(),
                MapUiJson.Objects(view["dialogs"])
                    .Select(dialog => new AppMessage(MapUiJson.Text(dialog["title"]), MapUiJson.Text(dialog["text"]), MapUiJson.Text(dialog["action"]).Trim()))
                    .ToList());
    }

    internal static string? Critical(IReadOnlyList<string> errors) =>
        errors.Count == 0 ? null : errors.Count > 1 ? $"{errors[0]} · {AdditionalErrors}" : errors[0];

    internal static IReadOnlyList<string> Quiet(IEnumerable<string> banners, IReadOnlyDictionary<string, DateTime> shownAt, DateTime now) =>
        banners.Distinct().Where(banner => !(shownAt.TryGetValue(banner, out var at) && now - at < RepeatQuiet)).ToList();

    internal static string PageOf(string destination) => destination.ToLowerInvariant() switch
    {
        "fly" => NavPane.Fly,
        "plan" => NavPane.Plan,
        "analyze" => NavPane.Analyze,
        "setup" => NavPane.VehicleSetup,
        _ => NavPane.Settings,
    };
}
