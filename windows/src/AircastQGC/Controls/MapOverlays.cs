using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;

namespace AircastQGC.Controls;

internal readonly record struct Geo(double Latitude, double Longitude);

internal sealed record MissionPin(Geo At, string Label, string Side, string Kind, int Command, bool Current, bool Dot);

internal sealed record FlownMission(IReadOnlyList<IReadOnlyList<Geo>> Route, IReadOnlyList<MissionPin> Pins)
{
    public static readonly FlownMission None = new([], []);
}

internal sealed record FenceArea(IReadOnlyList<Geo> Ring, bool KeepIn);

internal sealed record FlownFences(IReadOnlyList<FenceArea> Areas, IReadOnlyList<Geo> Enforced, IReadOnlyList<Geo> Rally)
{
    public static readonly FlownFences None = new([], [], []);
}

internal sealed record TrafficMark(Geo At, double? Heading, bool Alert, bool Stale, string Label);

internal sealed record CircleMark(Geo Centre, IReadOnlyList<Geo> Ring);

internal sealed record MapScale(string Text, double Fraction);

internal static class MapOverlayJson
{
    private const int CrowdedItems = 40;
    private const int ReturnToLaunch = 20;
    private const int HomeItem = 0;
    private const int RingSegments = 64;
    private const double EarthRadiusMetres = 6_371_000;

    private sealed record Item(
        int Index, int Sequence, Geo At, Geo? Exit, string Kind, int Command,
        bool Routed, bool LegBroken, bool ClosesRoute, bool Complex, int Folded, string Abbreviation);

    internal static Geo? Plottable(JsonNode? node) =>
        node is JsonObject point
        && MapUiJson.Number(point["latitude"]) is double latitude
        && MapUiJson.Number(point["longitude"]) is double longitude
        && (latitude != 0 || longitude != 0)
        && Math.Abs(latitude) <= 90
        && Math.Abs(longitude) <= 180
            ? new Geo(latitude, longitude)
            : null;

    internal static FlownMission Mission(JsonObject view)
    {
        var elements = (view["items"] as JsonArray ?? new JsonArray())
            .Select(node => node as JsonObject ?? new JsonObject())
            .ToList();
        var returns = elements.FindIndex(element => EndsRoute(element) && Command(element) == ReturnToLaunch);
        var endsAfter = returns < 0 ? int.MaxValue : returns;
        var flown = Enumerable.Range(0, elements.Count).Where(index => MapUiJson.Flag(elements[index]["flownLeg"])).ToList();
        var broken = flown.Skip(1)
            .Where((_, at) => EndsRoute(elements[flown[at]]) && Command(elements[flown[at]]) != ReturnToLaunch)
            .ToHashSet();
        var items = elements
            .Select((element, index) => (Element: element, Index: index, At: Plottable(element["coordinate"])))
            .Where(read => read.At != null)
            .Select(read => new Item(
                read.Index,
                (int)(MapUiJson.Number(read.Element["sequence"]) ?? read.Index),
                read.At!.Value,
                Plottable(read.Element["exitCoordinate"]),
                MapUiJson.Text(read.Element["kind"]),
                Command(read.Element),
                MapUiJson.Flag(read.Element["flownLeg"]) && read.Index <= endsAfter,
                broken.Contains(read.Index),
                MapUiJson.Flag(read.Element["closesRoute"]),
                !MapUiJson.Flag(read.Element["simple"], true),
                (int)(MapUiJson.Number(read.Element["foldedCommands"]) ?? 0),
                MapUiJson.Text(read.Element["abbreviation"])))
            .ToList();

        var selected = MapUiJson.Number(view["selected"]) is double chosen && chosen > 0 ? (int)chosen : -1;
        var crowded = items.Count > CrowdedItems;
        var pins = items.Where(item => item.Index != HomeItem)
            .Select(item => Pin(item, item.At, item.Sequence, item.Complex ? string.Empty : item.Abbreviation, crowded, selected))
            .Concat(items.Where(item => item.Complex && item.Exit != null)
                .Select(item => Pin(item, item.Exit!.Value, item.Sequence + item.Folded, string.Empty, crowded, selected)))
            .ToList();

        return new FlownMission(Route(items, MapUiJson.Flag(view["linksStartToHome"])), pins);
    }

    private static IReadOnlyList<IReadOnlyList<Geo>> Route(IReadOnlyList<Item> items, bool linksStartToHome)
    {
        var legs = items.Where(item => item.Routed && (linksStartToHome || item.Index != HomeItem)).ToList();
        var home = items.FirstOrDefault(item => item.Index == HomeItem && item.ClosesRoute);
        var route = home != null && legs.Any(item => item.Index != HomeItem) ? legs.Append(home with { Exit = null }) : legs;
        return route
            .Aggregate(ImmutableList<ImmutableList<Item>>.Empty, (runs, item) =>
                item.LegBroken || runs.IsEmpty ? runs.Add(ImmutableList.Create(item)) : runs.SetItem(runs.Count - 1, runs[^1].Add(item)))
            .Select(run => (IReadOnlyList<Geo>)run.SelectMany(item => item.Exit is { } exit ? new[] { item.At, exit } : new[] { item.At }).ToList())
            .Where(line => line.Count >= 2)
            .ToList();
    }

