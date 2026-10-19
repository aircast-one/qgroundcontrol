using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;

namespace AircastQGC.Controls;

public sealed partial class CameraOverlay : UserControl
{
    private const string Client = "camera-overlay";
    private const string CameraView = "view.camera";
    private const string SettingsView = "view.cameraSettings";
    private const string ZoomPath = "vehicle.cameraManager.currentCameraInstance.zoomLevel";
    private const string SetModeAction = "camera.setMode";
    private const string TakePhotoAction = "camera.takePhoto";
    private const string StopPhotoAction = "camera.stopPhoto";
    private const string ToggleRecordingAction = "camera.toggleRecording";
    private const string SetSettingAction = "cameraSettings.set";
    private const int PhotoMode = 0;
    private const int VideoMode = 1;
    private const string Timelapse = "timelapse";
    private const string CameraGlyph = "";
    private const string RecordGlyph = "";
    private const string StopGlyph = "";
    private const string Unanswered = "The camera command was not answered.";
    private const string SettingUnanswered = "The camera setting was not answered.";
    private const string ModeBusy = "The camera is capturing. It will not change mode until that finishes.";

    private readonly DispatcherQueueTimer _settingsPoll;
    private JsonObject _camera = new();
    private string _settingsShown = "";
    private bool _rendering;

    public CameraOverlay()
    {
        _settingsPoll = DispatcherQueue.CreateTimer();
        _settingsPoll.Interval = TimeSpan.FromSeconds(1);
        _settingsPoll.Tick += (_, _) => RenderSettings(Bridge.Get(SettingsView));
        InitializeComponent();
        Loaded += (_, _) => Start();
        Unloaded += (_, _) => Stop();
    }

    public event EventHandler? CloseRequested;

    public event EventHandler? CameraPresentChanged;

    public bool CameraPresent { get; private set; }

    public void Show()
    {
        Visibility = Visibility.Visible;
        SyncSettingsPoll();
    }

    public void Hide()
    {
        Visibility = Visibility.Collapsed;
        SyncSettingsPoll();
    }

    private void Start()
    {
        BridgeWatch.Watch(Client, new[] { CameraView }, Render);
        Render(Bridge.Get(CameraView));
    }

    private void Stop()
    {
        if (IsLoaded) return;
        BridgeWatch.Stop(Client);
        _settingsPoll.Stop();
    }

    private void SyncSettingsPoll()
    {
        var wanted = IsLoaded && Visibility == Visibility.Visible && CameraPresent;
        if (wanted == _settingsPoll.IsRunning) return;
        if (!wanted)
        {
            _settingsPoll.Stop();
            return;
        }
        _settingsPoll.Start();
        RenderSettings(Bridge.Get(SettingsView));
    }

    private Style Look(string key) => (Style)Resources[key];

