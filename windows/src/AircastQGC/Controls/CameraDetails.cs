using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

public sealed partial class CameraDetails : UserControl
{
    private const string CameraView = "view.camera";
    private const string CurrentCamera = "vehicle.cameraManager.currentCamera";
    private const string Instance = "vehicle.cameraManager.currentCameraInstance";
    private const string CameraSubject = "The camera";
    private static readonly TimeSpan WriteDelay = TimeSpan.FromMilliseconds(500);

    private readonly string _client = $"camera-details.{Guid.NewGuid():N}";
    private readonly StackPanel _body = new() { Spacing = 12 };
    private readonly TextBlock _refusal = FormLook.Caption("", "CriticalBrush");
    private readonly DispatcherQueueTimer _debounce;
    private Action? _pending;
    private string _shown = "";
    private int _groups;

    public CameraDetails()
    {
        Content = FormLook.Stack(8, _body, _refusal);
        _debounce = DispatcherQueue.GetForCurrentThread().CreateTimer();
        _debounce.Interval = WriteDelay;
        _debounce.IsRepeating = false;
        _debounce.Tick += (_, _) => Flush();
        Loaded += (_, _) =>
        {
            BridgeWatch.Watch(_client, [CameraView], Render);
            Render(Bridge.Get(CameraView));
        };
        Unloaded += (_, _) =>
        {
            if (IsLoaded) return;
            BridgeWatch.Stop(_client);
            Flush();
        };
    }

    private void Render(JsonObject view)
    {
        var present = VideoJson.Flag(view, "present");
        Visibility = VideoJson.Shown(present);
        var state = PayloadJson.Camera(view);
        var signature = present ? JsonSerializer.Serialize(state) : "";
        if (signature == _shown) return;
        _shown = signature;
        FormLook.Fill(_body, present ? Sections(state) : []);
    }

    private IEnumerable<UIElement?> Sections(CameraDetailsState state) =>
    [
        state.Labels.Count > 1 ? Section("CAMERAS", Radios(state.Labels, state.Selected, index => Write(CurrentCamera, index))) : null,
        state.Streams.Count > 1 ? Section("VIDEO STREAM", Radios(state.Streams, state.CurrentStream, index => Write($"{Instance}.currentStream", index))) : null,
        state.CapturesPhotos ? PhotoMode(state) : null,
        state.Tracking is { } tracking ? Wide(FormLook.Button(tracking.ToggleLabel, () => ToggleTracking(tracking), ButtonLook.Standard, "")) : null,
        state.ThermalMode is { } mode ? Thermal(mode, state.ThermalOpacity) : null,
        state.Destructive.Count > 0 ? Section("STORAGE AND RESET", FormLook.Stack(4, state.Destructive.Select(Destructive))) : null,
    ];

    private UIElement PhotoMode(CameraDetailsState state) => Section("PHOTO MODE", FormLook.Stack(8,
        FormLook.Line(8,
            FormLook.Button("Single", () => Write($"{Instance}.photoCaptureMode", 0), state.Timelapse ? ButtonLook.Standard : ButtonLook.Accent),
            FormLook.Button("Time lapse", () => Write($"{Instance}.photoCaptureMode", 1), state.Timelapse ? ButtonLook.Accent : ButtonLook.Standard)),
        state.Timelapse
            ? Level("Photo interval (seconds)", PayloadJson.LapseMin, PayloadJson.LapseMax, state.LapseSeconds, value => Write($"{Instance}.photoLapse", Math.Round(value)))
            : null));

    private UIElement Thermal(string mode, double? opacity) => FormLook.Stack(12,
        Section("THERMAL VIEW MODE", Radios(
            PayloadJson.ThermalModes.Select(m => m.Label).ToList(),
            PayloadJson.ThermalModes.ToList().FindIndex(m => m.Token == mode),
            index => Write($"{Instance}.thermalMode", index))),
        opacity is double blend
            ? Section("BLEND OPACITY", Level("Opacity", 0, 100, blend, value => Write($"{Instance}.thermalOpacity", Math.Round(value))))
            : null);

    private UIElement Destructive(DestructiveAction action)
    {
        var button = FormLook.Button(action.Button, () => Confirm(action));
        button.Resources["ButtonForeground"] = FormLook.Brush("CriticalBrush");
        button.Resources["ButtonForegroundPointerOver"] = FormLook.Brush("CriticalBrush");
        button.Resources["ButtonForegroundPressed"] = FormLook.Brush("CriticalBrush");
        button.IsEnabled = action.Ready;
        AutomationProperties.SetName(button, $"{action.Button}, {MapUiJson.SentenceCase(action.Label)}");
        var label = FormLook.Text(MapUiJson.SentenceCase(action.Label));
        label.VerticalAlignment = VerticalAlignment.Center;
        Grid.SetColumn(button, 1);
        return FormLook.Stack(2,
            new Grid
            {
                ColumnSpacing = 8,
                ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto } },
                Children = { label, button },
            },
            FormLook.Caption(action.BlockedReason ?? "", "TextTertiaryBrush"));
    }

    private async void Confirm(DestructiveAction action)
    {
        if (!await FormLook.Confirm(XamlRoot, action.Title, action.Prompt, "Yes", "No")) return;
        Report(action.Id switch
        {
            "resetSettings" => FormWrites.Call($"{Instance}.resetSettings", CameraSubject),
            "formatStorage" => FormWrites.Call($"{Instance}.formatCard", CameraSubject, 1),
            _ => $"{action.Title} is not available in this build.",
        });
    }

    private void ToggleTracking(TrackingState tracking) => Report(tracking.Requested
        ? FormWrites.Set($"{Instance}.trackingEnabled", false) ?? FormWrites.Call($"{Instance}.stopTracking", CameraSubject)
        : FormWrites.Set($"{Instance}.trackingEnabled", true));

    private UIElement Radios(IReadOnlyList<string> labels, int selected, Action<int> pick)
    {
        var group = $"{_client}.{++_groups}";
        return FormLook.Stack(0, labels.Select((label, index) => Radio(label, group, index == selected, () => pick(index))));
    }

    private static UIElement Radio(string label, string group, bool on, Action pick)
    {
        var radio = new RadioButton { Content = label, GroupName = group, IsChecked = on, FontFamily = PreflightLook.Font, MinHeight = 32 };
        radio.Checked += (_, _) => pick();
        return radio;
    }

    private UIElement Level(string title, double minimum, double maximum, double value, Action<double> write)
    {
        var text = FormLook.Text($"{title}  {value:0}", "FontSizeBody", "TextSecondaryBrush");
        var slider = new Slider { Minimum = minimum, Maximum = maximum, StepFrequency = 1, Value = value };
        AutomationProperties.SetName(slider, title);
        slider.ValueChanged += (_, e) =>
        {
            text.Text = $"{title}  {e.NewValue:0}";
            Later(() => write(e.NewValue));
        };
        return FormLook.Stack(0, text, slider);
    }

    private static UIElement Section(string title, UIElement body) => FormLook.Stack(4,
        new TextBlock { Text = title, Style = (Style)Application.Current.Resources["StatLabel"] },
        body);

    private static Button Wide(Button button)
    {
        button.HorizontalAlignment = HorizontalAlignment.Stretch;
        return button;
    }

    private void Write(string path, object value) => Report(FormWrites.Set(path, value));

    private void Later(Action write)
    {
        _pending = write;
        _debounce.Stop();
        _debounce.Start();
    }

    private void Flush()
    {
        var pending = _pending;
        _pending = null;
        pending?.Invoke();
    }

    private void Report(string? refusal) => FormLook.Say(_refusal, refusal);
}