    private static MissionPin Pin(Item item, Geo at, int sequence, string letters, bool crowded, int selected)
    {
        var lettered = letters.Length > 0 && letters[0] > 'A' && letters[0] < 'z';
        var current = item.Index == selected;
        return new MissionPin(
            at,
            crowded ? string.Empty : lettered ? letters[..1] : sequence.ToString(CultureInfo.InvariantCulture),
            !crowded && lettered && letters.Length > 1 ? letters : string.Empty,
            item.Kind,
            item.Command,
            current,
            crowded && !current);
    }

    private static bool EndsRoute(JsonObject element) => MapUiJson.Flag(element["endsRoute"]);

    private static int Command(JsonObject element) => (int)(MapUiJson.Number(element["command"]) ?? 0);

    internal static FlownFences Fences(JsonObject view) => new(
        MapUiJson.Objects(view["polygons"])
            .Select(polygon => new FenceArea(Points(polygon["vertices"]), MapUiJson.Flag(polygon["inclusion"], true)))
            .Where(area => area.Ring.Count >= 3)
            .Concat(MapUiJson.Objects(view["circles"])
                .Select(circle => (Centre: Plottable(circle["centre"]), Radius: MapUiJson.Number(circle["radiusMetres"]) ?? 0, KeepIn: MapUiJson.Flag(circle["inclusion"], true)))
                .Where(circle => circle.Centre != null && circle.Radius > 0)
                .Select(circle => new FenceArea(Ring(circle.Centre!.Value, circle.Radius), circle.KeepIn)))
            .ToList(),
        view["firmwareFence"] is JsonObject enforced
            && Plottable(enforced["centre"]) is { } centre
            && MapUiJson.Number(enforced["radiusMetres"]) is double radius
                ? Ring(centre, radius)
                : [],
        Points(view["rallyPoints"]));

    internal static IReadOnlyList<Geo> Track(JsonObject view) =>
        MapUiJson.Flag(view["available"]) ? Points(view["points"]) : [];

    internal static IReadOnlyList<TrafficMark> Traffic(JsonObject view)
    {
        var unit = MapUiJson.Text((view["units"] as JsonObject)?["altitude"]);
        return MapUiJson.Objects(view["contacts"])
            .Select(contact => (Contact: contact, At: Plottable(contact)))
            .Where(seen => seen.At != null)
            .Select(seen => new TrafficMark(
                seen.At!.Value,
                MapUiJson.Number(seen.Contact["headingDegrees"]),
                MapUiJson.Flag(seen.Contact["alert"]),
                MapUiJson.Flag(seen.Contact["stale"]),
                MapUiJson.Number(seen.Contact["altitude"]) is double altitude
                    ? FormattableString.Invariant($"{altitude:F0} {unit}\n{MapUiJson.Text(seen.Contact["callsign"]).Trim()}").TrimEnd()
                    : string.Empty))
            .ToList();
    }

    internal static CircleMark? GoTo(JsonObject mapClick) =>
        mapClick["gotoLocation"] is JsonObject location
        && !MapUiJson.Flag(location["arrived"])
        && Plottable(location) is { } at
            ? new CircleMark(at, MapUiJson.Number(location["loiterRadiusMetres"]) is double radius ? Ring(at, radius) : [])
            : null;

    internal static Geo? Roi(JsonObject guided) => Plottable(guided["roi"]);

    internal static CircleMark? Orbit(JsonObject view) =>
        MapUiJson.Flag(view["orbiting"])
        && Plottable(view["centre"]) is { } centre
        && MapUiJson.Number(view["radiusMetres"]) is double radius
        && radius > 0
            ? new CircleMark(centre, Ring(centre, radius))
            : null;

    internal static MapScale? Scale(JsonObject view) =>
        MapUiJson.Flag(view["available"])
        && MapUiJson.Number(view["fraction"]) is double fraction
        && fraction > 0
        && MapUiJson.Text(view["text"]) is { Length: > 0 } text
            ? new MapScale(text, fraction)
            : null;

    internal static IReadOnlyList<Geo> Ring(Geo centre, double metres) =>
        metres > 0 && double.IsFinite(metres)
            ? Enumerable.Range(0, RingSegments).Select(step => PointAt(centre, metres, 360.0 * step / RingSegments)).ToList()
            : [];

    private static IReadOnlyList<Geo> Points(JsonNode? node) =>
        MapUiJson.Objects(node).Select(Plottable).OfType<Geo>().ToList();

    internal static Geo PointAt(Geo centre, double metres, double bearingDegrees)
    {
        var angular = metres / EarthRadiusMetres;
        var latitude = Radians(centre.Latitude);
        var bearing = Radians(bearingDegrees);
        var pointLatitude = Math.Asin(Math.Sin(latitude) * Math.Cos(angular) + Math.Cos(latitude) * Math.Sin(angular) * Math.Cos(bearing));
        var pointLongitude = Radians(centre.Longitude) + Math.Atan2(
            Math.Sin(bearing) * Math.Sin(angular) * Math.Cos(latitude),
            Math.Cos(angular) - Math.Sin(latitude) * Math.Sin(pointLatitude));
        return new Geo(pointLatitude * 180 / Math.PI, pointLongitude * 180 / Math.PI);
    }

    private static double Radians(double degrees) => degrees * Math.PI / 180;
}
