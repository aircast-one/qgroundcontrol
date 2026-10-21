using System;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;

namespace AircastQGC.Controls;

internal static class GimbalTilt
{
    public const double Lowest = -90;
    public const double Highest = 30;
    private const double OffCentreDegrees = 2;

    public static bool OffCentre(double? pitch, double? yaw) =>
        Math.Abs(pitch ?? 0) > OffCentreDegrees || Math.Abs(yaw ?? 0) > OffCentreDegrees;

    public static double Shown(double? pitch) => Math.Clamp(pitch ?? 0, Lowest, Highest);
}

public sealed partial class GimbalTiltRail : UserControl
{
    private const string PitchPath = "gimbal.pitch";
    private const string CenterPath = "gimbal.center";
    private const string RecenterGlyph = "\uE81D";
    private const long SettlesMs = 2500;
    private static readonly TimeSpan RefusalShown = TimeSpan.FromSeconds(4);

    private readonly FontIcon _recenter = FormLook.Glyph(RecenterGlyph, 12, "TextPrimaryBrush");
    private readonly TextBlock _tiltText = FormLook.Caption("", "TextPrimaryBrush");
    private readonly TextBlock _refusal = FormLook.Caption("", "CriticalBrush");
    private readonly Button _tilt;
    private readonly Slider _slider;
    private readonly RcThrottle _throttle;
    private readonly DispatcherQueueTimer _refusalTimer;
    private Task _queue = Task.CompletedTask;
    private (double? Pitch, double? Yaw) _at;
    private (double Pitch, long At)? _sent;
    private bool _dragging;
    private bool _painting;

    public GimbalTiltRail()
    {
        _tiltText.Visibility = Visibility.Visible;
        _tilt = new Button
        {
            Height = 28,
            Padding = new Thickness(10, 0, 10, 0),
            CornerRadius = new CornerRadius(14),
            HorizontalAlignment = HorizontalAlignment.Center,
            Content = FormLook.Line(4, _recenter, _tiltText),
        };
        PreflightLook.Chip(_tilt);
        ToolTipService.SetToolTip(_tilt, "Recenter the camera");
        _tilt.Click += (_, _) => Send(CenterPath);
        _slider = new Slider
        {
            Orientation = Orientation.Vertical,
            Minimum = GimbalTilt.Lowest,
            Maximum = GimbalTilt.Highest,
            StepFrequency = 1,
            Height = 140,
            HorizontalAlignment = HorizontalAlignment.Center,
        };
        AutomationProperties.SetName(_slider, "Camera tilt");
        _throttle = new RcThrottle(pitch =>
        {
            _sent = (pitch, Environment.TickCount64);
            Send(PitchPath, (double)pitch);
        });
        _slider.ValueChanged += (_, e) =>
        {
            if (!_painting) _throttle.Offer((int)Math.Round(e.NewValue));
        };
        _slider.AddHandler(PointerPressedEvent, new PointerEventHandler((_, _) => _dragging = true), true);
        _slider.AddHandler(PointerReleasedEvent, new PointerEventHandler((_, _) => _dragging = false), true);
        _slider.AddHandler(PointerCaptureLostEvent, new PointerEventHandler((_, _) => _dragging = false), true);
        _refusal.TextWrapping = TextWrapping.Wrap;
        _refusal.TextAlignment = TextAlignment.Center;
        _refusal.MaxWidth = 140;
        Content = new StackPanel
        {
            Spacing = 8,
            HorizontalAlignment = HorizontalAlignment.Center,
            Children = { _tilt, _slider, _refusal },
        };
        _refusalTimer = DispatcherQueue.GetForCurrentThread().CreateTimer();
        _refusalTimer.Interval = RefusalShown;
        _refusalTimer.IsRepeating = false;
        _refusalTimer.Tick += (_, _) => FormLook.Say(_refusal, null);
        Visibility = Visibility.Collapsed;
        Live.Watch(this, GimbalCalls.View, Render);
    }

    private void Render(JsonObject view)
    {
        var shown = PayloadJson.Gimbal(view) is not null;
        Visibility = FormLook.Shown(shown);
        _at = (MapUiJson.Number(view["pitchDegrees"]), MapUiJson.Number(view["bodyYawDegrees"]));
        if (shown) Paint();
    }

    private void Paint()
    {
        var off = GimbalTilt.OffCentre(_at.Pitch, _at.Yaw);
        var settling = _sent is { } sent && Environment.TickCount64 - sent.At < SettlesMs;
        var pitch = GimbalTilt.Shown(settling ? _sent!.Value.Pitch : _at.Pitch);
        var text = SyntheticAim.Degrees(pitch);
        _tiltText.Text = $"Tilt {text}";
        _recenter.Visibility = FormLook.Shown(off);
        AutomationProperties.SetName(_tilt, off ? $"Tilt {text}, off centre" : $"Tilt {text}");
        if (_dragging) return;
        _painting = true;
        _slider.Value = pitch;
        _painting = false;
    }

    private void Send(string path, params object?[] args)
    {
        var ui = DispatcherQueue;
        _queue = _queue.ContinueWith(_ => GimbalCalls.Call(path, args), TaskScheduler.Default)
            .ContinueWith(done => ui.TryEnqueue(() => ShowRefusal(done.Result)), TaskScheduler.Default);
    }

    private void ShowRefusal(string? refusal)
    {
        if (refusal is null) return;
        FormLook.Say(_refusal, refusal);
        _refusalTimer.Stop();
        _refusalTimer.Start();
    }
}
