using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Linq;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;

namespace AircastQGC.Pages.Setup;

internal sealed record TestChannel(string Label, int Function, double Min, double Max, double? Default, bool IsMotor)
{
    private const double SnapFraction = 0.15;

    public bool Snap => Default is null;
    public double SnapRange => (Max - Min) * SnapFraction;
    public double From => Snap ? Min - SnapRange : Min;
    public double Rest => Default ?? From;

    public double Snapped(double value) =>
        !Snap || value >= Min ? value
        : value < Min - SnapRange / 2 ? Min - SnapRange
        : Min;

    public double? Sent(double value) => value < Min - SnapRange / 2 ? Default : value;

    public static TestChannel? Parse(JsonNode? node) => node is not JsonObject json ? null : new TestChannel(
        MapUiJson.Text(json["label"]),
        (int)(MapUiJson.Number(json["function"]) ?? 0),
        MapUiJson.Number(json["min"]) ?? 0,
        MapUiJson.Number(json["max"]) ?? 1,
        MapUiJson.Number(json["default"]),
        MapUiJson.Flag(json["isMotor"]));
}

internal static partial class ActuatorFacts
{
    public const string Bitset = "bitset";
    public const string TrueIfPositive = "true-if-positive";

    [GeneratedRegex("<[^>]+>")]
    private static partial Regex Tag();

    public static string Plain(string html) =>
        string.Join("\n", Tag().Replace(html.Replace("<br />", "\n"), "").Split('\n').Select(line => line.TrimEnd()));

    public static double? Raw(JsonObject control) => MapUiJson.Number(control["value"]) ?? FormText.Parse(FormJson.Shown(control["valueString"]));

    public static bool BitChecked(double raw, int bit) => ((long)raw & (1L << bit)) != 0;

    public static long BitWritten(double raw, int bit, bool on) => on ? (long)raw | (1L << bit) : (long)raw & ~(1L << bit);

    public static double SignWritten(double raw, bool on) => on ? Math.Abs(raw) : -Math.Abs(raw);

    public static string Label(JsonObject control) => MapUiJson.Text(control["label"]);
}

internal sealed class ActuatorsPanel : UserControl
{
    private const string View = "view.actuatorOutputs";
    private const string MixerSet = "actuatorMixer.set";
    private const string MixerAxis = "actuatorMixer.setAxis";
    private const string TestActive = "actuatorTest.setActive";
    private const string TestSet = "actuatorTest.setChannelTo";
    private const string TestStop = "actuatorTest.stopControl";
    private const string ActionTrigger = "actuatorAction.trigger";
    private const string AssignInit = "motorAssignment.init";
    private const string AssignStart = "motorAssignment.start";
    private const string AssignSelect = "motorAssignment.selectMotor";
    private const string AssignSpin = "motorAssignment.spinCurrentMotor";
    private const string AssignAbort = "motorAssignment.abort";
    private const string NotAvailable = "(Param not available)";
    private static readonly TimeSpan AssignPoll = TimeSpan.FromMilliseconds(300);
    private static readonly TimeSpan SendEvery = TimeSpan.FromMilliseconds(50);

    private readonly DispatcherTimer _assignPoll = new() { Interval = AssignPoll };
    private readonly DispatcherTimer _sender = new() { Interval = SendEvery };
    private readonly TextBlock _refusal = SetupUi.Error();
    private JsonObject _view = new();
    private int _tab;
    private bool _advanced;
    private bool _testing;
    private string _testShape = "";
    private ImmutableDictionary<int, double> _values = ImmutableDictionary<int, double>.Empty;
    private ImmutableHashSet<int> _moved = ImmutableHashSet<int>.Empty;
    private double _allMotors;
    private int _reads;

    public ActuatorsPanel()
    {
        Content = SetupUi.Notice("Reading actuator metadata.");
        _assignPoll.Tick += (_, _) => Refresh();
        _sender.Tick += (_, _) => Send();
        Loaded += (_, _) => Refresh();
        Unloaded += (_, _) =>
        {
            _assignPoll.Stop();
            _sender.Stop();
            SetupUi.InOrder(() => FormWrites.Ok(TestActive, false));
        };
    }

