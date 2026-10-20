using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;
using AircastQGC.Controls;
using AircastQGC.Forms;

namespace AircastQGC.Pages.Setup;

internal sealed record SetupComponent(int Index, string Name, string? Known, string ClassName, bool NeedsAttention, string? BlockedReason, string? Prerequisite, string? Page, string? Screen);

internal sealed record SetupPageInfo(string Name, bool ParameterSections, string Screen);

internal sealed record SetupReadiness(bool? SetupComplete, string Headline, string Detail, bool Connected, string Firmware, int? VehicleId);

internal sealed record ParameterWait(string Title, string Body, bool DownloadOffered);

internal sealed record SummaryLine(string Label, string Value, bool Warn);

internal enum SetupState { NeedsAttention, Done, Neutral, Unavailable }

internal enum DetailKind
{
    Prerequisite, Sensors, Radio, RemoteSupport, SubMotors, Motors, FlightModes, Px4Tuning, Px4Airframe, Actuators,
    ApmServos, ApmFollow, Scripting, Joystick, EspBridge, Syslink, SubFrame, ApmAirframe, OpticalFlow, NotSupported, Form, Desktop,
}

internal static partial class SetupModel
{
    public const string SetupPath = "view.setup";
    public const string FirmwarePath = "view.firmware";
    public const string SummaryPath = "view.setupSummary";
    public const string AdvancedUiPath = "view.advancedUi";
    public const string PageOpened = "setup.pageOpened";
    public const string DialogOpened = "setup.dialogOpened";
    public const string SummaryPage = "Summary";
    public const string ParametersPage = "Parameters";
    public const string ParameterRefresh = "parameterTools.refresh";
    public const string RebootVehicle = "vehicle.rebootVehicle";
    public const string ParametersStopped = "Setup needs them. Disconnect and connect the link to ask again.";
    public const string NothingToConfigure = "Nothing to Configure";
    public const string NothingToConfigureText = "Aircast doesn't support setup for this vehicle type. If it is already configured, you can still fly.";
    public const string AircraftSetup = "Aircraft setup";
    public const string LookingTitle = "Looking for your aircraft";
    public const string LookingHint = "Turn on the aircraft. A USB cable or telemetry radio connects by itself; for Wi-Fi, add a link.";
    public const string CalibrationBlock = "Complete or cancel the current calibration first";

    public const string Sensors = "Sensors";
    public const string Radio = "Radio";
    public const string RemoteSupport = "Remote Support";
    public const string Motors = "Motors";
    public const string FlightModes = "Flight Modes";

    private const string GlanceSeparator = " · ";

    private static readonly IReadOnlySet<string> OwnScreenHeads = new HashSet<string> { Sensors, Radio, RemoteSupport, Motors, FlightModes };

    private static readonly IReadOnlyDictionary<string, DetailKind> Screens = new Dictionary<string, DetailKind>
    {
        ["px4Tuning"] = DetailKind.Px4Tuning,
        ["px4Airframe"] = DetailKind.Px4Airframe,
        ["actuators"] = DetailKind.Actuators,
        ["apmServos"] = DetailKind.ApmServos,
        ["apmFollow"] = DetailKind.ApmFollow,
        ["scripting"] = DetailKind.Scripting,
        ["joystick"] = DetailKind.Joystick,
        ["espBridge"] = DetailKind.EspBridge,
        ["syslink"] = DetailKind.Syslink,
        ["apmSubFrame"] = DetailKind.SubFrame,
        ["apmSubMotors"] = DetailKind.SubMotors,
        ["apmAirframe"] = DetailKind.ApmAirframe,
        ["opticalFlow"] = DetailKind.OpticalFlow,
        ["notSupported"] = DetailKind.NotSupported,
    };

    private static readonly IReadOnlyList<(string Token, string Note)> Notes =
    [
        ("Failsafe", "What it does when the battery, radio or link fails"),
        ("FlightSafety", "Return altitude, landing speed and geofence"),
        ("Safety", "Return altitude, landing speed and geofence"),
        ("FlightModes", "Which mode each switch position selects"),
        ("Airframe", "The frame type the autopilot flies"),
        ("SubFrame", "The frame type the autopilot flies"),
        ("Gimbal", "Camera mount axes and their limits"),
        ("Joystick", "A gamepad as the stick instead of a radio"),
        ("Logging", "What the autopilot records and when"),
        ("Motor", "Spin each motor to check order and direction"),
        ("Power", "Battery monitor, capacity and voltage"),
        ("Radio", "Calibrate the sticks and switches"),
        ("RemoteSupport", "Share telemetry with a support engineer"),
        ("Scripting", "Lua scripts running on the autopilot"),
        ("Sensors", "Compass, accelerometer and level"),
        ("AdvancedTuning", "Every rate and filter, per axis"),
        ("Tuning", "How it responds to the sticks"),
        ("Actuator", "Outputs and what drives them"),
        ("Servo", "Outputs and what drives them"),
        ("Follow", "Follow a target or this computer"),
        ("Camera", "Camera trigger and gimbal"),
    ];

