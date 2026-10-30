using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;

namespace AircastQGC.Pages.Plan;

internal enum PlanLayer { Mission, Fence, Rally }

internal abstract record Hit;

internal sealed record WaypointHit(int Index) : Hit;

internal sealed record FenceVertexHit(int Polygon, int Vertex) : Hit;

internal sealed record CircleHit(int Index) : Hit;

internal sealed record RallyHit(int Index) : Hit;

internal sealed record SurveyVertexHit(int Item, int Vertex) : Hit;

internal sealed record BreachHit : Hit;

internal enum LandingPlace { Approach, Touchdown }

internal sealed record LandingHit(int Index, LandingPlace Place) : Hit;

internal sealed record CircleRadiusHit(int Index) : Hit;

internal sealed record ShapeCentreHit(bool Fence, int Owner) : Hit;

internal sealed record LandingPattern(int Index, Geo? Landing, Geo? SlopeStart, Geo? Approach, double? LoiterRadiusMetres, bool LoiterToAltitude);

internal sealed record ItemShape(
    string Shape, string Property, IReadOnlyList<Geo> Vertices, IReadOnlyList<Geo> Transects,
    IReadOnlyList<Geo> Outline, IReadOnlyList<Geo> FlightLoop)
{
    public bool Line => Shape == "line" || Property == "corridorPolyline";
}

internal sealed record PlanItem(
    int Index, int Sequence, int Folded, string Name, string Kind, int Command, Geo? At, Geo? Exit,
    double? Altitude, string AltitudeText, string AltitudeBand, string FrameText, int? AltitudeMode, string AltitudeUnits,
    bool SpecifiesCoordinate, bool Complex, bool Ready, string BlockedReason, int CameraShots, double ExtraSeconds,
    string SpeedText, string Abbreviation, bool AfterRouteEnds, string DistanceText, string AzimuthText,
    string HeadingText, string AltitudeChangeText, string GradientText, ItemShape? Shape, bool Routed);

internal sealed record FencePolygon(int Index, bool Inclusion, IReadOnlyList<Geo> Vertices, string Title, string Detail);

internal sealed record FenceCircle(
    int Index, bool Inclusion, Geo Centre, double Radius, double RadiusMetres, string Units,
    double? Minimum, double? Maximum, string Title, string Detail);

internal sealed record RallyPoint(int Index, Geo At, double AltitudeMetres, double? Altitude, string Units, string AltitudePath);

internal sealed record BreachReturn(Geo At, double? Altitude, string Units, string AltitudePath);

internal sealed record PlanFences(
    IReadOnlyList<FencePolygon> Polygons, IReadOnlyList<FenceCircle> Circles, IReadOnlyList<RallyPoint> Rally,
    BreachReturn? Breach, IReadOnlyList<Geo> Firmware)
{
    public static readonly PlanFences None = new([], [], [], null, []);
}

internal sealed record MissionKind(string Id, string Label, string Title, bool Enabled, string DisabledReason);

internal sealed record UploadGate(bool CanSend, bool CanProceed, bool PausesFirst, string Heading, string Refusal, string ProceedTitle);

internal sealed record ConfirmCopy(string Title, string Body, string Confirm, bool Destructive = false);

internal sealed record PromptCopy(string Title, string Text, string Primary, string Secondary, bool PrimaryKeeps);

internal sealed record ItemRow(int Index, string Number, string Name, string Detail, string Kind, int Command, bool Ready, string Seal);

internal sealed record ProfilePoint(double Distance, double? Terrain, double Planned, bool Collision);

internal sealed record ProfileMarker(int Sequence, double Distance, string Label);

internal sealed record TerrainProfile(
    IReadOnlyList<ProfilePoint> Points, IReadOnlyList<ProfileMarker> Markers, string Lowest, string Highest,
    string DistanceText, string HeightHeader, (double Low, double High)? Band)
{
    public static readonly TerrainProfile None = new([], [], "", "", "", "", null);

    public bool Drawable => Points.Count >= 2;
}

internal enum PlanConfirm { Open, NewPlan, ClearMission, Download }

internal abstract record UploadStep;

internal sealed record SendStep : UploadStep;

internal sealed record ConfirmStep(UploadGate Gate) : UploadStep;

internal sealed record RefuseStep(string Reason) : UploadStep;

