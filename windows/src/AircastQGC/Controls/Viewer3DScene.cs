using System;
using System.Collections.Generic;
using System.Linq;
using System.Numerics;
using System.Text.Json.Nodes;

namespace AircastQGC.Controls;

internal readonly record struct LonLatAlt(double Lon, double Lat, double Alt);

internal sealed record Building3D(IReadOnlyList<IReadOnlyList<(double Lon, double Lat)>> Outer, IReadOnlyList<IReadOnlyList<(double Lon, double Lat)>> Inner, double Height);

internal sealed record Scene3D(bool? Enabled, bool Available, string Reason, IReadOnlyList<Building3D> Buildings, (double Lon, double Lat)? Centre)
{
    public static readonly Scene3D None = new(null, false, string.Empty, [], null);
}

internal sealed record PathMarker3D(int Mission, LonLatAlt At, string Label, string Colour);

internal sealed record PathSegment3D(LonLatAlt From, LonLatAlt To, string Colour);

internal sealed record Path3D(IReadOnlyList<PathMarker3D> Markers, IReadOnlyList<PathSegment3D> Segments)
{
    public static readonly Path3D None = new([], []);
}

internal sealed record Vehicle3D(LonLatAlt At, double Heading);

internal sealed record Palette3D(uint Sky, uint Ground, uint Grid, uint Building, uint Track, uint Path, uint Return, uint Takeoff, uint Waypoint, uint Selected, uint FrontArm, uint RearArm, uint Rotor, uint Hub)
{
    public uint Named(string colour) => colour switch
    {
        "red" => Return,
        "green" => Takeoff,
        "black" => Waypoint,
        _ => Path,
    };
}

internal sealed record Ground3D(double OriginX, double OriginY, double PixelsPerMetre, Func<int, int, int, bool, byte[]?> Tile)
{
    public const int TileSize = 256;
    private const double EarthMetres = 40_075_016.686;

    public static Ground3D At(Origin3D origin, Func<int, int, int, bool, byte[]?> tile)
    {
        var lat = origin.Lat * Math.PI / 180;
        return new Ground3D(
            (origin.Lon + 180) / 360 * TileSize,
            (1 - Math.Log(Math.Tan(lat) + 1 / Math.Cos(lat)) / Math.PI) / 2 * TileSize,
            TileSize / (EarthMetres * Math.Cos(lat)),
            tile);
    }
}

internal sealed record Face3D(Vector3[] Points, int[] Ends, Vector3 Normal, Vector3 Centre, uint Colour);

internal sealed record Prism3D(IReadOnlyList<Vector2[]> Outer, IReadOnlyList<Vector2[]> Inner, float Base, float Top, uint Colour);

internal readonly record struct Origin3D(double Lon, double Lat)
{
    private const double MetresPerDegree = 111_320.0;

    private double Across => MetresPerDegree * Math.Cos(Lat * Math.PI / 180);

    public Vector2 Metres(double lon, double lat) => new((float)((lon - Lon) * Across), (float)((lat - Lat) * MetresPerDegree));

    public Vector3 Metres(LonLatAlt at) => new(Metres(at.Lon, at.Lat), (float)at.Alt);
}

internal readonly record struct Camera3D(Vector3 Target, float Bearing, float Tilt, float Distance)
{
    public const float FieldOfView = 36.87f;
    public const float LowestTilt = 0;
    public const float HighestTilt = 85;
    public const float Nearest = 15;
    public const float Farthest = 30_000;

    private static float Radians(float degrees) => degrees * MathF.PI / 180;

    public Vector3 Forward => new(MathF.Sin(Radians(Tilt)) * MathF.Sin(Radians(Bearing)), MathF.Sin(Radians(Tilt)) * MathF.Cos(Radians(Bearing)), -MathF.Cos(Radians(Tilt)));

    public Vector3 Right => new(MathF.Cos(Radians(Bearing)), -MathF.Sin(Radians(Bearing)), 0);

    public Vector3 Up => Vector3.Cross(Right, Forward);

    public Vector3 Eye => Target - Forward * Distance;

    public Camera3D Orbited(float turn, float lean) =>
        this with { Bearing = ((Bearing + turn) % 360 + 360) % 360, Tilt = Math.Clamp(Tilt + lean, LowestTilt, HighestTilt) };

    public Camera3D Zoomed(float factor) => this with { Distance = Math.Clamp(Distance * factor, Nearest, Farthest) };

    public Camera3D Panned(Vector2 screenDelta, float focal)
    {
        var metres = Distance / focal;
        var ahead = new Vector3(MathF.Sin(Radians(Bearing)), MathF.Cos(Radians(Bearing)), 0);
        return this with { Target = Target - Right * screenDelta.X * metres + ahead * screenDelta.Y * metres / MathF.Max(MathF.Cos(Radians(Tilt)), 0.2f) };
    }

    public static float Focal(int height) => height / 2f / MathF.Tan(Radians(FieldOfView) / 2);
}