    private static readonly IReadOnlyList<(string Token, string Glyph)> ClassGlyphs =
    [
        ("Failsafe", ""),
        ("Airframe", ""),
        ("SubFrame", ""),
        ("Gimbal", ""),
        ("Logging", ""),
        ("Motor", ""),
        ("RemoteSupport", ""),
        ("Scripting", ""),
        ("Tuning", ""),
        ("Actuator", ""),
        ("Servo", ""),
        ("Follow", ""),
        ("Airspeed", ""),
        ("Lights", ""),
        ("ESP8266", ""),
        ("Syslink", ""),
    ];

    private static readonly IReadOnlySet<string> UnreadValues = new HashSet<string> { "Unknown", "--", "" };

    [GeneratedRegex(@"(\S)\(")]
    private static partial Regex TightParenthesis();

    public static string Glyph(string? known, string className) => known switch
    {
        "radio" or "joystick" => "",
        "flightModes" => "",
        "sensors" => "",
        "safety" => "",
        "power" => "",
        "esc" => "",
        _ => ClassGlyphs.FirstOrDefault(g => className.Contains(g.Token, StringComparison.Ordinal)).Glyph ?? "",
    };

    public static string Note(string className) =>
        Notes.FirstOrDefault(n => className.Contains(n.Token, StringComparison.Ordinal)).Note ?? "";

    public static string AttentionAction(string className) =>
        new[] { "Sensors", "Radio" }.Any(token => className.Contains(token, StringComparison.Ordinal)) ? "Calibrate" : "Set up";

    public static string Subtitle(string vehicle, string firmware) =>
        string.Join(GlanceSeparator, new[] { firmware, vehicle }.Where(part => part.Trim().Length > 0));

    public static string? ReadinessNote(SetupReadiness? readiness, bool listed) =>
        readiness is { SetupComplete: not true } r && !listed
            ? string.Join(". ", new[] { r.Headline, r.Detail }.Where(part => part.Trim().Length > 0)) is { Length: > 0 } note ? note : null
            : null;

    public static string? ParameterCountText(int count) =>
        count > 0 ? string.Format(CultureInfo.InvariantCulture, "{0:N0} parameters", count) : null;

    public static string PrerequisiteText(string first, string wanted) =>
        $"{MapUiJson.SentenceCase(first)} has to be set up before {MapUiJson.SentenceCase(wanted)}.";

    public static string DisabledWhile(string reason) => $"Disabled while the vehicle is {reason}";

    public static bool Matches(string name, string search) =>
        search.Trim().Length == 0 || name.Contains(search.Trim(), StringComparison.OrdinalIgnoreCase);

    public static bool SectionMatches(FormSection section, string search) =>
        search.Trim() is var query && (query.Length == 0
            || section.Keywords.Prepend(section.Title).Any(word => word.Contains(query, StringComparison.OrdinalIgnoreCase)));

    public static bool OwnScreen(SetupComponent component) => component.Page is { } page && OwnScreenHeads.Contains(page);

    public static bool CanOpen(SetupComponent component, SetupPageInfo? page) =>
        page is not null
        && (new[] { Sensors, Radio, RemoteSupport, Motors }.Contains(page.Name) || page.ParameterSections || Screens.ContainsKey(component.Screen ?? ""));

    public static bool ReportsOpening(string? prerequisite, SetupPageInfo? page) =>
        prerequisite is not null || (page?.ParameterSections != true && page?.Screen != "notSupported");

    public static SetupComponent? FirstToOpen(IReadOnlyList<SetupComponent> components, Func<SetupComponent, bool> canOpen) =>
        components.Where(canOpen).ToList() is var openable ? openable.FirstOrDefault(c => c.NeedsAttention) ?? openable.FirstOrDefault() : null;

    public static DetailKind Kind(SetupComponent component, SetupPageInfo? page) =>
        component.Prerequisite is not null ? DetailKind.Prerequisite
        : component.Screen is { } screen && Screens.TryGetValue(screen, out var own) ? own
        : component.Page == Sensors ? DetailKind.Sensors
        : component.Page == Radio ? DetailKind.Radio
        : component.Page == RemoteSupport ? DetailKind.RemoteSupport
        : component.Page == Motors ? DetailKind.Motors
        : component.Page == FlightModes ? DetailKind.FlightModes
        : page?.ParameterSections == true ? DetailKind.Form
        : DetailKind.Desktop;

    public static bool Connected(JsonObject view) => MapUiJson.Flag(view["connected"]);