    private IReadOnlyList<TestChannel> Actuators => MapUiJson.Objects(_view["testing"]?["actuators"]).Select(TestChannel.Parse).OfType<TestChannel>().ToList();

    private bool Assigning => MapUiJson.Flag(_view["motorAssignment"]?["active"]);

    private async void Refresh()
    {
        var read = ++_reads;
        var view = await FormWrites.Off(() => Bridge.Get(View));
        if (read != _reads) return;
        _view = view;
        if (Assigning) _assignPoll.Start();
        else _assignPoll.Stop();
        var shape = string.Join(",", Actuators.Select(a => a.Function));
        if (shape != _testShape)
        {
            _testShape = shape;
            _values = Actuators.ToImmutableDictionary(a => a.Function, a => a.Rest);
            _moved = ImmutableHashSet<int>.Empty;
            _allMotors = TestChannel.Parse(_view["testing"]?["allMotors"])?.Rest ?? 0;
        }
        Render();
    }

    private void Render()
    {
        if (MapUiJson.Text(_view["class"]) != "ActuatorOutputs" || !MapUiJson.Flag(_view["available"]) || !MapUiJson.Flag(_view["showUi"], true))
        {
            Content = SetupUi.Notice(MapUiJson.Text(_view["reason"]) is { Length: > 0 } reason ? reason : "This vehicle does not configure its actuators here.");
            return;
        }
        var groups = MapUiJson.Objects(_view["groups"]);
        var index = Math.Clamp(_tab, 0, Math.Max(0, groups.Count - 1));
        var group = groups.ElementAtOrDefault(index);
        var advanced = FormLook.Check("Advanced", _advanced, on =>
        {
            _advanced = on;
            Render();
        });
        advanced.HorizontalAlignment = HorizontalAlignment.Right;
        Content = SetupUi.Page(new UIElement?[]
        {
            advanced,
            _view["geometry"] is JsonObject geometry ? Geometry(geometry) : null,
            _view["testing"] is JsonObject testing ? Testing(testing) : null,
            SetupUi.Heading("Actuator outputs"),
            MapUiJson.Flag(_view["hasUnsetRequiredFunctions"]) ? FormLook.Text("One or more actuator still needs to be assigned to an output.", "FontSizeBody", "CriticalBrush") : null,
            Assignment(group, index),
            Tabs(groups, index),
            group is null ? null : Group(group),
            _refusal,
        });
    }

    private UIElement Tabs(IReadOnlyList<JsonObject> groups, int index) =>
        FormLook.Line(8, groups.Select((group, at) =>
        {
            var tab = new ToggleButton { Content = MapUiJson.Text(group["label"]), IsChecked = at == index };
            tab.Click += (_, _) =>
            {
                _tab = at;
                Render();
            };
            return (UIElement?)tab;
        }).ToArray());

    private UIElement? Assignment(JsonObject? group, int index)
    {
        var assignment = _view["motorAssignment"] as JsonObject;
        if (assignment is null || !MapUiJson.Flag(assignment["multirotor"])) return null;
        var active = MapUiJson.Flag(assignment["active"]);
        var identify = !active && group is not null && MapUiJson.Flag(group["groupsVisible"])
            ? FormLook.Button("Identify & Assign Motors", () => Identify(index), ButtonLook.Accent)
            : null;
        if (identify is not null) identify.IsEnabled = MapUiJson.Flag(assignment["enabled"]) && !_testing;
        return FormLook.Line(8,
            identify,
            active ? FormLook.Button("Spin motor again", () => SetupUi.Detached(() => FormWrites.Ok(AssignSpin))) : null,
            active ? FormLook.Button("Abort", () => SetupUi.Run(() => FormWrites.Ok(AssignAbort), _ => Refresh())) : null);
    }