internal static class Scene3DJson
{
    private static LonLatAlt? Point(JsonNode? node) =>
        node is JsonArray { Count: 3 } array && MapUiJson.Number(array[0]) is double lon && MapUiJson.Number(array[1]) is double lat && MapUiJson.Number(array[2]) is double alt
            ? new LonLatAlt(lon, lat, alt)
            : null;

    private static IReadOnlyList<IReadOnlyList<(double Lon, double Lat)>> Rings(JsonNode? node) =>
        node is JsonArray rings
            ? rings.OfType<JsonArray>()
                .Select(ring => (IReadOnlyList<(double Lon, double Lat)>)ring.OfType<JsonArray>()
                    .Select(at => (Lon: MapUiJson.Number(at.FirstOrDefault()), Lat: MapUiJson.Number(at.ElementAtOrDefault(1))))
                    .Where(at => at.Lon is not null && at.Lat is not null)
                    .Select(at => (at.Lon!.Value, at.Lat!.Value))
                    .ToList())
                .Where(ring => ring.Count > 2)
                .ToList()
            : [];

    public static Scene3D Scene(JsonObject view) => new(
        view.ContainsKey("enabled") ? MapUiJson.Flag(view["enabled"]) : null,
        MapUiJson.Flag(view["available"]),
        MapUiJson.Text(view["reason"]),
        MapUiJson.Objects(view["buildings"])
            .Select(building => new Building3D(Rings(building["outer"]), Rings(building["inner"]), MapUiJson.Number(building["height"]) ?? 0))
            .Where(building => building.Outer.Count > 0 && building.Height > 0)
            .ToList(),
        view["bounds"] is JsonObject bounds && MapUiJson.Number(bounds["west"]) is double west && MapUiJson.Number(bounds["east"]) is double east
            && MapUiJson.Number(bounds["south"]) is double south && MapUiJson.Number(bounds["north"]) is double north
                ? ((west + east) / 2, (south + north) / 2)
                : null);

    public static Path3D Path(JsonObject view) => new(
        MapUiJson.Objects(view["markers"])
            .Select(marker => (Marker: marker, At: Point(marker["at"])))
            .Where(seen => seen.At is not null)
            .Select(seen => new PathMarker3D((int)(MapUiJson.Number(seen.Marker["mission"]) ?? 0), seen.At!.Value, MapUiJson.Text(seen.Marker["label"]), MapUiJson.Text(seen.Marker["colour"])))
            .ToList(),
        MapUiJson.Objects(view["segments"])
            .Select(segment => (From: Point(segment["from"]), To: Point(segment["to"]), Colour: MapUiJson.Text(segment["colour"])))
            .Where(seen => seen.From is not null && seen.To is not null)
            .Select(seen => new PathSegment3D(seen.From!.Value, seen.To!.Value, seen.Colour))
            .ToList());

    public static IReadOnlyList<Vehicle3D> Vehicles(JsonObject view) =>
        MapUiJson.Objects(view["vehicles"])
            .Select(vehicle => (At: Point(vehicle["at"]), Heading: MapUiJson.Number(vehicle["heading"]) ?? 0))
            .Where(seen => seen.At is not null)
            .Select(seen => new Vehicle3D(seen.At!.Value, seen.Heading))
            .ToList();
}

internal static class Geometry3D
{
    private const float RibbonWidth = 1.5f;
    private const float RibbonThickness = 1;
    private const float RibbonStep = 8;
    private const int RibbonMaxPieces = 64;
    private const float MarkerSize = 3;
    private const float LabelLift = 3;
    private const float ArmLength = 3;
    private const float ArmWidth = 0.5f;
    private const float RotorSize = 1.6f;
    private const float HubSize = 1.4f;
    private const float TrackLift = 0.3f;
    private const float StalkWidth = 0.3f;
    private const float MinimumPixels = 3;
    private const float MarkerPixels = 12;
    private const float VehiclePixels = 36;