    private void Render(JsonObject camera)
    {
        _camera = camera;
        var present = VideoJson.Flag(camera, "present");
        var recording = VideoJson.Flag(camera, "isRecording");
        var mode = VideoJson.Int(camera, "mode", -1);
        var canChangeMode = VideoJson.Flag(camera, "canChangeMode");
        var canRecord = VideoJson.Flag(camera, "canRecord");
        var battery = VideoJson.Text(camera, "batteryText");
        var zoom = VideoJson.Number(camera["zoomLevel"]) ?? 0;
        var aside = mode is PhotoMode or VideoMode ? "" : $"Mode: {VideoJson.Text(camera, "modeText")}";

        _rendering = true;
        TitleText.Text = present ? VideoJson.Or(VideoJson.Text(camera, "title"), "Camera") : "Camera";
        StateText.Text = present ? VideoJson.Text(camera, "stateText") : "";
        StateIcon.Glyph = recording ? RecordGlyph : CameraGlyph;
        StateIcon.Style = Look(recording ? "CameraIconRecording" : "CameraIconIdle");
        EmptyText.Visibility = VideoJson.Shown(!present);
        Body.Visibility = VideoJson.Shown(present);

        ModeRow.Visibility = VideoJson.Shown(VideoJson.Flag(camera, "hasModes"));
        PhotoModeButton.Style = Look(mode == PhotoMode ? "CameraButtonAccent" : "CameraButton");
        VideoModeButton.Style = Look(mode == VideoMode ? "CameraButtonAccent" : "CameraButton");
        PhotoModeButton.IsEnabled = canChangeMode;
        VideoModeButton.IsEnabled = canChangeMode;
        ModeAside.Text = string.Join(" ", new[] { aside, canChangeMode ? "" : ModeBusy }.Where(s => s.Length > 0));
        ModeAside.Visibility = VideoJson.Shown(ModeAside.Text.Length > 0);

        ShutterButton.Visibility = VideoJson.Shown(present && VideoJson.Flag(camera, "canPhoto"));
        ShutterText.Text = VideoJson.Flag(camera, "isTakingPhoto") ? "Taking…"
            : VideoJson.Text(camera, "photoMode") == Timelapse ? "Start interval"
            : "Take photo";
        StopIntervalButton.Visibility = VideoJson.Shown(VideoJson.Flag(camera, "canStopPhoto"));
        RecordButton.Visibility = VideoJson.Shown(present && canRecord);
        RecordIcon.Glyph = recording ? StopGlyph : RecordGlyph;
        RecordText.Text = recording ? "Stop" : "Record";

        ShotsText.Text = VideoJson.Or(VideoJson.Text(camera, "shotsText"), "—");
        ClockRow.Visibility = VideoJson.Shown(canRecord);
        ClockText.Text = VideoJson.Text(camera, "clockText");
        BatteryRow.Visibility = VideoJson.Shown(battery.Length > 0);
        BatteryText.Text = battery;
        StorageRow.Visibility = VideoJson.Shown(VideoJson.Flag(camera, "reportsStorage", true));
        StorageText.Text = VideoJson.Text(camera, "storageText");

        ZoomRow.Visibility = VideoJson.Shown(VideoJson.Flag(camera, "hasZoom"));
        ZoomText.Text = zoom.ToString("0", CultureInfo.InvariantCulture);
        if (ZoomSlider.FocusState == FocusState.Unfocused) ZoomSlider.Value = zoom;
        ModeUnknownText.Visibility = VideoJson.Shown(present && !VideoJson.Flag(camera, "modeKnown"));
        _rendering = false;

        if (present != CameraPresent)
        {
            CameraPresent = present;
            CameraPresentChanged?.Invoke(this, EventArgs.Empty);
        }
        SyncSettingsPoll();
    }

    private void OnClose(object sender, RoutedEventArgs e)
    {
        Hide();
        CloseRequested?.Invoke(this, EventArgs.Empty);
    }

    private void OnPhotoMode(object sender, RoutedEventArgs e) => SetMode(PhotoMode);

    private void OnVideoMode(object sender, RoutedEventArgs e) => SetMode(VideoMode);

    private void SetMode(int mode)
    {
        if (!VideoJson.Flag(_camera, "canChangeMode")) return;
        Ask(SetModeAction, new JsonArray(JsonValue.Create(mode == PhotoMode ? "photo" : "video")));
    }

    private void OnShutter(object sender, RoutedEventArgs e)
    {
        if (!VideoJson.Flag(_camera, "present") || !VideoJson.Flag(_camera, "canPhoto")) return;
        ShowNotice(CaptureNotice(Ask(TakePhotoAction)));
    }

    private void OnStopInterval(object sender, RoutedEventArgs e)
    {
        if (!VideoJson.Flag(_camera, "canStopPhoto")) return;
        Ask(StopPhotoAction);
        ShowNotice("");
    }

    private void OnRecord(object sender, RoutedEventArgs e)
    {
        if (!VideoJson.Flag(_camera, "present") || !VideoJson.Flag(_camera, "canRecord")) return;
        Ask(ToggleRecordingAction);
    }