    private async void Identify(int group)
    {
        var refused = await FormWrites.Off(() => SetupUi.Refusal(AssignInit, group));
        if (refused is not null)
        {
            await FormLook.Tell(XamlRoot, "Error", ActuatorFacts.Plain(refused));
            return;
        }
        var message = await FormWrites.Off(() => MapUiJson.Text(Bridge.Get(View)["motorAssignment"]?["message"]));
        if (!await FormLook.Confirm(XamlRoot, "Motor order identification and assignment", ActuatorFacts.Plain(message), "Yes", "No")) return;
        SetupUi.Run(() => FormWrites.Ok(AssignStart), _ => Refresh());
    }

    private UIElement Geometry(JsonObject geometry)
    {
        var editable = !_testing && !Assigning;
        var help = MapUiJson.Text(geometry["helpUrl"]) is { Length: > 0 } url
            ? FormLook.IconButton("", "Help", () => _ = Windows.System.Launcher.LaunchUriAsync(new Uri(url)))
            : null;
        var motors = MapUiJson.Objects(geometry["motors"]);
        var body = FormLook.Stack(8, MapUiJson.Objects(geometry["groups"]).SelectMany(group => new UIElement?[]
        {
            SetupUi.Heading(MapUiJson.Text(group["label"])),
            SetupUi.Control(group["count"]) is { } count ? SetupUi.Embedded([count], Refresh) : null,
        }
        .Concat(MapUiJson.Objects(group["channels"]).Select(channel => Card(MapUiJson.Text(channel["label"]),
            (channel["cells"] as JsonArray ?? []).OfType<JsonObject>().Where(cell => !MapUiJson.Flag(cell["hidden"])).Select(Cell))))
        .Concat(MapUiJson.Objects(group["params"]).Where(Visible).Select(FactRow))));
        body.IsHitTestVisible = editable;
        body.Opacity = editable ? 1 : 0.38;
        return FormLook.Stack(8,
            FormLook.Line(8, FormLook.Text(MapUiJson.Text(geometry["title"]), "FontSizeBodyLarge", "TextPrimaryBrush", true), help),
            motors.Count > 1 ? GeometryImage.Of(motors, (_view["motorAssignment"]?["highlighted"] as JsonArray ?? []).Select(n => (int)(MapUiJson.Number(n) ?? -1)).ToHashSet(),
                index => SetupUi.Run(() => FormWrites.Ok(AssignSelect, index), _ => Refresh())) : null,
            body);
    }

    private bool Visible(JsonObject item) => _advanced || !MapUiJson.Flag(item["advanced"]);

    private UIElement? Cell(JsonObject cell)
    {
        if (!Visible(cell)) return null;
        var disabled = MapUiJson.Flag(cell["disabled"]);
        if (MapUiJson.Flag(cell["axis"]))
        {
            var axes = MapUiJson.Texts(cell["options"]);
            var parameters = new JsonArray((cell["params"] as JsonArray ?? []).Select(p => p?.DeepClone()).ToArray());
            return SetupUi.Choice("Axis", axes, (int)(MapUiJson.Number(cell["index"]) ?? -1), picked =>
                SetupUi.Run(() => FormWrites.Ok(MixerAxis, parameters.DeepClone(), picked), _ => Refresh()), !disabled);
        }
        if (MapUiJson.Flag(cell["unavailable"])) return SetupUi.Pair(MapUiJson.Text(cell["label"]), NotAvailable);
        if (MapUiJson.Flag(cell["fixed"])) return SetupUi.Pair(MapUiJson.Text(cell["label"]), MapUiJson.Text(cell["valueString"]));
        var param = MapUiJson.Text(cell["param"]);
        var function = (int)(MapUiJson.Number(cell["channelFunction"]) ?? 0);
        var label = ActuatorFacts.Label(cell);
        void Set(object value) => SetupUi.Run(() => FormWrites.Ok(MixerSet, param, value, function), _ => Refresh());
        var raw = ActuatorFacts.Raw(cell);
        var showAs = MapUiJson.Text(cell["showAs"]);
        var bit = (int)(MapUiJson.Number(cell["bit"]) ?? 0);
        var options = MapUiJson.Objects(cell["options"]);
        return showAs == ActuatorFacts.TrueIfPositive && raw is { } positive ? Checked(label, positive > 0, on => Set(ActuatorFacts.SignWritten(positive, on)), !disabled)
            : showAs == ActuatorFacts.Bitset && raw is { } bits ? Checked(label, ActuatorFacts.BitChecked(bits, bit), on => Set(ActuatorFacts.BitWritten(bits, bit, on)), !disabled)
            : options.Count > 0 ? SetupUi.Choice(label, options.Select(o => MapUiJson.Text(o["label"])).ToList(),
                options.Select(o => MapUiJson.Text(o["label"])).ToList().IndexOf(FormJson.Shown(cell["display"])),
                picked => { if (FormText.Parse(FormJson.Shown(options[picked]["raw"])) is { } value) Set(value); }, !disabled)
            : SetupUi.Entry(label, FormJson.Shown(cell["valueString"]), typed => { if (FormText.Parse(typed) is { } value) Set(value); }, !disabled, units: MapUiJson.Text(cell["units"]));
    }

