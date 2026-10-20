using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Linq;
using System.Text.Json.Nodes;

namespace AircastQGC.Controls;

internal sealed record ObstacleWarning(string Label, bool Close);

internal sealed record ObstacleSample(double BearingDegrees, double Metres);

internal sealed record ObstacleRing(IReadOnlyList<ObstacleSample> Samples, double MaxMetres, double IncrementDegrees, double FloorMetres, bool Stale);

internal sealed record ObstacleDrawing(IReadOnlyList<double> Ranges, IReadOnlyList<string> Texts, double Increment, double Offset, double MaxMetres);

internal enum ArcTone { Alarm, Calm, Stale }

internal readonly record struct Spot(double X, double Y);

internal sealed record OverlayPoint(Spot Outer, Spot Inner, double Range, int Index);

internal sealed record MapObstacleShape(double GradientFrom, double GradientTo, IReadOnlyList<OverlayPoint> Points);

internal sealed record VideoObstacleSegment(double From, double To, double RadFrom, double RadTo, string? Label);

internal sealed record VideoObstacleRings(double GradientFrom, double GradientTo, IReadOnlyList<VideoObstacleSegment> Segments);

internal sealed record ShadeStop(double At, string Brush, double Alpha);

internal static class ObstacleModel
{
    public const string View = "view.obstacle";
    public const string OverlaySetting = "settings.flyViewSettings.showObstacleDistanceOverlay";
    public const double NearestVisibleFraction = 0.12;
    public const double ProbePixels = 100;
    public const double TrueScaleBelowMetres = 4;
    private const double SweepPadding = 0.85;
    private const int Segments = 16;
    private const double LevelMetres = 10;
    private const double SegmentGapRadians = 0.03;
    private const double TextStepMetres = 2;
    private const double EquatorMetres = 40_075_016.686;
    private const double TileSize = 256;
    private const string Near = "ObstacleNearBrush";
    private const string Mid = "ObstacleMidBrush";
    private const string Far = "ObstacleFarBrush";

    public static readonly IReadOnlyList<ShadeStop> MapStops =
    [
        new(0, Near, 1), new(0.1, Near, 0.7), new(0.5, Mid, 0.7), new(0.65, Mid, 0.3), new(0.95, Far, 0.3), new(1, Far, 0),
    ];

    public static readonly IReadOnlyList<ShadeStop> VideoStops =
    [
        new(0, Near, 0.9), new(0.1, Near, 0.3), new(0.5, Mid, 0.3), new(0.65, Mid, 0.2), new(0.95, Far, 0.1), new(1, Far, 0),
    ];

    private static double? Number(JsonNode? node) =>
        node is JsonValue value && value.TryGetValue<double>(out var number) && double.IsFinite(number) ? number : null;

    private static double? Positive(JsonNode? node) => Number(node) is > 0 and var number ? number : null;

    private static bool Flag(JsonNode? node) => node is JsonValue value && value.TryGetValue<bool>(out var flag) && flag;

    private static string Text(JsonNode? node) => node is JsonValue value && value.TryGetValue<string>(out var text) ? text : "";

    public static ObstacleWarning? Warning(JsonObject? view) =>
        view is not null && Flag(view["available"])
        && view["nearest"] is JsonObject nearest && !Flag(nearest["stale"])
        && Text(nearest["distanceText"]) is { } distance && !string.IsNullOrWhiteSpace(distance)
            ? new ObstacleWarning(
                string.Join(" ", new[] { distance, Text(nearest["sectorText"]) }.Where(part => !string.IsNullOrWhiteSpace(part))),
                Flag(nearest["close"]))
            : null;

    public static ObstacleRing? Ring(JsonObject? view)
    {
        if (view is null || !Flag(view["available"]) || view["ringMetres"] is not JsonArray ring) return null;
        if (Positive(view["ringIncrement"]) is not double increment || Positive(view["rangeMaxMetres"]) is not double ceiling) return null;
        var offset = Number(view["ringOffset"]) ?? 0;
        var samples = ring
            .Select((node, index) => (Metres: Number(node), Index: index))
            .Where(sample => sample.Metres is >= 0)
            .Select(sample => new ObstacleSample(offset + sample.Index * increment, sample.Metres!.Value))
            .ToList();
        return samples.Count == 0 ? null : new ObstacleRing(samples, ceiling, increment, Positive(view["rangeMinMetres"]) ?? 0, Flag(view["stale"]));
    }

    public static double ArcSweep(double increment) => increment * SweepPadding;

    public static double ArcRadiusFraction(double metres, double ceiling) =>
        ceiling <= 0 ? 0 : Math.Max(Math.Sqrt(Math.Clamp(metres / ceiling, 0, 1)), NearestVisibleFraction);

    public static bool SampleIsClose(double metres, double floorMetres) => metres < floorMetres * 2;

    public static ArcTone Tone(bool near, bool stale) => stale ? ArcTone.Stale : near ? ArcTone.Alarm : ArcTone.Calm;

    public static ObstacleDrawing? Drawing(JsonObject? view)
    {
        if (view?["overlay"] is not JsonObject drawn || drawn["ranges"] is not JsonArray ranges || ranges.Count == 0) return null;
        if (Positive(view["ringIncrement"]) is not double increment || Positive(drawn["maxMetres"]) is not double maxMetres) return null;
        var texts = drawn["texts"] as JsonArray;
        return new ObstacleDrawing(
            ranges.Select(node => Number(node) ?? double.NaN).ToList(),
            Enumerable.Range(0, ranges.Count).Select(index => texts is not null && index < texts.Count ? Text(texts[index]) : "").ToList(),
            increment,
            Number(view["ringOffset"]) ?? 0,
            maxMetres);
    }

