using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;

namespace AircastQGC.Pages.Setup;

internal sealed record ParamEntry(string Key, string Name, string Description, string LongDescription, string Category, string Group, string Value, bool Changed, bool ReadOnly)
{
    public string Path => ParameterModel.PathOf(Key);

    public override string ToString() => $"{Name}, {Value}";
}

internal sealed record ParamCategory(string Name, IReadOnlyList<string> Groups);

internal static class ParameterModel
{
    public const string Manager = "vehicle.parameterManager";
    private const int DefaultComponent = -1;
    private const int AutopilotComponent = 1;
    private const string StandardCategory = "Standard";
    private const string DefaultCategory = "Other";
    private const string DefaultGroup = "Misc";

    public static string PathOf(string key, string? quantity = null)
    {
        var colon = key.IndexOf(':');
        var component = colon > 0 && int.TryParse(key[..colon], NumberStyles.Integer, CultureInfo.InvariantCulture, out var id) ? id : (int?)null;
        var name = component is null ? key : key[(colon + 1)..];
        return $"{Manager}.getParameter({component ?? DefaultComponent},{name}{(quantity is null ? "" : $",{quantity}")})";
    }

    public static string ControlPath(string key) => $"view.control({PathOf(key)})";

    public static IReadOnlyList<string> Keys()
    {
        var components = Ints(FormWrites.Result($"{Manager}.componentIds"));
        var byComponent = components.Count == 0
            ? [(DefaultComponent, Names(DefaultComponent))]
            : components.Select(component => (component, Names(component))).ToList();
        return byComponent
            .SelectMany(pair => pair.Item2.Order(StringComparer.Ordinal)
                .Select(name => pair.Item1 is AutopilotComponent or DefaultComponent ? name : $"{pair.Item1}:{name}"))
            .ToList();
    }

    public static ParamEntry? Read(string key) => Entry(key, Bridge.Get(PathOf(key)));

    public static ParamEntry? Entry(string key, JsonObject fact) =>
        MapUiJson.Text(fact["kind"]) != "fact" || MapUiJson.Text(fact["name"]).Length == 0
            ? null
            : new ParamEntry(
                key,
                MapUiJson.Text(fact["name"]),
                MapUiJson.Text(fact["shortDescription"]),
                MapUiJson.Text(fact["longDescription"]),
                MapUiJson.Text(fact["category"]),
                MapUiJson.Text(fact["group"]),
                ValueText(fact),
                MapUiJson.Flag(fact["defaultValueAvailable"]) && !MapUiJson.Flag(fact["valueEqualsDefault"], true),
                MapUiJson.Flag(fact["readOnly"]));

    public static string ValueText(JsonObject fact)
    {
        var enums = MapUiJson.Texts(fact["enumStrings"]);
        var bitLabels = (fact["bitmaskStrings"] as JsonArray ?? []).Select(MapUiJson.Text).ToList();
        var bitValues = (fact["bitmaskValues"] as JsonArray ?? []).Select(node => (long)(MapUiJson.Number(node) ?? 0)).ToList();
        var valueString = MapUiJson.Text(fact["valueString"]);
        var isBitmask = bitLabels.Count > 0 && bitLabels.Count == bitValues.Count;
        var isEnum = enums.Count > 0 && bitLabels.Count == 0;
        var index = (int)(MapUiJson.Number(fact["enumIndex"]) ?? -1);
        var raw = (long)(MapUiJson.Number(fact["value"]) ?? FormText.Parse(valueString) ?? 0);
        var set = bitLabels.Where((_, at) => (raw & bitValues[at]) != 0).ToList();
        return isBitmask
            ? set.Count == 0 ? "None" : set.Count == bitLabels.Count ? "All" : string.Join(", ", set)
            : string.Join(" ", new[]
            {
                index >= 0 && index < enums.Count ? enums[index] : valueString,
                isEnum ? "" : MapUiJson.Text(fact["units"]),
            }.Where(part => part.Trim().Length > 0));
    }

