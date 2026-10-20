using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.WindowsRuntime;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media.Imaging;
using Windows.Storage.Streams;

namespace AircastQGC.Pages.Setup;

internal sealed record MotorGate(bool Connected, bool Armed, bool ContactLost, int? Reported)
{
    public const int UnknownMotorCount = 8;

    public int Count => Reported ?? UnknownMotorCount;

    public bool CanTest(bool propsOff) => Connected && propsOff && !Armed && !ContactLost;

    public bool CanStop => Connected;

    public string? CountNotice => Reported is null ? "Warning: Unable to determine motor count" : null;

    public string? Refusal =>
        !Connected ? "No vehicle is connected, so nothing will answer a motor test."
        : Armed ? "The vehicle is armed. Disarm it before testing a motor."
        : ContactLost ? "The vehicle has stopped answering. Check the link before testing a motor."
        : null;

    public static MotorGate Parse(JsonObject view) => new(
        MapUiJson.Flag(view["connected"]),
        MapUiJson.Flag(view["armed"]),
        MapUiJson.Flag(view["contactLost"]),
        MapUiJson.Number(view["motorCount"]) is { } count && count >= 1 ? (int)count : null);

    public static string Label(int motor, bool letters) => letters ? ((char)('A' + motor - 1)).ToString() : motor.ToString();
}

internal sealed class MotorsPanel : UserControl
{
    private const string FrameView = "view.frame";
    private const string MotorTest = "vehicle.motorTest";
    private const int TimeoutSeconds = 3;

    private readonly ToggleSwitch _propsOff = new() { OnContent = "", OffContent = "", MinWidth = 0 };
    private readonly TextBlock _propsLabel = FormLook.Text("", "FontSizeBody", "CautionBrush");
    private readonly TextBlock _percent = FormLook.Text("0%", "FontSizeBody", "AccentTextBrush", true);
    private readonly Slider _throttle = new() { Minimum = 0, Maximum = 100, StepFrequency = 1, Value = 0, IsThumbToolTipEnabled = false };
    private readonly TextBlock _countNotice = FormLook.Text("", "FontSizeBody", "CautionBrush");
    private readonly TextBlock _refusal = FormLook.Text("", "FontSizeBody", "CriticalBrush");
    private readonly TextBlock _answer = SetupUi.Error();
    private readonly ContentControl _motors = new() { HorizontalContentAlignment = HorizontalAlignment.Stretch, IsTabStop = false };
    private readonly Button _all;
    private readonly Button _stop;
    private MotorGate _gate = new(false, false, false, null);
    private bool _letters;
    private string _shape = "";

