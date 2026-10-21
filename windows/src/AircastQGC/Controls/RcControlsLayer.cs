using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Input;
using Windows.System;

namespace AircastQGC.Controls;

internal enum RcControlType { Slider, Button, Switch3, Momentary }

internal sealed record RcControl(string Label, int Channel, RcControlType Type);

internal sealed record RcControlState(RcControl Control, int? Pwm, int? Position, bool On);

internal sealed record RcCameraChannels(int Tilt, int Pan, int Zoom, int Light, int Record)
{
    public bool Any => new[] { Tilt, Pan, Zoom, Light, Record }.Any(channel => channel > 0);

    public static RcCameraChannels Of(IReadOnlyDictionary<string, int> settings)
    {
        int Read(string name) => settings.GetValueOrDefault(name);
        var tilt = Read(RcControls.TiltChannel);
        var pan = Read(RcControls.PanChannel);
        return new RcCameraChannels(tilt, pan == tilt ? 0 : pan, Read(RcControls.ZoomChannel), Read(RcControls.LightChannel), Read(RcControls.RecordChannel));
    }
}

internal static class RcControls
{
    public const string Setting = "settings.flyViewSettings.rcControls";
    public const string StatesView = "view.rcControls";
    public const string TiltChannel = "gimbalTiltChannel";
    public const string PanChannel = "gimbalPanChannel";
    public const string ZoomChannel = "cameraZoomChannel";
    public const string LightChannel = "cameraLightChannel";
    public const string RecordChannel = "cameraRecordChannel";
    public const int PwmMin = 1000;
    public const int PwmCenter = 1500;
    public const int PwmMax = 2000;
    public const int ChannelMin = 1;
    public const int ChannelMax = 18;

    public static readonly IReadOnlyList<(string Name, string Owner)> CameraChannels =
    [
        (TiltChannel, "Gimbal tilt"),
        (PanChannel, "Gimbal pan"),
        (ZoomChannel, "Camera zoom"),
        (LightChannel, "Camera light"),
        (RecordChannel, "Camera record"),
    ];

    public static readonly IReadOnlyList<int> SwitchPwms = [PwmMin, PwmCenter, PwmMax];

    public static readonly IReadOnlyList<string> SwitchLabels = ["Low", "Mid", "High"];

    public static readonly IReadOnlyList<RcControlType> Types = Enum.GetValues<RcControlType>();

    public static string ControlView(string name) => $"view.control(settings.flyViewSettings.{name})";

    public static RcControlType? TypeOf(string raw) => raw switch
    {
        "slider" => RcControlType.Slider,
        "button" => RcControlType.Button,
        "switch3" => RcControlType.Switch3,
        "momentary" => RcControlType.Momentary,
        _ => null,
    };

    public static string TypeName(RcControlType type) => type.ToString().ToLowerInvariant();

    public static string TypeLabel(RcControlType type) => type switch
    {
        RcControlType.Switch3 => "Three-way switch",
        _ => type.ToString(),
    };

    public static bool Usable(int channel) => channel is >= ChannelMin and <= ChannelMax;

    public static int Channel(JsonObject view) => (int)(MapUiJson.Number(view["value"]) ?? 0);

    public static IReadOnlyList<RcControlState> States(JsonObject view) =>
        MapUiJson.Objects(view["controls"])
            .Select(entry => (Entry: entry, Channel: (int)(MapUiJson.Number(entry["channel"]) ?? 0), Type: TypeOf(MapUiJson.Text(entry["type"]))))
            .Where(read => read.Channel > 0 && read.Type is not null)
            .Select(read => new RcControlState(
                new RcControl(MapUiJson.Text(read.Entry["label"]) is { Length: > 0 } label ? label : $"CH{read.Channel}", read.Channel, read.Type!.Value),
                (int?)MapUiJson.Number(read.Entry["pwm"]),
                (int?)MapUiJson.Number(read.Entry["position"]),
                MapUiJson.Flag(read.Entry["on"])))
            .ToList();
}

internal sealed class RcSender
{
    private const string Override = "vehicle.setRcChannelOverride";
    private const string Release = "vehicle.releaseRcChannelOverride";
    private const string Clear = "vehicle.clearRcChannelOverrides";
    private const string Subject = "The vehicle";

