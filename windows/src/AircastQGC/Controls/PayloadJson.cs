using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;

namespace AircastQGC.Controls;

internal sealed record GimbalChoice(string Name, int ManagerCompid, int DeviceId, bool Active);

internal sealed record VideoAim(string Label, string Hint, bool OnScreen, bool ClickAndDrag)
{
    public static readonly VideoAim Drag = new("Drag to move", "The camera follows your drag on the video.", false, false);
    public static readonly VideoAim Point = new("Tap to point", "Tap to turn the camera there, or drag to move it.", true, false);
    public static readonly VideoAim Hold = new("Hold and drag", "Press, then drag away from that spot; the further you drag, the faster the camera turns.", true, true);
    public static readonly IReadOnlyList<VideoAim> All = [Drag, Point, Hold];

    public static VideoAim Of(bool enabled, bool clickAndDrag) => !enabled ? Drag : clickAndDrag ? Hold : Point;

    public VideoAim Next => All[(All.ToList().IndexOf(this) + 1) % All.Count];
}

internal sealed record GimbalState(
    string StatusText,
    string PitchText,
    string YawText,
    string YawLockLabel,
    bool YawLocked,
    bool YawLockOffered,
    bool RetractOffered,
    bool ControlOffered,
    string ControlLabel,
    bool HaveControl,
    IReadOnlyList<GimbalChoice> Gimbals,
    VideoAim? Aim,
    long AskSerial)
{
    public string CellText => string.Join(" · ", new[]
    {
        Gimbals.Count > 1 ? Gimbals.FirstOrDefault(g => g.Active)?.Name ?? "" : "",
        StatusText,
        PitchText,
        YawText,
    }.Where(text => text.Length > 0));
}

internal sealed record RadarSector(int Bearing, double? Meters, string Text);

internal sealed record ProximityRadar(double Range, double? MaxMeters, IReadOnlyList<RadarSector> Sectors);

internal readonly record struct Painted(double Left, double Top, double Width, double Height);

internal readonly record struct TrackBox(double X, double Y, double Width, double Height);

internal sealed record TrackingState(bool Requested, IReadOnlyList<string> Shapes, TrackBox? Box)
{
    public bool CanAim => Requested && Shapes.Count > 0;
    public string ToggleLabel => Requested ? "Stop tracking" : "Track something";
}

internal sealed record PursuitState(bool Engaged, bool Ready, string Reason)
{
    public string Label => Engaged ? "Chasing — Stop" : "Chase";
}

internal abstract record TrackRequest;

internal sealed record TrackArea(TrackBox Rect) : TrackRequest;

internal sealed record TrackSpot(double X, double Y, double Radius) : TrackRequest;

internal sealed record PayloadProblem(string Code, string Text, bool Blocking, string Fix);

internal sealed record Payload(int Bays, IReadOnlyList<int> Empty, IReadOnlyList<PayloadProblem> Problems)
{
    public bool MultiBay => Bays > 1;
    public IReadOnlyList<int> BayNumbers => Enumerable.Range(1, Bays).ToList();
}

internal sealed record PayloadStatus(string Text, bool Warning);

internal sealed record DestructiveAction(string Id, string Label, string Button, string Title, string Prompt, string Offer, string Reason)
{
    public bool Ready => Offer == "ready";
    public string? BlockedReason => Offer == "blocked" && Reason.Length > 0 ? Reason : null;
}

internal sealed record CameraDetailsState(
    IReadOnlyList<string> Labels,
    int Selected,
    IReadOnlyList<string> Streams,
    int CurrentStream,
    bool CapturesPhotos,
    bool Timelapse,
    double LapseSeconds,
    TrackingState? Tracking,
    PursuitState? Pursuit,
    string? ThermalMode,
    double? ThermalOpacity,
    IReadOnlyList<DestructiveAction> Destructive);

internal static class PayloadJson
{
    public const double TrackSlop = 10;
    public const double TrackRadius = 50;
    public const double LapseMin = 1;
    public const double LapseMax = 60;
    public static readonly IReadOnlyList<string> GripperIds = ["release", "grab", "hold"];
    public static readonly IReadOnlyList<(string Token, string Label)> ThermalModes =
        [("off", "Off"), ("blend", "Blend"), ("full", "Full"), ("picInPic", "Picture in picture")];