internal sealed record PlanStatus(
    bool Offline, bool Syncing, double Progress, bool Dirty, bool ContainsItems, bool HasMissionItems,
    bool CanUndo, bool CanRedo, string Line, string? File, string? FilePath, int? GlobalFrame, bool HomeSet,
    bool CanOpen, bool CanSave, bool CanExportKml, bool CanNewPlan, bool CanClear, bool CanDownload,
    bool AddFence, bool AddRally, bool FenceRefused, bool RallyRefused, string? NotReady, int? NextNotReady,
    UploadGate? Gate, IReadOnlyList<string> Patterns, bool TemplatesEnabled, IReadOnlyList<string> Templates,
    string BlankTemplate, PromptCopy? VehiclePrompt, (string Title, string Text)? AltitudePrompt,
    JsonObject Defaults, (string Firmware, string Type)? PlanningFor)
{
    public static PlanStatus From(JsonObject view)
    {
        var actions = view["actions"] as JsonObject;
        bool Allowed(string name) => MapUiJson.Flag(actions?[name]);
        var readiness = view["readiness"] as JsonObject;
        var sync = view["sync"] as JsonObject;
        var templates = view["templates"] as JsonObject;
        var planning = view["planningFor"] as JsonObject;
        return new PlanStatus(
            MapUiJson.Flag(view["offline"]),
            MapUiJson.Text(sync?["state"]) == "busy",
            Math.Clamp(MapUiJson.Number(sync?["progress"]) ?? 0, 0, 1),
            MapUiJson.Flag(view["dirty"]),
            MapUiJson.Flag(view["containsItems"]),
            MapUiJson.Flag(view["hasMissionItems"]),
            MapUiJson.Flag(view["canUndo"]),
            MapUiJson.Flag(view["canRedo"]),
            MapUiJson.Text(view["status"]),
            NonEmpty(MapUiJson.Text(view["file"])),
            NonEmpty(MapUiJson.Text(view["filePath"])),
            MapUiJson.Number(view["globalAltitudeFrame"]) is double frame ? (int)frame : null,
            MapUiJson.Flag(templates?["homeSet"], true),
            Allowed("open"),
            Allowed("save"),
            Allowed("exportKml"),
            Allowed("newPlan"),
            Allowed("clearMission"),
            Allowed("download"),
            Allowed("addFence"),
            Allowed("addRally"),
            view["fenceSupported"] is JsonValue fence && fence.TryGetValue<bool>(out var fenced) && !fenced,
            view["rallySupported"] is JsonValue rally && rally.TryGetValue<bool>(out var rallied) && !rallied,
            readiness == null
                ? "The plan could not be checked for saving."
                : MapUiJson.Flag(readiness["ready"])
                    ? null
                    : NonEmpty(MapUiJson.Text(readiness["reason"])) ?? "The plan could not be checked for saving.",
            MapUiJson.Number(readiness?["next"]) is double next && next > 0 ? (int)next : null,
            view["upload"] is JsonObject upload
                ? new UploadGate(
                    MapUiJson.Flag(upload["canSend"]),
                    MapUiJson.Flag(upload["canProceed"]),
                    MapUiJson.Flag(upload["pausesFirst"]),
                    MapUiJson.Text(upload["heading"]),
                    MapUiJson.Text(upload["refusal"]),
                    MapUiJson.Text(upload["proceedTitle"]))
                : null,
            MapUiJson.Objects(view["patterns"]).Select(p => MapUiJson.Text(p["name"])).Where(n => n.Length > 0).ToList(),
            MapUiJson.Flag(templates?["enabled"]),
            MapUiJson.Texts(templates?["names"]),
            MapUiJson.Text(templates?["blank"]),
            view["vehicleChangePrompt"] is JsonObject prompt ? PlanRules.Prompt(prompt) : null,
            view["applyAltitudePrompt"] is JsonObject asked ? (MapUiJson.Text(asked["title"]), MapUiJson.Text(asked["text"])) : null,
            view["defaults"] as JsonObject ?? new JsonObject(),
            planning == null ? null : (MapUiJson.Text(planning["firmware"]), MapUiJson.Text(planning["type"])));
    }

    public static readonly PlanStatus Empty = From(new JsonObject());

    private static string? NonEmpty(string text) => text.Length > 0 ? text : null;
}

internal static class PlanJson
{
    internal static IReadOnlyList<Geo> Points(JsonNode? node) =>
        MapUiJson.Objects(node).Select(MapOverlayJson.Plottable).OfType<Geo>().ToList();