    private void OnZoomChanged(object sender, RangeBaseValueChangedEventArgs e)
    {
        if (_rendering || !double.IsFinite(e.NewValue)) return;
        if (!VideoJson.Flag(_camera, "present") || !VideoJson.Flag(_camera, "hasZoom")) return;
        var reason = VideoJson.Refusal(Bridge.Set(ZoomPath, JsonValue.Create(e.NewValue)), "");
        ShowRefusal(reason is null ? null : $"Could not change the camera zoom. {reason}".Trim());
    }

    private JsonObject Ask(string action, JsonArray? args = null)
    {
        var answer = Bridge.Invoke(action, args);
        ShowRefusal(VideoJson.Refusal(answer, Unanswered));
        Render(Bridge.Get(CameraView));
        return answer;
    }

    private void ShowRefusal(string? reason)
    {
        RefusalText.Text = reason ?? "";
        RefusalBox.Visibility = VideoJson.Shown(reason is not null);
    }

    private void ShowNotice(string notice)
    {
        NoticeText.Text = notice;
        NoticeText.Visibility = VideoJson.Shown(notice.Length > 0);
    }

    private static string CaptureNotice(JsonObject answer)
    {
        if (VideoJson.Text(answer, "started") != Timelapse) return "";
        var every = VideoJson.Number(answer["lapseSeconds"]) is double seconds
            ? $" every {seconds.ToString("0", CultureInfo.InvariantCulture)} s"
            : "";
        var count = VideoJson.Int(answer, "lapseCount", 0);
        return VideoJson.Flag(answer, "lapseUnlimited") ? $"Started an interval capture{every}. It will not stop on its own."
            : count > 0 ? $"Started {count} shots{every}."
            : $"Started an interval capture{every}.";
    }

    private void RenderSettings(JsonObject view)
    {
        var state = VideoJson.Text(view, "state");
        var rows = state == "ready" ? ActiveSettings(view) : new List<JsonObject>();
        SettingsState.Text = state switch
        {
            "fetching" => "Reading the camera's settings…",
            "failed" => VideoJson.Or(VideoJson.Text(view, "error"), "Could not read the camera's settings."),
            _ => "",
        };
        SettingsState.Visibility = VideoJson.Shown(SettingsState.Text.Length > 0);
        SettingsSection.Visibility = VideoJson.Shown(SettingsState.Text.Length > 0 || rows.Count > 0);

        var signature = string.Join("\n", rows.Select(Signature));
        if (signature == _settingsShown) return;
        _settingsShown = signature;
        SettingsList.Children.Clear();
        rows.Select(SettingRow).ToList().ForEach(SettingsList.Children.Add);
    }

    private static List<JsonObject> ActiveSettings(JsonObject view)
    {
        var byName = (view["parameters"] as JsonArray ?? new JsonArray())
            .OfType<JsonObject>()
            .GroupBy(p => VideoJson.Text(p, "name"))
            .ToDictionary(g => g.Key, g => g.Last());
        return (view["activeSettings"] as JsonArray ?? new JsonArray())
            .Select(n => n is JsonValue v && v.TryGetValue<string>(out var s) ? s : "")
            .Where(byName.ContainsKey)
            .Select(n => byName[n])
            .ToList();
    }

    private static string Signature(JsonObject p) =>
        string.Join("|",
            VideoJson.Text(p, "name"),
            p["value"]?.ToJsonString() ?? "",
            VideoJson.Int(p, "selected", -1),
            VideoJson.Flag(p, "readOnly"),
            p["options"]?.ToJsonString() ?? "");

