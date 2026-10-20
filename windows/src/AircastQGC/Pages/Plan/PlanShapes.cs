using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;

namespace AircastQGC.Pages.Plan;

internal static class PlanShapes
{
    private const double ShapeFraction = 0.75;
    private const double ShapeMaxMetres = 3000;
    private const int CircleSegments = 16;
    private const int RingSegments = 64;
    private const double MetresPerDegree = 111_320;

    private static Geo Offset(Geo centre, double north, double east) => new(
        centre.Latitude + north / MetresPerDegree,
        centre.Longitude + east / (MetresPerDegree * Math.Cos(centre.Latitude * Math.PI / 180)));

    private static (double Width, double Height)? HalfExtents(IReadOnlyList<Geo> view) =>
        view.Count == 4
            ? (Half(PlanRules.Metres(view[0], view[1])), Half(PlanRules.Metres(view[0], view[3])))
            : null;

    private static double Half(double metres) => Math.Min(metres * ShapeFraction, ShapeMaxMetres) / 2;

    private static Geo Centre(IReadOnlyList<Geo> view) => new(view.Average(at => at.Latitude), view.Average(at => at.Longitude));

    internal static IReadOnlyList<Geo> Rectangle(IReadOnlyList<Geo> view) =>
        HalfExtents(view) is { } half
            ? new[] { (half.Height, -half.Width), (half.Height, half.Width), (-half.Height, half.Width), (-half.Height, -half.Width) }
                .Select(offset => Offset(Centre(view), offset.Item1, offset.Item2)).ToList()
            : [];

    internal static IReadOnlyList<Geo> Circle(IReadOnlyList<Geo> view) =>
        HalfExtents(view) is { } half
            ? MapOverlayJson.Ring(Centre(view), Math.Min(half.Width, half.Height)).Where((_, at) => at % (RingSegments / CircleSegments) == 0).ToList()
            : [];

    internal static IReadOnlyList<Geo> Line(IReadOnlyList<Geo> view)
    {
        if (view.Count != 4) return [];
        var top = new Geo((view[0].Latitude + view[1].Latitude) / 2, (view[0].Longitude + view[1].Longitude) / 2);
        var bottom = new Geo((view[3].Latitude + view[2].Latitude) / 2, (view[3].Longitude + view[2].Longitude) / 2);
        return new[] { 0.25, 0.75 }
            .Select(fraction => new Geo(top.Latitude + (bottom.Latitude - top.Latitude) * fraction, top.Longitude + (bottom.Longitude - top.Longitude) * fraction))
            .ToList();
    }

    internal static IReadOnlyList<Geo> Inset(IReadOnlyList<Geo> corners, double fraction)
    {
        if (corners.Count < 3) return [];
        var latitude = corners.Average(at => at.Latitude);
        var longitude = corners.Average(at => at.Longitude);
        return corners.Select(at => new Geo(latitude + (at.Latitude - latitude) * fraction, longitude + (at.Longitude - longitude) * fraction)).ToList();
    }

    internal static bool Replace(string path, IReadOnlyList<Geo> vertices, bool line) =>
        vertices.Count >= (line ? 2 : 3)
        && PlanCalls.Invoke($"{path}.clear").Ok
        && PlanCalls.Invoke($"{path}.appendVertices", new JsonArray(vertices.Select(at => (JsonNode)PlanCalls.Coordinate(at)).ToArray())).Ok;
}
