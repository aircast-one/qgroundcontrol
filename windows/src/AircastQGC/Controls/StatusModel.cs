using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;

namespace AircastQGC.Controls;

internal enum StatusTone { Normal, Good, Caution, Critical, Muted }

internal sealed record DetailRow(string Label, string Value, int Severity = 0)
{
    public const int Secondary = -1;

    public StatusTone Tone => Severity switch
    {
        >= 2 => StatusTone.Critical,
        1 => StatusTone.Caution,
        Secondary => StatusTone.Muted,
        _ => StatusTone.Normal,
    };
}

internal static class StatusJson
{
    public static string Text(JsonNode? node) => MapUiJson.Text(node);

    public static bool Flag(JsonNode? node, bool fallback = false) => MapUiJson.Flag(node, fallback);

    public static double? Number(JsonNode? node) => MapUiJson.Number(node);

    public static int? Whole(JsonNode? node) => Number(node) is double number ? (int)number : null;

    public static bool? MaybeFlag(JsonNode? node) =>
        node is JsonValue value && value.TryGetValue<bool>(out var flag) ? flag : null;

    public static IReadOnlyList<JsonObject> Objects(JsonNode? node) => MapUiJson.Objects(node);

    public static IReadOnlyList<double> Numbers(JsonNode? node) =>
        node is JsonArray array ? array.Select(item => Number(item) ?? -1).ToList() : [];

    public static bool Is(JsonObject? view, string kind) => view is not null && Text(view["class"]) == kind;
}

internal sealed record TelemetryLink(int LocalRssi, int? RemoteRssi, int? LocalNoise, int? RemoteNoise, int? ReceiveErrors, int? ErrorsFixed, int? TxBuffer)
{
    public static TelemetryLink? Parse(JsonNode? node) =>
        node is JsonObject radio && StatusJson.Whole(radio["localRssiDbm"]) is int local
            ? new TelemetryLink(local,
                StatusJson.Whole(radio["remoteRssiDbm"]),
                StatusJson.Whole(radio["localNoise"]),
                StatusJson.Whole(radio["remoteNoise"]),
                StatusJson.Whole(radio["receiveErrors"]),
                StatusJson.Whole(radio["errorsFixed"]),
                StatusJson.Whole(radio["txBuffer"]))
            : null;

    public string CellText => $"{LocalRssi} dBm";

    public IReadOnlyList<DetailRow> Rows => new DetailRow?[]
    {
        new("Local RSSI:", $"{LocalRssi} dBm"),
        RemoteRssi is int remote ? new("Remote RSSI:", $"{remote} dBm") : null,
        ReceiveErrors is int errors ? new("RX Errors:", $"{errors}") : null,
        ErrorsFixed is int fixedErrors ? new("Errors Fixed:", $"{fixedErrors}") : null,
        TxBuffer is int buffer ? new("TX Buffer:", $"{buffer}") : null,
        LocalNoise is int localNoise ? new("Local Noise:", $"{localNoise}") : null,
        RemoteNoise is int remoteNoise ? new("Remote Noise:", $"{remoteNoise}") : null,
    }.OfType<DetailRow>().ToList();
}

internal sealed record FlyStatus(
    bool Connected,
    bool Armed,
    bool? ContactLost,
    string StateText,
    string SummaryDetail,
    bool Nominal,
    bool Fault,
    bool CanArm,
    bool ReadyToFly,
    bool Stale,
    bool RcSupported,
    bool RcRequired,
    string RcSignalText,
    int? RcSignal,
    bool? RcOverride,
    TelemetryLink? Telemetry)
{
    public static readonly FlyStatus None = new(false, false, null, "", "", true, false, true, false, false, false, true, "", null, null, null);

    public static FlyStatus Parse(JsonObject? view) =>
        StatusJson.Is(view, "FlyState")
            ? new FlyStatus(
                StatusJson.Flag(view!["connected"]),
                StatusJson.Flag(view["armed"]),
                StatusJson.MaybeFlag(view["contactLost"]),
                string.Join(" · ", StatusJson.Text(view["stateText"]).Split(" · ").Select(MapUiJson.SentenceCase)),
                StatusJson.Text(view["summaryDetail"]),
                StatusJson.Flag(view["nominal"], true),
                StatusJson.Flag(view["fault"]),
                StatusJson.Flag(view["canArm"], true),
                StatusJson.Flag(view["readyToFly"]),
                StatusJson.Text(view["staleNotice"]).Length > 0,
                StatusJson.Flag(view["rcSupported"]),
                StatusJson.Flag(view["rcRequired"], true),
                StatusJson.Text(view["rcSignalText"]),
                StatusJson.Whole(view["rcSignal"]),
                StatusJson.MaybeFlag(view["rcOverride"]),
                TelemetryLink.Parse(view["telemetry"]))
            : None;

    public (string Text, bool Lost)? RcCell =>
        !RcSupported || RcSignalText.Trim().Length == 0 ? null
            : RcSignal == 0 && !RcRequired ? ("No RC", false)
            : ($"{RcSignalText} RC", RcSignal == 0);

    public IReadOnlyList<DetailRow> RcRows =>
        RcSignalText.Trim().Length > 0 ? [new DetailRow("RSSI", RcSignalText)] : [];

    public bool OverrideShown => RcOverride == true;
}