    private static Vector2 Flat(Vector3 at) => new(at.X, at.Y);

    private static float SignedArea(Vector2[] ring) =>
        ring.Select((a, at) => (a, b: ring[(at + 1) % ring.Length])).Sum(edge => edge.a.X * edge.b.Y - edge.b.X * edge.a.Y) / 2;

    private static Vector2[] Wound(Vector2[] ring, bool counterClockwise) =>
        SignedArea(ring) > 0 == counterClockwise ? ring : ring.Reverse().ToArray();

    private static Vector2[] Open(IEnumerable<Vector2> ring)
    {
        var points = ring.ToArray();
        return points.Length > 1 && points[0] == points[^1] ? points[..^1] : points;
    }

    public static IEnumerable<Face3D> Faces(Prism3D prism)
    {
        var rings = prism.Outer.Select(ring => Wound(Open(ring), true)).Concat(prism.Inner.Select(ring => Wound(Open(ring), false)))
            .Where(ring => ring.Length > 2)
            .ToList();
        if (rings.Count == 0) return [];
        var walls = prism.Top - prism.Base > 0.01f
            ? rings.SelectMany(ring => ring.Select((a, at) => (a, b: ring[(at + 1) % ring.Length])))
                .Where(edge => Vector2.DistanceSquared(edge.a, edge.b) > 1e-6f)
                .Select(edge => Wall(edge.a, edge.b, prism.Base, prism.Top, prism.Colour))
            : [];
        var top = Cap(rings, prism.Top, Vector3.UnitZ, prism.Colour);
        var bottom = prism.Base > 0.05f ? [Cap(rings, prism.Base, -Vector3.UnitZ, prism.Colour)] : Array.Empty<Face3D>();
        return walls.Append(top).Concat(bottom);
    }

    private static Face3D Wall(Vector2 a, Vector2 b, float bottom, float top, uint colour)
    {
        var along = Vector2.Normalize(b - a);
        Vector3[] points = [new(a, bottom), new(b, bottom), new(b, top), new(a, top)];
        return new Face3D(points, [4], new Vector3(along.Y, -along.X, 0), new Vector3((a + b) / 2, (bottom + top) / 2), colour);
    }

    private static Face3D Cap(IReadOnlyList<Vector2[]> rings, float height, Vector3 normal, uint colour)
    {
        var points = rings.SelectMany(ring => ring.Select(at => new Vector3(at, height))).ToArray();
        var ends = rings.Select((_, index) => rings.Take(index + 1).Sum(ring => ring.Length)).ToArray();
        var centre = points.Aggregate(Vector3.Zero, (sum, at) => sum + at) / points.Length;
        return new Face3D(points, ends, normal, centre, colour);
    }

    public static IEnumerable<Prism3D> Buildings(IEnumerable<Building3D> buildings, Origin3D origin, uint colour) =>
        buildings.Select(building => new Prism3D(
            building.Outer.Select(ring => ring.Select(at => origin.Metres(at.Lon, at.Lat)).ToArray()).ToList(),
            building.Inner.Select(ring => ring.Select(at => origin.Metres(at.Lon, at.Lat)).ToArray()).ToList(),
            0, (float)building.Height, colour));

    private static Prism3D Box(Vector3 at, float size, uint colour, float height) =>
        Slab([Flat(at) + new Vector2(-size, -size) / 2, Flat(at) + new Vector2(size, -size) / 2, Flat(at) + new Vector2(size, size) / 2, Flat(at) + new Vector2(-size, size) / 2],
            at.Z - height / 2, at.Z + height / 2, colour);

    private static Prism3D Slab(Vector2[] corners, float bottom, float top, uint colour) =>
        new([corners], [], MathF.Max(bottom, 0), MathF.Max(top, 0.1f), colour);