    private UIElement FactRow(JsonObject item)
    {
        var raw = ActuatorFacts.Raw(item);
        var showAs = MapUiJson.Text(item["showAs"]);
        var label = ActuatorFacts.Label(item);
        var path = MapUiJson.Text(item["path"]);
        var bit = (int)(MapUiJson.Number(item["bit"]) ?? 0);
        void Write(object value) => SetupUi.Run(() => SetupUi.Write(path, value), refused =>
        {
            FormLook.Say(_refusal, refused);
            Refresh();
        });
        return showAs == ActuatorFacts.Bitset && raw is { } bits ? Checked(label, ActuatorFacts.BitChecked(bits, bit), on => Write(ActuatorFacts.BitWritten(bits, bit, on)))
            : showAs == ActuatorFacts.TrueIfPositive && raw is { } positive ? Checked(label, positive > 0, on => Write(ActuatorFacts.SignWritten(positive, on)))
            : SetupUi.Control(item) is { } control ? SetupUi.Embedded([label.Length > 0 ? SetupUi.Titled(control, label) : control], Refresh)
            : new Border();
    }

    private static FrameworkElement Checked(string label, bool on, Action<bool> changed, bool enabled = true)
    {
        var box = FormLook.Check(label, on, changed);
        box.IsEnabled = enabled;
        return FormLook.Margined(box, new Thickness(16, 4, 16, 4));
    }

    private static UIElement Card(string title, IEnumerable<UIElement?> rows) =>
        FormLook.Stack(4, FormLook.Margined(FormLook.Text(title, "FontSizeBody", "TextPrimaryBrush", true), new Thickness(4, 8, 0, 0)), FormLook.Card(rows.OfType<UIElement>()));

    private UIElement Group(JsonObject group)
    {
        var columns = (JsonObject subgroup) => MapUiJson.Objects(subgroup["columns"]);
        return FormLook.Stack(8, new UIElement?[] { group["enable"] is JsonObject enable ? FactRow(enable) : null }
            .Concat(MapUiJson.Flag(group["groupsVisible"])
                ? MapUiJson.Objects(group["subgroups"]).SelectMany(subgroup => new UIElement?[]
                {
                    MapUiJson.Text(subgroup["label"]) is { Length: > 0 } label ? SetupUi.Heading(label) : null,
                    subgroup["primary"] is JsonObject primary ? FactRow(primary) : null,
                }
                .Concat(MapUiJson.Objects(subgroup["channels"]).Select(channel => Card(MapUiJson.Text(channel["label"]),
                    (channel["configs"] as JsonArray ?? []).Select((config, at) => (Config: config as JsonObject, Column: columns(subgroup).ElementAtOrDefault(at)))
                        .Where(pair => pair.Column is { } column && MapUiJson.Flag(column["visible"]) && (_advanced || !MapUiJson.Flag(column["advanced"])))
                        .Select(pair => pair.Config is { } config
                            ? FactRow(new JsonObject(config.Select(kv => KeyValuePair.Create(kv.Key, kv.Value?.DeepClone()))) { ["label"] = MapUiJson.Text(pair.Column!["label"]) })
                            : SetupUi.Pair(MapUiJson.Text(pair.Column!["label"]), NotAvailable)))))
                .Concat(MapUiJson.Objects(subgroup["params"]).Select(FactRow)))
                : [])
            .Concat(MapUiJson.Objects(group["params"]).Select(FactRow))
            .Concat(MapUiJson.Texts(group["notes"]).Select(note => (UIElement?)FormLook.Caption(note))));
    }

