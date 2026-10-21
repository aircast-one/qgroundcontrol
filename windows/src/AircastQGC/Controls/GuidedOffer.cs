using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using Microsoft.UI.Xaml;

namespace AircastQGC.Controls;

public sealed record GuidedAsk(string Title, string Prompt, Func<string?> Send)
{
    public string Detail { get; init; } = "";
    public string Warning { get; init; } = "";
    public bool Destructive { get; init; }
    public UIElement? Fields { get; init; }
    public (string Title, Func<string?> Send)? Alternative { get; init; }
    public string WatchPath { get; init; } = "";
    public Func<JsonObject, bool> StillOffered { get; init; } = _ => true;
}

public sealed record GuidedOffer(
    string Id,
    string Title,
    string Prompt,
    string Reason,
    string Option,
    bool Destructive,
    bool Confirm,
    bool CarriesValue,
    bool Shown,
    bool Ready)
{
    public const string ViewPath = "view.guidedActions";

    private static readonly IReadOnlyDictionary<string, string> Glyphs = new Dictionary<string, string>
    {
        ["arm"] = "",
        ["takeoff"] = "",
        ["startMission"] = "",
        ["continueMission"] = "",
        ["resumeMission"] = "",
        ["cancelRoi"] = "",
        ["pause"] = "",
        ["changeAltitude"] = "",
        ["changeSpeed"] = "",
        ["landAbort"] = "",
        ["land"] = "",
        ["rtl"] = "",
        ["disarm"] = "",
        ["grab"] = "",
        ["release"] = "",
        ["hold"] = "",
        ["emergencyStop"] = "",
        ["vtolTransitionToFixedWing"] = "",
        ["vtolTransitionToMultiRotor"] = "",
        ["forceArm"] = "",
    };

    public string Glyph => Glyphs[Id];
    public bool Blocked => Shown && !Ready;
    public string Explanation => Reason.Length == 0 ? Prompt : Reason;

    public static IReadOnlyList<GuidedOffer> List(JsonObject? view) =>
        view?["actions"] is JsonArray actions
            ? actions.OfType<JsonObject>().Select(Parse).OfType<GuidedOffer>().ToList()
            : Array.Empty<GuidedOffer>();

    private static GuidedOffer? Parse(JsonObject json)
    {
        var id = GuidedJson.Text(json, "id");
        var offer = GuidedJson.Text(json, "offer");
        return Glyphs.ContainsKey(id) && offer.Length > 0
            ? new GuidedOffer(
                id,
                GuidedJson.Text(json, "title"),
                GuidedJson.Text(json, "prompt"),
                GuidedJson.Text(json, "reason"),
                GuidedJson.Text(json, "option"),
                GuidedJson.Flag(json, "destructive", false),
                GuidedJson.Flag(json, "confirm", true),
                GuidedJson.Flag(json, "carriesValue", false),
                offer != "hidden",
                offer == "ready")
            : null;
    }
}

public sealed record GuidedRange(string Label, string Unit, double Minimum, double Maximum, double Initial)
{
    private const double WholeNumberFrom = 10.0;

    public static GuidedRange? From(JsonObject view) =>
        GuidedJson.Flag(view, "available", false)
        && GuidedJson.Number(view, "minimum") is double lo
        && GuidedJson.Number(view, "maximum") is double hi
        && hi > lo
            ? new GuidedRange(
                GuidedJson.Text(view, "label"),
                GuidedJson.Text(view, "unit"),
                lo,
                hi,
                Math.Clamp(GuidedJson.Number(view, "initial") ?? GuidedJson.Number(view, "current") ?? lo, lo, hi))
            : null;

    public double Clamped(double value) => Math.Clamp(value, Minimum, Maximum);

    public string Text(double value)
    {
        var shown = Clamped(value);
        var digits = Math.Abs(shown) < WholeNumberFrom ? 1 : 0;
        var rounded = Math.Round(shown, digits);
        var number = (rounded == 0 ? 0 : rounded).ToString(digits == 1 ? "F1" : "F0", CultureInfo.InvariantCulture);
        return Unit.Length == 0 ? number : $"{number} {Unit}";
    }
}

internal static class GuidedJson
{
    public static string Text(JsonObject? json, string key) =>
        json?[key] is JsonValue value && value.TryGetValue<string>(out var text) ? text : "";

    public static bool Flag(JsonObject? json, string key, bool fallback) =>
        json?[key] is JsonValue value && value.TryGetValue<bool>(out var flag) ? flag : fallback;

    public static double? Number(JsonObject? json, string key) =>
        json?[key] is JsonValue value && value.TryGetValue<double>(out var number) && double.IsFinite(number)
            ? number
            : null;

    public static int? Whole(JsonObject? json, string key) =>
        json?[key] is JsonValue value && value.TryGetValue<int>(out var whole) ? whole : null;

    public static string FirstText(params string[] candidates) =>
        candidates.FirstOrDefault(c => c.Length > 0) ?? "";
}