    internal static IReadOnlyList<PlanItem> Items(JsonObject view)
    {
        var elements = (view["items"] as JsonArray ?? new JsonArray()).Select(node => node as JsonObject ?? new JsonObject()).ToList();
        var endsAfter = elements.FindIndex(e => MapUiJson.Flag(e["endsRoute"]) && Command(e) == PlanRules.ReturnToLaunch);
        var routeEnd = endsAfter < 0 ? int.MaxValue : endsAfter;
        return elements.Select((element, index) =>
        {
            var at = MapOverlayJson.Plottable(element["coordinate"]);
            var exit = MapOverlayJson.Plottable(element["exitCoordinate"]);
            return new PlanItem(
                index,
                (int)(MapUiJson.Number(element["sequence"]) ?? index),
                (int)(MapUiJson.Number(element["foldedCommands"]) ?? 0),
                MapUiJson.Text(element["name"]),
                MapUiJson.Text(element["kind"]),
                Command(element),
                at,
                exit != null && exit != at ? exit : null,
                MapUiJson.Number(element["altitude"]),
                MapUiJson.Text(element["altitudeText"]),
                MapUiJson.Text(element["altitudeBandText"]),
                MapUiJson.Text(element["altitudeFrameText"]),
                MapUiJson.Number(element["altitudeMode"]) is double mode ? (int)mode : null,
                MapUiJson.Text(element["altitudeEditUnits"]),
                MapUiJson.Flag(element["specifiesCoordinate"]),
                !MapUiJson.Flag(element["simple"], true),
                !MapUiJson.Flag(element["blocked"]) && !MapUiJson.Flag(element["awaitingTerrain"]),
                MapUiJson.Text(element["blockedReason"]),
                (int)(MapUiJson.Number(element["cameraShots"]) ?? 0),
                MapUiJson.Number(element["extraSeconds"]) ?? 0,
                MapUiJson.Text(element["speedChangeText"]),
                MapUiJson.Text(element["abbreviation"]),
                endsAfter >= 0 && index > endsAfter,
                MapUiJson.Text(element["distanceText"]),
                MapUiJson.Text(element["azimuthText"]),
                MapUiJson.Text(element["headingText"]),
                MapUiJson.Text(element["altitudeChangeText"]),
                MapUiJson.Text(element["gradientText"]),
                Shape(element["geometry"] as JsonObject),
                MapUiJson.Flag(element["flownLeg"]) && index <= routeEnd);
        }).ToList();
    }

    internal static LandingPattern? Landing(int index, JsonObject view)
    {
        var pattern = new LandingPattern(
            index,
            MapOverlayJson.Plottable(view["landing"]),
            MapOverlayJson.Plottable(view["slopeStart"]),
            MapOverlayJson.Plottable(view["finalApproach"]),
            MapUiJson.Number(view["loiterRadiusMetres"]),
            MapUiJson.Flag(view["loiterToAltitude"]));
        return MapUiJson.Text(view["kind"]) != "object" || view["reason"] != null || pattern is { Landing: null, SlopeStart: null, Approach: null }
            ? null
            : pattern;
    }

    private static ItemShape? Shape(JsonObject? geometry) =>
        geometry == null || MapUiJson.Text(geometry["shape"]).Length == 0 || MapUiJson.Text(geometry["property"]).Length == 0
            ? null
            : new ItemShape(
                MapUiJson.Text(geometry["shape"]),
                MapUiJson.Text(geometry["property"]),
                Points(geometry["vertices"]),
                Points(geometry["transects"]),
                Points(geometry["outline"]),
                Points(geometry["flightLoop"]));

    private static int Command(JsonObject element) => (int)(MapUiJson.Number(element["command"]) ?? 0);