    public static GimbalState? Gimbal(JsonObject view) =>
        MapUiJson.Flag(view["shown"])
            ? new GimbalState(
                MapUiJson.Text(view["statusText"]),
                MapUiJson.Text(view["pitchText"]),
                MapUiJson.Text(view["yawText"]),
                MapUiJson.Text(view["yawLockLabel"]),
                MapUiJson.Flag(view["yawLocked"]),
                MapUiJson.Flag(view["yawLockOffered"]),
                MapUiJson.Flag(view["retractOffered"]),
                MapUiJson.Flag(view["controlOffered"]),
                MapUiJson.Text(view["controlLabel"]),
                MapUiJson.Flag(view["haveControl"]),
                MapUiJson.Objects(view["gimbals"])
                    .Select(g => new GimbalChoice(
                        MapUiJson.Text(g["name"]),
                        Whole(g["managerCompid"]),
                        Whole(g["deviceId"]),
                        MapUiJson.Flag(g["active"])))
                    .ToList(),
                view["onScreen"] is JsonObject aim ? VideoAim.Of(MapUiJson.Flag(aim["enabled"]), MapUiJson.Flag(aim["clickAndDrag"])) : null,
                (long)(MapUiJson.Number(view["askSerial"]) ?? -1))
            : null;

    public static (double Pan, double Tilt) ScreenFraction(double x, double y, double width, double height) =>
        (x / width * 2 - 1, -(y / height * 2 - 1));

    public static double AimFraction(double delta, double size) => delta / (Math.Max(size, 1) / 2);

    public static ProximityRadar? Radar(JsonObject view) =>
        MapUiJson.Flag(view["shown"])
            ? new ProximityRadar(
                MapUiJson.Number(view["rangeMeters"]) ?? 6,
                MapUiJson.Number(view["maxMeters"]) is double max && max > 0 ? max : null,
                MapUiJson.Objects(view["sectors"])
                    .Select(s => new RadarSector(Whole(s["bearing"]), MapUiJson.Number(s["meters"]), MapUiJson.Text(s["text"])))
                    .ToList())
            : null;

    public static double? Aspect(JsonObject video) =>
        video["sourceSize"] is JsonObject size
        && MapUiJson.Number(size["width"]) is double width
        && MapUiJson.Number(size["height"]) is double height
        && width > 0 && height > 0
            ? width / height
            : null;

    public static Painted Centred(double width, double height, (double Width, double Height) shown) =>
        new((width - shown.Width) / 2, (height - shown.Height) / 2, shown.Width, shown.Height);

    public static TrackingState? Tracking(JsonObject camera) =>
        camera["tracking"] is JsonObject tracking && MapUiJson.Flag(tracking["supported"])
            ? new TrackingState(
                MapUiJson.Flag(tracking["requested"]),
                MapUiJson.Texts(tracking["shapes"]),
                tracking["rect"] is JsonObject rect
                && MapUiJson.Number(rect["x"]) is double x
                && MapUiJson.Number(rect["y"]) is double y
                && MapUiJson.Number(rect["width"]) is double w && w > 0
                && MapUiJson.Number(rect["height"]) is double h && h > 0
                    ? new TrackBox(x, y, w, h)
                    : null)
            : null;

    public static PursuitState? Pursuit(JsonObject camera) =>
        camera["pursuit"] is JsonObject pursuit
            ? new PursuitState(
                MapUiJson.Flag(pursuit["engaged"]),
                MapUiJson.Flag(pursuit["ready"]),
                MapUiJson.Text(pursuit["reason"]))
            : null;

    public static TrackRequest? Track((double X, double Y) press, (double X, double Y) release, Painted picture)
    {
        if (picture.Width <= 0 || picture.Height <= 0) return null;
        double Across(double value) => Math.Clamp((value - picture.Left) / picture.Width, 0, 1);
        double Down(double value) => Math.Clamp((value - picture.Top) / picture.Height, 0, 1);
        var (x0, x1) = (Across(Math.Min(press.X, release.X)), Across(Math.Max(press.X, release.X)));
        var (y0, y1) = (Down(Math.Min(press.Y, release.Y)), Down(Math.Max(press.Y, release.Y)));
        var tapped = Math.Abs(release.X - press.X) < TrackSlop && Math.Abs(release.Y - press.Y) < TrackSlop;
        return tapped ? new TrackSpot(x0, y0, TrackRadius / picture.Width)
            : x1 - x0 <= 0 || y1 - y0 <= 0 ? null
            : new TrackArea(new TrackBox(x0, y0, x1 - x0, y1 - y0));
    }

    public static Payload PayloadOf(JsonObject guided) =>
        guided["payload"] is JsonObject payload
            ? new Payload(
                Math.Max(1, Whole(payload["bays"], 1)),
                (payload["empty"] as JsonArray ?? []).Select(n => (int)(MapUiJson.Number(n) ?? 0)).ToList(),
                MapUiJson.Objects(payload["problems"])
                    .Select(p => new PayloadProblem(MapUiJson.Text(p["code"]), MapUiJson.Text(p["text"]), MapUiJson.Flag(p["blocking"]), MapUiJson.Text(p["fix"])))
                    .ToList())
            : new Payload(1, [], []);

