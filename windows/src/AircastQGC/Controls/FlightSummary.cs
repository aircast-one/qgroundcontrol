using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;

namespace AircastQGC.Controls;

internal sealed record ArmTrack(bool WasArmed, bool FlewWhileArmed, int? ArmedBattery)
{
    internal static readonly ArmTrack Idle = new(false, false, null);

    internal ArmTrack Next(bool armedNow, string state) => this with
    {
        WasArmed = armedNow,
        FlewWhileArmed = armedNow && (FlewWhileArmed || state is "flying" or "landing"),
    };
}

internal static class FlightSummary
{
    internal const string BatteryView = "view.battery";
    private const string FlightTime = "vehicle.flightTime";
    private const string FlightDistance = "vehicle.flightDistance";

    internal static string? DisarmNotice(bool wasArmed, bool flewWhileArmed, bool armedNow) =>
        wasArmed && !armedNow ? flewWhileArmed ? "Landed and disarmed" : "Disarmed" : null;

    internal static string FlightTimeText(double? seconds)
    {
        var whole = seconds is double s && double.IsFinite(s) && s > 0 ? (int)s : 0;
        return $"{whole / 60:00}'{whole % 60:00}\"";
    }

    internal static string? FlownDistance(JsonObject fact) =>
        MapUiJson.Number(fact["value"]) > 0 && MapUiJson.Text(fact["valueString"]) is { Length: > 0 } shown
            ? $"{shown} {MapUiJson.Text(fact["units"])}".Trim()
            : null;

    internal static string Landed(double? seconds, string? distance, int? batteryUsed) =>
        string.Join(" · ", new[]
        {
            seconds >= 1 ? FlightTimeText(seconds) : null,
            string.IsNullOrWhiteSpace(distance) ? null : distance,
            batteryUsed > 0 ? $"{batteryUsed}% battery used" : null,
        }.OfType<string>());

    internal static int? BatteryPercent(JsonObject battery) =>
        MapUiJson.Flag(battery["available"])
        && MapUiJson.Objects(battery["indicatorPacks"])
            .Select(pack => string.Join(" ", new[]
            {
                MapUiJson.Text(pack["indicatorLabel"]),
                string.Join(" · ", MapUiJson.Texts(pack["indicatorLines"])),
            }.Where(part => part.Length > 0)))
            .FirstOrDefault(text => text.Length > 0) is { } reading
        && reading.Split(" · ")[0].Split(' ').LastOrDefault(token => token.Any(char.IsDigit)) is { } token
        && int.TryParse(token.TrimEnd('%'), out var percent)
            ? System.Math.Clamp(percent, 0, 100)
            : null;

    internal static int? BatteryPercentNow() => BatteryPercent(Bridge.Get(BatteryView));

    internal static string LandedNow(int? startBattery) =>
        Landed(
            MapUiJson.Number(Bridge.Get(FlightTime)["value"]),
            FlownDistance(Bridge.Get(FlightDistance)),
            startBattery is int start && BatteryPercentNow() is int now ? start - now : null);
}