    private FrameworkElement SettingRow(JsonObject parameter)
    {
        var name = VideoJson.Text(parameter, "name");
        var control = SettingControl(parameter, name);
        control.IsEnabled = !VideoJson.Flag(parameter, "readOnly");
        control.VerticalAlignment = VerticalAlignment.Center;
        Grid.SetColumn(control, 1);
        var label = new TextBlock
        {
            Text = VideoJson.Or(VideoJson.Text(parameter, "description"), name),
            Style = Look("CameraRowLabel"),
        };
        var row = new Grid { ColumnSpacing = 8 };
        row.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        row.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        row.Children.Add(label);
        row.Children.Add(control);
        return row;
    }

    private Control SettingControl(JsonObject parameter, string name)
    {
        var value = parameter["value"];
        var options = (parameter["options"] as JsonArray ?? new JsonArray()).OfType<JsonObject>().ToList();
        var range = (Min: VideoJson.Number(parameter["min"]), Max: VideoJson.Number(parameter["max"]), Step: VideoJson.Number(parameter["step"]));
        return VideoJson.Flag(parameter, "isBool") ? ToggleBox(name, IsOn(value))
            : options.Count > 0 ? ChoiceBox(name, options, VideoJson.Int(parameter, "selected", -1))
            : range is (double min, double max, double step) ? RangeBox(name, min, max, step, VideoJson.Number(value) ?? min)
            : EntryBox(name, value);
    }

    private static bool IsOn(JsonNode? value) =>
        value is JsonValue v && (v.TryGetValue<bool>(out var on) ? on : v.TryGetValue<double>(out var n) && Math.Truncate(n) == 1);

    private Control ToggleBox(string name, bool on)
    {
        var toggle = new ToggleSwitch { IsOn = on, OnContent = "", OffContent = "", MinWidth = 0 };
        toggle.Toggled += (_, _) => WriteSetting(name, JsonValue.Create(toggle.IsOn ? 1 : 0));
        return toggle;
    }

    private Control ChoiceBox(string name, List<JsonObject> options, int selected)
    {
        var shown = selected >= 0 && selected < options.Count ? selected : -1;
        var box = new ComboBox
        {
            ItemsSource = options
                .Select(o => VideoJson.Or(VideoJson.Text(o, "label"), o["value"]?.ToJsonString() ?? ""))
                .ToList(),
            SelectedIndex = shown,
            MinWidth = 140,
        };
        box.SelectionChanged += (_, _) =>
        {
            var picked = box.SelectedIndex;
            if (picked == shown || picked < 0 || picked >= options.Count) return;
            WriteSetting(name, options[picked]["value"]?.DeepClone());
        };
        return box;
    }

    private Control RangeBox(string name, double min, double max, double step, double value)
    {
        var box = new NumberBox
        {
            Minimum = min,
            Maximum = max,
            SmallChange = step,
            Value = value,
            SpinButtonPlacementMode = NumberBoxSpinButtonPlacementMode.Compact,
            MinWidth = 140,
        };
        box.ValueChanged += (_, e) =>
        {
            if (double.IsNaN(e.NewValue) || e.NewValue == value) return;
            WriteSetting(name, JsonValue.Create(e.NewValue));
        };
        return box;
    }

    private Control EntryBox(string name, JsonNode? value)
    {
        var box = new TextBox
        {
            Text = value is JsonValue v && v.TryGetValue<string>(out var s) ? s : value?.ToJsonString() ?? "",
            MinWidth = 140,
        };
        box.KeyDown += (_, e) =>
        {
            if (e.Key != Windows.System.VirtualKey.Enter) return;
            WriteSetting(name, double.TryParse(box.Text, NumberStyles.Float, CultureInfo.InvariantCulture, out var number)
                ? JsonValue.Create(number)
                : JsonValue.Create(box.Text));
        };
        return box;
    }

    private void WriteSetting(string name, JsonNode? value)
    {
        var answer = Bridge.Invoke(SetSettingAction, new JsonArray(JsonValue.Create(name), value));
        ShowRefusal(VideoJson.Refusal(answer, SettingUnanswered));
        DispatcherQueue.TryEnqueue(() => RenderSettings(Bridge.Get(SettingsView)));
    }
}