    public static string Ago(long atMs, long nowMs)
    {
        var seconds = Math.Max(0, (nowMs - atMs) / 1000);
        return seconds < 60 ? $"{seconds} s ago" : seconds < 3600 ? $"{seconds / 60} min ago" : $"{seconds / 3600} h ago";
    }

    public static PayloadStatus? Status(JsonObject? state, long nowMs)
    {
        if (state is null) return null;
        var action = MapUiJson.Text(state["action"]);
        var bay = Whole(state["bay"]);
        var (plain, done) = action switch
        {
            "release" => ("Drop", "Dropped"),
            "grab" => ("Grab", "Grabbed"),
            _ => ("Hold", "Holding"),
        };
        var verb = bay > 0 ? $"{plain} {bay}" : plain;
        var finished = bay > 0 ? $"Bay {bay} {done.ToLowerInvariant()}" : done;
        var ago = Ago((long)(MapUiJson.Number(state["atMs"]) ?? 0), nowMs);
        return MapUiJson.Text(state["outcome"]) switch
        {
            "sent" => new PayloadStatus($"{verb} sent, waiting for the aircraft", false),
            "confirmed" => new PayloadStatus($"{finished} {ago}", false),
            "refused" => new PayloadStatus($"{verb} refused by the flight controller {ago}", true),
            "unanswered" => new PayloadStatus(action == "release"
                ? $"{verb} not confirmed {ago}. It may have dropped, check before dropping again."
                : $"{verb} not confirmed {ago}. Check the gripper before trying again.", true),
            _ => null,
        };
    }

    public static JsonObject? GripperState(JsonObject read) =>
        read["value"] is JsonObject value ? value : read.ContainsKey("outcome") ? read : null;

    public static string? OrbitLabel(JsonObject view)
    {
        if (MapUiJson.Text(view["class"]) != "Orbit" || !MapUiJson.Flag(view["orbiting"])) return null;
        var turn = view["clockwise"] is JsonValue value && value.TryGetValue<bool>(out var clockwise)
            ? clockwise ? "clockwise" : "anticlockwise"
            : "";
        return string.Join(" ", new[] { "Orbiting", MapUiJson.Text(view["radiusText"]), turn }.Where(t => t.Length > 0));
    }

    public static string RecordClock(long seconds) =>
        string.Create(CultureInfo.InvariantCulture, $"{seconds / 3600:00}:{seconds / 60 % 60:00}:{seconds % 60:00}");

    public static IReadOnlyList<(double Yaw, bool Active)> Azimuths(JsonObject view) =>
        MapUiJson.Objects(view["gimbals"])
            .Select(g => (Yaw: MapUiJson.Number(g["yaw"]), Active: MapUiJson.Flag(g["active"])))
            .Where(g => g.Yaw.HasValue)
            .Select(g => (g.Yaw!.Value, g.Active))
            .ToList();

    public static IReadOnlyList<Geo> Beam(JsonObject view)
    {
        var points = MapUiJson.Flag(view["available"])
            ? MapUiJson.Objects(view["beam"]).Select(MapOverlayJson.Plottable).ToList()
            : [];
        return points.Count >= 4 && points.All(p => p.HasValue) ? points.Select(p => p!.Value).ToList() : [];
    }

    public static CameraDetailsState Camera(JsonObject view) => new(
        MapUiJson.Texts(view["labels"]),
        Whole(view["selected"]),
        MapUiJson.Texts(view["streamLabels"]),
        Whole(view["currentStream"]),
        MapUiJson.Flag(view["capturesPhotos"]),
        MapUiJson.Text(view["photoMode"]) == "timelapse",
        Math.Clamp(MapUiJson.Number(view["lapseSeconds"]) ?? LapseMin, LapseMin, LapseMax),
        Tracking(view),
        Pursuit(view),
        MapUiJson.Flag(view["thermalAvailable"]) && MapUiJson.Text(view["thermalMode"]) is { Length: > 0 } mode ? mode : null,
        MapUiJson.Number(view["thermalOpacity"]),
        MapUiJson.Objects(view["destructiveActions"])
            .Select(a => new DestructiveAction(
                MapUiJson.Text(a["id"]),
                MapUiJson.Text(a["label"]),
                MapUiJson.Text(a["button"]),
                MapUiJson.Text(a["title"]),
                MapUiJson.Text(a["prompt"]),
                MapUiJson.Text(a["offer"]),
                MapUiJson.Text(a["reason"])))
            .Where(a => a.Id.Length > 0 && a.Offer != "hidden")
            .ToList());

    private static int Whole(JsonNode? node, int fallback = 0) => (int)(MapUiJson.Number(node) ?? fallback);
}
