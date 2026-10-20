using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using AircastQGC.Pages.Setup;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Pages.Settings;

internal sealed record PilotSetting(string Label, IReadOnlyList<string> Parameters, string Section, string Hint, string? WhenUnlimited = null, string? Quantity = null);

internal static class PilotSettings
{
    public const string SafetyPage = "Aircraft safety";
    public const string LimitsPage = "Aircraft limits";
    public const string OfflineNote = "Connect the aircraft to see and change these.";
    public const string LoadingNote = "Loading these from the aircraft…";

    private const string ReturnHome = "Return to home";
    private const string FlightProtection = "Flight protection";
    private const string Failsafes = "If something goes wrong";
    private const string FlightLimits = "Flight limits";
    private const string Altitude = "altitude";
    private const string Distance = "distance";
    private const string Speed = "speed";

    private static readonly IReadOnlyDictionary<string, string> Choices = new Dictionary<string, string>
    {
        ["Disabled"] = "Do nothing",
        ["Hold mode"] = "Hover",
        ["Loiter mode"] = "Hover",
        ["Return mode"] = "Return home",
        ["Land mode"] = "Land",
        ["Warning"] = "Warn only",
        ["Return at critical level, land at emergency level"] = "Return home, land if critical",
        ["Terminate"] = "Stop motors",
    };

    public static IReadOnlyList<(string Title, SettingsGroup Group)> Pages { get; } =
        [(SafetyPage, SettingsGroup.Safety), (LimitsPage, SettingsGroup.Control)];

    public static IReadOnlyList<PilotSetting> For(SettingsGroup group) => group switch
    {
        SettingsGroup.Safety =>
        [
            new("Return-to-home altitude", ["RTL_RETURN_ALT", "RTL_ALT_M", "RTL_ALT"], ReturnHome, "Climbs to this height before flying home.", Quantity: Altitude),
            new("Max altitude", ["GF_MAX_VER_DIST", "FENCE_ALT_MAX"], FlightProtection, "The aircraft will not climb above this.", "Most countries cap flights at 120 m.", Altitude),
            new("Max distance", ["GF_MAX_HOR_DIST", "FENCE_RADIUS"], FlightProtection, "The aircraft will not fly farther from home than this.", Quantity: Distance),
            new("Signal lost", ["NAV_DLL_ACT", "FS_GCS_ENABLE"], Failsafes, "When the link to this app drops."),
            new("Remote controller lost", ["NAV_RCL_ACT", "FS_THR_ENABLE"], Failsafes, "When the remote controller drops."),
            new("Low battery", ["COM_LOW_BAT_ACT", "BATT_FS_LOW_ACT"], Failsafes, "When the battery runs low."),
        ],
        SettingsGroup.Control =>
        [
            new("Max horizontal speed", ["MPC_XY_VEL_MAX", "LOIT_SPEED_MS", "LOIT_SPEED"], FlightLimits, "Fastest the aircraft flies forward and sideways.", Quantity: Speed),
            new("Max climb speed", ["MPC_Z_VEL_MAX_UP", "PILOT_SPD_UP", "PILOT_SPEED_UP"], FlightLimits, "Fastest the aircraft climbs.", Quantity: Speed),
            new("Max descent speed", ["MPC_Z_VEL_MAX_DN", "PILOT_SPD_DN", "PILOT_SPEED_DN"], FlightLimits, "Fastest the aircraft descends.", "At 0 it descends as fast as it climbs.", Speed),
        ],
        _ => [],
    };

    public static string Keywords(SettingsGroup group) =>
        string.Join(" ", For(group).SelectMany(s => s.Parameters.Prepend(s.Label).Prepend(s.Section))).ToLowerInvariant();

    public static IReadOnlyList<PilotSetting> SearchHits(string query) =>
        query.Trim().ToLowerInvariant() is not { Length: > 0 } wanted
            ? []
            : Pages
                .SelectMany(page => For(page.Group).Select(s => s with { Section = $"{SettingsCatalog.GroupTitle(page.Group)} › {s.Section}" }))
                .Where(s => s.Parameters.Prepend(s.Section).Prepend(s.Label).Any(text => text.ToLowerInvariant().Contains(wanted)))
                .ToList();

    public static string Choice(string label) => Choices.TryGetValue(label, out var shown) ? shown : label;