    public MotorsPanel()
    {
        AutomationProperties.SetName(_propsOff, "Propellers are removed");
        AutomationProperties.SetName(_throttle, "Throttle");
        _propsOff.Toggled += (_, _) =>
        {
            if (!_propsOff.IsOn) _throttle.Value = 0;
            Paint();
        };
        _throttle.ValueChanged += (_, _) => _percent.Text = $"{(int)_throttle.Value}%";
        _all = FormLook.Button("All", () => Enumerable.Range(1, _gate.Count).ToList().ForEach(motor => Spin(motor, Throttle)), ButtonLook.Accent);
        _stop = FormLook.Button("Stop", () => Enumerable.Range(1, _gate.Count).ToList().ForEach(motor => Spin(motor, 0)));
        var warning = new Border
        {
            Background = FormLook.Brush("CautionBgBrush"),
            CornerRadius = new CornerRadius(20),
            Height = 40,
            Padding = new Thickness(16, 0, 16, 0),
            Child = FormLook.Line(12, FormLook.Glyph("", 16, "CautionBrush"), FormLook.Text("Make sure you remove all props.", "FontSizeBody", "CautionBrush", true)),
        };
        ((StackPanel)warning.Child).VerticalAlignment = VerticalAlignment.Center;
        var throttleRow = new Grid { ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto } } };
        throttleRow.Children.Add(FormLook.Text("Throttle"));
        Grid.SetColumn(_percent, 1);
        throttleRow.Children.Add(_percent);
        Content = SetupUi.Page(
            FormLook.Margined(warning, new Thickness(0, 4, 0, 0)),
            _countNotice,
            _refusal,
            FormLook.Line(12, _propsOff, _propsLabel),
            throttleRow,
            _throttle,
            _motors,
            FormLook.Line(8, _all, _stop),
            _answer);
        Live.Watch(this, FrameView, view =>
        {
            _gate = MotorGate.Parse(view);
            Paint();
        });
        Live.Watch(this, SetupModel.SetupPath, view =>
        {
            var letters = MapUiJson.Text(view["firmware"]) == "apm";
            if (letters == _letters) return;
            _letters = letters;
            _shape = "";
            Paint();
        });
        Unloaded += (_, _) => _propsOff.IsOn = false;
    }

    private int Throttle => (int)_throttle.Value;

    private void Paint()
    {
        var canTest = _gate.CanTest(_propsOff.IsOn);
        FormLook.Say(_countNotice, _gate.CountNotice);
        FormLook.Say(_refusal, _gate.Refusal);
        FormLook.Say(_propsLabel, _propsOff.IsOn ? "Careful: motors are enabled" : "Propellers are removed - enable slider and motors");
        _propsLabel.VerticalAlignment = VerticalAlignment.Center;
        _throttle.IsEnabled = canTest;
        _all.IsEnabled = canTest;
        _stop.IsEnabled = _gate.CanStop;
        var shape = $"{_gate.Count}|{_letters}|{canTest}";
        if (shape == _shape) return;
        _shape = shape;
        var grid = new Grid
        {
            ColumnSpacing = 12,
            RowSpacing = 12,
            ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition() },
        };
        Enumerable.Range(0, (_gate.Count + 1) / 2).ToList().ForEach(_ => grid.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto }));
        _motors.Content = FormLook.Fill(grid, Enumerable.Range(1, _gate.Count).Select(motor => Tile(motor, canTest, (motor - 1) / 2, (motor - 1) % 2)));
    }

    private UIElement Tile(int motor, bool canTest, int row, int column)
    {
        var title = $"Motor {MotorGate.Label(motor, _letters)}";
        var label = FormLook.Stack(4, FormLook.Text(title), FormLook.Caption(canTest ? "Click to spin" : "Locked"));
        label.HorizontalAlignment = HorizontalAlignment.Center;
        label.Padding = new Thickness(0, 16, 0, 16);
        var button = FormLook.Stretched(label, title, () => Spin(motor, Throttle));
        FormLook.Paint(button, ButtonLook.Standard);
        button.IsEnabled = canTest;
        button.CornerRadius = FormLook.Radius("RadiusOverlay");
        Grid.SetRow(button, row);
        Grid.SetColumn(button, column);
        return button;
    }

    private void Spin(int motor, int percent)
    {
        var seconds = percent == 0 ? 0 : TimeoutSeconds;
        SetupUi.InOrder(() =>
        {
            var refused = SetupUi.Refusal(MotorTest, motor, percent, seconds, true);
            DispatcherQueue.TryEnqueue(() => FormLook.Say(_answer, refused));
        });
    }
}

internal sealed class SubMotorsPanel : UserControl
{
    private const string View = "view.apmSubMotors";
    private const string FrameView = "view.apmSubFrame";
    private const double Neutral = 50;
    private static readonly IReadOnlySet<int> DisplayFrames = new HashSet<int> { 0, 1, 2, 4, 5 };
    private static readonly TimeSpan Poll = TimeSpan.FromMilliseconds(500);
    private static readonly TimeSpan TestEvery = TimeSpan.FromMilliseconds(50);
    private static readonly TimeSpan CoolDown = TimeSpan.FromSeconds(11);