    internal static PlanFences Fences(JsonObject view) => new(
        MapUiJson.Objects(view["polygons"])
            .Select((p, at) => new FencePolygon(
                (int)(MapUiJson.Number(p["index"]) ?? at),
                MapUiJson.Flag(p["inclusion"], true),
                Points(p["vertices"]),
                MapUiJson.Text(p["kindText"]) is { Length: > 0 } title ? title : $"Polygon {at + 1}",
                MapUiJson.Text(p["detailText"])))
            .Where(p => p.Vertices.Count >= 3)
            .ToList(),
        MapUiJson.Objects(view["circles"])
            .Select((c, at) => (Json: c, At: at, Centre: MapOverlayJson.Plottable(c["centre"]), Radius: MapUiJson.Number(c["radius"])))
            .Where(c => c.Centre != null && c.Radius is > 0)
            .Select(c => new FenceCircle(
                (int)(MapUiJson.Number(c.Json["index"]) ?? c.At),
                MapUiJson.Flag(c.Json["inclusion"], true),
                c.Centre!.Value,
                c.Radius!.Value,
                MapUiJson.Number(c.Json["radiusMetres"]) ?? c.Radius!.Value,
                MapUiJson.Text(c.Json["radiusUnits"]),
                Positive(c.Json["radiusMinimum"]),
                Positive(c.Json["radiusMaximum"]),
                MapUiJson.Text(c.Json["kindText"]) is { Length: > 0 } title ? title : $"Circle {c.At + 1}",
                MapUiJson.Text(c.Json["detailText"])))
            .ToList(),
        MapUiJson.Objects(view["rallyPoints"])
            .Select((r, at) => (Json: r, At: at, Where: MapOverlayJson.Plottable(r)))
            .Where(r => r.Where != null)
            .Select(r => new RallyPoint(
                (int)(MapUiJson.Number(r.Json["index"]) ?? r.At),
                r.Where!.Value,
                MapUiJson.Number(r.Json["altitudeMetres"]) ?? 0,
                MapUiJson.Number(r.Json["altitude"]),
                MapUiJson.Text(r.Json["altitudeUnits"]),
                MapUiJson.Text(r.Json["altitudePath"])))
            .ToList(),
        MapOverlayJson.Plottable(view["breachReturnPoint"]) is { } breach
            ? new BreachReturn(
                breach,
                MapUiJson.Number((view["breachReturnAltitude"] as JsonObject)?["value"]),
                MapUiJson.Text((view["breachReturnAltitude"] as JsonObject)?["units"]),
                MapUiJson.Text((view["breachReturnAltitude"] as JsonObject)?["path"]))
            : null,
        MapOverlayJson.Fences(view).Enforced);

    private static double? Positive(JsonNode? node) => MapUiJson.Number(node) is double value && value > 0 ? value : null;

    internal static IReadOnlyList<MissionKind> Kinds(JsonObject view) =>
        MapUiJson.Objects(view["kinds"])
            .Select(kind => new MissionKind(
                MapUiJson.Text(kind["id"]),
                MapUiJson.Text(kind["complexName"]),
                MapUiJson.Text(kind["title"]),
                MapUiJson.Flag(kind["enabled"], true),
                MapUiJson.Text(kind["disabledReason"])))
            .ToList();

    internal static TerrainProfile Profile(JsonObject view) =>
        view["points"] is not JsonArray
            ? TerrainProfile.None
            : new TerrainProfile(
                MapUiJson.Objects(view["points"])
                    .Select(p => new ProfilePoint(
                        MapUiJson.Number(p["distance"]) ?? 0,
                        MapUiJson.Number(p["terrainAltitude"]),
                        MapUiJson.Number(p["missionAltitude"]) ?? double.NaN,
                        MapUiJson.Flag(p["collision"])))
                    .ToList(),
                MapUiJson.Objects(view["markers"])
                    .Select(m => new ProfileMarker((int)(MapUiJson.Number(m["sequence"]) ?? 0), MapUiJson.Number(m["distance"]) ?? 0, MapUiJson.Text(m["label"])))
                    .ToList(),
                MapUiJson.Text(view["lowestText"]),
                MapUiJson.Text(view["highestText"]),
                MapUiJson.Text(view["distanceText"]),
                MapUiJson.Text(view["heightHeader"]),
                MapUiJson.Number(view["minAltitudeMeters"]) is double low && MapUiJson.Number(view["maxAltitudeMeters"]) is double high
                    ? (low, high)
                    : null);

    internal static IReadOnlySet<int> Colliding(JsonObject view, string key) =>
        (view[key] as JsonArray ?? new JsonArray()).Select(MapUiJson.Number).OfType<double>().Select(n => (int)n).Where(n => n >= 0).ToHashSet();

