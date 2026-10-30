using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Windows.Foundation;

namespace AircastQGC.Controls;

internal sealed record GuidedPick(string Label, Func<double, double> Target, bool Preset);

internal static class GuidedPicks
{
    private static readonly IReadOnlySet<string> MetricUnits = new HashSet<string> { "m", "m/s", "km/h" };
    private static readonly IReadOnlyList<double> MetricPresets = [5, 10, 20, 50];
    private static readonly IReadOnlyList<double> FeetPresets = [15, 30, 60, 150];
    private static readonly IReadOnlyList<int> QuickSteps = [10, 20];

    public static int Decimals(string unit) => MetricUnits.Contains(unit) ? 1 : 0;

    public static double Rounded(double value, string unit) => Math.Round(value, Decimals(unit), MidpointRounding.AwayFromZero);

    public static string Number(double value, string unit) =>
        value.ToString(Decimals(unit) == 1 ? "F1" : "F0", CultureInfo.InvariantCulture);

    public static IReadOnlyList<double> Presets(string unit, double minimum, double maximum) =>
        (unit == "ft" ? FeetPresets : MetricPresets).Where(preset => preset >= minimum && preset <= maximum).ToList();

    public static IReadOnlyList<GuidedPick> For(string offerId, GuidedRange range) => offerId switch
    {
        "takeoff" => Presets(range.Unit, range.Minimum, range.Maximum)
            .Select(preset => new GuidedPick(Labelled(Plain(preset, range.Unit), range.Unit), _ => preset, true))
            .ToList(),
        "changeAltitude" or "pause" => QuickSteps
            .Select(step => new GuidedPick(Labelled($"+{step}", range.Unit), value => Rounded(Math.Clamp(value + step, range.Minimum, range.Maximum), range.Unit), false))
            .Append(new GuidedPick(Labelled($"Max {Plain(range.Maximum, range.Unit)}", range.Unit), _ => Rounded(range.Maximum, range.Unit), false))
            .ToList(),
        _ => [],
    };

    public static bool Same(double a, double b, string unit) => Rounded(a, unit) == Rounded(b, unit);

    public static string Reading(double value, string unit) => unit.Length == 0 ? Number(value, unit) : $"{Number(value, unit)} {unit}";

    public static double Stepped(double value, int delta, GuidedRange range) =>
        Rounded(Math.Clamp(value + delta, range.Minimum, range.Maximum), range.Unit);

    public static double? Typed(string text, GuidedRange range) =>
        text.Trim() is var typed
        && typed.Count(c => c == ',') <= 1
        && !(typed.Contains(',') && typed.Contains('.'))
        && FormText.Parse(typed.Replace(',', '.')) is double number
            ? Rounded(Math.Clamp(number, range.Minimum, range.Maximum), range.Unit)
            : null;

    private static string Plain(double value, string unit)
    {
        var text = Number(value, unit);
        return text.EndsWith(".0", StringComparison.Ordinal) ? text[..^2] : text;
    }

    private static string Labelled(string text, string unit) => unit.Length == 0 ? text : $"{text} {unit}";
}

internal sealed record MissionIdentity(string Title, string Facts, string? Warning)
{
    public const string OnDrone = "The mission already on the drone";

    public static MissionIdentity Of(JsonObject plan, JsonObject summary) =>
        !MapUiJson.Flag(plan["hasMissionItems"])
            ? new MissionIdentity(OnDrone, "", "It isn't open in Plan, so its route can't be shown here.")
            : MapUiJson.Flag(plan["dirty"])
                ? new MissionIdentity(OnDrone, "", $"{PlanTitle(MapUiJson.Text(plan["file"]))} has changes that aren't on the drone. Upload it first to fly it.")
                : new MissionIdentity(
                    PlanTitle(MapUiJson.Text(plan["file"])),
                    string.Join(" · ", new[] { MapUiJson.Text(plan["status"]), Row(summary, "Distance"), Row(summary, "Time") }.Where(part => part.Trim().Length > 0)),
                    null);

    public static string PlanTitle(string file) =>
        System.IO.Path.GetFileNameWithoutExtension(file.Trim()) is { Length: > 0 } name ? name : "New plan";

    private static string Row(JsonObject summary, string label) =>
        MapUiJson.Objects(summary["rows"]).FirstOrDefault(row => MapUiJson.Text(row["label"]) == label) is { } row
            ? MapUiJson.Text(row["value"]).Trim()
            : "";
}

internal static class GuidedExtras
{
    public const string PlanView = "view.plan";
    public const string SummaryView = "view.missionSummary";
    public const string LandFromView = "view.instruments(altitudeRelative)";
    private static readonly IReadOnlySet<string> MissionActions = new HashSet<string> { "startMission", "continueMission" };

