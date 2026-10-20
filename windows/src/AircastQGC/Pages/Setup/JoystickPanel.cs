using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Pages.Setup;

internal sealed class JoystickPanel : UserControl
{
    private const string View = "view.joystickSetup";
    private const string Select = "joystick.select";
    private const string Enable = "joystick.enable";
    private const string Setting = "joystick.setting";
    private const string Calibration = "joystick.calibration";
    private const string ButtonAction = "joystick.buttonAction";
    private const string ButtonRepeat = "joystick.buttonRepeat";
    private const string BuiltInSticks = "joystick.builtInSticks";
    private const string NoAction = "No Action";
    private const double AxisRange = 32767;
    private static readonly string[] Basic = ["standby", "throttleModeCenterZero", "throttleSmoothing", "exponentialPct", "negativeThrust"];
    private static readonly string[] Advanced = ["circleCorrection", "axisFrequencyHz", "buttonFrequencyHz", "useDeadband"];
    private static readonly string[] Extensions = ["enableManualControlPitchExtension", "enableManualControlRollExtension"];
    private static readonly string[] Additional = Enumerable.Range(1, 6).Select(n => $"enableAdditionalAxis{n}").ToArray();
    private static readonly IReadOnlyList<(string Key, string Low, string High)> CheckRows = [("roll", "Left", "Right"), ("pitch", "Down", "Up"), ("yaw", "Left", "Right"), ("throttle", "Down", "Up")];
    private static readonly IReadOnlyDictionary<string, string> Functions = new Dictionary<string, string> { ["roll"] = "Roll", ["pitch"] = "Pitch", ["yaw"] = "Yaw", ["throttle"] = "Throttle" };

    private readonly Dictionary<int, ProgressBar> _axisBars = [];
    private readonly Dictionary<int, TextBlock> _buttonLabels = [];
    private ContentControl _live = LiveHost();
    private JsonObject _view = new();
    private string _shape = "";
    private string? _refusal;
    private bool _advanced;

    public JoystickPanel() => Live.Watch(this, View, Apply);

    private static readonly IReadOnlySet<string> Volatile = new HashSet<string> { "live", "livePositions", "holding", "raw", "event" };

    private static JsonNode? Stable(JsonNode? node) => node switch
    {
        JsonObject json => new JsonObject(json.Where(kv => !Volatile.Contains(kv.Key)).Select(kv => KeyValuePair.Create(kv.Key, Stable(kv.Value)))),
        JsonArray array => new JsonArray(array.Select(Stable).ToArray()),
        _ => node?.DeepClone(),
    };

    private void Apply(JsonObject view)
    {
        _view = view;
        var shape = Stable(view)?.ToJsonString() ?? "";
        if (shape != _shape)
        {
            _shape = shape;
            Render();
        }
        Update();
    }

    private bool Available => MapUiJson.Flag(_view["available"]);
    private IReadOnlyList<string> Names => MapUiJson.Texts(_view["names"]);
    private string? ActiveName => _view["active"] is JsonValue value && value.TryGetValue<string>(out var name) ? name : null;
    private JsonObject Cal => _view["calibration"] as JsonObject ?? new JsonObject();
    private JsonObject? Sticks => _view["builtInSticks"] is JsonObject sticks && MapUiJson.Flag(sticks["available"]) ? sticks : null;

    private IReadOnlyList<JsonObject> Settings => MapUiJson.Objects(_view["settings"]).Where(s => MapUiJson.Flag(s["visible"], true)).ToList();

    private JsonObject? SettingOf(string name) => Settings.FirstOrDefault(s => MapUiJson.Text(s["name"]) == name);