    public static FormControl Worded(FormControl control, PilotSetting setting) => control with
    {
        ShortLabel = setting.Label,
        Label = setting.Hint,
        Display = control.IsChoice ? Choice(control.Display) : control.Display,
        Options = control.Options.Select(option => option with { Label = Choice(option.Label) }).ToList(),
        Problem = setting.WhenUnlimited is { } note && control.Number == 0 ? note : control.Problem,
    };

    public static string ControlPath(string name, string? quantity) => $"view.control({ParameterModel.PathOf(name, quantity)})";

    public static UIElement Tab(SettingsGroup group, Action<string> openSettings) => FormLook.Stack(0,
        new PilotSettingsPanel(For(group), openSettings),
        group == SettingsGroup.Safety ? new SensorChecks() : null,
        new TabSetupRows(group));

    public static UIElement SetupRow(string title, string page) =>
        FormLook.Stretched(FormLook.Row(FormLook.Text(title), FormLook.Glyph("", 12)), title, () => FlyRequests.OpenSetup(page));

    public static UIElement OfflineRow(string label, string? hint)
    {
        var words = FormLook.Stack(2, FormLook.Text(label), hint is null ? null : FormLook.Caption(hint));
        var value = FormLook.Text("—", "FontSizeBody", "TextSecondaryBrush");
        words.Opacity = value.Opacity = 0.38;
        return FormLook.Row(words, value);
    }
}

internal sealed class PilotSettingsPanel : UserControl
{
    private const string Pilot = "pilot";

    private readonly IReadOnlyList<PilotSetting> _settings;
    private readonly Action<string> _openSettings;
    private readonly FormView _form;
    private readonly string _client = Live.Client("pilotRows");
    private string _state = "";

    public PilotSettingsPanel(IReadOnlyList<PilotSetting> settings, Action<string> openSettings)
    {
        _settings = settings;
        _openSettings = openSettings;
        _form = new FormView(Read, Parse) { LoadingText = PilotSettings.LoadingNote, Empty = () => new Border() };
        Visibility = FormLook.Shown(settings.Count > 0);
        Loaded += (_, _) =>
        {
            BridgeWatch.Watch(_client, settings.SelectMany(s => s.Parameters).Distinct().Select(name => ParameterModel.PathOf(name)).Prepend(SetupModel.SetupPath).ToList(), _ => Refresh());
            Refresh();
        };
        Unloaded += (_, _) => BridgeWatch.Stop(_client);
    }

    private async void Refresh()
    {
        if (_settings.Count == 0) return;
        var setup = await FormWrites.Off(() => Bridge.Get(SetupModel.SetupPath));
        var state = !SetupModel.Connected(setup) ? "offline" : !SetupModel.ParametersReady(setup) ? "loading" : "ready";
        if (state == _state)
        {
            if (state == "ready") _form.Reload();
            return;
        }
        _state = state;
        Content = state == "ready" ? _form : Offline(state == "offline");
    }

    private UIElement Offline(bool offline)
    {
        var link = offline ? FormLook.Button("Add a link", () => _openSettings(SettingsCatalog.Connections), ButtonLook.Subtle) : null;
        return FormLook.Stack(0,
            new UIElement[] { FormLook.Row(FormLook.Text(offline ? PilotSettings.OfflineNote : PilotSettings.LoadingNote, "FontSizeBody", "TextSecondaryBrush"), link) }
                .Concat(_settings.GroupBy(s => s.Section).SelectMany(section => new UIElement[]
                {
                    FormLook.Heading(MapUiJson.SentenceCase(section.Key)),
                    FormLook.Card(section.Select(s => PilotSettings.OfflineRow(s.Label, s.Hint))),
                })));
    }

    private JsonObject Read() => new()
    {
        ["rows"] = new JsonArray(_settings
            .Select((setting, at) => (At: at, Control: setting.Parameters
                .Select(name => Bridge.Get(PilotSettings.ControlPath(name, setting.Quantity)))
                .FirstOrDefault(json => MapUiJson.Text(json["name"]).Length > 0)))
            .Where(row => row.Control is not null)
            .Select(row => (JsonNode)new JsonObject { ["at"] = (double)row.At, ["control"] = row.Control })
            .ToArray()),
    };