    private static Task _queue = Task.CompletedTask;
    private ImmutableHashSet<int> _held = ImmutableHashSet<int>.Empty;

    public event Action<string?>? Answered;

    public void Hold(int channel, int pwm)
    {
        if (channel <= 0) return;
        _held = _held.Add(channel);
        Send(() => FormWrites.Call(Override, Subject, channel, pwm));
    }

    public void ReleaseAll()
    {
        var held = _held.Order().ToList();
        _held = ImmutableHashSet<int>.Empty;
        if (held.Count > 0) Send(() => held.Select(channel => FormWrites.Call(Release, Subject, channel)).LastOrDefault());
    }

    public void Forget() => _held = ImmutableHashSet<int>.Empty;

    public static void GiveBack(params RcSender[] senders)
    {
        senders.ToList().ForEach(sender => sender.Forget());
        Enqueue(() => FormWrites.Call(Clear, Subject));
    }

    private void Send(Func<string?> work)
    {
        var ui = Microsoft.UI.Dispatching.DispatcherQueue.GetForCurrentThread();
        Enqueue(work).ContinueWith(done => ui?.TryEnqueue(() => Answered?.Invoke(done.Result)), TaskScheduler.Default);
    }

    private static Task<string?> Enqueue(Func<string?> work)
    {
        var next = _queue.ContinueWith(_ => work(), TaskScheduler.Default);
        _queue = next;
        return next;
    }
}

internal sealed class RcThrottle
{
    private static readonly TimeSpan Interval = TimeSpan.FromMilliseconds(100);

    private readonly Action<int> _send;
    private readonly DispatcherTimer _trail = new() { Interval = Interval };
    private DateTime _sentAt = DateTime.MinValue;
    private int? _pending;

    public RcThrottle(Action<int> send)
    {
        _send = send;
        _trail.Tick += (_, _) => Flush();
    }

    public void Offer(int pwm)
    {
        if (DateTime.UtcNow - _sentAt >= Interval) Send(pwm);
        else
        {
            _pending = pwm;
            _trail.Start();
        }
    }

    private void Flush()
    {
        _trail.Stop();
        if (_pending is { } pwm) Send(pwm);
    }

    private void Send(int pwm)
    {
        _pending = null;
        _sentAt = DateTime.UtcNow;
        _send(pwm);
    }
}

public sealed class RcControlsLayer : UserControl
{
    private const string FlyState = "view.flyState";
    private const string RecordingView = "video.recording";
    private const string StartRecording = "video.startRecording";
    private const string StopRecording = "video.stopRecording";
    private const double SliderWidth = 180;
    private const double LabelWidth = 84;

    private readonly RcSender _custom = new();
    private readonly RcSender _camera = new();
    private readonly StackPanel _controls = new() { Spacing = 6 };
    private readonly StackPanel _cameraControls = new() { Spacing = 6 };
    private readonly StackPanel _holding;
    private readonly TextBlock _refusal = FormLook.Caption("", "CriticalBrush");
    private readonly Border _card;
    private IReadOnlyList<RcControlState> _states = [];
    private ImmutableDictionary<string, int> _channels = ImmutableDictionary<string, int>.Empty;
    private ImmutableList<Action> _updaters = ImmutableList<Action>.Empty;
    private string _shape = "";
    private string _cameraShape = "";
    private bool _connected;
    private bool _overriding;
    private bool _streamRecording;
    private bool _channelRecording;
    private bool _lightOn;
    private bool _gimbalManager;
    private int _tilt = RcControls.PwmCenter;
    private int _pan = RcControls.PwmCenter;
    private int _zoom = RcControls.PwmCenter;
    private Action? _updateCamera;

