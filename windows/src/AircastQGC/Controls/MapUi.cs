using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using Windows.Foundation;

namespace AircastQGC.Controls;

public sealed class MapPointEventArgs(double latitude, double longitude) : EventArgs
{
    public double Latitude { get; } = latitude;
    public double Longitude { get; } = longitude;
}

public sealed record PictureGround(double Latitude, double Longitude, IReadOnlyList<string> Actions, string Warning, Point At);

public sealed record OrbitPreview(double Latitude, double Longitude, double RadiusMetres);

public sealed record MapClickAction(string Kind, string Path, string Label, string Title, string Message, bool Confirm)
{
    public string Glyph => Kind switch
    {
        "GoTo" => "",
        "Orbit" => "",
        "Roi" => "",
        "SetHome" => "",
        "Heading" => "",
        "EstimatorOrigin" => "",
        _ => string.Empty,
    };

    public override string ToString() => Label;
}

internal static class MapUiJson
{
    internal static double? Number(JsonNode? node) =>
        node is JsonValue value && value.TryGetValue<double>(out var number) && double.IsFinite(number) ? number : null;

    internal static string Text(JsonNode? node) =>
        node is JsonValue value && value.TryGetValue<string>(out var text) ? text : string.Empty;

    internal static bool Flag(JsonNode? node, bool fallback = false) =>
        node is JsonValue value && value.TryGetValue<bool>(out var flag) ? flag : fallback;

    internal static IReadOnlyList<JsonObject> Objects(JsonNode? node) =>
        node is JsonArray array ? array.OfType<JsonObject>().ToList() : [];

    internal static IReadOnlyList<string> Texts(JsonNode? node) =>
        node is JsonArray array ? array.Select(Text).Where(text => text.Length > 0).ToList() : [];

    internal static string SentenceCase(string text) =>
        string.Join(' ', text.Split(' ').Select((word, at) =>
            at == 0 || word.Skip(1).Any(char.IsUpper) || !word.Any(char.IsLower)
                ? word
                : char.ToLowerInvariant(word[0]) + word[1..]));

    internal static string? RefusalOf(JsonObject answer) =>
        Flag(answer["ok"])
            ? null
            : Text(answer["reason"]).Trim() is { Length: > 0 } reason ? reason : "The vehicle refused.";

    internal static (double Latitude, double Longitude)? VehiclePosition()
    {
        var coordinate = Bridge.Get("vehicle.coordinate");
        return Flag(coordinate["valid"], true)
            && Number(coordinate["latitude"]) is double latitude
            && Number(coordinate["longitude"]) is double longitude
            && (latitude != 0 || longitude != 0)
                ? (latitude, longitude)
                : null;
    }
}