internal sealed record Readiness(string Text, string Detail, StatusTone Tone)
{
    private const string Ready = "Ready to fly";
    private const string NotReady = "Not ready";
    private const string AllPassed = "All checks passed.";
    private const string SetupIncomplete = "Aircraft setup is not complete.";
    private const string PreArm = "PreArm:";

    public static Readiness? Of(FlyStatus fly, JsonObject? warnings)
    {
        if (!fly.Connected || fly.Armed || fly.ContactLost == true) return null;
        var blocker = StatusJson.Text(warnings?["armingBlocker"]).Trim();
        var more = VehicleReports.Checks(warnings).Count - 1;
        var detail = fly.SummaryDetail == AllPassed && !fly.ReadyToFly ? SetupIncomplete : fly.SummaryDetail.Trim();
        return blocker.Length > 0
            ? new(Join(NotReady, Short(blocker) + (more > 0 ? $" +{more}" : "")), blocker, StatusTone.Critical)
            : fly.ReadyToFly && fly.Nominal
                ? new(Ready, detail, StatusTone.Good)
                : new(Join(fly.StateText, Short(detail)), detail, fly.Fault || !fly.CanArm ? StatusTone.Critical : StatusTone.Caution);
    }

    private static string Short(string reason)
    {
        var text = reason.Trim();
        var bare = text.StartsWith(PreArm, StringComparison.Ordinal) ? text[PreArm.Length..].TrimStart() : text;
        return bare.Split(". ")[0].TrimEnd('.');
    }

    private static string Join(params string[] parts) => string.Join(" · ", parts.Where(part => part.Length > 0));
}

internal sealed record BatteryHeadline(
    string Text,
    string Detail,
    int Severity,
    string LevelName,
    string Margin,
    int Index,
    double? Percent,
    string TimeLeft,
    double Reserve,
    double? ReturnAt,
    bool ReturnNow,
    double? DrainPerMinute)
{
    public const string ReturnNowSpoken = "Battery needed to return home. Return now.";

    public static BatteryHeadline? Parse(JsonObject? view) =>
        view is not null && StatusJson.Flag(view["available"]) && view["headline"] is JsonObject it
            ? new BatteryHeadline(
                StatusJson.Text(it["text"]),
                StatusJson.Text(it["detail"]),
                StatusJson.Whole(it["severity"]) ?? 0,
                StatusJson.Text(it["level"]),
                StatusJson.Text(it["margin"]),
                StatusJson.Whole(it["index"]) ?? 0,
                StatusJson.Number(it["percent"]),
                StatusJson.Text(it["timeLeft"]),
                StatusJson.Number(it["reserve"]) ?? 0,
                StatusJson.Number(it["returnAt"]),
                StatusJson.Flag(it["returnNow"]),
                StatusJson.Number(it["drainPerMinute"]))
            : null;

    public StatusTone Level => BatteryModel.LevelOf(LevelName);

    public StatusTone HeadlineTone => Severity > 0 ? (Severity >= 2 ? StatusTone.Critical : StatusTone.Caution) : Level;

    public StatusTone BarTone =>
        (Percent ?? 100) <= Reserve ? StatusTone.Critical
        : ReturnNow ? StatusTone.Caution
        : LevelName is "warning" or "critical" ? Level
        : StatusTone.Good;

    public string BarDescription => string.Join(", ", new[]
    {
        Percent is double percent ? $"Battery {Math.Round(percent)}%" : null,
        ReturnAt is double at ? $"return home needed at {Math.Round(at)}%" : null,
        DrainPerMinute is double drain ? $"using {Math.Round(drain)}% a minute" : null,
        ReturnNow ? ReturnNowSpoken : null,
    }.OfType<string>());
}

internal sealed record BatteryPack(string Text, StatusTone Level, int Packs);

internal static class BatteryModel
{
    private const string LimitingPack = "lowest";
    private static readonly int[] CriticalChargeStates = [3, 4, 5, 6];

    public static StatusTone LevelOf(string name) => name switch
    {
        "critical" => StatusTone.Critical,
        "warning" or "caution" => StatusTone.Caution,
        _ => StatusTone.Normal,
    };

    public static bool Available(JsonObject? view) => view is not null && StatusJson.Flag(view["available"]);