    public RcControlsLayer()
    {
        _holding = FormLook.Line(8,
            FormLook.Margined(FormLook.Caption("These channels are held by this computer."), new Thickness(0, 6, 0, 0)),
            FormLook.Button("Give back", () => RcSender.GiveBack(_custom, _camera)));
        _card = new Border
        {
            Background = FormLook.Brush("CardBrush"),
            BorderBrush = FormLook.Brush("CardStrokeBrush"),
            BorderThickness = new Thickness(1),
            CornerRadius = FormLook.Radius("RadiusOverlay"),
            Padding = new Thickness(12, 8, 12, 8),
            Child = FormLook.Stack(8, _controls, _cameraControls, _holding, _refusal),
        };
        AutomationProperties.SetName(_card, "On-screen RC controls");
        Content = _card;
        Visibility = Visibility.Collapsed;
        _custom.Answered += refusal => FormLook.Say(_refusal, refusal);
        _camera.Answered += refusal => FormLook.Say(_refusal, refusal);
        Live.Watch(this, RcControls.StatesView, view =>
        {
            _states = RcControls.States(view);
            Render();
        });
        Live.Watch(this, FlyState, view =>
        {
            _connected = MapUiJson.Flag(view["connected"]);
            _overriding = MapUiJson.Flag(view["rcOverride"]);
            Render();
        });
        Live.Watch(this, RecordingView, view =>
        {
            _streamRecording = MapUiJson.Flag(view["value"]);
            _updateCamera?.Invoke();
        });
        RcControls.CameraChannels.ToList().ForEach(channel => Live.Watch(this, RcControls.ControlView(channel.Name), view =>
        {
            _channels = _channels.SetItem(channel.Name, RcControls.Channel(view));
            Render();
        }));
        Live.Watch(this, GimbalCalls.View, view =>
        {
            _gimbalManager = PayloadJson.Gimbal(view) is not null;
            Render();
        });
        Unloaded += (_, _) => Release();
        Presence.OnHidden(this, Release);
    }

    private RcCameraChannels Camera => _gimbalManager ? RcCameraChannels.Of(_channels) with { Tilt = 0, Pan = 0 } : RcCameraChannels.Of(_channels);

    private void Release()
    {
        _custom.ReleaseAll();
        _camera.ReleaseAll();
    }

    private void Render()
    {
        var camera = Camera;
        Visibility = FormLook.Shown(_connected && (_states.Count > 0 || camera.Any));
        _holding.Visibility = FormLook.Shown(_overriding);
        var shape = string.Join("|", _states.Select(state => state.Control));
        if (shape != _shape)
        {
            _shape = shape;
            var built = _states.Select(Build).ToList();
            FormLook.Fill(_controls, built.Select(item => item.View));
            _updaters = built.Select(item => item.Update).ToImmutableList();
        }
        _updaters.ForEach(update => update());
        var cameraShape = camera.ToString();
        if (cameraShape != _cameraShape)
        {
            _cameraShape = cameraShape;
            BuildCamera(camera);
        }
        _controls.Visibility = FormLook.Shown(_states.Count > 0);
        _cameraControls.Visibility = FormLook.Shown(camera.Any);
    }

    private RcControlState? Current(RcControl control) => _states.FirstOrDefault(state => state.Control == control);

    private (UIElement View, Action Update) Build(RcControlState state) => state.Control.Type switch
    {
        RcControlType.Slider => SliderControl(state.Control),
        RcControlType.Button => ToggleControl(state.Control),
        RcControlType.Switch3 => SwitchControl(state.Control),
        _ => MomentaryControl(state.Control),
    };

    private (UIElement, Action) SliderControl(RcControl control)
    {
        var (row, applying) = PwmSlider(control.Label, pwm => _custom.Hold(control.Channel, pwm));
        return (row, () => applying.Set(Current(control)?.Pwm ?? RcControls.PwmCenter));
    }

    private (UIElement, Action) ToggleControl(RcControl control)
    {
        var chip = new Chip(control.Label, on => _custom.Hold(control.Channel, on ? RcControls.PwmMax : RcControls.PwmMin));
        return (chip.View, () => chip.Set(Current(control)?.On == true));
    }

    private (UIElement, Action) SwitchControl(RcControl control)
    {
        var chips = new List<Chip>();
        void Update() => chips.Select((chip, at) => (chip, at)).ToList().ForEach(pair => pair.chip.Set(Current(control)?.Position == pair.at));
        chips.AddRange(RcControls.SwitchPwms.Select((pwm, at) => new Chip(RcControls.SwitchLabels[at], on =>
        {
            if (on) _custom.Hold(control.Channel, pwm);
            else Update();
        }, $"{control.Label} {RcControls.SwitchLabels[at]}")));
        return (Labelled(control.Label, FormLook.Line(4, chips.Select(chip => chip.View).ToArray())), Update);
    }

