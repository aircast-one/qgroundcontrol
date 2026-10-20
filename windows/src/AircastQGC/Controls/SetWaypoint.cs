using System;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using Windows.Foundation;

namespace AircastQGC.Controls;

internal static class SetWaypoint
{
    private const string MissionPath = "view.flyMissionItems";
    private const string SetPath = "vehicle.setCurrentMissionSequence";
    private const double HitRadius = 16;

    public static GuidedAsk Ask(int sequence)
    {
        var target = Math.Max(sequence, 1);
        return new GuidedAsk("Set waypoint", $"Adjust current waypoint to {target}",
            () => MapUiJson.RefusalOf(Bridge.Invoke(SetPath, new JsonArray(target))));
    }

    public static int? PinAt(MapView map, Point tapped) =>
        MapOverlayJson.Mission(Bridge.Get(MissionPath)).Pins
            .Select(pin => (pin.Sequence, Distance: Distance(map.ScreenOf(pin.At), tapped)))
            .Where(hit => hit.Distance <= HitRadius)
            .OrderBy(hit => hit.Distance)
            .Select(hit => (int?)hit.Sequence)
            .FirstOrDefault();

    private static double Distance(Point a, Point b) => Math.Sqrt(Math.Pow(a.X - b.X, 2) + Math.Pow(a.Y - b.Y, 2));
}