    public static IReadOnlyList<BatteryPack> Packs(JsonObject? view) =>
        Available(view)
            ? StatusJson.Objects(view!["indicatorPacks"])
                .Select(pack => (Pack: pack, Lines: (pack["indicatorLines"] as JsonArray ?? []).Select(StatusJson.Text).Where(line => line.Trim().Length > 0).ToList()))
                .Where(p => p.Lines.Count > 0)
                .Select(p => new BatteryPack(
                    string.Join(" ", new[] { StatusJson.Text(p.Pack["indicatorLabel"]), string.Join(" · ", p.Lines) }.Where(part => part.Length > 0)),
                    LevelOf(StatusJson.Text(p.Pack["level"])),
                    StatusJson.Whole(p.Pack["packCount"]) ?? 1))
                .ToList()
            : [];

    public static string CountText(int packs) => packs > 1 ? $"×{packs}" : "";

    public static StatusTone Worst(IEnumerable<BatteryPack> packs) =>
        packs.Select(p => p.Level).DefaultIfEmpty(StatusTone.Normal).Max();

    public static IReadOnlyList<DetailRow> Rows(JsonObject? view)
    {
        var packs = Available(view) ? StatusJson.Objects(view!["packs"]) : [];
        var rowsOf = (JsonObject pack) => StatusJson.Objects(pack["rows"])
            .Select(row => new DetailRow(StatusJson.Text(row["label"]), StatusJson.Text(row["value"]), StatusJson.Whole(row["severity"]) ?? DetailRow.Secondary))
            .ToList();
        var limiting = BatteryHeadline.Parse(view)?.Index;
        return packs.Count < 2
            ? packs.Take(1).SelectMany(rowsOf).ToList()
            : packs.Select((pack, index) => (Rows: rowsOf(pack), Index: index))
                .Select(p => new DetailRow(
                    string.Join(" · ", new[] { $"Battery {p.Index + 1}", p.Index == limiting ? LimitingPack : null }.OfType<string>()),
                    string.Join(" · ", p.Rows.Select(r => r.Value)),
                    Math.Max(0, p.Rows.Select(r => r.Severity).DefaultIfEmpty(DetailRow.Secondary).Max())))
                .ToList();
    }

    public static bool ReturnOffered(JsonObject? view) =>
        Available(view) && StatusJson.Objects(view!["packs"]).Any(pack => CriticalChargeStates.Contains(StatusJson.Whole(pack["chargeState"]) ?? 0));

    private static double? PackTotal(JsonObject? view, string name)
    {
        var values = Available(view)
            ? StatusJson.Objects(view!["packs"])
                .Select(pack => StatusJson.Objects(pack["facts"]).FirstOrDefault(fact => StatusJson.Text(fact["name"]) == name) is JsonObject fact
                    ? StatusJson.Number(fact["value"])
                    : null)
                .ToList()
            : [];
        return values.Count > 0 && values.All(v => v.HasValue) ? values.Sum(v => v!.Value) : null;
    }

    public static string? TotalCurrent(JsonObject? view) =>
        PackTotal(view, "current") is double amps ? string.Format(CultureInfo.InvariantCulture, "{0:0.0}A", amps) : null;

    public static string? TotalDraw(JsonObject? view) =>
        PackTotal(view, "instantPower") is double watts ? $"{Math.Round(watts, MidpointRounding.AwayFromZero)}W" : TotalCurrent(view);
}

internal enum FixLevel { None, TwoD, Good }

internal sealed record GpsStatus(int? Satellites, double? Lock, string LockText, IReadOnlyList<DetailRow> Rows)
{
    public const string NoCount = "--";

    public static GpsStatus? Parse(JsonObject? view) =>
        view is not null && StatusJson.Flag(view["available"])
            ? new GpsStatus(
                StatusJson.Whole(view["satellites"]),
                StatusJson.Number(view["lock"]),
                StatusJson.Text(view["lockText"]),
                StatusJson.Objects(view["rows"])
                    .Select(row => new DetailRow(StatusJson.Text(row["label"]), StatusJson.Text(row["value"])))
                    .Where(row => row.Label.Trim().Length > 0 && row.Value.Trim().Length > 0)
                    .ToList())
            : null;

    public FixLevel? Fix => Lock switch
    {
        null => null,
        < 2 => FixLevel.None,
        < 3 => FixLevel.TwoD,
        _ => FixLevel.Good,
    };

    public string CellText => Fix switch
    {
        FixLevel.None => "No fix",
        FixLevel.TwoD => Satellites is int twoD ? $"{twoD} sats · 2D only" : "2D only",
        FixLevel.Good => Satellites is int count ? $"{count} sats" : NoCount,
        _ => NoCount,
    };

    public StatusTone Tone => Fix switch
    {
        FixLevel.None => StatusTone.Critical,
        FixLevel.TwoD => StatusTone.Caution,
        FixLevel.Good => StatusTone.Good,
        _ => StatusTone.Normal,
    };

    public IReadOnlyList<DetailRow> Detail =>
        (LockText.Trim().Length > 0 ? new[] { new DetailRow("GPS Lock", LockText) } : []).Concat(Rows).ToList();
}

internal sealed record VehicleLinkRow(string Name, bool? CommLost);