    private readonly Image _frame = new() { MaxHeight = 240, Stretch = Microsoft.UI.Xaml.Media.Stretch.Uniform };
    private readonly ContentControl _motors = new() { HorizontalContentAlignment = HorizontalAlignment.Stretch, IsTabStop = false };
    private readonly ToggleSwitch _arm = new() { OnContent = "", OffContent = "", MinWidth = 0 };
    private readonly TextBlock _armLabel = FormLook.Text("", "FontSizeBody", "CriticalBrush");
    private readonly TextBlock _warning = FormLook.Caption("");
    private readonly ContentControl _detect = new() { HorizontalContentAlignment = HorizontalAlignment.Stretch, IsTabStop = false };
    private readonly TextBlock _refusal = SetupUi.Error();
    private readonly DispatcherTimer _tester = new() { Interval = TestEvery };
    private readonly DispatcherTimer _cooling = new() { Interval = CoolDown };
    private IReadOnlyList<Slider> _sliders = [];
    private JsonObject _view = new();
    private bool? _armed;
    private bool _shouldTest;
    private int _lastIndex;
    private string _shape = "";
    private bool _painting;

    public SubMotorsPanel()
    {
        AutomationProperties.SetName(_arm, "Arm the vehicle for the motor test");
        _arm.Toggled += (_, _) =>
        {
            if (_painting) return;
            var on = _arm.IsOn;
            SetupUi.Run(() => SetupUi.Refusal("apmSubMotors.arm", on), refused => FormLook.Say(_refusal, refused));
        };
        _tester.Tick += (_, _) =>
        {
            var index = _lastIndex;
            var value = index < _sliders.Count ? _sliders[index].Value : Neutral;
            SetupUi.InOrder(() => FormWrites.Ok("apmSubMotors.test", index, value));
        };
        _cooling.Tick += (_, _) =>
        {
            _cooling.Stop();
            Paint();
        };
        Content = SetupUi.Page(_frame, _motors, FormLook.Caption("Reverse motor direction"), _warning, FormLook.Line(12, _arm, _armLabel), _detect, _refusal);
        Loaded += (_, _) =>
        {
            SetupUi.Detached(() => FormWrites.Ok("apmSubMotors.open"));
            LoadFrame();
        };
        Unloaded += (_, _) =>
        {
            _tester.Stop();
            _cooling.Stop();
        };
        _ = Live.Changes(this, Poll, () => Bridge.Get(View), Apply);
    }

    private bool Testing => MapUiJson.Flag(_view["canRunManualTest"]) && _shouldTest;

    private async void LoadFrame()
    {
        var png = await Task.Run(() =>
        {
            var frames = Bridge.Get(FrameView);
            return MapUiJson.Flag(frames["available"]) && MapUiJson.Number(frames["selected"]) is { } selected && DisplayFrames.Contains((int)selected)
                ? MapUiJson.Text(Bridge.Get($"view.apmSubFrameImage({(int)selected})")["png"])
                : "";
        });
        _frame.Source = await Images.Png(png);
        _frame.Visibility = FormLook.Shown(_frame.Source is not null);
    }

    private void Apply(JsonObject view)
    {
        _view = view;
        if (!MapUiJson.Flag(view["available"]))
        {
            Content = SetupUi.Notice("This page is for an ArduSub vehicle.");
            return;
        }
        var armed = MapUiJson.Flag(view["armed"]);
        if (armed != _armed)
        {
            var wasKnown = _armed is not null;
            _armed = armed;
            _shouldTest = armed;
            _sliders.ToList().ForEach(s => s.Value = Neutral);
            if (!armed && wasKnown) _cooling.Start();
        }
        Paint();
    }

