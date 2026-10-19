using System.Text.Json.Nodes;
using Microsoft.UI.Xaml;

namespace AircastQGC.Controls;

internal static class VideoJson
{
    internal static bool Flag(JsonObject o, string key, bool fallback = false) =>
        o[key] is JsonValue v && v.TryGetValue<bool>(out var b) ? b : fallback;

    internal static string Text(JsonObject o, string key) =>
        o[key] is JsonValue v && v.TryGetValue<string>(out var s) ? s : "";

    internal static double? Number(JsonNode? node) =>
        node is JsonValue v && v.TryGetValue<double>(out var d) && double.IsFinite(d) ? d : null;

    internal static int Int(JsonObject o, string key, int fallback) =>
        Number(o[key]) is double d ? (int)d : fallback;

    internal static string Or(string text, string fallback) =>
        string.IsNullOrEmpty(text) ? fallback : text;

    internal static Visibility Shown(bool shown) =>
        shown ? Visibility.Visible : Visibility.Collapsed;

    internal static string? Refusal(JsonObject answer, string unanswered) =>
        answer["ok"] is JsonValue v && v.TryGetValue<bool>(out var ok)
            ? ok ? null : Or(Text(answer, "reason"), unanswered)
            : unanswered;
}