    private UIElement Testing(JsonObject testing)
    {
        var actuators = Actuators;
        var failed = MapUiJson.Flag(testing["hadFailure"]);
        if (actuators.Count == 0) return FormLook.Stack(8, SetupUi.Heading("Actuator testing"), FormLook.Text("Configure some outputs in order to test them."), Actions());
        var toggle = new ToggleSwitch { IsOn = _testing && !failed, IsEnabled = !failed && !Assigning, OnContent = "", OffContent = "", MinWidth = 0 };
        AutomationProperties.SetName(toggle, "Enable actuator sliders");
        toggle.Toggled += (_, _) => Enable(toggle.IsOn);
        var all = TestChannel.Parse(testing["allMotors"]);
        return FormLook.Stack(8, new UIElement?[]
        {
            SetupUi.Heading("Actuator testing"),
            FormLook.Line(12, toggle, FormLook.Text(_testing ? "Careful: actuator sliders are enabled" : "Propellers are removed - enable sliders", "FontSizeBody", "CautionBrush")),
            all is null ? null : TestSlider(all, _allMotors, value =>
            {
                _allMotors = value;
                var motors = actuators.Where(a => a.IsMotor).Select(a => a.Function).ToList();
                _values = _values.SetItems(motors.Select(f => KeyValuePair.Create(f, value)));
                _moved = _moved.Union(motors);
            }),
        }
        .Concat(actuators.Select(channel => TestSlider(channel, _values.GetValueOrDefault(channel.Function, channel.Rest), value =>
        {
            _values = _values.SetItem(channel.Function, value);
            _moved = _moved.Add(channel.Function);
        })))
        .Append(Actions()));
    }

    private UIElement TestSlider(TestChannel channel, double value, Action<double> changed)
    {
        var slider = new Slider { Minimum = channel.From, Maximum = channel.Max, Value = value, IsEnabled = _testing, StepFrequency = Math.Max(1e-6, (channel.Max - channel.From) / 1000), IsThumbToolTipEnabled = false };
        AutomationProperties.SetName(slider, channel.Label);
        slider.ValueChanged += (_, args) =>
        {
            var snapped = channel.Snapped(args.NewValue);
            if (Math.Abs(snapped - args.NewValue) > 1e-9) slider.Value = snapped;
            changed(snapped);
            if (_testing && !_sender.IsEnabled) _sender.Start();
        };
        return FormLook.Stack(2, FormLook.Caption(channel.Label, "TextPrimaryBrush"), slider);
    }

    private UIElement? Actions()
    {
        var groups = MapUiJson.Objects(_view["actions"]);
        if (groups.Count == 0) return null;
        return FormLook.Line(8, groups.Select(group =>
        {
            var type = (int)(MapUiJson.Number(group["type"]) ?? 0);
            var menu = new MenuFlyout();
            MapUiJson.Objects(group["actions"]).Select(action =>
            {
                var function = (int)(MapUiJson.Number(action["function"]) ?? 0);
                var item = new MenuFlyoutItem { Text = MapUiJson.Text(action["label"]) };
                item.Click += (_, _) => SetupUi.Detached(() => FormWrites.Ok(ActionTrigger, type, function));
                return item;
            }).ToList().ForEach(menu.Items.Add);
            return (UIElement?)new DropDownButton { Content = MapUiJson.Text(group["label"]), Flyout = menu, IsEnabled = !_testing && !Assigning };
        }).ToArray());
    }