    internal static IReadOnlyList<(Geo From, Geo To)> CollisionLegs(JsonObject view) =>
        MapUiJson.Objects(view["collisionLegs"])
            .Select(leg => (From: MapOverlayJson.Plottable(leg["from"]), To: MapOverlayJson.Plottable(leg["to"])))
            .Where(leg => leg.From != null && leg.To != null)
            .Select(leg => (leg.From!.Value, leg.To!.Value))
            .ToList();

    internal static string? SummaryRow(JsonObject summary, string label) =>
        MapUiJson.Objects(summary["rows"])
            .Where(row => MapUiJson.Text(row["label"]) == label)
            .Select(row => MapUiJson.Text(row["value"]))
            .FirstOrDefault(value => value.Length > 0);
}

internal static class PlanRules
{
    internal const int ReturnToLaunch = 20;
    internal const int HomeItem = 0;
    internal const int AtEnd = -1;
    internal const int BeforeTheRest = 1;
    internal const string HomeSeal = "H";
    internal const string NotReadySeal = "?";
    internal const string Uploaded = "Uploaded";
    internal const string NoGeofence = "No geofence – keep the vehicle inside a boundary, or out of an area.";
    internal const string NoRally = "No rally points – alternate landing points for Return to Launch. Click the map to place one.";
    internal const string GeofenceRefused = "Not supported – this vehicle does not support geofence.";
    internal const string RallyRefused = "Not supported – this vehicle does not support rally points.";
    internal const string FarFromAircraftText = "The aircraft is not where this route begins. Move the route so it starts at the aircraft, or upload it as it is.";
    internal const string TerrainConflictHere = "Too close to the terrain here";
    private const double FarFromAircraftMetres = 1000;
    private const double MinimumCircleMetres = 0.1;
    private const double EarthRadiusMetres = 6_371_000;
    private static readonly HashSet<string> DrawnKinds = ["settings", "takeoff", "land", "waypoint", "command", "altitude", "roi", "survey", "corridor", "structure"];
    private static readonly HashSet<string> PatternKinds = ["survey", "corridor", "structure"];

    internal static bool IsPattern(string kind) => PatternKinds.Contains(kind);

    internal static PromptCopy Prompt(JsonObject prompt)
    {
        var connected = MapUiJson.Flag(prompt["connected"]);
        var dirty = MapUiJson.Flag(prompt["dirty"]);
        var stored = MapUiJson.Number(prompt["aircraftItems"]) is double count && count > 0
            ? $" ({ItemCountText((int)count)})"
            : string.Empty;
        return (connected, dirty) switch
        {
            (true, true) => new("Aircraft connected", $"Keep the route you drew, or replace it with the route stored on the aircraft{stored}?", "Keep my route", "Load the aircraft's route", true),
            (true, false) => new("Aircraft connected", $"Load the route stored on the aircraft{stored}, or keep this one?", "Load the aircraft's route", "Keep this route", false),
            (false, true) => new("Aircraft disconnected", "Keep the route you were working on?", "Keep my route", "Discard it", true),
            _ => new("Aircraft disconnected", "Keep this route, or start a new plan?", "Keep this route", "Start a new plan", true),
        };
    }

    internal static ConfirmCopy Confirm(PlanConfirm kind) => kind switch
    {
        PlanConfirm.Open => new("Plan overwrite", "You have unsaved/unsent changes. Loading from a file will lose these changes. Are you sure you want to load from a file?", "Load from file"),
        PlanConfirm.NewPlan => new("Create Plan", "Are you sure you want to remove current plan and create a new plan?", "Create plan"),
        PlanConfirm.Download => new("Plan overwrite", "You have unsaved/unsent changes. Loading from the Vehicle will lose these changes. Are you sure you want to load from the Vehicle?", "Load from vehicle"),
        _ => new("Clear the mission from the vehicle?", "This removes the mission from the aircraft as well as from this plan. It cannot be undone.", "Clear mission", true),
    };

    internal static string ItemCountText(int count) => count == 1 ? "1 item" : $"{count} items";

    internal static string ReplaceWarning(int count) => $"Your unsaved plan here ({ItemCountText(count)}) will be replaced.";

    internal static string PlanTitle(string? file) =>
        string.IsNullOrWhiteSpace(file) ? "New plan" : Path.GetFileNameWithoutExtension(file) is { Length: > 0 } name ? name : Path.GetFileName(file);

    internal static string UploadLabel(PlanStatus status) => status switch
    {
        { Syncing: true } => "Uploading…",
        { Dirty: false, ContainsItems: true, Offline: false } => Uploaded,
        _ => "Upload",
    };