    private void Render()
    {
        _axisBars.Clear();
        _buttonLabels.Clear();
        if (!Available)
        {
            Content = null;
            return;
        }
        var refusal = _refusal is null ? null : FormLook.Text(_refusal, "FontSizeBody", "CriticalBrush");
        if (Names.Count == 0 || ActiveName is null)
        {
            Content = Sticks is null
                ? FormLook.Empty("", "No joystick", "No joysticks or gamepads detected. Pair one over Bluetooth or plug it in over USB.")
                : SetupUi.Page(BuiltIn(Sticks), refusal);
            return;
        }
        var vehicle = MapUiJson.Flag(_view["vehicle"]);
        var enabled = MapUiJson.Flag(_view["enabled"]);
        var calibrated = MapUiJson.Flag(_view["calibrated"]);
        var armed = MapUiJson.Flag(_view["armed"]);
        var calibrating = MapUiJson.Flag(Cal["calibrating"]);
        var enable = FormLook.Check("Enable", enabled, on => Act(Enable, on));
        enable.IsEnabled = vehicle && (calibrated || enabled);
        var head = new UIElement?[]
        {
            Names.Count > 1 ? SetupUi.Choice("Joystick", Names, Names.ToList().IndexOf(ActiveName), at => Act(Select, Names[at])) : null,
            Sticks is null ? null : BuiltIn(Sticks),
            FormLook.Line(12, enable, vehicle ? null : FormLook.Text("Not currently available", "FontSizeBody", "TextSecondaryBrush")),
            FormLook.Text(calibrated ? "Calibrated · saved on this device" : "Calibrate before enabling", "FontSizeBody", calibrated ? "TextPrimaryBrush" : "CriticalBrush"),
            MapUiJson.Text(_view["sendingText"]) is { Length: > 0 } sending ? FormLook.Text(sending, "FontSizeBody", MapUiJson.Flag(_view["state"]?["commanding"]) ? "AccentTextBrush" : "CriticalBrush") : null,
            refusal,
        };
        var calibration = armed ? [] : new UIElement?[]
        {
            SetupUi.Heading("Calibration"),
            calibrating ? null : SetupUi.Choice("Stick mode", Enumerable.Range(1, 4).Select(m => $"Mode {m}").ToList(),
                (int)(MapUiJson.Number(_view["transmitterMode"]) ?? 2) - 1, at => Act(Setting, "transmitterMode", at + 1), detail: MapUiJson.Text(_view["transmitterModeText"])),
            CalibrationPanel(),
        };
        var rest = calibrating ? [] : Monitor().Concat(ButtonRows()).Concat(SettingRows());
        Content = SetupUi.Page(head.Concat(calibration).Concat(rest).ToArray());
    }

    private UIElement BuiltIn(JsonObject sticks)
    {
        var live = MapUiJson.Text(sticks["status"]) == "live";
        var receiving = MapUiJson.Flag(sticks["receiving"]);
        var reason = MapUiJson.Text(sticks["reason"]);
        var text = live && receiving ? "The AX12's own sticks and switches are flying the aircraft over this app's link."
            : live ? "Waiting for the sticks."
            : reason.Length > 0 ? reason
            : "Fly with the AX12's own sticks and switches over any link, 4G included.";
        return FormLook.Stack(0, SetupUi.Heading("Built-in sticks"), FormLook.Card(SetupUi.Toggle("Built-in sticks", live, on => Act(BuiltInSticks, on), detail: text)));
    }

