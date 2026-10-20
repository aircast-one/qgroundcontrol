using System;
using System.Collections.Immutable;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Forms;
using AircastQGC.Pages.Analyze;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

internal sealed class VehicleStatusSheet
{
    private const string FlyStatePath = "view.flyState";
    private const string SensorsPath = "view.sensors";
    private const string MessagesPath = "view.messages";
    private const string WarningsPath = "view.warnings";
    private const string ResetMessages = "vehicle.resetAllMessages";
    private const string ClearMessages = "vehicle.clearMessages";
    private const string StatusSettingsPage = "Status Settings";
    private const string ParametersPage = "Parameters";
    private const string OverviewPage = "";
    private const double MessagesHeight = 320;
    private const double SettingsHeight = 360;

    private static readonly string[] Watched = [FlyStatePath, SensorsPath, MessagesPath, WarningsPath];

    private readonly string _client = $"vehicleStatusSheet.{Guid.NewGuid():N}";
    private readonly Sheet _sheet;
    private ImmutableDictionary<string, JsonObject> _views = ImmutableDictionary<string, JsonObject>.Empty;
    private ImmutableHashSet<int> _expanded = ImmutableHashSet<int>.Empty;
    private bool _forceOpen;
    private bool _showAllSensors;
    private bool _messagesShown;

    public VehicleStatusSheet()
    {
        _sheet = new Sheet("Vehicle status", Body, Fixed, Opened, Closed, Key);
    }

    public void ShowAt(FrameworkElement anchor) => _sheet.ShowAt(anchor);

    private JsonObject View(string path) => _views.TryGetValue(path, out var view) ? view : new JsonObject();

    private void Opened()
    {
        _forceOpen = false;
        _showAllSensors = false;
        _messagesShown = false;
        _expanded = ImmutableHashSet<int>.Empty;
        _views = Watched.ToImmutableDictionary(path => path, Bridge.Get);
        Watched.ToList().ForEach(path => BridgeWatch.Watch($"{_client}.{path}", [path], view =>
        {
            _views = _views.SetItem(path, view);
            _sheet.Refresh();
        }));
        Ui.Off(() => Bridge.Invoke(ResetMessages));
    }

    private void Closed() => Watched.ToList().ForEach(path => BridgeWatch.Stop($"{_client}.{path}"));

    private string Key()
    {
        var state = FlyStatus.Parse(View(FlyStatePath));
        return string.Join("\n",
            state with { RcSignal = null, RcSignalText = "", Telemetry = null },
            View(SensorsPath).ToJsonString(),
            View(MessagesPath).ToJsonString(),
            View(WarningsPath).ToJsonString(),
            _forceOpen, _showAllSensors, _messagesShown, string.Join(",", _expanded.Order()));
    }

    private void Close(Action next)
    {
        _sheet.Hide();
        next();
    }

    private void Changed(Action change)
    {
        change();
        _sheet.Refresh();
    }

    private UIElement Body()
    {
        var state = FlyStatus.Parse(View(FlyStatePath));
        return Ui.Column(
        [
            state.SummaryDetail.Trim().Length > 0
                ? Ui.Column([StatusLook.Line(state.StateText).With(t => t.FontWeight = FontWeights.SemiBold), StatusLook.Line(state.SummaryDetail, StatusTone.Muted, small: true)], 2)
                : null,
            StatusLook.ListRow(StatusLook.Chevron(Ui.Line([Ui.Glyph(""), Ui.Text("Messages")], 12)), "Messages", () => Close(FlyRequests.OpenMessages)),
            state.Connected ? ArmSection(state) : null,
            Sensors(),
            Messages(),
            OverallStatus(),
        ], 6);
    }

