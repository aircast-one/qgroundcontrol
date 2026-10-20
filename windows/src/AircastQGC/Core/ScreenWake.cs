using System;
using System.Linq;
using System.Text.Json.Nodes;
using Windows.System.Display;

namespace AircastQGC.Core;

public static class ScreenWake
{
    private const string VehiclesPath = "view.vehicles";
    private const string LinksPath = "view.links";
    private const string Client = "screenWake";

    private static DisplayRequest? _request;
    private static JsonObject _vehicles = new();
    private static JsonObject _links = new();
    private static bool _held;

    public static bool Held => _held;

    public static void Start()
    {
        if (_request is not null || !CoreHost.Started) return;
        _request = new DisplayRequest();
        _vehicles = Bridge.Get(VehiclesPath);
        _links = Bridge.Get(LinksPath);
        BridgeWatch.Watch($"{Client}.vehicles", [VehiclesPath], view => Apply(_vehicles = view));
        BridgeWatch.Watch($"{Client}.links", [LinksPath], view => Apply(_links = view));
        Apply(_vehicles);
    }

    public static bool Wanted(JsonObject vehicles, JsonObject links) =>
        (vehicles["count"] is JsonValue count && count.TryGetValue<double>(out var n) && n > 0)
        || (links["configured"] is JsonArray configured
            && configured.OfType<JsonObject>().Any(link => link["connected"] is JsonValue on && on.TryGetValue<bool>(out var connected) && connected));

    private static void Apply(JsonObject _)
    {
        var wanted = Wanted(_vehicles, _links);
        if (wanted == _held || _request is null) return;
        if (wanted) _request.RequestActive();
        else _request.RequestRelease();
        _held = wanted;
    }
}
