using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Linq;
using System.Text.Json.Nodes;

namespace AircastQGC.Controls;

internal readonly record struct ArSpot(double X, double Up);

internal sealed record ArMark(ArSpot At, string Label, string Text);

internal sealed record ArReading(
    ArMark? Home,
    IReadOnlyList<ArMark> Waypoints,
    IReadOnlyList<(ArSpot From, ArSpot To)> Route,
    ArMark? Aim = null,
    ArMark? Impact = null,
    ArSpot? Lead = null,
    (ArSpot From, ArSpot To)? LeadLine = null,
    IReadOnlyList<ArSpot?>? Orbit = null);

internal sealed record DetectionBox(double X, double Y, double W, double H, string Caption, bool Target);

internal sealed record Detections(bool Available, bool Stale, IReadOnlyList<DetectionBox> Boxes, string? Error);

internal sealed record LensReading(string Summary, bool Approximate, bool Measuring, string Progress, string? Error, bool CanMeasure, string? Reason, IReadOnlyList<double> MeasuredZooms);

internal static class VideoMarksModel
{
    public const string ArView = "view.videoAr";
    public const string DetectionsView = "view.detections";
    public const string LensView = "view.lens";
    public const string NoFrames = "no frames from the detector";
    public static readonly IReadOnlyList<double> GridFractions = [0.33, 0.66];
    private const double OutlineReach = 1.5;

    private static double? Number(JsonNode? node) =>
        node is JsonValue value && value.TryGetValue<double>(out var number) && double.IsFinite(number) ? number : null;

    private static bool Flag(JsonNode? node) => node is JsonValue value && value.TryGetValue<bool>(out var flag) && flag;

    private static string Text(JsonNode? node) => node is JsonValue value && value.TryGetValue<string>(out var text) ? text : "";

    private static string? Said(JsonNode? node) => Text(node) is { Length: > 0 } text && !string.IsNullOrWhiteSpace(text) ? text : null;

    private static IEnumerable<JsonObject> Objects(JsonNode? node) => node is JsonArray array ? array.OfType<JsonObject>() : [];

    private static ArSpot? Spot(JsonNode? node) =>
        node is JsonObject spot && Number(spot["x"]) is double x && Number(spot["up"]) is double up ? new ArSpot(x, up) : null;

    private static ArMark? Mark(JsonNode? node) =>
        Spot(node) is ArSpot at ? new ArMark(at, Text(node!["label"]), Text(node["text"])) : null;

    private static (ArSpot From, ArSpot To)? Leg(JsonNode? node) =>
        node is JsonObject leg && Spot(leg["from"]) is ArSpot from && Spot(leg["to"]) is ArSpot to ? (from, to) : null;

    public static ArReading? Ar(JsonObject? view) =>
        view is not null && Flag(view["available"])
            ? new ArReading(
                Mark(view["home"]),
                Objects(view["waypoints"]).Select(Mark).OfType<ArMark>().ToList(),
                Objects(view["route"])
                    .Select(leg => (From: Spot(leg["from"]), To: Spot(leg["to"])))
                    .Where(leg => leg.From is not null && leg.To is not null)
                    .Select(leg => (leg.From!.Value, leg.To!.Value))
                    .ToList(),
                Mark(view["aim"]),
                Mark(view["impact"]),
                Spot(view["lead"]),
                Leg(view["leadLine"]),
                Outline((view["orbit"] as JsonObject)?["outline"]))
            : null;

    public static IReadOnlyList<ArSpot?> Outline(JsonNode? node) =>
        node is JsonArray array ? array.Select(Spot).ToList() : [];

    public static IReadOnlyList<IReadOnlyList<ArSpot>> OutlineRuns(IReadOnlyList<ArSpot?> outline) =>
        outline
            .Select(spot => spot is { } at && Math.Abs(at.X - 0.5) <= OutlineReach && Math.Abs(at.Up) <= OutlineReach ? spot : null)
            .Aggregate(ImmutableList.Create(ImmutableList<ArSpot>.Empty), (runs, spot) =>
                spot is { } at ? runs.SetItem(runs.Count - 1, runs[^1].Add(at)) : runs.Add(ImmutableList<ArSpot>.Empty))
            .Where(run => run.Count > 1)
            .ToList<IReadOnlyList<ArSpot>>();

    public static (double X, double Y) ArOffset(ArSpot spot, double width, double height) => (spot.X * width, height / 2 - spot.Up * width);

    public static string ArCaption(ArMark mark) =>
        string.Join(" · ", new[] { mark.Label, mark.Text }.Where(part => !string.IsNullOrWhiteSpace(part)));

    public static Detections? Read(JsonObject? view) =>
        view is not null && Text(view["class"]) == "Detections"
            ? new Detections(
                Flag(view["available"]),
                Flag(view["stale"]),
                Objects(view["boxes"]).Select(box => new DetectionBox(
                    Number(box["x"]) ?? 0,
                    Number(box["y"]) ?? 0,
                    Number(box["w"]) ?? 0,
                    Number(box["h"]) ?? 0,
                    Text(box["caption"]),
                    Flag(box["target"]))).ToList(),
                Said(view["error"]))
            : null;

    public static IReadOnlyList<DetectionBox> VisibleBoxes(Detections? reading) =>
        reading is { Available: true, Stale: false } ? reading.Boxes : [];

    public static string? Trouble(Detections? reading) =>
        reading is { Available: true } ? reading.Error ?? (reading.Stale ? NoFrames : null) : null;

    public static LensReading? Lens(JsonObject? view) =>
        view is not null && Text(view["class"]) == "Lens"
            ? new LensReading(
                Text(view["summary"]),
                Flag(view["approximate"]),
                Flag(view["measuring"]),
                Text(view["progress"]),
                Said(view["error"]),
                Flag(view["canMeasure"]),
                Said(view["reason"]),
                Objects(view["samples"]).Select(sample => Number(sample["zoom"])).OfType<double>().ToList())
            : null;

    public static string? MeasuredText(IReadOnlyList<double> zooms) =>
        zooms.Count == 0 ? null : "Measured at " + string.Join(", ", zooms.Order().Select(zoom => $"{(int)zoom}%")) + " zoom";

    public static int PinchStep(double scale) => (int)Math.Floor((scale < 1 ? scale * -10 : scale) + 0.5);

    public static int WheelStep(int delta) => Math.Sign(delta);
}