    private UIElement ArmSection(FlyStatus state)
    {
        var controls = ArmControls.For(state, _forceOpen);
        return Ui.Column(
        [
            Ui.Button(controls.ArmText, () => Close(() => FlyRequests.AskGuided(state.Armed ? "disarm" : "arm")), state.Armed ? Look.Critical : Look.Accent, controls.ArmEnabled)
                .With(b => b.HorizontalAlignment = HorizontalAlignment.Stretch),
            controls.MayBeRefused ? StatusLook.Line("Arming may be refused.", StatusTone.Muted, small: true) : null,
            controls.ForceLink ? StatusLook.Link("Force arm…", () => Changed(() => _forceOpen = true)) : null,
            controls.ForceShown
                ? Ui.Button("Force arm", () => Close(() => FlyRequests.AskGuided("forceArm")), Look.Critical).With(b => b.HorizontalAlignment = HorizontalAlignment.Stretch)
                : null,
        ], 4);
    }

    private UIElement? Sensors()
    {
        var (shown, sensors) = SensorHealth.Parse(View(SensorsPath));
        if (!shown) return null;
        var normal = sensors.Count(s => s.State == SensorHealth.Healthy);
        return Ui.Column(
        [
            StatusLook.Heading("Sensors"),
            .. SensorHealth.Shown(sensors, _showAllSensors).Select(sensor => (UIElement?)StatusLook.ListRow(
                StatusLook.Chevron(
                    StatusLook.Line(sensor.Name, sensor.State == SensorHealth.Faulty ? StatusTone.Normal : StatusTone.Muted),
                    StatusLook.Line(sensor.Label, sensor.State == SensorHealth.Faulty ? StatusTone.Critical : StatusTone.Muted, small: true)),
                $"{sensor.Name} {sensor.Label}",
                () => Close(() => FlyRequests.OpenSetup(OverviewPage)))),
            normal > 0 ? StatusLook.Link(_showAllSensors ? "Show less" : $"Show {normal} more", () => Changed(() => _showAllSensors = !_showAllSensors)) : null,
        ], 2);
    }

    private UIElement? Messages()
    {
        var lines = VehicleReports.Messages(View(MessagesPath)).Reverse().ToList();
        if (lines.Count == 0) return null;
        return Ui.Column(
        [
            StatusLook.Spread(StatusLook.Heading("Messages"), _messagesShown ? StatusLook.Link("Clear", () => Ui.Off(() => Bridge.Invoke(ClearMessages))) : null),
            StatusLook.Link(_messagesShown ? "Hide messages" : "Show messages", () => Changed(() => _messagesShown = !_messagesShown)),
            _messagesShown
                ? new ScrollViewer
                {
                    MaxHeight = MessagesHeight,
                    Content = Ui.Column(lines.Select(m => (UIElement?)Ui.Line(
                    [
                        Ui.Caption(m.Time, "TextTertiaryBrush"),
                        StatusLook.Line(m.Text, m.Level).With(t => t.MaxWidth = 280),
                    ], 8)), 4),
                }
                : null,
        ], 2);
    }

    private UIElement? OverallStatus()
    {
        var checks = VehicleReports.Checks(View(WarningsPath));
        if (checks.Count == 0) return null;
        return Ui.Column(
        [
            StatusLook.Heading("Overall status"),
            .. checks.Select((check, index) =>
            {
                var open = _expanded.Contains(index);
                var body = Ui.Column(
                [
                    StatusLook.Line(check.Message, check.Tone),
                    open ? StatusLook.Line(check.Description, StatusTone.Muted, small: true) : null,
                ], 2);
                return check.Description.Trim().Length == 0
                    ? body
                    : (UIElement?)StatusLook.ListRow(body, open ? "Hide details" : "Show details",
                        () => Changed(() => _expanded = open ? _expanded.Remove(index) : _expanded.Add(index)));
            }),
        ], 2);
    }

    private System.Collections.Generic.IEnumerable<UIElement?> Fixed()
    {
        var advanced = StatusJson.Flag(Bridge.Get("view.advancedUi")["shown"], true);
        return
        [
            StatusLook.Action("Aircraft setup", () => Close(() => FlyRequests.OpenSetup(OverviewPage))),
            new ScrollViewer { MaxHeight = SettingsHeight, Content = FormView.SetupPage(StatusSettingsPage) },
            advanced
                ? StatusLook.Spread(StatusLook.Line("Vehicle parameters"), Ui.Button("Configure", () => Close(() => FlyRequests.OpenSetup(ParametersPage))))
                : null,
        ];
    }
}
