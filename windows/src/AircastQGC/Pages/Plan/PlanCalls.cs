using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;
using AircastQGC.Core;

namespace AircastQGC.Pages.Plan;

internal sealed record Outcome(bool Ok, string Reason, int? Index = null)
{
    public static Outcome Of(JsonObject answer) =>
        MapUiJson.Flag(answer["ok"])
            ? new Outcome(true, string.Empty, MapUiJson.Number(answer["index"]) is double index ? (int)index : null)
            : new Outcome(false, MapUiJson.Text(answer["reason"]).Trim() is { Length: > 0 } reason ? reason : "The plan did not answer.");
}

internal sealed record EditableShape(string Path, IReadOnlyList<Geo> Midpoints, string SplitInvokable, bool CanRemoveVertex, string Caption);

internal static class PlanCalls
{
    internal const string ItemRoot = "plan.missionController.visualItems";
    internal const string FenceRoot = "plan.geoFenceController";
    internal const string RallyRoot = "plan.rallyPointController";
    internal const string PlanView = "view.plan";
    internal const string ItemsView = "view.missionItems(geometry)";
    internal const string FencesView = "view.fences";
    internal const string KindsView = "view.missionKinds";
    internal const string SummaryView = "view.missionSummary";
    internal const string TerrainView = "view.terrainProfile";
    internal const string MapTypesView = "view.mapTypes";
    internal const string ShowStatusSetting = "settings.planViewSettings.showMissionItemStatus";
    internal const string GlobalAltitudeMode = "plan.missionController.globalAltitudeMode";
    internal const string RepositionMission = "plan.missionController.repositionMission";

    internal static JsonObject Read(string path)
    {
        try { return Bridge.Get(path); }
        catch (Exception) { return new JsonObject(); }
    }

    internal static string Raw(string path)
    {
        try { return Bridge.Raw(path); }
        catch (Exception) { return string.Empty; }
    }

    internal static JsonObject Coordinate(Geo at, double altitude = 0) =>
        new() { ["latitude"] = at.Latitude, ["longitude"] = at.Longitude, ["altitude"] = altitude };

    internal static JsonArray Args(params JsonNode?[] values) => new(values.Select(value => value?.DeepClone()).ToArray());

    internal static Outcome Invoke(string path, params JsonNode?[] args)
    {
        try { return Outcome.Of(Bridge.Invoke(path, Args(args))); }
        catch (Exception) { return new Outcome(false, "The plan did not answer."); }
    }

    internal static Outcome Set(string path, JsonNode? value)
    {
        try { return Outcome.Of(Bridge.Set(path, value)); }
        catch (Exception) { return new Outcome(false, "The plan did not answer."); }
    }

    internal static JsonNode? Result(string path, params JsonNode?[] args)
    {
        try { return Bridge.Invoke(path, Args(args))["result"]?.DeepClone(); }
        catch (Exception) { return null; }
    }

    internal static string? Refusal(Outcome outcome) => outcome.Ok ? null : outcome.Reason;

    internal static Outcome Insert(string kind, Geo at, int index) =>
        Invoke("mission.insert", kind, at.Latitude, at.Longitude, index);

    internal static Outcome Remove(int index) => Invoke("mission.remove", index);

    internal static bool Move(int index, Geo at) => Set($"{ItemRoot}.{index}.coordinate", Coordinate(at)).Ok;

    internal static Outcome SetAltitude(int index, double shown) => Set($"{ItemRoot}.{index}.altitude", shown);

    internal static Outcome SetAltitudeMode(int index, int raw) => Set($"{ItemRoot}.{index}.altitudeMode", raw);

    internal static bool SelectSequence(int sequence) =>
        Invoke("plan.missionController.setCurrentPlanViewSeqNum", sequence, true).Ok;

    internal static bool PlaceTakeoff(int index, Geo at) => Set($"{ItemRoot}.{index}.launchCoordinate", Coordinate(at)).Ok;

    internal static bool PlaceLandingIfUnplaced(int index, Geo at)
    {
        var view = Read($"view.landingPattern({index})");
        var placed = MapUiJson.Text(view["kind"]) != "null" && view["reason"] == null && MapOverlayJson.Plottable(view["landing"]) != null;
        if (placed || !Set($"{ItemRoot}.{index}.landingCoordinate", Coordinate(at)).Ok) return false;
        Invoke($"{ItemRoot}.{index}.setLandingHeadingToTakeoffHeading");
        return Set($"{ItemRoot}.{index}.wizardMode", false).Ok;
    }