    private static double Wrap(double value, double over) => (value % over + over) % over;

    public static int RangeIndex(double degrees, ObstacleDrawing drawing, double heading)
    {
        var count = drawing.Ranges.Count;
        var wrapped = Wrap(degrees - drawing.Offset - heading, 360);
        return (int)Wrap(count + (int)Math.Ceiling(wrapped / drawing.Increment), count);
    }

    public static double MetresAcross(double latitude, double zoom, double pixels) =>
        EquatorMetres * Math.Cos(latitude * Math.PI / 180) / (TileSize * Math.Pow(2, zoom)) * pixels;

    public static MapObstacleShape MapShape(ObstacleDrawing drawing, Spot centre, double height, double metresInProbe, double probePixels, double heading, double mapBearing)
    {
        var maxRadius = 0.9 * height / 2;
        var minRadius = maxRadius * 0.2;
        var pixelsPerMetre = probePixels / metresInProbe;
        var trueScale = metresInProbe < TrueScaleBelowMetres;
        var gradientFrom = trueScale ? 0 : minRadius;
        var gradientTo = trueScale ? drawing.MaxMetres * pixelsPerMetre : maxRadius;
        var metresToPixels = trueScale ? pixelsPerMetre : (maxRadius - minRadius) / drawing.MaxMetres;
        var band = minRadius / 8;
        var points = drawing.Ranges.Select((_, at) =>
        {
            var degrees = at * drawing.Increment;
            var radians = (degrees - mapBearing) * Math.PI / 180;
            var index = RangeIndex(degrees, drawing, heading);
            var metres = drawing.Ranges[index];
            var pixels = gradientFrom + metres * metresToPixels;
            Spot Along(double reach) => new(centre.X + reach * Math.Sin(radians), centre.Y - reach * Math.Cos(radians));
            return new OverlayPoint(Along(pixels), Along(pixels - band), metres, index);
        }).ToList();
        return new MapObstacleShape(gradientFrom, gradientTo, points);
    }

    public static IReadOnlyList<OverlayPoint> MapLabels(ObstacleDrawing drawing, IReadOnlyList<OverlayPoint> points) =>
        Enumerable.Range(0, points.Count)
            .Where(at => at % 3 == 0)
            .Select(at => points[(at + 1) % points.Count] is var next && next.Range < points[at].Range ? next : points[at])
            .Aggregate(
                (Shown: ImmutableList<OverlayPoint>.Empty, Previous: -1.0),
                (state, point) => point.Range < drawing.MaxMetres && Math.Abs(point.Range - state.Previous) > TextStepMetres
                    ? (state.Shown.Add(point), point.Range)
                    : state)
            .Shown;

    public static VideoObstacleRings? VideoRings(ObstacleDrawing drawing, double height, bool showText)
    {
        var maxRadius = 0.9 * height / 2;
        var segmentHeight = maxRadius * 0.2 / 8;
        var levels = drawing.MaxMetres / LevelMetres;
        var gradientFrom = maxRadius - segmentHeight * levels * 2;
        if (maxRadius <= 0 || gradientFrom < 0) return null;
        var count = drawing.Ranges.Count;
        var step = 360.0 / Segments;
        var segments = Enumerable.Range(0, Segments).SelectMany(segment =>
        {
            var degrees = segment * step;
            var first = RangeIndex(degrees, drawing, 0);
            var next = RangeIndex(degrees + step, drawing, 0);
            var end = first < next ? next : count + next;
            var nearest = Enumerable.Range(first, Math.Max(0, end - first))
                .Select(at => at % count)
                .Where(at => drawing.Ranges[at] < drawing.MaxMetres)
                .OrderBy(at => drawing.Ranges[at])
                .Select(at => (int?)at)
                .FirstOrDefault();
            var rangeMin = nearest is int closest ? drawing.Ranges[closest] : drawing.MaxMetres;
            var radFrom = degrees * Math.PI / 180;
            var radTo = radFrom + step * Math.PI / 180 - SegmentGapRadians;
            var drawn = Enumerable.Range(0, (int)Math.Ceiling(levels)).Select(level =>
            {
                var from = maxRadius - level * segmentHeight * 2;
                var reached = rangeMin > drawing.MaxMetres - (level + 1) * LevelMetres || level >= levels - 1;
                var range = reached ? rangeMin : drawing.MaxMetres;
                var label = nearest is int labelled && showText && reached && range < drawing.MaxMetres ? drawing.Texts[labelled] : null;
                return (Segment: new VideoObstacleSegment(from, from - segmentHeight, radFrom, radTo, label), Reaches: range == rangeMin);
            }).ToList();
            var upTo = drawn.FindIndex(level => level.Reaches) is var found && found < 0 ? drawn.Count : found + 1;
            return drawn.Take(upTo).Select(level => level.Segment);
        }).ToList();
        var positive = segments.FindIndex(segment => segment.To < 0) is var below && below < 0 ? segments.Count : below;
        return new VideoObstacleRings(gradientFrom, maxRadius, segments.Take(positive).ToList());
    }

    public static double StopOffset(double at, double from, double to) => to > 0 ? (from + at * (to - from)) / to : at;
}