    public static bool ParametersReady(JsonObject view) => Connected(view) && MapUiJson.Flag(view["parametersReady"]);

    public static bool Px4(JsonObject view) => MapUiJson.Text(view["firmware"]) == "px4";

    public static IReadOnlyList<SetupComponent> Components(JsonObject view) =>
        MapUiJson.Objects(view["components"])
            .Select((element, index) => (Element: element, Index: index, Name: MapUiJson.Text(element["name"])))
            .Where(c => c.Name.Trim().Length > 0)
            .Select(c => new SetupComponent(
                c.Index,
                c.Name,
                Optional(c.Element["known"]),
                MapUiJson.Text(c.Element["className"]),
                MapUiJson.Flag(c.Element["needsAttention"]),
                Optional(c.Element["blockedReason"]),
                Optional(c.Element["prerequisite"]),
                Optional(c.Element["page"]),
                Optional(c.Element["screen"])))
            .ToList();

    public static IReadOnlyList<SetupPageInfo> Pages(JsonObject view) =>
        MapUiJson.Objects(view["groups"])
            .SelectMany(group => MapUiJson.Objects(group["pages"]))
            .Select(page => new SetupPageInfo(MapUiJson.Text(page["name"]), MapUiJson.Flag(page["parameterSections"]), MapUiJson.Text(page["screen"])))
            .ToList();

    public static SetupPageInfo? Page(JsonObject view, SetupComponent component) =>
        component.Page is { } page ? Pages(view).FirstOrDefault(p => p.Name == page) : null;

    public static SetupReadiness Readiness(JsonObject view) => new(
        view["setupComplete"] is JsonValue complete && complete.TryGetValue<bool>(out var done) ? done : null,
        MapUiJson.Text(view["headline"]),
        MapUiJson.Text(view["detail"]),
        Connected(view),
        MapUiJson.Text(view["firmware"]),
        MapUiJson.Number(view["vehicleId"]) is { } id ? (int)id : null);

    public static ParameterWait? Wait(JsonObject view) =>
        MapUiJson.Flag(view["parametersReady"]) ? null
        : MapUiJson.Text(view["parametersReason"]) switch
        {
            "" or "noVehicle" => null,
            "loading" => new ParameterWait("Loading parameters from the vehicle.", "", false),
            "skipped" => new ParameterWait(MapUiJson.Text(view["parametersText"]), "", true),
            var reason => new ParameterWait(
                MapUiJson.Text(view["parametersText"]) is { Length: > 0 } text ? text : $"This vehicle has not sent its parameters ({reason}).",
                ParametersStopped,
                false),
        };

    public static string? ParametersIncomplete(JsonObject view) =>
        MapUiJson.Text(view["parametersReason"]) == "incomplete" ? $"Parameters Incomplete. {MapUiJson.Text(view["parametersText"])}" : null;

    public static IReadOnlyDictionary<string, IReadOnlyList<SummaryLine>> Summaries(JsonObject view) =>
        MapUiJson.Objects(view["components"])
            .GroupBy(component => MapUiJson.Text(component["name"]))
            .ToDictionary(
                group => group.Key,
                group => (IReadOnlyList<SummaryLine>)MapUiJson.Objects(group.First()["rows"])
                    .Select(row => new SummaryLine(MapUiJson.Text(row["label"]), MapUiJson.Text(row["value"]), MapUiJson.Flag(row["warn"])))
                    .ToList());

    public static IReadOnlyList<SummaryLine> GlanceParts(IReadOnlyList<SummaryLine> summary)
    {
        var read = summary
            .Select(line => line with { Label = line.Label.TrimEnd(':', ' '), Value = TightParenthesis().Replace(line.Value.Trim(), "$1 (") })
            .Where(line => !UnreadValues.Contains(line.Value))
            .ToList();
        var prefixes = read.Select(line => line.Value.Contains(' ') ? line.Value[..line.Value.IndexOf(' ')] : "").Distinct().ToList();
        var shared = prefixes.Count == 1 && prefixes[0].Length > 0 && read.Count > 1 ? prefixes[0] : null;
        var parts = shared is null
            ? read.DistinctBy(line => line.Value).ToList()
            : read.Select(line => line with { Value = $"{line.Label} {line.Value[shared.Length..].Trim()}" }).ToList();
        return parts.Select(line => line with { Value = MapUiJson.SentenceCase(line.Value) }).ToList();
    }

    public static string? Glance(IReadOnlyList<SummaryLine> summary) =>
        GlanceParts(summary) is { Count: > 0 } parts ? string.Join(GlanceSeparator, parts.Select(p => p.Value)) : null;

    private static string? Optional(JsonNode? node) => MapUiJson.Text(node) is { } text && text.Trim().Length > 0 ? text : null;
}