    internal static bool MoveRally(int index, Geo at, double altitudeMetres) =>
        Set($"{RallyRoot}.points.{index}.coordinate", Coordinate(at, altitudeMetres)).Ok;

    internal static bool AddRally(Geo at) => Invoke($"{RallyRoot}.addPoint", Coordinate(at)).Ok;

    internal static bool RemoveRally(int index) => Invoke($"{RallyRoot}.removePoint", $"@{RallyRoot}.points.{index}").Ok;

    internal static bool AddPolygon(Geo topLeft, Geo bottomRight) =>
        Invoke($"{FenceRoot}.addInclusionPolygon", Coordinate(topLeft), Coordinate(bottomRight)).Ok;

    internal static bool AddCircle(Geo topLeft, Geo bottomRight) =>
        Invoke($"{FenceRoot}.addInclusionCircle", Coordinate(topLeft), Coordinate(bottomRight)).Ok;

    internal static bool DeletePolygon(int index) => Invoke($"{FenceRoot}.deletePolygon", index).Ok;

    internal static bool DeleteCircle(int index) => Invoke($"{FenceRoot}.deleteCircle", index).Ok;

    internal static bool MoveCircle(int index, Geo at) => Set($"{FenceRoot}.circles.{index}.center", Coordinate(at)).Ok;

    internal static bool SetCircleRadius(int index, double shown) => Set($"{FenceRoot}.circles.{index}.radius", shown).Ok;

    internal static bool SetCircleInclusion(int index, bool inclusion) => Set($"{FenceRoot}.circles.{index}.inclusion", inclusion).Ok;

    internal static bool SetInclusion(int polygon, bool inclusion) => Set($"{FenceRoot}.polygons.{polygon}.inclusion", inclusion).Ok;

    internal static bool AdjustVertex(string shapePath, int vertex, Geo at) =>
        Invoke($"{shapePath}.adjustVertex", vertex, Coordinate(at)).Ok;

    internal static bool RemoveVertex(string shapePath, int vertex) => Invoke($"{shapePath}.removeVertex", vertex).Ok;

    internal static bool SetBreach(Geo? at) => Set($"{FenceRoot}.breachReturnPoint", at is { } point ? Coordinate(point) : null).Ok;

    internal static bool SetNumber(string path, double shown) => Set(path, shown).Ok;

    internal static bool Reposition(Geo at) => Invoke(RepositionMission, Coordinate(at), true, true).Ok;

    internal static string FencePath(int polygon) => $"{FenceRoot}.polygons.{polygon}";

    internal static string ShapePath(PlanItem item) => item.Shape is { } shape ? $"{ItemRoot}.{item.Index}.{shape.Property}" : string.Empty;

    internal static EditableShape? Editable(string path, string shape = "")
    {
        var view = Read(shape.Length == 0 ? $"view.polygon({path})" : $"view.polygon({path},{shape})");
        return MapUiJson.Text(view["class"]) != "EditablePolygon"
            ? null
            : new EditableShape(
                path,
                PlanJson.Points(view["midpoints"]),
                MapUiJson.Text(view["splitInvokable"]),
                MapUiJson.Flag(view["canRemoveVertex"]),
                MapUiJson.Text(view["caption"]));
    }

    internal static Geo? VehiclePosition()
    {
        try { return MapUiJson.VehiclePosition() is { } at ? new Geo(at.Latitude, at.Longitude) : null; }
        catch (Exception) { return null; }
    }

    internal static bool HasVehicle() => MapUiJson.Text(Read("vehicle")["kind"]) == "object";

    internal static IReadOnlyList<string> Categories() =>
        MapUiJson.Texts(Result("missionCommandTree.categoriesForVehicle"));

    internal static IReadOnlyList<(int Command, string Name, string Description)> Commands(string category) =>
        MapUiJson.Objects(Result("missionCommandTree.getCommandsForCategory", null, category, true))
            .Select(command => ((int)(MapUiJson.Number(command["command"]) ?? 0), MapUiJson.Text(command["friendlyName"]), MapUiJson.Text(command["description"])))
            .ToList();
}
