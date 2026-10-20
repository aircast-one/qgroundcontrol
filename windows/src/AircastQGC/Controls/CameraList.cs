using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;

namespace AircastQGC.Controls;

internal sealed record CameraKind(string Raw, string Label, string Group, bool NeedsUrl, string Hint, string Description, string DefaultAddress)
{
    public string Title => Raw == CameraList.SyntheticSource ? CameraList.SyntheticLabel : CameraList.KindLabel(Label);
}

internal sealed record CameraEntry(
    int Slot,
    int? Stored,
    string Title,
    string Short,
    string Name,
    string Source,
    string Url,
    double? Tilt,
    string Problem,
    bool FromDrone,
    bool Active,
    string Status)
{
    public string Detail =>
        FromDrone ? "From the drone"
        : Url.Contains("://") ? Url
        : Url.Length > 0 ? $"{CameraList.KindLabel(Source)} · {Url}"
        : CameraList.KindLabel(Source);

    public bool Live => Status == CameraList.LiveStatus;
}

internal sealed record CameraList(bool Readable, string Reason, bool PipEnabled, int? PipSlot, IReadOnlyList<CameraEntry> Cameras, IReadOnlyList<CameraKind> Kinds)
{
    public const string View = "view.cameras";
    public const string SyntheticSource = "Synthetic View";
    public const string SyntheticLabel = "Synthetic view";
    public const string LiveStatus = "live";

    private static readonly string[] GroupOrder = ["Video streams", "Vehicle and radio presets", "This device"];

    private static readonly IReadOnlyDictionary<string, (string Label, string Brush)> Statuses = new Dictionary<string, (string, string)>
    {
        [LiveStatus] = ("Live", "SuccessBrush"),
        ["connecting"] = ("Connecting", "CautionBrush"),
        ["noSignal"] = ("No signal", "CriticalBrush"),
        ["idle"] = ("Not playing", "TextTertiaryBrush"),
    };

    public IReadOnlyList<CameraEntry> Stored => Cameras.Where(c => c.Stored is not null).ToList();

    public CameraEntry? PipCamera => PipEnabled && PipSlot is int slot ? Cameras.FirstOrDefault(c => c.Slot == slot) : null;

    public IReadOnlyList<CameraKind> Addable(string? keeping) =>
        Kinds.Where(kind => kind.NeedsUrl || kind.Raw == keeping || Stored.All(c => c.Source != kind.Raw))
            .OrderBy(kind => GroupRank(kind.Group))
            .ToList();

    public static (string Label, string Brush) StatusLook(string status) =>
        Statuses.TryGetValue(status, out var look) ? look : Statuses["idle"];

    public static string KindLabel(string label) =>
        MapUiJson.SentenceCase(label.EndsWith(" Video Stream") && label.Length > 13 ? label[..^13] : label);

    public static CameraList? Parse(JsonObject view) => MapUiJson.Text(view["class"]) != "Cameras"
        ? null
        : new CameraList(
            MapUiJson.Flag(view["readable"], true),
            MapUiJson.Text(view["reason"]),
            MapUiJson.Flag((view["pip"] as JsonObject)?["enabled"]),
            Whole((view["pip"] as JsonObject)?["slot"]),
            MapUiJson.Objects(view["cameras"]).Select(Entry).ToList(),
            MapUiJson.Objects(view["kinds"]).Select(Kind).ToList());

    private static int GroupRank(string group) => GroupOrder.ToList().IndexOf(group) is var at and >= 0 ? at : GroupOrder.Length;

    private static int? Whole(JsonNode? node) => MapUiJson.Number(node) is double d ? (int)d : null;

    private static CameraEntry Entry(JsonObject camera) => new(
        Whole(camera["slot"]) ?? 0,
        Whole(camera["stored"]),
        MapUiJson.Text(camera["title"]),
        MapUiJson.Text(camera["short"]),
        MapUiJson.Text(camera["name"]),
        MapUiJson.Text(camera["source"]),
        MapUiJson.Text(camera["url"]),
        MapUiJson.Number(camera["tilt"]),
        MapUiJson.Text(camera["problem"]),
        MapUiJson.Flag(camera["fromDrone"]),
        MapUiJson.Flag(camera["active"]),
        MapUiJson.Text(camera["status"]));

    private static CameraKind Kind(JsonObject kind) => new(
        MapUiJson.Text(kind["raw"]),
        MapUiJson.Text(kind["label"]),
        MapUiJson.Text(kind["group"]),
        MapUiJson.Flag(kind["needsUrl"]),
        MapUiJson.Text(kind["hint"]),
        MapUiJson.Text(kind["description"]),
        MapUiJson.Text(kind["defaultAddress"]));
}
