using System;
using System.Diagnostics;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

public sealed partial class StreamShutter : UserControl
{
    private const string VideoView = "view.video";
    private const string CameraView = "view.camera";
    private const string ShownSetting = "view.control(settings.flyViewSettings.showPhotoVideoControl)";
    private const string StartRecording = "video.startRecording";
    private const string StopRecording = "video.stopRecording";
    private const string Unanswered = "The recording was not started.";
    private const double Ring = 56;
    private const double Dot = 26;
    private const double Stop = 20;
    private static readonly TimeSpan Tick = TimeSpan.FromMilliseconds(500);
    private static readonly TimeSpan RefusalShown = TimeSpan.FromSeconds(4);

    private readonly string _client = $"stream-shutter.{Guid.NewGuid():N}";
    private readonly Button _button;
    private readonly Border _mark;
    private readonly Border _readout;
    private readonly Border _blink;
    private readonly TextBlock _clock;
    private readonly TextBlock _refusal;
    private readonly DispatcherQueueTimer _tick;
    private readonly DispatcherQueueTimer _refusalTimer;
    private readonly Stopwatch _elapsed = new();
    private JsonObject _video = new();
    private JsonObject _camera = new();
    private bool _settingOn = true;
    private bool _recording;

    public StreamShutter()
    {
        _mark = new Border { Background = FormLook.Brush("RecordBrush"), HorizontalAlignment = HorizontalAlignment.Center, VerticalAlignment = VerticalAlignment.Center };
        _button = new Button
        {
            Width = Ring,
            Height = Ring,
            Padding = new Thickness(0),
            CornerRadius = new CornerRadius(Ring / 2),
            BorderThickness = new Thickness(4),
            HorizontalAlignment = HorizontalAlignment.Center,
            HorizontalContentAlignment = HorizontalAlignment.Center,
            VerticalContentAlignment = VerticalAlignment.Center,
            Content = _mark,
            Resources = new ResourceDictionary
            {
                ["ButtonBackground"] = FormLook.Brush("ChipBrush"),
                ["ButtonBackgroundPointerOver"] = FormLook.Brush("ChipHoverBrush"),
                ["ButtonBackgroundPressed"] = FormLook.Brush("SubtleSelectedBrush"),
                ["ButtonBorderBrush"] = FormLook.Brush("TextPrimaryBrush"),
                ["ButtonBorderBrushPointerOver"] = FormLook.Brush("TextPrimaryBrush"),
                ["ButtonBorderBrushPressed"] = FormLook.Brush("TextSecondaryBrush"),
            },
        };
        _button.Click += (_, _) => Press();
        _blink = FormLook.Dot("RecordBrush");
        _clock = FormLook.Caption("", "TextPrimaryBrush");
        _clock.Visibility = Visibility.Visible;
        _readout = new Border
        {
            Height = 20,
            Padding = new Thickness(8, 0, 8, 0),
            CornerRadius = new CornerRadius(10),
            Background = FormLook.Brush("ChipBrush"),
            HorizontalAlignment = HorizontalAlignment.Center,
            Visibility = Visibility.Collapsed,
            Child = FormLook.Line(4, _blink, _clock),
        };
        _refusal = FormLook.Caption("", "CriticalBrush");
        _refusal.MaxWidth = 160;
        _refusal.TextAlignment = TextAlignment.Center;
        Content = FormLook.Stack(4, _button, _readout, _refusal);
        Visibility = Visibility.Collapsed;

        _tick = DispatcherQueue.GetForCurrentThread().CreateTimer();
        _tick.Interval = Tick;
        _tick.Tick += (_, _) => Paint();
        _refusalTimer = DispatcherQueue.GetForCurrentThread().CreateTimer();
        _refusalTimer.Interval = RefusalShown;
        _refusalTimer.IsRepeating = false;
        _refusalTimer.Tick += (_, _) => FormLook.Say(_refusal, null);
        Loaded += (_, _) => Start();
        Unloaded += (_, _) => Halt();
    }

    private void Start()
    {
        _settingOn = MapUiJson.Flag(Bridge.Get(ShownSetting)["value"], true);
        BridgeWatch.Watch(_client + ".video", [VideoView], OnVideo);
        BridgeWatch.Watch(_client + ".camera", [CameraView], OnCamera);
        _camera = Bridge.Get(CameraView);
        OnVideo(Bridge.Get(VideoView));
    }

    private void Halt()
    {
        if (IsLoaded) return;
        BridgeWatch.Stop(_client + ".video");
        BridgeWatch.Stop(_client + ".camera");
        _tick.Stop();
    }

    private void OnVideo(JsonObject video)
    {
        _video = video;
        _settingOn = MapUiJson.Flag(Bridge.Get(ShownSetting)["value"], true);
        Render();
    }

    private void OnCamera(JsonObject camera)
    {
        _camera = camera;
        Render();
    }

    private void Render()
    {
        var cameraPanel = VideoJson.Flag(_camera, "present") && _camera["panel"] is JsonObject panel && VideoJson.Flag(panel, "visible");
        Visibility = VideoJson.Shown(_settingOn && VideoJson.Flag(_video, "decoding") && !cameraPanel);
        var recording = VideoJson.Flag(_video, "recording");
        if (recording != _recording)
        {
            _recording = recording;
            if (recording) _elapsed.Restart(); else _elapsed.Reset();
        }
        var label = recording ? "Stop recording" : "Start recording";
        AutomationProperties.SetName(_button, label);
        ToolTipService.SetToolTip(_button, label);
        _mark.Width = recording ? Stop : Dot;
        _mark.Height = recording ? Stop : Dot;
        _mark.CornerRadius = new CornerRadius(recording ? 4 : Dot / 2);
        _readout.Visibility = VideoJson.Shown(recording);
        if (recording && Visibility == Visibility.Visible) _tick.Start(); else _tick.Stop();
        Paint();
    }

    private void Paint()
    {
        _clock.Text = $"REC {PayloadJson.RecordClock((long)_elapsed.Elapsed.TotalSeconds)}";
        AutomationProperties.SetName(_clock, _clock.Text);
        _blink.Opacity = _recording && _elapsed.Elapsed.TotalMilliseconds % 1200 >= 600 ? 0.2 : 1;
    }

    private void Press()
    {
        var refusal = VideoJson.Refusal(Bridge.Invoke(_recording ? StopRecording : StartRecording), Unanswered);
        FormLook.Say(_refusal, refusal);
        if (refusal is not null) _refusalTimer.Start();
        OnVideo(Bridge.Get(VideoView));
    }
}