    private void Paint()
    {
        var motors = MapUiJson.Objects(_view["motors"]);
        var shape = string.Join("|", motors.Select(m => $"{m["motor"]}:{m["reversed"]}")) + $"|{Testing}";
        if (shape != _shape)
        {
            _shape = shape;
            _sliders = motors.Select(_ => new Slider { Minimum = 0, Maximum = 100, Value = Neutral, IsThumbToolTipEnabled = false }).ToList();
            _motors.Content = FormLook.Stack(4, motors.Select((motor, at) => (UIElement?)MotorRow(motor, at)));
        }
        _sliders.ToList().ForEach(s => s.IsEnabled = Testing);
        if (Testing) _tester.Start();
        else _tester.Stop();
        _painting = true;
        _arm.IsOn = MapUiJson.Flag(_view["armed"]);
        _painting = false;
        _arm.IsEnabled = !_cooling.IsEnabled;
        FormLook.Say(_armLabel, _cooling.IsEnabled ? "A 10 second coooldown is required before testing again, please stand by..." : "Slide this switch to arm the vehicle and enable the motor test (CAUTION!)");
        FormLook.Say(_warning, MapUiJson.Text(_view["warning"]));
        _detect.Content = MapUiJson.Flag(_view["offersAutoDetect"]) ? FormLook.Stack(8,
            SetupUi.Heading("Automatic motor direction detection"),
            FormLook.Caption(MapUiJson.Text(_view["autoDetectHelp"])),
            Detect(),
            FormLook.Caption(MapUiJson.Text(_view["detectionMessages"]))) : null;
    }

    private Button Detect()
    {
        var button = FormLook.Button("Auto-Detect Directions", () => SetupUi.Run(() => SetupUi.Refusal("apmSubMotors.autoDetect"), refused => FormLook.Say(_refusal, refused)), ButtonLook.Accent);
        button.IsEnabled = !MapUiJson.Flag(_view["detecting"]);
        return button;
    }

    private UIElement MotorRow(JsonObject motor, int at)
    {
        var number = (int)(MapUiJson.Number(motor["motor"]) ?? at + 1);
        var slider = _sliders[at];
        AutomationProperties.SetName(slider, $"Motor {number}");
        slider.ValueChanged += (_, _) =>
        {
            if (Math.Abs(slider.Value - Neutral) > 0.01) _lastIndex = at;
        };
        slider.AddHandler(PointerReleasedEvent, new PointerEventHandler((_, _) => slider.Value = Neutral), true);
        slider.AddHandler(PointerCaptureLostEvent, new PointerEventHandler((_, _) => slider.Value = Neutral), true);
        var reversed = FormLook.Check("", MapUiJson.Flag(motor["reversed"]), on =>
        {
            slider.Value = Neutral;
            SetupUi.Run(() => SetupUi.Refusal("apmSubMotors.reverse", number, on), refused => FormLook.Say(_refusal, refused));
        });
        reversed.IsEnabled = Testing;
        AutomationProperties.SetName(reversed, $"Reverse motor {number}");
        var grid = new Grid
        {
            ColumnSpacing = 12,
            ColumnDefinitions = { new ColumnDefinition { Width = new GridLength(24) }, new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto } },
        };
        var label = FormLook.Text(number.ToString());
        label.VerticalAlignment = VerticalAlignment.Center;
        grid.Children.Add(label);
        Grid.SetColumn(slider, 1);
        grid.Children.Add(slider);
        Grid.SetColumn(reversed, 2);
        grid.Children.Add(reversed);
        return grid;
    }
}

internal static class Images
{
    public static async Task<BitmapImage?> Png(string base64)
    {
        if (base64.Trim().Length == 0) return null;
        try
        {
            var bytes = Convert.FromBase64String(base64);
            using var stream = new InMemoryRandomAccessStream();
            await stream.WriteAsync(bytes.AsBuffer());
            stream.Seek(0);
            var image = new BitmapImage();
            await image.SetSourceAsync(stream);
            return image;
        }
        catch (FormatException) { return null; }
        catch (Exception error) when (error is COMException or ArgumentException) { return null; }
    }
}