    public static UIElement? For(string offerId) =>
        MissionActions.Contains(offerId) ? MissionCard()
        : offerId == "land" ? LandFrom()
        : null;

    public static (string Value, string Units)? From(JsonObject view) =>
        MapUiJson.Objects(view["items"]).FirstOrDefault() is { } item
        && !MapUiJson.Flag(item["missing"])
        && MapUiJson.Text(item["value"]) is { Length: > 0 } value
        && value != "—"
            ? (value, MapUiJson.Text(item["units"]))
            : null;

    private static UIElement MissionCard()
    {
        var title = FormLook.Text("", "FontSizeBody", "TextPrimaryBrush", true);
        var facts = FormLook.Text("", "FontSizeBody", "TextSecondaryBrush");
        var warning = FormLook.Text("", "FontSizeBody", "CautionBrush");
        var card = Tile(FormLook.Stack(4, title, facts, warning));
        AutomationProperties.SetName(card, "Mission to fly");
        var plan = new JsonObject();
        var summary = new JsonObject();
        void Paint()
        {
            var mission = MissionIdentity.Of(plan, summary);
            FormLook.Say(title, mission.Title);
            FormLook.Say(facts, mission.Facts);
            FormLook.Say(warning, mission.Warning);
        }
        Live.Watch(card, PlanView, view =>
        {
            plan = view;
            Paint();
        });
        Live.Watch(card, SummaryView, view =>
        {
            summary = view;
            Paint();
        });
        return card;
    }

    private static UIElement LandFrom()
    {
        var value = FormLook.Text("", "FontSizeSubtitle", "TextPrimaryBrush", true);
        var units = FormLook.Caption("");
        units.VerticalAlignment = VerticalAlignment.Bottom;
        var tile = Tile(FormLook.Stack(2, FormLook.Caption("FROM"), FormLook.Line(4, value, units)));
        tile.HorizontalAlignment = HorizontalAlignment.Left;
        tile.Visibility = Visibility.Collapsed;
        Live.Watch(tile, LandFromView, view =>
        {
            var from = From(view);
            tile.Visibility = FormLook.Shown(from != null);
            FormLook.Say(value, from?.Value);
            FormLook.Say(units, from?.Units);
            AutomationProperties.SetName(tile, from is { } shown ? $"From {shown.Value} {shown.Units}".Trim() : "From");
        });
        return tile;
    }

    private static Border Tile(UIElement child) => new()
    {
        Background = FormLook.Brush("ControlBrush"),
        CornerRadius = FormLook.Radius("RadiusControl"),
        Padding = new Thickness(12, 8, 12, 8),
        Child = child,
    };
}

internal static class RoiTap
{
    private const string SetPath = "vehicle.guidedModeROI";
    private const string StopPath = "vehicle.stopGuidedModeROI";
    private const double HitRadius = 16;

    public static Geo? At(MapView map, Point tapped) =>
        MapOverlayJson.Roi(Bridge.Get(GuidedOffer.ViewPath)) is { } roi && Distance(map.ScreenOf(roi), tapped) <= HitRadius ? roi : null;

    public static GuidedAsk Ask(Geo roi)
    {
        var latitude = Coordinate("Latitude", roi.Latitude, -90, 90);
        var longitude = Coordinate("Longitude", roi.Longitude, -180, 180);
        return new GuidedAsk("Edit ROI position", "Move where the camera looks, or stop looking here.",
            () => double.IsFinite(latitude.Value) && double.IsFinite(longitude.Value)
                ? MapContextMenu.Execute(SetPath, latitude.Value, longitude.Value)
                : "Enter a latitude and a longitude.")
        {
            Fields = FormLook.Stack(8, latitude, longitude),
            Alternative = ("Cancel ROI", () => MapUiJson.RefusalOf(Bridge.Invoke(StopPath))),
            WatchPath = GuidedOffer.ViewPath,
            StillOffered = view => MapOverlayJson.Roi(view) != null,
        };
    }

    private static NumberBox Coordinate(string name, double value, double minimum, double maximum)
    {
        var box = new NumberBox
        {
            Header = name,
            Value = Math.Round(value, 7),
            Minimum = minimum,
            Maximum = maximum,
            SmallChange = 0.00001,
            ValidationMode = NumberBoxValidationMode.InvalidInputOverwritten,
            NumberFormatter = new Windows.Globalization.NumberFormatting.DecimalFormatter
            {
                FractionDigits = 7,
                IntegerDigits = 1,
                NumberRounder = new Windows.Globalization.NumberFormatting.IncrementNumberRounder { Increment = 0.0000001 },
            },
        };
        AutomationProperties.SetName(box, $"ROI {name.ToLowerInvariant()}");
        return box;
    }

    private static double Distance(Point a, Point b) => Math.Sqrt(Math.Pow(a.X - b.X, 2) + Math.Pow(a.Y - b.Y, 2));
}