    internal static string? SyncRefusal(PlanStatus status, string action) => status switch
    {
        { Offline: true } => $"No vehicle to {action}",
        { Syncing: true } => "Already syncing, wait for it to finish",
        _ => null,
    };

    internal static UploadStep Step(PlanStatus status) => status switch
    {
        { NotReady: { } reason } => new RefuseStep(reason),
        { Gate: null } => new RefuseStep("The plan could not be checked against the vehicle."),
        { Gate.CanSend: true } => new SendStep(),
        { Gate.CanProceed: true } => new ConfirmStep(status.Gate),
        _ => new RefuseStep(status.Gate.Refusal.Length > 0 ? status.Gate.Refusal : "This plan cannot be uploaded."),
    };

    internal static string UploadHeading(UploadGate gate) => gate.Heading.Length > 0 ? gate.Heading : "Upload this plan?";

    internal static string Seal(PlanItem item) =>
        item.Index == HomeItem ? HomeSeal : Label(item.Sequence, item.Complex ? string.Empty : item.Abbreviation);

    internal static string Label(int sequence, string abbreviation) =>
        abbreviation.Length > 0 && abbreviation[0] > 'A' && abbreviation[0] < 'z'
            ? abbreviation[..1]
            : sequence.ToString(CultureInfo.InvariantCulture);

    internal static string ItemTitle(PlanItem item)
    {
        var name = MapUiJson.SentenceCase(item.Name.Length > 0 ? item.Name : "Item");
        var seal = Seal(item);
        return seal.All(char.IsDigit) ? $"{name} {seal}" : name;
    }

    internal static string SequenceLabel(PlanItem item) =>
        item.Folded > 0 ? $"{item.Sequence}–{item.Sequence + item.Folded}" : item.Sequence.ToString(CultureInfo.InvariantCulture);

    internal static string? AltitudeWithFrame(PlanItem item)
    {
        var height = item.AltitudeText.Length > 0 ? item.AltitudeText : item.AltitudeBand.Length > 0 ? item.AltitudeBand : null;
        return height == null ? null : item.FrameText.Length > 0 ? $"{height} {item.FrameText}" : height;
    }

    internal static string Detail(PlanItem item) => string.Join(" · ", new[]
    {
        AltitudeWithFrame(item) ?? (item.At == null && item.SpecifiesCoordinate ? "no position" : null),
        item.AfterRouteEnds ? "after the route ends" : null,
        item.SpeedText.Length > 0 ? item.SpeedText : null,
        item.CameraShots switch { <= 0 => null, 1 => "1 photo", var shots => $"{shots} photos" },
        item.ExtraSeconds > 0 ? $"holds {(int)item.ExtraSeconds} s" : null,
        item.BlockedReason.Length > 0 ? item.BlockedReason : null,
    }.OfType<string>());

    internal static string? LegDetail(PlanItem item)
    {
        var legs = string.Join(" · ", new[]
        {
            ("Alt diff", item.AltitudeChangeText), ("Azimuth", item.AzimuthText), ("Heading", item.HeadingText),
            ("Gradient", item.GradientText), ("Prev WP", item.DistanceText),
        }.Where(leg => leg.Item2.Length > 0).Select(leg => $"{leg.Item1} {leg.Item2}"));
        var parts = new[] { item.Folded > 0 ? $"Mission items {SequenceLabel(item)}" : string.Empty, legs }.Where(text => text.Length > 0).ToList();
        return parts.Count == 0 ? null : string.Join(" · ", parts);
    }

    internal static IReadOnlyList<ItemRow> Rows(IReadOnlyList<PlanItem> items) =>
        items.Select(item => new ItemRow(
            item.Index,
            item.Ready ? SequenceLabel(item) : NotReadySeal,
            item.Name.Length > 0 ? item.Name : $"Item {item.Sequence}",
            Detail(item),
            item.Kind,
            item.Command,
            item.Ready,
            item.Ready ? Seal(item) : NotReadySeal)).ToList();

    internal static int InsertAfter(Hit? selected, IReadOnlyList<PlanItem> items) =>
        MissionIndex(selected) is { } index && items.Any(item => item.Index == index)
            ? index + 1 >= items.Count ? AtEnd : index + 1
            : AtEnd;

