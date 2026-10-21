using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;

namespace AircastQGC.Pages.Settings;

internal enum FlasherStep { Card, Setup, Write }

internal sealed record FlasherRelease(string Version, string Label, string Published, string SizeText, bool Recommended);

internal sealed record FlasherDisk(string Id, string Name);

internal sealed record FlasherRow(string Label, string Value, bool Warn);

internal sealed record FlasherForm(
    string Hostname,
    string Ssid,
    string WifiPassword,
    bool NoWifi,
    string Country,
    string SshMode,
    string AuthorizedKey,
    string DevicePassword,
    string AuthKey,
    string ControlServer);

internal sealed record FlasherState(
    bool Available,
    string Channel,
    IReadOnlyList<string> Channels,
    string ReleasesState,
    string ReleasesError,
    IReadOnlyList<FlasherRelease> Releases,
    string Selected,
    FlasherForm Form,
    IReadOnlyDictionary<string, string> Problems,
    string KeyIdentity,
    IReadOnlyList<FlasherRow> Summary,
    IReadOnlyList<FlasherDisk> Disks,
    string CardState,
    string CardId,
    string CardText,
    string CardError,
    string DownloadState,
    string DownloadText,
    string Phase,
    bool Busy,
    double Percent,
    string SpeedText,
    string RemainingText,
    string JobError,
    string Hostname,
    bool CanStart,
    string Blocked,
    string FirstProblemField,
    string FirstProblem)
{
    public bool Finished => Flasher.FinishedPhases.Contains(Phase);

    public bool NeedsAdmin => CardState == "failed" && CardError == Flasher.NeedsAdmin;

    public FlasherRelease? Release => Releases.FirstOrDefault(r => r.Version == Selected);
}

internal static class Flasher
{
    public const string Page = "Aircast card";
    public const string View = "view.flasher";
    public const string OpenCommand = "flasher.open";
    public const string ChannelCommand = "flasher.channel";
    public const string SelectCommand = "flasher.select";
    public const string DownloadCommand = "flasher.download";
    public const string FormCommand = "flasher.form";
    public const string DisksCommand = "flasher.disks";
    public const string ChooseDiskCommand = "flasher.chooseDisk";
    public const string StartCommand = "flasher.start";
    public const string CancelCommand = "flasher.cancel";
    public const string AgainCommand = "flasher.again";
    public const string NeedsAdmin = "Writing a card needs administrator rights";
    public const string PasswordMode = "password";
    public const string KeyOnlyMode = "key-only";
    public const string Keywords = "sd card, image, flash, aircast os, wifi, hostname, ssh, tailscale";
    public const string Glance = "Set up an SD card for a new drone";
    public const string BusyHold = "Wait for the card to finish writing or cancel it first";

    public static readonly IReadOnlySet<string> FinishedPhases = new HashSet<string> { "done", "failed", "cancelled" };
    public static readonly IReadOnlySet<string> SetupFields = new HashSet<string> { "hostname", "ssid", "wifiPassword", "devicePassword", "authorizedKey", "controlServer" };
    public static readonly IReadOnlySet<string> MoreFields = new HashSet<string> { "authorizedKey", "controlServer" };
    public static readonly IReadOnlySet<string> MoreRows = new HashSet<string> { "Image", "Remote access" };
    public static readonly IReadOnlyDictionary<string, string> FieldLabels = new Dictionary<string, string>
    {
        ["hostname"] = "Drone name",
        ["ssid"] = "WiFi network",
        ["wifiPassword"] = "WiFi password",
        ["devicePassword"] = "Password for pi",
        ["authorizedKey"] = "SSH public key",
        ["controlServer"] = "Headscale server",
    };
    public static readonly IReadOnlyList<(string Mode, string Label)> SshModes =
        [(PasswordMode, "Password"), (KeyOnlyMode, "SSH key"), ("disabled", "Off")];
    public static readonly IReadOnlyList<string> StepTitles = ["Card", "Setup", "Write"];

    public static FlasherForm Form(JsonObject? form) => new(
        MapUiJson.Text(form?["hostname"]),
        MapUiJson.Text(form?["ssid"]),
        MapUiJson.Text(form?["wifiPassword"]),
        MapUiJson.Flag(form?["noWifi"]),
        MapUiJson.Text(form?["country"]),
        MapUiJson.Text(form?["sshMode"]) is { Length: > 0 } mode ? mode : KeyOnlyMode,
        MapUiJson.Text(form?["authorizedKey"]),
        MapUiJson.Text(form?["devicePassword"]),
        MapUiJson.Text(form?["authKey"]),
        MapUiJson.Text(form?["controlServer"]));

    public static string CardText(JsonObject? card) => MapUiJson.Text(card?["state"]) switch
    {
        "opening" => "Checking the card reader…",
        "waiting" => "Waiting for Windows to let Aircast use the card reader…",
        "ready" => string.Join(" · ", new[] { MapUiJson.Text(card?["label"]) is { Length: > 0 } label ? label : "Card reader", MapUiJson.Text(card?["capacityText"]) }.Where(t => t.Length > 0)),
        "failed" => MapUiJson.Text(card?["error"]),
        _ => "",
    };

    public static string DownloadText(JsonObject? download) => MapUiJson.Text(download?["state"]) switch
    {
        "running" => $"Downloading Aircast OS… {(int)(MapUiJson.Number(download?["percent"]) ?? 0)}%",
        "ready" => "Aircast OS is downloaded",
        "failed" => $"Download failed: {MapUiJson.Text(download?["error"])}",
        _ => "",
    };