    private IReadOnlyList<FormSection> Parse(JsonObject raw) =>
        MapUiJson.Objects(raw["rows"])
            .Select(row => (Setting: _settings.ElementAtOrDefault((int)(MapUiJson.Number(row["at"]) ?? -1)), Control: row["control"] is JsonObject json ? FormControl.Parse(json) : null))
            .Where(row => row.Setting is not null && row.Control is not null)
            .Select(row => (row.Setting!.Section, Control: PilotSettings.Worded(row.Control!, row.Setting)))
            .GroupBy(row => row.Section)
            .Select(section => new FormSection(section.Key, Pilot, "", "", [], [new FormBlock("", section.Select(row => row.Control).ToList())]))
            .ToList();
}

internal sealed class SensorChecks : UserControl
{
    private const string Section = "Sensors";
    private const string Calibrated = "Calibrated";
    private static readonly string[] Checked = ["compass", "accelerometer", "gyro"];
    private static readonly string[] OfflineSensors = ["Compass", "Accelerometer", "Gyroscope"];

    private bool _connected;
    private CalibrationState? _calibration;

    public SensorChecks()
    {
        Live.Watch(this, SetupModel.SetupPath, setup =>
        {
            if (SetupModel.Connected(setup) == _connected && Content is not null) return;
            _connected = SetupModel.Connected(setup);
            Render();
        });
        Live.Watch(this, CalibrationState.View, view =>
        {
            _calibration = CalibrationState.Parse(view);
            Render();
        });
    }

    public static IReadOnlyList<CalibrationRoutine> Checks(CalibrationState? state) =>
        Checked.Select(id => state?.Routines.FirstOrDefault(r => r.Id == id)).OfType<CalibrationRoutine>().ToList();

    public static bool Healthy(CalibrationRoutine routine) => routine.Status == Calibrated;

    public static string StatusText(CalibrationRoutine routine) =>
        Healthy(routine) ? "Normal" : routine.Status.Trim().Length > 0 ? routine.Status : "Not checked";

    private void Render()
    {
        var checks = Checks(_calibration);
        Content = !_connected
            ? FormLook.Stack(0, FormLook.Heading(Section), FormLook.Card(OfflineSensors.Select(name => PilotSettings.OfflineRow(name, null))))
            : checks.Count == 0
                ? new Border()
                : FormLook.Stack(0,
                    FormLook.Heading(Section),
                    FormLook.Card(checks.Select(Row).Append(PilotSettings.SetupRow("Calibrate sensors", SetupModel.Sensors))));
    }

    private static UIElement Row(CalibrationRoutine routine)
    {
        var calibrate = Healthy(routine) ? null : FormLook.Button("Calibrate", () => FlyRequests.OpenSetup(SetupModel.Sensors), ButtonLook.Subtle);
        if (calibrate is not null) calibrate.IsEnabled = routine.Enabled;
        var status = FormLook.Text(StatusText(routine), "FontSizeBody", Healthy(routine) ? "TextSecondaryBrush" : "CriticalBrush");
        status.VerticalAlignment = VerticalAlignment.Center;
        return FormLook.Row(FormLook.Text(MapUiJson.SentenceCase(routine.Title)), FormLook.Line(8, status, calibrate));
    }
}

internal sealed class TabSetupRows : UserControl
{
    private const string SafetySetup = "Safety";

    private static readonly IReadOnlyDictionary<string, string> Titles = new Dictionary<string, string>
    {
        [SafetySetup] = "More failsafe settings",
    };

    private string _signature = "";

    public TabSetupRows(SettingsGroup group)
    {
        var wanted = Pages(group);
        Live.Watch(this, SetupModel.SetupPath, setup =>
        {
            var shown = wanted.Where(name => SetupModel.Components(setup).Any(c => c.Name == name)).ToList();
            var signature = string.Join("|", shown);
            if (signature == _signature && Content is not null) return;
            _signature = signature;
            Content = shown.Count == 0
                ? new Border()
                : FormLook.Margined(FormLook.Card(shown.Select(name =>
                    PilotSettings.SetupRow(Titles.TryGetValue(name, out var title) ? title : MapUiJson.SentenceCase(name), name))), new Thickness(0, 20, 0, 0));
        });
    }

    public static IReadOnlyList<string> Pages(SettingsGroup group) => group switch
    {
        SettingsGroup.Safety => [SafetySetup],
        SettingsGroup.Control => [SetupModel.FlightModes],
        _ => [],
    };
}
