using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;

namespace AircastQGC.Pages.Device;

internal sealed record Badge(int Level, string Text);

internal sealed record Option(string Value, string Label);

internal sealed record Chart(string Label, IReadOnlyList<double> Values, int Max, string Value, int Level);

internal sealed record PageLink(string Title, string Href, string Icon, string Summary, bool Needs);

internal sealed record Endpoint(string Proto, string Label, string Url, string Hint, string Note, bool Down);

internal static class DJ
{
    public static string Text(JsonObject? json, string key) => MapUiJson.Text(json?[key]);

    public static int Int(JsonObject? json, string key, int fallback = 0) => (int)(MapUiJson.Number(json?[key]) ?? fallback);

    public static double Number(JsonObject? json, string key, double fallback = 0) => MapUiJson.Number(json?[key]) ?? fallback;

    public static bool Flag(JsonObject? json, string key) => MapUiJson.Flag(json?[key]);

    public static JsonObject? Obj(JsonObject? json, string key) => json?[key] as JsonObject;

    public static JsonObject Section(JsonObject? json, string key) => Obj(json, key) ?? new JsonObject();

    public static IReadOnlyList<JsonObject> Objs(JsonObject? json, string key) => MapUiJson.Objects(json?[key]);

    public static IReadOnlyList<string> Texts(JsonObject? json, string key) => MapUiJson.Texts(json?[key]);

    public static Badge Badge(JsonObject? json) => new(Int(json, "level"), Text(json, "text"));

    public static Badge Badge(JsonObject? json, string key) => Badge(Obj(json, key));

    public static Badge? MaybeBadge(JsonObject? json, string key) => Obj(json, key) is { } badge ? Badge(badge) : null;

    public static IReadOnlyList<Badge> Badges(JsonObject? json, string key) => Objs(json, key).Select(Badge).ToList();

    public static IReadOnlyList<Option> Options(JsonObject? json, string key) =>
        Objs(json, key).Select(option => new Option(Text(option, "value"), Text(option, "label"))).ToList();

    public static Chart? Chart(JsonObject? json) => json is null ? null : new Chart(
        Text(json, "label"),
        (json["values"] as JsonArray ?? []).Select(MapUiJson.Number).OfType<double>().ToList(),
        Int(json, "max", 100),
        Text(json, "value"),
        Int(json, "level", DeviceLevel.Neutral));

    public static IReadOnlyList<Chart> Charts(JsonObject? json, string key) => Objs(json, key).Select(Chart).OfType<Chart>().ToList();

    public static PageLink Page(JsonObject? json) =>
        new(Text(json, "title"), Text(json, "href"), Text(json, "icon"), Text(json, "summary"), Flag(json, "needs"));

    public static IReadOnlyList<Endpoint> Endpoints(JsonObject? json, string key) =>
        Objs(json, key).Select(e => new Endpoint(Text(e, "proto"), Text(e, "label"), Text(e, "url"), Text(e, "hint"), Text(e, "note"), Flag(e, "down"))).ToList();

    public static string Sig(JsonNode? node) => node?.ToJsonString() ?? "";

    public static string Sig(params object?[] parts) => string.Join("\u001f", parts.Select(part => part switch
    {
        JsonNode node => node.ToJsonString(),
        null => "",
        _ => part.ToString(),
    }));
}
