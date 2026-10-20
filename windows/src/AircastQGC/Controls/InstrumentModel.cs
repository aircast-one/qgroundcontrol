using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace AircastQGC.Controls;

internal static class InstrumentJson
{
    public static string Text(JsonNode? node, string key) =>
        node is JsonObject o && o[key] is JsonValue v && v.TryGetValue<string>(out var s) ? s : "";

    public static double? Number(JsonNode? node, string key) =>
        node is JsonObject o && o[key] is JsonValue v && v.TryGetValue<double>(out var d) && double.IsFinite(d) ? d : null;

    public static bool Flag(JsonNode? node, string key) =>
        node is JsonObject o && o[key] is JsonValue v && v.TryGetValue<bool>(out var b) && b;

    public static IEnumerable<JsonObject> Objects(JsonNode? node) =>
        node is JsonArray a ? a.OfType<JsonObject>() : Enumerable.Empty<JsonObject>();
}

public sealed record InstrumentSelection(string Group, string FactName)
{
    public const string VehicleGroup = "";
    public const char Separator = '/';

    public string Id => $"{Group}{Separator}{FactName}";

    public string Stored => Id;

    public static InstrumentSelection Vehicle(string factName) => new(VehicleGroup, factName);

    public static readonly IReadOnlyList<InstrumentSelection> Defaults = new[]
    {
        Vehicle("altitudeRelative"),
        Vehicle("groundSpeed"),
        Vehicle("climbRate"),
        Vehicle("distanceToHome"),
        Vehicle("heading"),
        Vehicle("altitudeAMSL"),
    };

    public static InstrumentSelection? Decode(string stored)
    {
        var slash = stored.IndexOf(Separator);
        var factName = slash < 0 ? "" : stored[(slash + 1)..];
        return factName.Length == 0 ? null : new InstrumentSelection(stored[..slash], factName);
    }

    public static IReadOnlyList<InstrumentSelection> Restore(IEnumerable<string>? stored)
    {
        var listed = (stored ?? Enumerable.Empty<string>()).Select(Decode).OfType<InstrumentSelection>().ToList();
        return listed.Count == 0 ? Defaults : listed;
    }

    public static InstrumentSelection FirstUnused(IEnumerable<InstrumentSelection> used)
    {
        var taken = used.Select(s => s.Id).ToHashSet();
        return Defaults.FirstOrDefault(d => !taken.Contains(d.Id)) ?? Defaults[0];
    }
}

public sealed record InstrumentFact(string Name, string Label);

public sealed record InstrumentGroup(string Group, string Title, IReadOnlyList<InstrumentFact> Facts)
{
    public const string VehicleTitle = "Vehicle";

    private static IReadOnlyList<InstrumentFact> FactsOf(JsonNode? json) =>
        InstrumentJson.Objects(json)
            .Select(f => new InstrumentFact(InstrumentJson.Text(f, "name"), InstrumentJson.Text(f, "label")))
            .Where(f => f.Name.Length > 0)
            .ToList();

    public static IReadOnlyList<InstrumentGroup> Served(JsonNode? json) =>
        InstrumentJson.Objects(json)
            .Select(entry => new InstrumentGroup(
                InstrumentJson.Text(entry, "group"), InstrumentJson.Text(entry, "title"), FactsOf(entry["facts"])))
            .Where(g => g.Group.Length > 0 && g.Facts.Count > 0)
            .ToList();

    public static IReadOnlyList<InstrumentGroup> Own(JsonNode? json)
    {
        var listed = FactsOf(json);
        return listed.Count == 0
            ? Array.Empty<InstrumentGroup>()
            : new[] { new InstrumentGroup(InstrumentSelection.VehicleGroup, VehicleTitle, listed) };
    }

    public static IReadOnlyList<InstrumentGroup> Discovered(JsonObject view) =>
        Own(view["vehicleFacts"]).Concat(Served(view["groups"])).Concat(Served(view["packGroups"])).ToList();
}

public sealed record InstrumentValue(
    string Id, string Group, string Name, string Label, string Value, string Units, bool Missing, string MissingReason,
    double? Raw = null, string DefaultIcon = "")
{
    public const string AbsentText = "not on this aircraft";

    public bool AbsentHere => Missing && MissingReason == "noSuchFact";

    public static InstrumentValue? From(JsonObject json)
    {
        var id = InstrumentJson.Text(json, "id");
        var name = InstrumentJson.Text(json, "name");
        return id.Length == 0 || name.Length == 0
            ? null
            : new InstrumentValue(id, InstrumentJson.Text(json, "group"), name,
                InstrumentJson.Text(json, "label"), InstrumentJson.Text(json, "value"),
                InstrumentJson.Text(json, "units"), InstrumentJson.Flag(json, "missing"),
                InstrumentJson.Text(json, "missingReason"), InstrumentJson.Number(json, "raw"),
                InstrumentJson.Text(json, "defaultIcon"));
    }

    public static IReadOnlyList<InstrumentValue> List(JsonNode? json) =>
        InstrumentJson.Objects(json).Select(From).OfType<InstrumentValue>().ToList();
}

internal static class InstrumentSelectionStore
{
    private static readonly string StorePath = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
        "AircastQGC", "instrument-selections.json");

    internal static bool Recoverable(Exception e) =>
        e is IOException or UnauthorizedAccessException or JsonException or NotSupportedException;

    public static IReadOnlyList<InstrumentSelection> Load()
    {
        try
        {
            var stored = File.Exists(StorePath) ? JsonNode.Parse(File.ReadAllText(StorePath)) as JsonArray : null;
            return InstrumentSelection.Restore(stored?.Select(n => n is JsonValue v && v.TryGetValue<string>(out var s) ? s : ""));
        }
        catch (Exception e) when (Recoverable(e))
        {
            return InstrumentSelection.Defaults;
        }
    }

    public static void Save(IEnumerable<InstrumentSelection> selections)
    {
        try
        {
            Directory.CreateDirectory(Path.GetDirectoryName(StorePath)!);
            File.WriteAllText(StorePath,
                new JsonArray(selections.Select(s => (JsonNode?)JsonValue.Create(s.Stored)).ToArray()).ToJsonString());
        }
        catch (Exception e) when (Recoverable(e))
        {
        }
    }
}

internal static class AttitudeCardStore
{
    private static readonly string StorePath = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
        "AircastQGC", "instrument-attitude-card.json");

    public static bool Load()
    {
        try
        {
            return File.Exists(StorePath) && JsonNode.Parse(File.ReadAllText(StorePath)) is JsonValue v && v.TryGetValue<bool>(out var shown) && shown;
        }
        catch (Exception e) when (InstrumentSelectionStore.Recoverable(e))
        {
            return false;
        }
    }

    public static void Save(bool shown)
    {
        try
        {
            Directory.CreateDirectory(Path.GetDirectoryName(StorePath)!);
            File.WriteAllText(StorePath, JsonValue.Create(shown).ToJsonString());
        }
        catch (Exception e) when (InstrumentSelectionStore.Recoverable(e))
        {
        }
    }
}