    private UIElement CalibrationPanel()
    {
        _live = LiveHost();
        var calibrating = MapUiJson.Flag(Cal["calibrating"]);
        var step = MapUiJson.Number(Cal["step"]);
        var steps = MapUiJson.Number(Cal["steps"]);
        var status = Cal["status"] as JsonObject;
        var lines = MapUiJson.Texts(status?["lines"]);
        var bullets = MapUiJson.Flag(status?["bullets"]);
        var checking = MapUiJson.Flag(Cal["checking"]);
        var cancel = calibrating ? FormLook.Button("Cancel", () => Step("cancel"), ButtonLook.Subtle) : null;
        if (cancel is not null) cancel.IsEnabled = MapUiJson.Flag(Cal["cancelEnabled"]);
        var next = FormLook.Button(MapUiJson.Text(Cal["nextText"]) is { Length: > 0 } text ? text : "Calibrate", () => Step("next"), ButtonLook.Accent);
        next.IsEnabled = MapUiJson.Flag(Cal["nextEnabled"], true);
        return FormLook.Stack(8,
            calibrating && step is { } at && steps is { } total ? FormLook.Text($"Step {(int)at} of {(int)total}", "FontSizeBody", "TextSecondaryBrush", true) : null,
            _live,
            lines.Count == 0 ? null
                : bullets ? FormLook.Stack(4, lines.Select(line => (UIElement?)FormLook.Text($"• {line}")))
                : FormLook.Text(string.Join(" ", lines)),
            MapUiJson.Text(Cal["warning"]) is { Length: > 0 } warning ? FormLook.Text(warning, "FontSizeBody", "CriticalBrush") : null,
            checking ? FormLook.Caption("Save keeps this calibration on this device.") : null,
            FormLook.Line(8, cancel, MapUiJson.Flag(Cal["oneSidedVisible"]) ? FormLook.Button("One-Sided", () => Step("oneSided")) : null, next));
    }

    private IEnumerable<UIElement?> Monitor() =>
        new UIElement?[] { SetupUi.Heading("Axis monitor") }.Append(FormLook.Card(MapUiJson.Objects(_view["state"]?["axes"]).Select(axis =>
        {
            var index = (int)(MapUiJson.Number(axis["index"]) ?? 0);
            var bar = SetupUi.Bar(0.5);
            _axisBars[index] = bar;
            return (UIElement)FormLook.Row(FormLook.Text(MapUiJson.Text(axis["label"])), null, bar);
        })));

    private IEnumerable<UIElement?> ButtonRows()
    {
        var actions = MapUiJson.Objects(_view["assignableActions"]);
        var names = actions.Select(a => MapUiJson.Text(a["action"])).ToList();
        var calibrated = MapUiJson.Flag(_view["calibrated"]);
        return new UIElement?[]
        {
            SetupUi.Heading("Buttons"),
            FormLook.Caption("Multiple buttons that have the same action must be pressed simultaneously to invoke the action."),
            FormLook.Card(MapUiJson.Objects(_view["state"]?["buttons"]).Select(button =>
            {
                var index = (int)(MapUiJson.Number(button["index"]) ?? 0);
                var action = button["action"] is JsonValue value && value.TryGetValue<string>(out var named) ? named : NoAction;
                var label = FormLook.Text(index.ToString(CultureInfo.InvariantCulture), "FontSizeBody", "TextPrimaryBrush", true);
                label.Width = 32;
                label.VerticalAlignment = VerticalAlignment.Center;
                _buttonLabels[index] = label;
                var combo = FormLook.Combo(names, names.IndexOf(action), 240);
                AutomationProperties.SetName(combo, $"Button {index} action");
                combo.SelectionChanged += (_, _) =>
                {
                    if (combo.SelectedIndex >= 0 && names[combo.SelectedIndex] != action) Act(ButtonAction, index, names[combo.SelectedIndex]);
                };
                var canRepeat = actions.FirstOrDefault(a => MapUiJson.Text(a["action"]) == action) is { } found && MapUiJson.Flag(found["canRepeat"]);
                var repeat = FormLook.Check("Repeat", MapUiJson.Flag(button["repeat"]), on => Act(ButtonRepeat, index, on));
                repeat.IsEnabled = canRepeat && calibrated;
                return (UIElement)FormLook.Margined(FormLook.Line(8, label, combo, repeat), new Thickness(16, 6, 16, 6));
            })),
        };
    }