    internal static int? MissionIndex(Hit? hit) => hit switch
    {
        WaypointHit w => w.Index,
        SurveyVertexHit s => s.Item,
        _ => null,
    };

    internal static Hit? AfterRemove(int removed, int countBefore) =>
        countBefore - 2 > HomeItem ? new WaypointHit(Math.Min(removed, countBefore - 2)) : null;

    internal static Hit? RallyAfterRemove(int removed, int countBefore) =>
        countBefore - 2 >= 0 ? new RallyHit(Math.Min(removed, countBefore - 2)) : null;

    internal static int AppendSequence(IReadOnlyList<PlanItem> items) =>
        items.Count == 0 ? HomeItem : items.MaxBy(item => item.Index)!.Sequence;

    internal static PlanLayer? LayerOf(Hit? hit) => hit switch
    {
        null => null,
        RallyHit => PlanLayer.Rally,
        FenceVertexHit or CircleHit or BreachHit => PlanLayer.Fence,
        _ => PlanLayer.Mission,
    };

    internal static bool TakeoffMissing(IReadOnlyList<PlanItem> items) =>
        items.Any(item => item.At != null) && items.All(item => item.Kind != "takeoff");

    internal static bool Offered(IReadOnlyList<MissionKind> kinds, string id) => kinds.Count == 0 || kinds.Any(kind => kind.Id == id);

    internal static bool Allows(IReadOnlyList<MissionKind> kinds, string id) => kinds.FirstOrDefault(kind => kind.Id == id)?.Enabled ?? true;

    internal static string KindLabel(IReadOnlyList<MissionKind> kinds, string id) =>
        kinds.FirstOrDefault(kind => kind.Id == id)?.Title is { Length: > 0 } title ? title : char.ToUpperInvariant(id[0]) + id[1..];

    internal static IReadOnlyList<MissionKind> ScanPatterns(IReadOnlyList<MissionKind> kinds) => kinds.Where(kind => kind.Label.Length > 0).ToList();

    internal static IReadOnlyList<string> Undrawn(IReadOnlyList<PlanItem> items) =>
        items.Where(item => !DrawnKinds.Contains(item.Kind)).Select(item => item.Name).Where(name => name.Length > 0).Distinct().ToList();

    internal static string? UndrawnWarning(IReadOnlyList<string> names) =>
        names.Count == 0 ? null : $"The map cannot draw {string.Join(", ", names)}. Those items are still in the plan and will still be flown.";

    internal static string? TerrainWarning(int legs, int items) => (legs, items) switch
    {
        ( > 0, _) => $"{legs} {(legs == 1 ? "leg hits" : "legs hit")} the terrain",
        (_, > 0) => $"{items} {(items == 1 ? "item hits" : "items hit")} the terrain",
        _ => null,
    };

    internal static string StatsLine(int itemCount, IReadOnlyList<PlanItem> items, JsonObject summary) =>
        string.Join(" · ", new[]
        {
            itemCount > 0 ? ItemCountText(itemCount) : null,
            PlanJson.SummaryRow(summary, "Distance"),
            PlanJson.SummaryRow(summary, "Time") is { } time ? (time.StartsWith("00:", StringComparison.Ordinal) ? time[3..] : time) : null,
            items.Where(item => item.Index != HomeItem && item.Altitude != null && item.AltitudeText.Length > 0)
                .MaxBy(item => item.Altitude!.Value)?.AltitudeText is { } highest ? $"max {highest}" : null,
        }.OfType<string>());

    internal static string SummaryText(JsonObject summary) =>
        string.Join(" · ", new[]
        {
            PlanJson.SummaryRow(summary, "Distance") is { } distance ? $"Distance {distance}" : null,
            PlanJson.SummaryRow(summary, "Time") is { } time ? $"Time {time}" : null,
            PlanJson.SummaryRow(summary, "Furthest from launch") is { } far ? $"Max telem {far}" : null,
        }.OfType<string>());

    internal static Geo? RouteStart(IReadOnlyList<PlanItem> items) =>
        items.Where(item => item.At != null && item.Index != HomeItem).MinBy(item => item.Index)?.At
        ?? items.FirstOrDefault(item => item.At != null && item.Index == HomeItem)?.At;

    internal static double? FarFromAircraft(Geo? aircraft, IReadOnlyList<PlanItem> items) =>
        aircraft is { } at && RouteStart(items) is { } start && Metres(at, start) is var metres && metres > FarFromAircraftMetres ? metres : null;