internal sealed record VehicleLinks(bool Available, IReadOnlyList<VehicleLinkRow> Links, string Primary, bool? ContactLost)
{
    public static VehicleLinks? Parse(JsonObject? view) =>
        StatusJson.Is(view, "VehicleLinks")
            ? new VehicleLinks(
                StatusJson.Flag(view!["available"]),
                StatusJson.Objects(view["links"]).Select(link => new VehicleLinkRow(StatusJson.Text(link["name"]), StatusJson.MaybeFlag(link["commLost"]))).ToList(),
                StatusJson.Text(view["primary"]),
                StatusJson.MaybeFlag(view["contactLost"]))
            : null;

    public IReadOnlyList<DetailRow> Detail =>
        Available
            ? Links.Select(link => new DetailRow(link.Name, string.Join(" · ", new[]
                {
                    link.Name == Primary ? "carrying" : null,
                    link.CommLost == true ? "no contact" : null,
                }.OfType<string>().DefaultIfEmpty("standing by")),
                link.CommLost == true ? 2 : 0)).ToList()
            : [];
}

internal static class StatusText
{
    public const string Footnote = "Readings come from the aircraft and stop updating when it stops answering.";
    public const string NothingYet = "The vehicle has not reported anything else about this yet.";

    public static string SilenceDuration(long seconds) => seconds < 60 ? $"{seconds} s" : $"{seconds / 60} min";

    public static string Silence(long seconds) => $"No data from the aircraft for {SilenceDuration(seconds)}";

    public static string FlightTime(double? seconds)
    {
        var whole = seconds is double s && double.IsFinite(s) && s > 0 ? (int)s : 0;
        return $"{whole / 60:00}'{whole % 60:00}\"";
    }
}

internal enum TrafficLevel { Good, Caution, Warning, Critical }

internal sealed record TrafficContact(int Icao, string Callsign, double? Distance, double? Bearing, double? Altitude, double? RelativeAltitude,
    string Emergency, bool? Alert, bool Stale, string AltitudeType, bool Simulated)
{
    public string Name => Callsign.Length > 0 ? Callsign : Icao.ToString("X6", CultureInfo.InvariantCulture);
    public bool Located => Distance.HasValue && Bearing.HasValue;
    public bool Urgent => Alert == true || Emergency.Trim().Length > 0;
}

internal sealed record TrafficUnits(string Distance, string Altitude, string Heading);

internal sealed record TrafficAlert(TrafficLevel Level, string Title, string Detail);