    private IEnumerable<UIElement?> SettingRows()
    {
        var additional = SettingOf("additionalAxesFunction");
        var viaRc = additional is null || (int)(MapUiJson.Number(additional["value"]) ?? 1) == 1;
        var toggle = FormLook.Button(_advanced ? "Hide advanced settings" : "Advanced settings", () =>
        {
            _advanced = !_advanced;
            Render();
        }, ButtonLook.Subtle);
        var basic = new UIElement?[]
        {
            SetupUi.Heading("Settings"),
            FormLook.Card(Basic.Select(SettingOf).OfType<JsonObject>().Select(s => SettingRow(s))),
            MapUiJson.Text(_view["standby"]?["text"]) is { Length: > 0 } standby ? FormLook.Caption(standby) : null,
            toggle,
        };
        if (!_advanced) return basic;
        return basic.Concat(
        [
            FormLook.Card(Advanced.Select(SettingOf).OfType<JsonObject>().Select(s => SettingRow(s))),
            FormJson.IsTrue(SettingOf("useDeadband")?["value"]) ? FormLook.Caption("Deadband can be set during the first step of calibration by gently wiggling each axis. ") : null,
            SetupUi.Heading("MANUAL_CONTROL Extensions"),
            FormLook.Card(Extensions.Select(SettingOf).Zip(["Pitch", "Roll"]).Where(p => p.First is not null).Select(p => SettingRow(p.First!, p.Second))),
            SetupUi.Heading("Additional axes"),
            additional is null ? null : FormLook.Stack(4,
                Radio("Send using MANUAL_CONTROL", !viaRc, () => Act(Setting, "additionalAxesFunction", 0)),
                Radio("Send using RC_CHANNELS_OVERRIDE", viaRc, () => Act(Setting, "additionalAxesFunction", 1))),
            FormLook.Card(Additional.Select(SettingOf).Select((s, at) => (Setting: s, At: at)).Where(p => p.Setting is not null)
                .Select(p => SettingRow(p.Setting!, viaRc ? $"Channel {p.At + 5}" : $"Aux{p.At + 1}"))),
        ]);
    }

    private static RadioButton Radio(string text, bool on, Action pick)
    {
        var radio = new RadioButton { Content = text, IsChecked = on, GroupName = "joystickAxes" };
        radio.Checked += (_, _) =>
        {
            if (!on) pick();
        };
        return radio;
    }

    private UIElement SettingRow(JsonObject setting, string? label = null)
    {
        var name = MapUiJson.Text(setting["name"]);
        var title = label ?? MapUiJson.Text(setting["label"]);
        var value = setting["value"];
        if (MapUiJson.Text(setting["type"]) == "bool") return SetupUi.Toggle(title, FormJson.IsTrue(value), on => Act(Setting, name, on));
        var entry = SetupUi.Entry(title, MapUiJson.Number(value) is { } number ? FormText.Spell(number) : "", typed =>
        {
            if (FormText.Parse(typed) is { } parsed) Act(Setting, name, parsed);
        }, units: MapUiJson.Text(setting["units"]));
        if (setting["slider"] is not JsonObject range || FormJson.Slider(range) is not { } slider) return entry;
        var bar = new Slider
        {
            Minimum = slider.From,
            Maximum = slider.To,
            Value = Math.Clamp(MapUiJson.Number(value) ?? slider.From, slider.From, slider.To),
            StepFrequency = Math.Pow(10, -Math.Clamp(slider.Decimals, 0, 6)),
            IsThumbToolTipEnabled = false,
            Margin = new Thickness(16, 0, 16, 8),
        };
        AutomationProperties.SetName(bar, title);
        _ = new SliderCommit(bar, v => Math.Round(v, Math.Clamp(slider.Decimals, 0, 6)), (chosen, _) => Act(Setting, name, chosen));
        return FormLook.Stack(0, entry, bar);
    }