    internal static string StartsFromAircraft(double metres, bool imperial) => $"Starts {DistanceWords(metres, imperial)} from the aircraft";

    internal static string DistanceWords(double metres, bool imperial)
    {
        var miles = metres / 1609.344;
        var kilometres = metres / 1000;
        return (imperial, miles, metres) switch
        {
            (true, < 0.2, _) => string.Format(CultureInfo.InvariantCulture, "{0:N0} ft", Math.Round(metres * 3.28084)),
            (true, < 10, _) => string.Format(CultureInfo.InvariantCulture, "{0:F1} mi", miles),
            (true, _, _) => string.Format(CultureInfo.InvariantCulture, "{0:N0} mi", Math.Round(miles)),
            (_, _, < 1000) => string.Format(CultureInfo.InvariantCulture, "{0} m", Math.Round(metres)),
            _ when kilometres < 10 => string.Format(CultureInfo.InvariantCulture, "{0:F1} km", kilometres),
            _ => string.Format(CultureInfo.InvariantCulture, "{0:N0} km", Math.Round(kilometres)),
        };
    }

    internal static double Metres(Geo from, Geo to)
    {
        var fromLat = Radians(from.Latitude);
        var toLat = Radians(to.Latitude);
        var a = Math.Pow(Math.Sin((toLat - fromLat) / 2), 2)
            + Math.Cos(fromLat) * Math.Cos(toLat) * Math.Pow(Math.Sin(Radians(to.Longitude - from.Longitude) / 2), 2);
        return 2 * EarthRadiusMetres * Math.Asin(Math.Sqrt(Math.Min(1, a)));
    }

    private static double Radians(double degrees) => degrees * Math.PI / 180;

    internal static (Geo TopLeft, Geo BottomRight) FenceWindow(IReadOnlyList<Geo> visible, Geo at) =>
        visible.Count == 4
            ? (visible[0], visible[2])
            : (new Geo(at.Latitude + 0.002, at.Longitude - 0.002), new Geo(at.Latitude - 0.002, at.Longitude + 0.002));

    internal static double? TypedNumber(string text)
    {
        var typed = text.Trim();
        var commas = typed.Count(c => c == ',');
        return commas > 1 || (commas == 1 && typed.Contains('.'))
            ? null
            : double.TryParse(typed.Replace(',', '.'), NumberStyles.Float, CultureInfo.InvariantCulture, out var value) && double.IsFinite(value)
                ? value
                : null;
    }

    internal static double? TypedCoordinate(string text, double limit) => TypedNumber(text) is double value && Math.Abs(value) <= limit ? value : null;

    internal static string Plain(double value) =>
        value == Math.Floor(value) ? ((long)value).ToString(CultureInfo.InvariantCulture) : value.ToString("0.0#", CultureInfo.InvariantCulture);

    internal static string RadiusText(double radius) =>
        radius.ToString("0.0", CultureInfo.InvariantCulture).TrimEnd('0').TrimEnd('.');

    internal static double? Grown(FenceCircle circle) =>
        circle.Maximum is { } ceiling ? circle.Radius >= ceiling ? null : Math.Min(circle.Radius * 1.5, ceiling) : circle.Radius * 1.5;

    internal static double? Shrunk(FenceCircle circle) =>
        circle.Minimum is { } floor
            ? circle.Radius <= floor ? null : Math.Max(circle.Radius / 1.5, floor)
            : circle.Radius / 1.5 is var wanted && wanted > 0 ? wanted : null;

    internal static double DraggedRadius(FenceCircle circle, Geo to)
    {
        var shownPerMetre = circle.RadiusMetres > 0 ? circle.Radius / circle.RadiusMetres : 1;
        return Math.Min(Math.Max(Metres(circle.Centre, to) * shownPerMetre, circle.Minimum ?? MinimumCircleMetres * shownPerMetre), circle.Maximum ?? double.MaxValue);
    }

    internal static double? TypedRadius(string text, FenceCircle circle) =>
        TypedNumber(text) is double wanted && wanted > 0
        && (circle.Minimum is not { } low || wanted >= low)
        && (circle.Maximum is not { } high || wanted <= high)
            ? wanted
            : null;

    internal static string Invariant(FormattableString text) => FormattableString.Invariant(text);
}