    private (UIElement, Action) MomentaryControl(RcControl control)
    {
        var button = FormLook.Button(control.Label, () => { });
        var held = false;
        void Press()
        {
            if (held) return;
            held = true;
            _custom.Hold(control.Channel, RcControls.PwmMax);
        }
        void Let()
        {
            if (!held) return;
            held = false;
            _custom.Hold(control.Channel, RcControls.PwmMin);
        }
        button.AddHandler(PointerPressedEvent, new PointerEventHandler((_, e) =>
        {
            if (!e.GetCurrentPoint(button).Properties.IsLeftButtonPressed) return;
            button.CapturePointer(e.Pointer);
            Press();
        }), true);
        button.AddHandler(PointerReleasedEvent, new PointerEventHandler((_, _) => Let()), true);
        button.AddHandler(PointerCanceledEvent, new PointerEventHandler((_, _) => Let()), true);
        button.AddHandler(PointerCaptureLostEvent, new PointerEventHandler((_, _) => Let()), true);
        button.AddHandler(KeyDownEvent, new KeyEventHandler((_, e) =>
        {
            if (e.Key is VirtualKey.Space or VirtualKey.Enter) Press();
        }), true);
        button.AddHandler(KeyUpEvent, new KeyEventHandler((_, e) =>
        {
            if (e.Key is VirtualKey.Space or VirtualKey.Enter) Let();
        }), true);
        button.LostFocus += (_, _) => Let();
        return (button, () => { });
    }

    private void BuildCamera(RcCameraChannels camera)
    {
        var sliders = new[]
        {
            camera.Tilt > 0 ? PwmSlider("Tilt", pwm => _camera.Hold(camera.Tilt, _tilt = pwm)) : default,
            camera.Pan > 0 ? PwmSlider("Pan", pwm => _camera.Hold(camera.Pan, _pan = pwm)) : default,
            camera.Zoom > 0 ? PwmSlider("Zoom", pwm => _camera.Hold(camera.Zoom, _zoom = pwm)) : default,
        };
        var record = camera.Record > 0 ? new Chip("Record", on => ToggleRecording(camera.Record, on)) : null;
        var light = camera.Light > 0 ? new Chip("Light", on =>
        {
            _lightOn = on;
            _camera.Hold(camera.Light, on ? RcControls.PwmMax : RcControls.PwmMin);
        }) : null;
        var recenter = camera.Tilt > 0 || camera.Pan > 0 ? FormLook.Button("Recenter", () =>
        {
            _tilt = RcControls.PwmCenter;
            _pan = RcControls.PwmCenter;
            _camera.Hold(camera.Tilt, RcControls.PwmCenter);
            _camera.Hold(camera.Pan, RcControls.PwmCenter);
            _updateCamera?.Invoke();
        }) : null;
        if (recenter is not null) AutomationProperties.SetName(recenter, "Recenter gimbal");
        FormLook.Fill(_cameraControls, sliders.Select(slider => slider.Row)
            .Append(record is null && light is null && recenter is null ? null : FormLook.Line(4, record?.View, light?.View, recenter)));
        _updateCamera = () =>
        {
            var levels = new[] { _tilt, _pan, _zoom };
            sliders.Zip(levels).Where(pair => pair.First.Applying is not null).ToList()
                .ForEach(pair => pair.First.Applying.Set(pair.Second));
            record?.Set(_streamRecording || _channelRecording);
            light?.Set(_lightOn);
        };
        _updateCamera();
    }

    private async void ToggleRecording(int channel, bool next)
    {
        _channelRecording = next;
        _camera.Hold(channel, next ? RcControls.PwmMax : RcControls.PwmMin);
        _updateCamera?.Invoke();
        await FormWrites.Off(() => FormWrites.Ok(next ? StartRecording : StopRecording));
    }