    private void Update()
    {
        MapUiJson.Objects(_view["state"]?["axes"]).ToList().ForEach(axis =>
        {
            if (_axisBars.TryGetValue((int)(MapUiJson.Number(axis["index"]) ?? 0), out var bar))
                bar.Value = Math.Clamp(((MapUiJson.Number(axis["raw"]) ?? 0) / AxisRange + 1) / 2, 0, 1);
        });
        MapUiJson.Objects(_view["state"]?["buttons"]).ToList().ForEach(button =>
        {
            if (_buttonLabels.TryGetValue((int)(MapUiJson.Number(button["index"]) ?? 0), out var label))
                label.Foreground = FormLook.Brush(MapUiJson.Text(button["event"]) is "down" or "repeat" ? "AccentTextBrush" : "TextPrimaryBrush");
        });
        _live.Content = Available && Names.Count > 0 && ActiveName is not null ? LiveCalibration() : null;
    }

    private UIElement? LiveCalibration()
    {
        var calibrating = MapUiJson.Flag(Cal["calibrating"]);
        var checking = MapUiJson.Flag(Cal["checking"]);
        var positions = Numbers(Cal["stickPositions"]);
        var live = Numbers(Cal["livePositions"]);
        var holding = MapUiJson.Number(Cal["holding"]);
        var functions = Cal["live"] as JsonObject;
        return FormLook.Stack(8,
            calibrating && !checking ? StickDiagram.Of(positions, MapUiJson.Flag(Cal["singleStickDisplay"]), live) : null,
            calibrating && !checking ? FormLook.Caption("Ring: where to move the stick. Dot: where your stick is.") : null,
            holding is { } held ? FormLook.Line(8, Sized(SetupUi.Bar(held), 160), FormLook.Caption("Got it, hold it there")) : null,
            checking ? FormLook.Card(CheckRows.Select(row =>
            {
                var grid = new Grid
                {
                    Padding = new Thickness(16, 6, 16, 6),
                    ColumnSpacing = 8,
                    ColumnDefinitions =
                    {
                        new ColumnDefinition { Width = new GridLength(80) },
                        new ColumnDefinition { Width = new GridLength(40) },
                        new ColumnDefinition(),
                        new ColumnDefinition { Width = new GridLength(40) },
                    },
                };
                var cells = new FrameworkElement[]
                {
                    FormLook.Text(Functions[row.Key]),
                    FormLook.Caption(row.Low),
                    SetupUi.Bar(((MapUiJson.Number(functions?[row.Key]) ?? 0) + 1) / 2),
                    FormLook.Caption(row.High),
                };
                cells.Select((cell, at) => (Cell: cell, At: at)).ToList().ForEach(c =>
                {
                    c.Cell.VerticalAlignment = VerticalAlignment.Center;
                    Grid.SetColumn(c.Cell, c.At);
                    grid.Children.Add(c.Cell);
                });
                return (UIElement)grid;
            })) : null);
    }

    private static ContentControl LiveHost() => new() { HorizontalContentAlignment = HorizontalAlignment.Stretch, IsTabStop = false };

    private static T Sized<T>(T element, double width) where T : FrameworkElement
    {
        element.Width = width;
        return element;
    }

    private static IReadOnlyList<double> Numbers(JsonNode? node) =>
        (node as JsonArray ?? []).Select(n => MapUiJson.Number(n) ?? 0).ToList() is { Count: >= 4 } list ? list : [0, 0, 0, 0];

    private void Step(string op) => SetupUi.Run(() => Bridge.Invoke(Calibration, FormJson.Args(op)), async answer =>
    {
        _refusal = MapUiJson.Flag(answer["ok"]) ? null : MapUiJson.Text(answer["reason"]) is { Length: > 0 } reason ? reason : null;
        _shape = "";
        Apply(Bridge.Get(View));
        if (!MapUiJson.Flag(answer["completed"]) || MapUiJson.Flag(_view["enabled"])) return;
        if (await FormLook.Confirm(XamlRoot, "Enable joystick", $"{MapUiJson.Text(answer["name"])} calibration is complete. Enable it now?", "Yes", "No")) Act(Enable, true);
    });

    private void Act(string path, params object?[] args) => SetupUi.Run(() => SetupUi.Refusal(path, args), refused =>
    {
        _refusal = refused;
        _shape = "";
        Apply(Bridge.Get(View));
    });
}