internal sealed record TrafficReading(bool Enabled, bool Available, bool Connected, bool Receiving, bool OwnPositionKnown, bool? Alerting,
    int AlertUnknown, string Emergency, string ErrorToken, TrafficUnits Units, IReadOnlyList<TrafficContact> Contacts)
{
    private static readonly string[] CompassPoints = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];

    public static TrafficReading? Parse(JsonObject? view) =>
        StatusJson.Is(view, "AdsbTraffic")
            ? new TrafficReading(
                StatusJson.Flag(view!["enabled"]),
                StatusJson.Flag(view["available"]),
                StatusJson.Flag(view["connected"]),
                StatusJson.Flag(view["receiving"]),
                StatusJson.Flag(view["ownPositionKnown"]),
                StatusJson.MaybeFlag(view["alerting"]),
                StatusJson.Whole(view["alertUnknown"]) ?? 0,
                StatusJson.Text(view["emergency"]),
                view["error"] is JsonObject error ? StatusJson.Text(error["token"]) : "",
                view["units"] is JsonObject units
                    ? new TrafficUnits(StatusJson.Text(units["distance"]), StatusJson.Text(units["altitude"]), StatusJson.Text(units["heading"]))
                    : new TrafficUnits("", "", ""),
                StatusJson.Objects(view["contacts"]).Select(contact => new TrafficContact(
                    StatusJson.Whole(contact["icaoAddress"]) ?? 0,
                    StatusJson.Text(contact["callsign"]).Trim(),
                    StatusJson.Number(contact["distance"]),
                    StatusJson.Number(contact["bearingDegrees"]),
                    StatusJson.Number(contact["altitude"]),
                    StatusJson.Number(contact["relativeAltitude"]),
                    StatusJson.Text(contact["emergency"]),
                    StatusJson.MaybeFlag(contact["alert"]),
                    StatusJson.Flag(contact["stale"]),
                    StatusJson.Text(contact["altitudeType"]),
                    StatusJson.Flag(contact["simulated"]))).ToList())
            : null;

    public bool Shown => Enabled || Contacts.Count > 0;

    public TrafficLevel Level =>
        Emergency.Trim().Length > 0 ? TrafficLevel.Critical
        : Alerting == true ? TrafficLevel.Warning
        : ErrorToken.Length > 0 ? TrafficLevel.Caution
        : !Receiving ? TrafficLevel.Caution
        : AlertUnknown > 0 && Connected ? TrafficLevel.Caution
        : TrafficLevel.Good;

    private string? FeedFault =>
        ErrorToken == "connectFailed" ? "Traffic server unreachable"
        : ErrorToken == "linkLost" ? "Traffic feed dropped"
        : ErrorToken.Length > 0 ? "Traffic feed failed"
        : !Available ? "No traffic receiver"
        : !Receiving ? "No traffic feed"
        : null;

    public string Summary
    {
        get
        {
            var real = Contacts.Count(c => !c.Simulated);
            var synthetic = Contacts.Count(c => c.Simulated);
            return real > 0 && synthetic > 0 ? $"Traffic: {real} aircraft · {synthetic} simulated"
                : real > 0 ? $"Traffic: {real} aircraft"
                : synthetic > 0 ? $"Traffic: {synthetic} simulated"
                : FeedFault ?? "Traffic clear";
        }
    }

    public string Caption => OwnPositionKnown
        ? "Range, bearing and height are relative to the vehicle"
        : "No vehicle position, so nothing can be ranged";

    public static string EmergencyText(string token) => token switch
    {
        "hijack" => "squawking hijack",
        "radioFailure" => "squawking radio failure",
        "general" => "squawking emergency",
        _ => "",
    };

    private static string Reading(double? value, string unit, bool fine = false)
    {
        if (value is not double number) return "";
        var printed = fine && Math.Abs(number) < 10
            ? number.ToString("0.0", CultureInfo.InvariantCulture)
            : number.ToString("0", CultureInfo.InvariantCulture);
        return unit.Trim().Length == 0 ? printed : $"{printed} {unit}";
    }

    private static string Datum(string altitudeType) => altitudeType switch
    {
        "pressureQnh" => "by pressure",
        "geometric" => "by GPS",
        _ => "",
    };

    public string HeightText(TrafficContact contact) =>
        contact.RelativeAltitude is not double relative
            ? string.Join(" ", new[] { Reading(contact.Altitude, Units.Altitude), Datum(contact.AltitudeType) }.Where(p => p.Length > 0))
            : Math.Abs(relative) < 0.5 ? "my level"
            : relative > 0 ? $"{Reading(Math.Abs(relative), Units.Altitude)} above"
            : $"{Reading(Math.Abs(relative), Units.Altitude)} below";

    public string ContactText(TrafficContact contact) => string.Join("  ", new[] { contact.Simulated ? "simulated" : "" }
        .Concat(contact.Located
            ? [Reading(contact.Distance, Units.Distance, fine: true), Reading(contact.Bearing, Units.Heading), HeightText(contact)]
            : ["bearing unknown", HeightText(contact)])
        .Concat([EmergencyText(contact.Emergency), contact.Alert == true ? "alerting" : "", contact.Stale ? "stale" : ""])
        .Where(part => part.Length > 0));

    public static string CompassPoint(double bearing) =>
        CompassPoints[(int)(((Math.Round(bearing / 45.0) % 8) + 8) % 8)];

    public TrafficContact? Threat => Contacts.Where(c => c.Urgent).OrderBy(c => c.Distance ?? double.MaxValue).FirstOrDefault();

    public string ThreatText(TrafficContact contact) => string.Join(" · ", new[]
    {
        contact.Name,
        string.Join(" ", new[] { Reading(contact.Distance, Units.Distance, fine: true), contact.Bearing is double b ? CompassPoint(b) : "" }.Where(p => p.Length > 0)),
        HeightText(contact),
    }.Where(part => part.Length > 0));

    public TrafficAlert? Alert
    {
        get
        {
            var level = Level;
            if (!Shown || level == TrafficLevel.Good) return null;
            var threat = level >= TrafficLevel.Warning ? Threat : null;
            var title = level switch
            {
                TrafficLevel.Critical => $"Aircraft {(EmergencyText(Emergency) is { Length: > 0 } said ? said : "in emergency")}",
                TrafficLevel.Warning => "Aircraft nearby",
                _ => FeedFault ?? $"Collision alerts unknown for {AlertUnknown} aircraft",
            };
            return new TrafficAlert(level, title, threat is null ? "" : ThreatText(threat));
        }
    }
}

internal sealed record EscMotor(string Title, bool Healthy, IReadOnlyList<(string Label, string Value)> Rows);

internal sealed record EscSummary(int OnlineCount, bool Healthy, string HealthText, string HealthyMotorsText, long TotalErrors, IReadOnlyList<EscMotor> Motors)
{
    public static EscSummary? Parse(JsonObject? view) =>
        view is not null && StatusJson.Flag(view["shown"])
            ? new EscSummary(
                StatusJson.Whole(view["onlineCount"]) ?? 0,
                StatusJson.Flag(view["healthy"]),
                StatusJson.Text(view["healthText"]),
                StatusJson.Text(view["healthyMotorsText"]),
                (long)(StatusJson.Number(view["totalErrors"]) ?? 0),
                StatusJson.Objects(view["motors"]).Select(motor => new EscMotor(
                    StatusJson.Text(motor["title"]),
                    StatusJson.Flag(motor["healthy"]),
                    new[] { ("RPM", "rpm"), ("Temp", "temperature"), ("Voltage", "voltage"), ("Current", "current"), ("Errors", "errors") }
                        .Select(pair => (pair.Item1, StatusJson.Text(motor[pair.Item2])))
                        .Where(pair => pair.Item2.Trim().Length > 0)
                        .ToList())).ToList())
            : null;