    public static IReadOnlyList<ParamCategory> Tree(IReadOnlyList<ParamEntry> entries)
    {
        var categories = entries.Select(e => e.Category).Distinct().ToList();
        var ordered = categories.Where(c => c == StandardCategory)
            .Concat(categories.Where(c => c != StandardCategory && c != DefaultCategory))
            .Concat(categories.Where(c => c == DefaultCategory));
        return ordered.Select(category =>
        {
            var groups = entries.Where(e => e.Category == category).Select(e => e.Group).Distinct().ToList();
            return new ParamCategory(category, groups.Where(g => g != DefaultGroup).Concat(groups.Where(g => g == DefaultGroup)).ToList());
        }).ToList();
    }

    public static bool Matches(ParamEntry entry, string search) =>
        search.Split(' ', StringSplitOptions.RemoveEmptyEntries).All(word =>
        {
            var texts = new[] { entry.Key.Contains(':') ? entry.Key[(entry.Key.IndexOf(':') + 1)..] : entry.Key, entry.Description, entry.LongDescription }
                .Where(text => text.Length > 0);
            var pattern = Pattern(word);
            return texts.Any(text => Found(pattern, text, word));
        });

    private static bool Found(Regex? pattern, string text, string word)
    {
        try { return pattern?.IsMatch(text) ?? text.Contains(word, StringComparison.OrdinalIgnoreCase); }
        catch (RegexMatchTimeoutException) { return text.Contains(word, StringComparison.OrdinalIgnoreCase); }
    }

    public static bool Shown(ParamEntry entry, string search, bool modifiedOnly) => Matches(entry, search) && (!modifiedOnly || entry.Changed);

    public static string CountLine(int shown, int changed) =>
        string.Join(" · ", new[]
        {
            string.Format(CultureInfo.InvariantCulture, "{0:N0} parameter{1}", shown, shown == 1 ? "" : "s"),
            changed > 0 ? string.Format(CultureInfo.InvariantCulture, "{0:N0} changed", changed) : "",
        }.Where(part => part.Length > 0));

    public static string RangeLine(FormControl control)
    {
        var units = control.Units.Trim().Length > 0 && !control.IsChoice ? $" {control.Units}" : "";
        var min = control.MinimumText.Trim().Length > 0 ? control.MinimumText : null;
        var max = control.MaximumText.Trim().Length > 0 ? control.MaximumText : null;
        var bounds = (min, max) switch
        {
            ({ } low, { } high) => $"Range {low}–{high}{units}",
            ({ } low, null) => $"Min {low}{units}",
            (null, { } high) => $"Max {high}{units}",
            _ => "",
        };
        return string.Join(" · ", new[] { bounds, control.DefaultText.Length > 0 ? $"default {control.DefaultText}{units}" : "" }.Where(part => part.Length > 0));
    }

    public static IReadOnlyList<string> RebootNotes(FormControl control) => new[]
    {
        control.VehicleReboot ? "Vehicle reboot required after change" : "",
        control.AppRestart ? "Application restart required after change" : "",
    }.Where(note => note.Length > 0).ToList();

    private static IReadOnlyList<string> Names(int component) =>
        (FormWrites.Result($"{Manager}.parameterNames", component) as JsonArray ?? []).Select(MapUiJson.Text).Where(name => name.Length > 0).ToList();

    private static IReadOnlyList<int> Ints(JsonNode? node) =>
        (node as JsonArray ?? []).Select(MapUiJson.Number).OfType<double>().Select(n => (int)n).ToList();

    private static Regex? Pattern(string word)
    {
        try { return new Regex(word, RegexOptions.IgnoreCase | RegexOptions.CultureInvariant, TimeSpan.FromMilliseconds(50)); }
        catch (ArgumentException) { return null; }
    }
}
