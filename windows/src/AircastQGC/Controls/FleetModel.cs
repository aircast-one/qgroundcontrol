using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using System.Text.Json.Nodes;
using AircastQGC.Core;

namespace AircastQGC.Controls;

internal sealed record VehicleChoice(
    int Id,
    string Name,
    string State,
    string Link,
    bool ContactLost,
    bool Active,
    Geo? At,
    bool Selected,
    double? Heading,
    bool Armed,
    IReadOnlyList<(string Label, string Value)> Telemetry,
    int Index,
    IReadOnlyList<string> FlightModes)
{
    internal string FlightModePath => $"vehicles.vehicles.{Index}.flightMode";

    internal string? TelemetryLine =>
        Telemetry.Count == 0 ? null : string.Join(" · ", Telemetry.Select(row => $"{row.Label} {row.Value}"));

    internal string Line(bool distinguishes)
    {
        var link = distinguishes && Link.Length > 0 ? Link : null;
        return string.Join(" · ", (ContactLost ? new[] { "No contact", link } : new[] { State.Length > 0 ? State : null, link }).OfType<string>());
    }
}

internal sealed record FleetAction(string Id, string Title, string ConfirmTitle, string Prompt, string Offer, string Reason)
{
    internal bool Ready => Offer == "ready";

    internal bool Destructive => Id != "mvPause";

    internal string Line => !Ready && Reason.Length > 0 ? Reason : Prompt.Length > 0 ? Prompt : Title;
}

internal sealed record VehicleChoices(
    bool Ambiguous,
    IReadOnlyList<VehicleChoice> Choices,
    bool CanSelectAll,
    bool CanDeselectAll,
    IReadOnlyList<FleetAction> Actions)
{
    internal const string ViewPath = "view.vehicles";
    internal const string ChooserTitle = "Fly which aircraft?";

    internal static readonly VehicleChoices None = new(false, [], false, false, []);

    internal VehicleChoice? Active => Choices.FirstOrDefault(choice => choice.Active);

    internal IReadOnlyList<int> SelectedIds => Choices.Where(choice => choice.Selected).Select(choice => choice.Id).Order().ToList();

    internal bool LinkDistinguishes => Choices.Select(choice => choice.Link).Where(link => link.Length > 0).Distinct().Count() > 1;

    internal string FleetHeading => $"Vehicles Selected: {(SelectedIds.Count == 0 ? "-" : string.Join(", ", SelectedIds))}";

    internal string? LostText
    {
        get
        {
            var lost = Choices.Where(choice => choice.ContactLost && !choice.Active).ToList();
            return lost.Count switch
            {
                0 => null,
                1 => $"{lost[0].Name} is not answering",
                _ => $"{lost.Count} other vehicles are not answering",
            };
        }
    }

    internal static VehicleChoices Parse(JsonObject view) => new(
        MapUiJson.Flag(view["ambiguous"]),
        MapUiJson.Objects(view["vehicles"]).Select(Choice).OfType<VehicleChoice>().ToList(),
        MapUiJson.Flag(view["canSelectAll"]),
        MapUiJson.Flag(view["canDeselectAll"]),
        MapUiJson.Objects(view["actions"]).Select(Action).OfType<FleetAction>().ToList());

    private static VehicleChoice? Choice(JsonObject entry) =>
        MapUiJson.Number(entry["id"]) is double id && id >= 0
            ? new VehicleChoice(
                (int)id,
                MapUiJson.Text(entry["name"]) is { Length: > 0 } name ? name : $"Vehicle {(int)id}",
                State(entry),
                MapUiJson.Text(entry["link"]),
                MapUiJson.Flag(entry["contactLost"]),
                MapUiJson.Flag(entry["active"]),
                MapOverlayJson.Plottable(entry["coordinate"]),
                MapUiJson.Flag(entry["selected"]),
                MapUiJson.Number(entry["heading"]),
                MapUiJson.Flag(entry["armed"]),
                MapUiJson.Objects(entry["telemetry"]).Select(row => (MapUiJson.Text(row["label"]), MapUiJson.Text(row["value"]))).ToList(),
                (int)(MapUiJson.Number(entry["index"]) ?? -1),
                MapUiJson.Texts(entry["flightModes"]))
            : null;

    private static string State(JsonObject entry) =>
        string.Join(" · ", new[]
        {
            MapUiJson.Text(entry["flightMode"]),
            MapUiJson.Flag(entry["flying"]) ? "Flying" : MapUiJson.Flag(entry["armed"]) ? "Armed" : "Disarmed",
        }.Where(part => part.Length > 0));

    private static FleetAction? Action(JsonObject entry) =>
        MapUiJson.Text(entry["id"]) is { Length: > 0 } id
            ? new FleetAction(
                id,
                MapUiJson.Text(entry["title"]),
                MapUiJson.Text(entry["confirmTitle"]) is { Length: > 0 } confirm ? confirm : MapUiJson.Text(entry["title"]),
                MapUiJson.Text(entry["prompt"]),
                MapUiJson.Text(entry["offer"]),
                MapUiJson.Text(entry["reason"]))
            : null;

    internal static string? HandoverNotice(VehicleChoices? before, VehicleChoices now, int? asked) =>
        before?.Active is { } left && now.Active is { } arrived && left.Id != arrived.Id && arrived.Id != asked
            ? now.Choices.Any(choice => choice.Id == left.Id)
                ? $"{left.Name} stopped answering. Now flying {arrived.Name}."
                : $"{left.Name} is gone. Now flying {arrived.Name}."
            : null;

    internal static bool ActiveChanged(VehicleChoices? before, VehicleChoices now) =>
        before?.Active is { } was && now.Active is { } isNow && was.Id != isNow.Id;

    internal static VehicleChoices? Remembered(VehicleChoices? previous, VehicleChoices now) =>
        now.Choices.Count == 0 ? null : now.Active != null ? now : previous;
}