    private static (FrameworkElement Row, SliderState Applying) PwmSlider(string label, Action<int> send)
    {
        var slider = new Slider
        {
            Minimum = RcControls.PwmMin,
            Maximum = RcControls.PwmMax,
            StepFrequency = 1,
            Value = RcControls.PwmCenter,
            Width = SliderWidth,
            VerticalAlignment = VerticalAlignment.Center,
        };
        AutomationProperties.SetName(slider, label);
        var state = new SliderState(slider);
        var throttle = new RcThrottle(send);
        slider.AddHandler(PointerPressedEvent, new PointerEventHandler((_, _) => state.Dragging = true), true);
        slider.AddHandler(PointerReleasedEvent, new PointerEventHandler((_, _) => state.Dragging = false), true);
        slider.AddHandler(PointerCaptureLostEvent, new PointerEventHandler((_, _) => state.Dragging = false), true);
        slider.ValueChanged += (_, e) =>
        {
            if (state.Applying) return;
            throttle.Offer((int)Math.Round(e.NewValue));
        };
        return (Labelled(label, slider), state);
    }

    private static FrameworkElement Labelled(string label, FrameworkElement control)
    {
        var text = FormLook.Caption(label, "TextPrimaryBrush");
        text.Width = LabelWidth;
        text.VerticalAlignment = VerticalAlignment.Center;
        return FormLook.Line(8, text, control);
    }

    internal sealed class Chip
    {
        private bool _applying;

        public Chip(string text, Action<bool> flipped, string? name = null)
        {
            View = new ToggleButton
            {
                Content = text,
                FontFamily = PreflightLook.Font,
                FontSize = FormLook.Size("FontSizeBody"),
                MinHeight = FormLook.Size("SizeControl"),
                Padding = new Thickness(12, 4, 12, 4),
                CornerRadius = FormLook.Radius("RadiusControl"),
                Resources = Look(),
            };
            AutomationProperties.SetName(View, name ?? text);
            View.Checked += (_, _) => Flip(flipped, true);
            View.Unchecked += (_, _) => Flip(flipped, false);
        }

        public ToggleButton View { get; }

        public void Set(bool on)
        {
            if (View.IsChecked == on) return;
            _applying = true;
            View.IsChecked = on;
            _applying = false;
        }

        private void Flip(Action<bool> flipped, bool on)
        {
            if (!_applying) flipped(on);
        }

        private static ResourceDictionary Look() => new()
        {
            ["ToggleButtonBackground"] = FormLook.Brush("ControlBrush"),
            ["ToggleButtonBackgroundPointerOver"] = FormLook.Brush("ControlHoverBrush"),
            ["ToggleButtonBackgroundPressed"] = FormLook.Brush("SubtleSelectedBrush"),
            ["ToggleButtonForeground"] = FormLook.Brush("TextPrimaryBrush"),
            ["ToggleButtonForegroundPointerOver"] = FormLook.Brush("TextPrimaryBrush"),
            ["ToggleButtonForegroundPressed"] = FormLook.Brush("TextPrimaryBrush"),
            ["ToggleButtonBorderBrush"] = FormLook.Brush("ControlStrokeBrush"),
            ["ToggleButtonBorderBrushPointerOver"] = FormLook.Brush("ControlStrokeBrush"),
            ["ToggleButtonBorderBrushPressed"] = FormLook.Brush("ControlStrokeBrush"),
            ["ToggleButtonBackgroundChecked"] = FormLook.Brush("AccentBrush"),
            ["ToggleButtonBackgroundCheckedPointerOver"] = FormLook.Brush("AccentHoverBrush"),
            ["ToggleButtonBackgroundCheckedPressed"] = FormLook.Brush("AccentHoverBrush"),
            ["ToggleButtonForegroundChecked"] = FormLook.Brush("AccentOnBrush"),
            ["ToggleButtonForegroundCheckedPointerOver"] = FormLook.Brush("AccentOnBrush"),
            ["ToggleButtonForegroundCheckedPressed"] = FormLook.Brush("AccentOnBrush"),
            ["ToggleButtonBorderBrushChecked"] = FormLook.Brush("AccentBrush"),
            ["ToggleButtonBorderBrushCheckedPointerOver"] = FormLook.Brush("AccentHoverBrush"),
            ["ToggleButtonBorderBrushCheckedPressed"] = FormLook.Brush("AccentHoverBrush"),
        };
    }

    private sealed class SliderState(Slider slider)
    {
        public bool Dragging { get; set; }

        public bool Applying { get; private set; }

        public void Set(int pwm)
        {
            if (Dragging || (int)Math.Round(slider.Value) == pwm) return;
            Applying = true;
            slider.Value = pwm;
            Applying = false;
        }
    }
}
