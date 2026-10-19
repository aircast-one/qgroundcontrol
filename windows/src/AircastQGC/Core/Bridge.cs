using System;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace AircastQGC.Core;

public static class Bridge
{
    private static JsonObject Decode(string raw)
    {
        if (string.IsNullOrEmpty(raw)) return new JsonObject();
        try { return JsonNode.Parse(raw) as JsonObject ?? new JsonObject(); }
        catch (JsonException) { return new JsonObject(); }
    }

    public static JsonObject Get(string path) =>
        Decode(NativeMethods.Take(NativeMethods.qgc_bridge_get(path)));

    public static JsonObject GetFields(string path, params string[] fields) =>
        Decode(NativeMethods.Take(NativeMethods.qgc_bridge_get_fields(path, string.Join(",", fields))));

    public static JsonObject Group(string path) => Get(path);

    public static JsonObject Set(string path, JsonNode? value)
    {
        var payload = new JsonObject { ["value"] = value?.DeepClone() };
        return Decode(NativeMethods.Take(NativeMethods.qgc_bridge_set(path, payload.ToJsonString())));
    }

    public static JsonObject Invoke(string path, JsonArray? args = null) =>
        Decode(NativeMethods.Take(NativeMethods.qgc_bridge_invoke(path, (args ?? new JsonArray()).ToJsonString())));

    public static string Raw(string path) => NativeMethods.Take(NativeMethods.qgc_bridge_get(path));
}