internal static class FleetBridge
{
    internal const string PanelSetting = "settings.appSettings.enableMultiVehiclePanel";
    internal const string PanelValue = PanelSetting + ".rawValue";
    private const string Manager = "vehicles";
    private const string Selected = Manager + ".selectedVehicles";

    internal static int? LastAsked { get; private set; }

    internal static void Forget() => LastAsked = null;

    internal static bool PanelEnabled(JsonObject setting) => MapUiJson.Flag(setting["value"], true);

    internal static bool PanelToggleShown() => MapUiJson.Flag(Bridge.Get(PanelSetting)["visible"], true);

    internal static string? SetPanel(bool enabled) => MapUiJson.RefusalOf(Bridge.Set(PanelSetting, enabled));

    internal static string? AskFor(int id)
    {
        var answer = Bridge.Invoke($"{Manager}.setActive", new JsonArray(id));
        var refusal = MapUiJson.Flag(answer["ok"])
            ? null
            : MapUiJson.Text(answer["reason"]) is { Length: > 0 } reason ? reason : "That vehicle did not take control.";
        LastAsked = refusal == null ? id : LastAsked;
        return refusal;
    }

    internal static string? SetSelected(int id, bool selected) =>
        MapUiJson.RefusalOf(Bridge.Invoke(selected ? $"{Manager}.selectVehicle" : $"{Manager}.deselectVehicle", new JsonArray(id)));

    internal static string? SelectAll(VehicleChoices choices) =>
        choices.Choices.Where(choice => !choice.Selected).Select(choice => SetSelected(choice.Id, true)).ToList().OfType<string>().FirstOrDefault();

    internal static string? DeselectAll() => MapUiJson.RefusalOf(Bridge.Invoke($"{Manager}.deselectAllVehicles"));

    internal static string? SetMode(VehicleChoice choice, string mode)
    {
        var payload = new JsonObject { ["value"] = mode, ["vehicle"] = choice.Id };
        var raw = NativeMethods.Take(NativeMethods.qgc_bridge_set(choice.FlightModePath, payload.ToJsonString()));
        return MapUiJson.RefusalOf(Decode(raw));
    }

    internal static bool Command(string action, IReadOnlyCollection<int> confirmed)
    {
        var targets = SelectedPaths().Where(target => confirmed.Contains(target.Id)).ToList();
        var sent = action switch
        {
            "mvArm" => targets.Select(target => Still(target, path => Ok(Bridge.Set($"{path}.armed", true)))).ToList(),
            "mvDisarm" => targets.Select(target => Still(target, path => Ok(Bridge.Set($"{path}.armed", false)))).ToList(),
            "mvPause" => targets.Select(target => Still(target, path => Ok(Bridge.Invoke($"{path}.pauseVehicle")))).ToList(),
            "mvStartMission" => targets.Where(target => ArmedAt(target.Path))
                .Select(target => Still(target, path => Ok(Bridge.Invoke($"{path}.startMission"))))
                .ToList(),
            _ => [],
        };
        return sent.Count > 0 && sent.All(ok => ok);
    }

    private static bool Still((string Path, int Id) target, Func<string, bool> write) =>
        IdAt(target.Path) == target.Id && write(target.Path);

    private static IReadOnlyList<(string Path, int Id)> SelectedPaths() =>
        Enumerable.Range(0, Math.Max(0, (int)(MapUiJson.Number(Bridge.Get($"{Selected}.count")["value"]) ?? 0)))
            .Select(index => $"{Selected}.{index}")
            .Select(path => (Path: path, Id: IdAt(path)))
            .Where(target => target.Id != null)
            .Select(target => (target.Path, target.Id!.Value))
            .ToList();

    private static int? IdAt(string path) =>
        MapUiJson.Number(Bridge.GetFields(path, "id")["id"]) is double id && id >= 0 ? (int)id : null;

    private static bool ArmedAt(string path) => MapUiJson.Flag(Bridge.GetFields(path, "armed")["armed"]);

    private static bool Ok(JsonObject answer) => MapUiJson.Flag(answer["ok"]);

    private static JsonObject Decode(string raw)
    {
        try { return JsonNode.Parse(raw) as JsonObject ?? new JsonObject(); }
        catch (JsonException) { return new JsonObject(); }
    }
}