    public static FlasherState? State(JsonObject view)
    {
        if (view["job"] is not JsonObject job) return null;
        var releases = view["releases"] as JsonObject;
        var identity = view["keyIdentity"] as JsonObject;
        var problems = view["problems"] as JsonObject;
        var card = view["card"] as JsonObject;
        var download = view["download"] as JsonObject;
        var first = view["firstProblem"] as JsonObject;
        return new FlasherState(
            MapUiJson.Flag(view["available"]),
            MapUiJson.Text(view["channel"]),
            MapUiJson.Texts(view["channels"]),
            MapUiJson.Text(releases?["state"]),
            MapUiJson.Text(releases?["error"]),
            MapUiJson.Objects(releases?["items"])
                .Select(r => new FlasherRelease(MapUiJson.Text(r["version"]), MapUiJson.Text(r["label"]), MapUiJson.Text(r["published"]), MapUiJson.Text(r["sizeText"]), MapUiJson.Flag(r["recommended"])))
                .ToList(),
            MapUiJson.Text(view["selected"]),
            Form(view["form"] as JsonObject),
            (problems ?? new JsonObject()).ToDictionary(p => p.Key, p => MapUiJson.Text(p.Value)),
            identity is null ? "" : string.Join(" ", new[] { "algorithm", "fingerprint", "comment" }.Select(k => MapUiJson.Text(identity[k])).Where(t => t.Length > 0)),
            MapUiJson.Objects(view["summary"])
                .Select(r => new FlasherRow(MapUiJson.Text(r["label"]), MapUiJson.Text(r["value"]), MapUiJson.Flag(r["warn"])))
                .ToList(),
            MapUiJson.Objects(view["disks"]).Select(d => new FlasherDisk(MapUiJson.Text(d["id"]), MapUiJson.Text(d["name"]))).ToList(),
            MapUiJson.Text(card?["state"]),
            MapUiJson.Text(card?["id"]),
            CardText(card),
            MapUiJson.Text(card?["error"]),
            MapUiJson.Text(download?["state"]),
            DownloadText(download),
            MapUiJson.Text(job["phase"]),
            MapUiJson.Flag(job["busy"]),
            MapUiJson.Number(job["percent"]) ?? 0,
            MapUiJson.Text(job["speedText"]),
            MapUiJson.Text(job["remainingText"]),
            MapUiJson.Text(job["error"]),
            MapUiJson.Text(job["hostname"]),
            MapUiJson.Flag(view["canStart"]),
            MapUiJson.Text(view["blocked"]),
            MapUiJson.Text(first?["field"]),
            MapUiJson.Text(first?["message"]));
    }

    public static string PhaseLabel(string phase) => phase switch
    {
        "downloading" => "Downloading Aircast OS",
        "preparing" => "Preparing the card",
        "writing" => "Writing the card",
        "verifying" => "Checking what was written",
        "customizing" => "Setting up WiFi and access",
        "done" => "Card ready",
        "failed" => "Writing failed",
        "cancelled" => "Cancelled",
        _ => "",
    };

    public static FlasherStep? StepFor(FlasherState? state) =>
        state is { } s && (s.Busy || s.Finished) ? FlasherStep.Write : null;

    public static string? ReaderToOpen(FlasherState state) =>
        state.Disks.Count == 1 && state.CardState == "none" && !state.Busy && !state.Finished ? state.Disks[0].Id : null;

    public static IReadOnlyDictionary<string, string> VisibleProblems(IReadOnlyDictionary<string, string> problems, IReadOnlySet<string> touched, bool showAll) =>
        problems.Where(p => showAll || touched.Contains(p.Key)).ToDictionary(p => p.Key, p => p.Value);

    public static bool SetupComplete(IReadOnlyDictionary<string, string> problems) => !problems.Keys.Any(SetupFields.Contains);

    public static string? DefaultSshMode(FlasherForm form) =>
        form.SshMode == KeyOnlyMode && form.AuthorizedKey.Trim().Length == 0 ? PasswordMode : null;

    public static string? MeteredNote(FlasherRelease? release, string downloadState, bool metered) =>
        release is not null && metered && downloadState != "ready" && downloadState != "running"
            ? $"Aircast OS is a {release.SizeText} download and this computer is on a metered connection."
            : null;

    public static string ProblemNote(FlasherState state) =>
        state.FirstProblem.Trim().Length == 0 ? ""
        : FieldLabels.TryGetValue(state.FirstProblemField, out var label) ? $"{label}: {state.FirstProblem}"
        : state.FirstProblem;

    public static bool ProblemInMoreOptions(FlasherState state) => MoreFields.Contains(state.FirstProblemField);

    public static FlasherStep? PreviousStep(FlasherStep step, bool busy, bool finished) =>
        step != FlasherStep.Card && !busy && !finished ? step - 1 : null;

    public static bool ShowsChannels(bool debug, string channel) => debug || channel != "stable";

    public static bool KeepsAwake(FlasherState? state) => state is { } s && (s.Busy || s.DownloadState == "running");

    public static IReadOnlyList<string> NextSteps(string ssid, string hostname, bool noWifi) =>
    [
        "Unplug the reader, put the card in the drone and power it on.",
        noWifi
            ? "Give it about 3 minutes for its first boot on Ethernet or cellular."
            : $"Give it about 3 minutes for its first boot. It joins {ssid} by itself.",
        hostname.Trim().Length > 0
            ? $"Open http://{hostname}.local on a phone or computer on the same network to see the drone and its links."
            : "Find it on your network under the image's default name.",
    ];

    public static string ReleaseLine(FlasherRelease release) =>
        string.Join(" · ", new[] { release.Published, $"{release.SizeText} download", release.Recommended ? "Recommended" : "" }.Where(t => t.Length > 0));

    public static string Again(string phase) => phase == "done" ? "Flash another card" : "Try again";
}