    private void Enable(bool on)
    {
        if (on == _testing) return;
        _testing = on;
        if (!on)
        {
            _values = Actuators.ToImmutableDictionary(a => a.Function, a => a.Rest);
            _moved = ImmutableHashSet<int>.Empty;
            _allMotors = TestChannel.Parse(_view["testing"]?["allMotors"])?.Rest ?? 0;
            _sender.Stop();
        }
        SetupUi.InOrder(() => FormWrites.Ok(TestActive, on));
        Render();
    }

    private void Send()
    {
        if (!_testing || _moved.IsEmpty)
        {
            _sender.Stop();
            return;
        }
        var sends = Actuators.Where(a => _moved.Contains(a.Function))
            .Select(a => (Channel: a, Value: a.Sent(_values.GetValueOrDefault(a.Function, a.Rest))))
            .ToList();
        var stopped = sends.Where(s => s.Value is null).Select(s => s.Channel).ToList();
        SetupUi.InOrder(() => sends.ForEach(s =>
        {
            if (s.Value is { } value) FormWrites.Ok(TestSet, s.Channel.Function, value);
            else FormWrites.Ok(TestStop, s.Channel.Function);
        }));
        if (stopped.Count == 0) return;
        _values = _values.SetItems(stopped.Select(c => KeyValuePair.Create(c.Function, c.Rest)));
        _moved = _moved.Except(stopped.Select(c => c.Function));
        Render();
    }
}

internal static class GeometryImage
{
    private const double Size = 240;
    private const double Margin = 16;

    public static UIElement? Of(IReadOnlyList<JsonObject> motors, IReadOnlySet<int> highlighted, Action<int> onMotor)
    {
        var points = motors.Select(m => (Index: (int)(MapUiJson.Number(m["index"]) ?? 0), Label: (int)(MapUiJson.Number(m["label"]) ?? 0), X: MapUiJson.Number(m["x"]) ?? 0, Y: MapUiJson.Number(m["y"]) ?? 0)).ToList();
        if (points.Count <= 1) return null;
        var (minX, maxX, minY, maxY) = (points.Min(p => p.X), points.Max(p => p.X), points.Min(p => p.Y), points.Max(p => p.Y));
        if (maxX - minX < 0.0001 || maxY - minY < 0.0001) return null;
        var usable = Size - 2 * Margin;
        var rotor = usable * (points.Count <= 6 ? 0.31 : 0.25);
        var scale = Math.Min((usable - rotor) / (maxY - minY), (usable - rotor) / (maxX - minX));
        var offsetX = Margin + usable / 2 - (maxY + minY) / 2 * scale;
        var offsetY = Margin + usable / 2 + (maxX + minX) / 2 * scale;
        var canvas = new Canvas { Width = Size, Height = Size, HorizontalAlignment = HorizontalAlignment.Left };
        points.ForEach(point =>
        {
            var lit = highlighted.Contains(point.Index);
            var x = offsetX + point.Y * scale - rotor / 2;
            var y = offsetY - point.X * scale - rotor / 2;
            var label = FormLook.Text(point.Label.ToString(System.Globalization.CultureInfo.InvariantCulture), "FontSizeBody", lit ? "AccentOnBrush" : "TextPrimaryBrush", true);
            label.HorizontalAlignment = HorizontalAlignment.Center;
            label.VerticalAlignment = VerticalAlignment.Center;
            var rotorView = new Grid
            {
                Width = rotor,
                Height = rotor,
                Children =
                {
                    new Microsoft.UI.Xaml.Shapes.Ellipse
                    {
                        Stroke = FormLook.Brush("TextSecondaryBrush"),
                        StrokeThickness = 2,
                        Fill = FormLook.Brush(lit ? "AccentBrush" : "ControlBrush"),
                    },
                    label,
                },
            };
            AutomationProperties.SetName(rotorView, $"Motor {point.Label}");
            rotorView.Tapped += (_, _) => onMotor(point.Index);
            Canvas.SetLeft(rotorView, x);
            Canvas.SetTop(rotorView, y);
            canvas.Children.Add(rotorView);
        });
        return canvas;
    }
}