    public static IEnumerable<Prism3D> Ribbon(Vector3 from, Vector3 to, uint colour, float width, float thickness)
    {
        var flat = Flat(to) - Flat(from);
        var length = flat.Length();
        if (length < width / 2)
            return [Box(from, width, colour, 0) with { Base = MathF.Max(MathF.Min(from.Z, to.Z) - thickness / 2, 0), Top = MathF.Max(MathF.Max(from.Z, to.Z) + thickness / 2, 0.1f) }];
        var pieces = Math.Clamp((int)MathF.Ceiling(length / RibbonStep), 1, RibbonMaxPieces);
        var climb = MathF.Abs(to.Z - from.Z) / pieces;
        var across = new Vector2(-flat.Y, flat.X) / length * width / 2;
        var half = MathF.Max(thickness, climb) / 2;
        return Enumerable.Range(0, pieces).Select(piece =>
        {
            var (t0, t1) = ((float)piece / pieces, (float)(piece + 1) / pieces);
            var alt = from.Z + (to.Z - from.Z) * (t0 + t1) / 2;
            var (a, b) = (Flat(from) + flat * t0, Flat(from) + flat * t1);
            return Slab([a + across, b + across, b - across, a - across], alt - half, alt + half, colour);
        });
    }

    private static float Scale(float metresPerPixel, float pixels, float metres) => MathF.Max(1, pixels * metresPerPixel / metres);

    public static IEnumerable<Prism3D> Path(Path3D path, Origin3D origin, Palette3D palette, ISet<int> selected, float metresPerPixel)
    {
        var width = MathF.Max(RibbonWidth, MinimumPixels * metresPerPixel);
        var marker = MarkerSize * Scale(metresPerPixel, MarkerPixels, MarkerSize);
        return path.Segments
            .SelectMany(segment => Ribbon(origin.Metres(segment.From), origin.Metres(segment.To), palette.Named(segment.Colour), width, MathF.Max(RibbonThickness, width * 2 / 3)))
            .Concat(path.Markers.Select((at, index) => Box(origin.Metres(at.At), marker, selected.Contains(index) ? palette.Selected : palette.Named(at.Colour), marker)));
    }

    public static IEnumerable<(Vector3 At, string Text)> Labels(Path3D path, Origin3D origin) =>
        path.Markers.Where(marker => !string.IsNullOrWhiteSpace(marker.Label))
            .Select(marker => (origin.Metres(marker.At) + Vector3.UnitZ * LabelLift, marker.Label));

    public static IEnumerable<Prism3D> Track(IReadOnlyList<(double Lon, double Lat)> track, Origin3D origin, uint colour, float metresPerPixel)
    {
        var width = MathF.Max(1, MinimumPixels * metresPerPixel);
        return track.Zip(track.Skip(1))
            .Select(leg => (From: new Vector3(origin.Metres(leg.First.Lon, leg.First.Lat), TrackLift), To: new Vector3(origin.Metres(leg.Second.Lon, leg.Second.Lat), TrackLift)))
            .Where(leg => Vector2.Distance(Flat(leg.From), Flat(leg.To)) > 0.1f)
            .Select(leg => Slab(Corners(leg.From, leg.To, width), 0, TrackLift, colour));
    }

    private static Vector2[] Corners(Vector3 from, Vector3 to, float width)
    {
        var flat = Flat(to) - Flat(from);
        var across = new Vector2(-flat.Y, flat.X) / flat.Length() * width / 2;
        return [Flat(from) + across, Flat(to) + across, Flat(to) - across, Flat(from) - across];
    }

    public static IEnumerable<Prism3D> Vehicle(Vehicle3D vehicle, Origin3D origin, Palette3D palette, float metresPerPixel)
    {
        var at = origin.Metres(vehicle.At);
        var scale = Scale(metresPerPixel, VehiclePixels, ArmLength * 2);
        var heading = (float)(vehicle.Heading * Math.PI / 180);
        var arms = new[] { 45f, -45f, 135f, -135f }.SelectMany(arm =>
        {
            var bearing = heading + arm * MathF.PI / 180;
            var motor = at + new Vector3(MathF.Sin(bearing), MathF.Cos(bearing), 0) * ArmLength * scale;
            var colour = MathF.Abs(arm) < 90 ? palette.FrontArm : palette.RearArm;
            return Ribbon(at, motor, colour, ArmWidth * scale, ArmWidth * scale)
                .Append(Box(motor + Vector3.UnitZ * ArmWidth * scale, RotorSize * scale, palette.Rotor, ArmWidth / 2 * scale));
        });
        var stalk = at.Z > 1 ? [Box(at with { Z = at.Z / 2 }, MathF.Max(StalkWidth, metresPerPixel), palette.Hub, at.Z)] : Array.Empty<Prism3D>();
        return arms.Append(Box(at, HubSize * scale, palette.Hub, HubSize / 2 * scale)).Concat(stalk);
    }
}