    public string CellText => $"ESC {OnlineCount} {HealthText}";
}

internal sealed record RemoteIdStatus(string State, bool Comms, bool Arm, string ArmError, bool Gps, bool BasicId, bool OperatorIdShown, bool OperatorId, bool Emergency, long HoldMs)
{
    public static RemoteIdStatus? Parse(JsonObject? view) =>
        view is not null && StatusJson.Flag(view["shown"])
            ? new RemoteIdStatus(
                StatusJson.Text(view["state"]),
                StatusJson.Flag(view["comms"]),
                StatusJson.Flag(view["armStatus"]),
                StatusJson.Text(view["armStatusError"]),
                StatusJson.Flag(view["gcsGps"]),
                StatusJson.Flag(view["basicId"]),
                StatusJson.Flag(view["operatorIdShown"]),
                StatusJson.Flag(view["operatorId"]),
                StatusJson.Flag(view["emergency"]),
                (long)(StatusJson.Number(view["emergencyHoldMs"]) ?? 800))
            : null;

    public StatusTone Tone => State switch
    {
        "healthy" => StatusTone.Good,
        "warning" => StatusTone.Caution,
        "error" => StatusTone.Critical,
        _ => StatusTone.Muted,
    };

    public IReadOnlyList<(string Label, bool Good)> Rows => new (string, bool)?[]
    {
        (Comms ? "RID COMMS" : "NOT CONNECTED", Comms),
        Comms ? ("ARM STATUS", Arm) : null,
        Comms ? ("GCS GPS", Gps) : null,
        Comms ? ("BASIC ID", BasicId) : null,
        Comms && OperatorIdShown ? ("OPERATOR ID", OperatorId) : null,
    }.OfType<(string, bool)>().ToList();

    public string EmergencyHint => Emergency
        ? "EMERGENCY HAS BEEN DECLARED, Press and Hold for 3 seconds to cancel"
        : "Press and Hold below button to declare emergency";

    public string EmergencyButton => Emergency ? "Clear Emergency" : "EMERGENCY";

    public const string SelfIdNote = "If an emergency is declared, Emergency Text will be broadcast even if Broadcast setting is not enabled.";
    public const string SendSelfId = "sendSelfID";
    public static readonly IReadOnlyList<string> SelfIdFacts = [SendSelfId, "selfIDType", "selfIDFree", "selfIDExtended", "selfIDEmergency"];
    public static readonly IReadOnlySet<string> BroadcastGated = new HashSet<string> { "selfIDType", "selfIDFree", "selfIDExtended" };
    public static readonly IReadOnlyDictionary<string, string> SelfIdLabels = new Dictionary<string, string> { [SendSelfId] = "Broadcast", ["selfIDType"] = "Broadcast message" };
}

internal sealed record ResilienceMark(bool Shown, string Colour)
{
    public StatusTone Tone => Colour switch
    {
        "good" => StatusTone.Good,
        "warning" or "alert" => StatusTone.Caution,
        "error" => StatusTone.Critical,
        _ => StatusTone.Muted,
    };
}

internal sealed record ResilienceSection(string Title, IReadOnlyList<(string Label, string Text)> Rows);

internal sealed record GpsResilience(ResilienceMark Interference, ResilienceMark Authentication, IReadOnlyList<ResilienceSection> Sections)
{
    private static ResilienceMark Mark(JsonNode? node) =>
        node is JsonObject mark ? new ResilienceMark(StatusJson.Flag(mark["shown"]), StatusJson.Text(mark["colour"])) : new ResilienceMark(false, "");

    public static GpsResilience? Parse(JsonObject? view) =>
        view is not null && StatusJson.Flag(view["shown"])
            ? new GpsResilience(
                Mark(view["interference"]),
                Mark(view["authentication"]),
                StatusJson.Objects(view["sections"]).Select(section => new ResilienceSection(
                    StatusJson.Text(section["title"]),
                    StatusJson.Objects(section["rows"]).Select(row => (StatusJson.Text(row["label"]), StatusJson.Text(row["text"]))).ToList())).ToList())
            : null;
}

internal sealed record RtkStatus(bool Active, bool Valid, int? Satellites, double? Duration, double? Accuracy, string AccuracyText, double? Latitude, double? Longitude, double? Altitude)
{
    public const string SettingsPage = "RTK GPS";
    public const string SettingsGroup = "settings.rtkSettings";
    public const string AutoConnectPath = "settings.autoConnectSettings.autoConnectRTKGPS";

