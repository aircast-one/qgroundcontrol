using System.Collections.Generic;
using System.Collections.Immutable;
using System.Linq;
using System.Text.Json.Nodes;

namespace AircastQGC.Controls;

internal static class PreflightJson
{
    public static string Text(JsonNode? node) =>
        node is JsonValue value && value.TryGetValue<string>(out var text) ? text : "";

    public static bool Flag(JsonNode? node, bool fallback = false) =>
        node is JsonValue value && value.TryGetValue<bool>(out var flag) ? flag : fallback;
}

internal sealed record PreflightCheck(string Name, string Prompt, string Verdict, string Reason)
{
    public const string BlockedWithoutReason = "Fix this before it can be checked off";

    private static readonly string[] Verdicts = ["manual", "passing", "failing", "overridable"];

    public bool AlreadyMet => Verdict == "passing";

    public bool Blocks => Verdict == "failing";

    public bool Warns => Verdict == "overridable";

    public bool Tickable => !AlreadyMet && !Blocks;

    public bool Met(ImmutableHashSet<string> ticked) => !Blocks && (AlreadyMet || ticked.Contains(Name));

    public string Hint =>
        Blocks ? (Reason.Length == 0 ? BlockedWithoutReason : Reason)
        : Warns ? $"{Reason} Check it off to fly anyway."
        : Prompt;

    public static PreflightCheck? From(JsonObject json) =>
        PreflightJson.Text(json["name"]) is { Length: > 0 } name
        && json["verdict"] is JsonValue verdict
        && verdict.TryGetValue<string>(out var raw)
            ? new PreflightCheck(
                name,
                PreflightJson.Text(json["prompt"]),
                Verdicts.Contains(raw) ? raw : "manual",
                PreflightJson.Text(json["reason"]))
            : null;
}

internal sealed record PreflightGroup(string Name, IReadOnlyList<PreflightCheck> Checks)
{
    public static PreflightGroup? From(JsonObject json) =>
        PreflightJson.Text(json["name"]) is { Length: > 0 } name
            ? new PreflightGroup(
                name,
                json["checks"] is JsonArray checks
                    ? checks.OfType<JsonObject>().Select(PreflightCheck.From).OfType<PreflightCheck>().ToList()
                    : [])
            : null;

    public static IReadOnlyList<PreflightGroup> Parse(JsonNode? node) =>
        node is JsonArray groups
            ? groups.OfType<JsonObject>().Select(From).OfType<PreflightGroup>().ToList()
            : [];
}