    public static RtkStatus? Parse(JsonObject? view) =>
        view is not null && StatusJson.Flag(view["connected"])
            ? new RtkStatus(
                StatusJson.Flag(view["active"]),
                StatusJson.Flag(view["valid"]),
                StatusJson.Whole(view["numSatellites"]),
                StatusJson.Number(view["currentDuration"]),
                StatusJson.Number(view["currentAccuracy"]),
                StatusJson.Text(view["currentAccuracyText"]),
                StatusJson.Number(view["currentLatitude"]),
                StatusJson.Number(view["currentLongitude"]),
                StatusJson.Number(view["currentAltitude"]))
            : null;

    public string Headline => Active ? "Survey-in Active" : "RTK Streaming";

    public IReadOnlyList<(string Label, string Value)> Rows => new (string, string)?[]
    {
        ("Satellites", Satellites?.ToString(CultureInfo.InvariantCulture) ?? ""),
        ("Duration", $"{(Duration ?? 0).ToString("0", CultureInfo.InvariantCulture)} s"),
        AccuracyText.Length > 0 && (Accuracy ?? 0) > 0 ? (Valid ? "Accuracy" : "Current Accuracy", AccuracyText) : null,
    }.OfType<(string, string)>().ToList();

    public IReadOnlyList<(string Path, double Value)>? BaseWrites =>
        Valid && Latitude is double lat && Longitude is double lon && Altitude is double alt && Accuracy is double acc
            ? [($"{SettingsGroup}.fixedBasePositionLatitude", lat), ($"{SettingsGroup}.fixedBasePositionLongitude", lon),
               ($"{SettingsGroup}.fixedBasePositionAltitude", alt), ($"{SettingsGroup}.fixedBasePositionAccuracy", acc)]
            : null;
}

internal sealed record AircastLink(string QualityText, string SignalText, string Network, string Modem, string BitrateText, IReadOnlyList<double> QualityHistory, IReadOnlyList<double> BitrateHistory)
{
    public const double SignalMaximum = 100;

    public static AircastLink? Parse(JsonObject? view) =>
        view is not null && StatusJson.Flag(view["shown"])
            ? new AircastLink(
                StatusJson.Text(view["qualityText"]),
                StatusJson.Text(view["signalText"]),
                StatusJson.Text(view["network"]),
                StatusJson.Text(view["modem"]),
                StatusJson.Text(view["bitrateText"]),
                StatusJson.Numbers(view["qualityHistory"]),
                StatusJson.Numbers(view["bitrateHistory"]))
            : null;

    public string CellText => $"{QualityText} {BitrateText}";

    public static IReadOnlyList<IReadOnlyList<(double X, double Y)>> SparkRuns(IReadOnlyList<double> values, double maximum)
    {
        if (values.Count < 2) return [];
        var top = values.Max();
        var peak = maximum > 0 ? maximum : top > 0 ? top : 1;
        var step = 1.0 / (values.Count - 1);
        return values.Select((value, index) => (value, index))
            .Aggregate(ImmutableList.Create(ImmutableList<(double X, double Y)>.Empty), (runs, point) => point.value < 0
                ? runs[^1].IsEmpty ? runs : runs.Add(ImmutableList<(double X, double Y)>.Empty)
                : runs.SetItem(runs.Count - 1, runs[^1].Add((point.index * step, 1 - Math.Min(point.value, peak) / peak))))
            .Where(run => run.Count >= 2)
            .Select(run => (IReadOnlyList<(double X, double Y)>)run)
            .ToList();
    }
}

internal sealed record JoystickBadge(string Heading, string EnabledText, bool Warn, string TypeText, string InputsText, IReadOnlyList<DetailRow> Details)
{
    public static JoystickBadge? Parse(JsonObject? view) =>
        view?["indicator"] is JsonObject it
            ? new JoystickBadge(
                StatusJson.Text(it["heading"]),
                StatusJson.Text(it["enabledText"]),
                StatusJson.Flag(it["warn"]),
                StatusJson.Text(it["typeText"]),
                StatusJson.Text(it["inputsText"]),
                StatusJson.Objects(it["details"]).Select(row => new DetailRow(StatusJson.Text(row["label"]), StatusJson.Text(row["value"]), StatusJson.Flag(row["warn"]) ? 2 : 0)).ToList())
            : null;
}

internal sealed record GcsBatteryReading(string State, string LevelText, string StateText, string Heading, string Title)
{
    public static string PathFor(int percent, bool charging) =>
        $"view.gcsBattery({percent.ToString(CultureInfo.InvariantCulture)},{(charging ? "true" : "false")})";

    public static GcsBatteryReading? Parse(JsonObject? view) =>
        view is not null && view.ContainsKey("state")
            ? new GcsBatteryReading(StatusJson.Text(view["state"]), StatusJson.Text(view["levelText"]), StatusJson.Text(view["stateText"]),
                StatusJson.Text(view["heading"]), StatusJson.Text(view["title"]))
            : null;

    public StatusTone Tone => State switch
    {
        "charging" => StatusTone.Good,
        "critical" => StatusTone.Critical,
        "low" => StatusTone.Caution,
        _ => StatusTone.Normal,
    };
}

internal sealed record OfflineLink(int Index, string Text, string Description, bool Retry, bool Failed, bool Connected);

internal sealed record OfflineStatus(string Title, string Footnote, bool Busy, bool NoLinks, bool EditAddress, IReadOnlyList<OfflineLink> Links)
{
    public const string LookingHint = "Turn on the aircraft. A USB cable or telemetry radio connects by itself; for Wi-Fi, add a link.";

    public static OfflineStatus? Parse(JsonObject? view) =>
        view is not null && view.ContainsKey("title")
            ? new OfflineStatus(
                StatusJson.Text(view["title"]),
                StatusJson.Text(view["footnote"]),
                StatusJson.Flag(view["busy"]),
                StatusJson.Flag(view["noLinks"]),
                StatusJson.Flag(view["editAddress"]),
                StatusJson.Objects(view["links"]).Select(link => new OfflineLink(
                    StatusJson.Whole(link["index"]) ?? 0,
                    StatusJson.Text(link["text"]),
                    StatusJson.Text(link["description"]),
                    StatusJson.Flag(link["retry"]),
                    StatusJson.Flag(link["failed"]),
                    StatusJson.Flag(link["connected"]))).ToList())
            : null;
}

internal sealed record ArmControls(string ArmText, bool ArmEnabled, bool MayBeRefused, bool ForceLink, bool ForceShown)
{
    public const string Unavailable = "Arming is not available right now.";

    public static ArmControls For(FlyStatus state, bool forceOpen) => new(
        state.Armed ? "Disarm…" : "Arm…",
        state.CanArm,
        !state.Armed && !state.Nominal && state.CanArm && !forceOpen,
        !state.Armed && !forceOpen && (!state.CanArm || state.Fault),
        !state.Armed && forceOpen);
}

internal sealed record SensorHealth(string Name, string State, string Label)
{
    public const string Healthy = "healthy";
    public const string Faulty = "unhealthy";

    public static (bool Shown, IReadOnlyList<SensorHealth> Sensors) Parse(JsonObject? view)
    {
        var sensors = StatusJson.Is(view, "SensorHealth")
            ? StatusJson.Objects(view!["sensors"])
                .Select(item => new SensorHealth(StatusJson.Text(item["name"]), StatusJson.Text(item["state"]), StatusJson.Text(item["label"])))
                .Where(sensor => sensor.Name.Trim().Length > 0)
                .ToList()
            : [];
        var shown = StatusJson.Is(view, "SensorHealth") && !StatusJson.Flag(view!["healthChecksSupported"]) && StatusJson.Flag(view["available"]) && sensors.Count > 0;
        return (shown, sensors);
    }

    public static IReadOnlyList<SensorHealth> Shown(IReadOnlyList<SensorHealth> sensors, bool all) =>
        all ? sensors : sensors.Where(s => s.State != Healthy).ToList();
}

internal sealed record StatusMessage(int Index, string Time, StatusTone Level, string Text);

internal sealed record ArmingCheck(string Message, string Description, StatusTone Tone);

internal static class VehicleReports
{
    private static readonly System.Text.RegularExpressions.Regex ClockWithMillis = new(@"^(\d{1,2}:\d{2}:\d{2})\.\d+$");

    public static string MessageTime(string served) =>
        ClockWithMillis.Match(served.Trim()) is { Success: true } match ? match.Groups[1].Value : served;

    public static StatusTone LevelOf(string name) => name switch
    {
        "error" => StatusTone.Critical,
        "warning" => StatusTone.Caution,
        _ => StatusTone.Normal,
    };

    public static IReadOnlyList<StatusMessage> Messages(JsonObject? view)
    {
        var read = StatusJson.Objects(view?["items"])
            .Select((item, at) => new StatusMessage(StatusJson.Whole(item["index"]) ?? at, MessageTime(StatusJson.Text(item["time"])), LevelOf(StatusJson.Text(item["level"])), StatusJson.Text(item["text"])))
            .Where(message => message.Text.Trim().Length > 0)
            .ToList();
        return StatusJson.Text(view?["order"]) == "oldestFirst" ? read : read.AsEnumerable().Reverse().ToList();
    }

    public static IReadOnlyList<ArmingCheck> Checks(JsonObject? view) =>
        StatusJson.Objects(view?["armingChecks"])
            .Select(check => new ArmingCheck(StatusJson.Text(check["message"]), StatusJson.Text(check["description"]), LevelOf(StatusJson.Text(check["severity"]))))
            .Where(check => check.Message.Trim().Length > 0)
            .ToList();
}
