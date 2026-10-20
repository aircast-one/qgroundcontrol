using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Shapes;

namespace AircastQGC.Pages.Setup;

internal sealed record CalibrationRoutine(string Id, string Title, string Invocation, IReadOnlyList<bool> Arguments, bool Enabled, string Description, string DialogHelp, string Status, string Warning, bool SpinsPropeller, string DialogTitle);

internal sealed record CalibrationSide(string Key, string Title, bool Visible, string Stage, bool Rotate);

internal sealed record CompassResult(int Compass, double Green, double Yellow, double Range, double Position);

internal sealed record FastCompass(string Invocation, string Help, bool VehicleHasPosition, double? GcsLatitude, double? GcsLongitude);

internal sealed record CalibrationState(
    bool InProgress, bool ShowsSides, bool NextEnabled, bool CancelEnabled, double Progress, bool WaitingForCancel,
    string HelpText, string StatusText, string Completed, bool Px4, string SettingsTitle, string SettingsDialogTitle,
    FastCompass? Fast, IReadOnlyList<CalibrationSide> Sides, IReadOnlyList<CalibrationRoutine> Routines, IReadOnlyList<CompassResult> CompassResults)
{
    public const string View = "view.calibration";
    public const string Accel = "accelerometer";
    public const string CompassRoutine = "compass";
    public const string Complete = "Calibration complete";
    private const string MustReboot = "YOU MUST REBOOT YOUR VEHICLE AFTER EACH CALIBRATION.";
    private const string CompassQuality = "Shown in the indicator bars is the quality of the calibration for each compass.\n\n" +
        "- Green indicates a well functioning compass.\n" +
        "- Yellow indicates a questionable compass or calibration.\n" +
        "- Red indicates a compass which should not be used.\n\n";

    public static CalibrationState? Parse(JsonObject view) => MapUiJson.Text(view["class"]) != "Calibration" ? null : new CalibrationState(
        MapUiJson.Flag(view["inProgress"]),
        MapUiJson.Flag(view["showsSides"]),
        MapUiJson.Flag(view["nextEnabled"]),
        MapUiJson.Flag(view["cancelEnabled"]),
        MapUiJson.Number(view["progress"]) ?? 0,
        MapUiJson.Flag(view["waitingForCancel"]),
        MapUiJson.Text(view["helpText"]),
        MapUiJson.Text(view["statusText"]),
        MapUiJson.Text(view["completed"]),
        MapUiJson.Flag(view["px4"]),
        MapUiJson.Text(view["settingsTitle"]),
        MapUiJson.Text(view["settingsDialogTitle"]),
        view["fastCompass"] is JsonObject fast ? new FastCompass(
            MapUiJson.Text(fast["invocation"]),
            MapUiJson.Text(fast["help"]),
            MapUiJson.Flag(fast["vehicleHasPosition"]),
            GcsValid(fast) ? MapUiJson.Number(fast["gcsPosition"]?["latitude"]) : null,
            GcsValid(fast) ? MapUiJson.Number(fast["gcsPosition"]?["longitude"]) : null) : null,
        MapUiJson.Objects(view["sides"]).Select(s => new CalibrationSide(
            MapUiJson.Text(s["key"]), MapUiJson.Text(s["title"]), MapUiJson.Flag(s["visible"]), MapUiJson.Text(s["stage"]), MapUiJson.Flag(s["rotate"]))).ToList(),
        MapUiJson.Objects(view["routines"]).Select(r => new CalibrationRoutine(
            MapUiJson.Text(r["id"]),
            MapUiJson.Text(r["title"]),
            MapUiJson.Text(r["invocation"]),
            (r["arguments"] as JsonArray ?? []).Select(a => MapUiJson.Flag(a)).ToList(),
            MapUiJson.Flag(r["enabled"]),
            MapUiJson.Text(r["description"]),
            MapUiJson.Text(r["dialogHelp"]),
            MapUiJson.Text(r["status"]),
            MapUiJson.Text(r["warning"]),
            MapUiJson.Flag(r["spinsPropeller"]),
            MapUiJson.Text(r["dialogTitle"]))).ToList(),
        MapUiJson.Objects(view["compassResults"]).Select(c => new CompassResult(
            (int)(MapUiJson.Number(c["compass"]) ?? 0),
            MapUiJson.Number(c["green"]) ?? 0,
            MapUiJson.Number(c["yellow"]) ?? 0,
            MapUiJson.Number(c["range"]) ?? 1,
            MapUiJson.Number(c["position"]) ?? 0)).ToList());

    public static string PostTitle(string? routine, bool px4) =>
        px4 && routine == CompassRoutine ? "Compass calibration complete" : Complete;

    public static string? PostPrompt(string? routine, string helpText, string completed, bool px4) =>
        px4 ? (helpText == Complete && routine == CompassRoutine ? "Reboot the vehicle prior to flight." : null)
        : completed == CompassRoutine ? CompassQuality + MustReboot
        : completed == Accel ? MustReboot
        : null;

    public static string? PositionsText(IReadOnlyList<CalibrationSide> sides)
    {
        var shown = sides.Where(s => s.Visible).ToList();
        var reached = shown.Count(s => s.Stage is "done" or "inProgress");
        return shown.Count == 0 ? null : $"{Math.Max(reached, 1)} of {shown.Count} positions";
    }

    public static string SideState(CalibrationSide side) => side.Stage switch
    {
        "inProgress" => side.Rotate ? "Rotate" : "Hold still",
        "done" => "Done",
        _ => "Pending",
    };

    public static string SideImage(string key, bool rotating) => key switch
    {
        "UpsideDown" => "cal_vehicle_upside_down",
        "Left" => "cal_vehicle_left",
        "Right" => "cal_vehicle_right",
        "NoseDown" => "cal_vehicle_nose_down",
        "TailDown" => "cal_vehicle_tail_down",
        _ => "cal_vehicle_down",
    } + (rotating ? "_rotate.png" : ".png");

    public static string RunningTitle(string name) => name.Trim().Length > 0 ? $"Calibrating {name.Trim()}" : "Calibration in progress";

    private static bool GcsValid(JsonObject fast) => MapUiJson.Flag(fast["gcsPosition"]?["valid"]);
}

internal sealed class SensorsPanel : UserControl
{
    private const string Cal = "sensorsCal";
    private const string FactoryReset = "sensorSettings.factoryReset";
    private const string HealthView = "view.sensors";
    private const string CancelWaitTitle = "Cancelling calibration";
    private const string CancelWaitText = "Waiting for Vehicle to response to Cancel. This may take a few seconds.";
    private const string Footnote = "Calibrate where the aircraft will fly, away from metal, with the propellers off. CompassMot is the exception and says so when you open it: it runs the motors, with the propellers inverted.";
    private static readonly TimeSpan StartWait = TimeSpan.FromSeconds(5);
    private static readonly TimeSpan StartPoll = TimeSpan.FromMilliseconds(150);

    private JsonObject _calibration = new();
    private JsonObject _health = new();
    private bool _connected;
    private bool _wasInProgress;
    private string _runningName = "";
    private string? _ranRoutine;
    private string? _notice;

    public SensorsPanel()
    {
        Live.Watch(this, NoVehicleState.FlyStatePath, view =>
        {
            var connected = !NoVehicleState.IsNoVehicle(view);
            if (connected == _connected && Content is not null) return;
            _connected = connected;
            Render();
        });
        Live.Watch(this, CalibrationState.View, view =>
        {
            _calibration = view;
            Render();
        });
        Live.Watch(this, HealthView, view =>
        {
            _health = view;
            Render();
        });
        Unloaded += (_, _) =>
        {
            NavigationLock.Release(SetupModel.CalibrationBlock);
            if (CalibrationState.Parse(_calibration)?.InProgress == true) SetupUi.InOrder(() => FormWrites.Ok($"{Cal}.cancelCalibration"));
        };
    }

    private void Render()
    {
        var state = CalibrationState.Parse(_calibration);
        if (state is not null) Transition(state);
        Content = !_connected ? SetupUi.Notice("Connect a vehicle to calibrate its sensors.")
            : state is null ? SetupUi.Notice("Reading the vehicle's calibration state.")
            : state.InProgress ? Running(state)
            : Idle(state);
    }

    private void Transition(CalibrationState state)
    {
        if (_wasInProgress && !state.InProgress && CalibrationState.PostPrompt(_ranRoutine, state.HelpText, state.Completed, state.Px4) is { } prompt)
            RebootPrompt(state, prompt);
        _wasInProgress = state.InProgress;
        if (state.InProgress && state.Px4) NavigationLock.Hold(SetupModel.CalibrationBlock);
        else NavigationLock.Release(SetupModel.CalibrationBlock);
    }

    private UIElement Running(CalibrationState state)
    {
        var ring = new ProgressRing
        {
            IsIndeterminate = false,
            Minimum = 0,
            Maximum = 1,
            Value = Math.Clamp(state.Progress, 0, 1),
            Width = 180,
            Height = 180,
            Background = FormLook.Brush("ControlBrush"),
        };
        var percent = FormLook.Text($"{(int)(state.Progress * 100)}%", "FontSizeTitle", "TextPrimaryBrush", true);
        percent.HorizontalAlignment = HorizontalAlignment.Center;
        percent.VerticalAlignment = VerticalAlignment.Center;
        var dial = new Grid { HorizontalAlignment = HorizontalAlignment.Center, Margin = new Thickness(0, 8, 0, 8), Children = { ring, percent } };
        var cancel = FormLook.Button("Cancel", () => SetupUi.Detached(() => FormWrites.Ok($"{Cal}.cancelCalibration")), ButtonLook.Subtle);
        cancel.IsEnabled = state.CancelEnabled;
        var next = FormLook.Button("Next", () => SetupUi.Detached(() => FormWrites.Ok($"{Cal}.nextClicked")), ButtonLook.Accent);
        next.IsEnabled = state.NextEnabled;
        var buttons = FormLook.Line(8, cancel, next);
        buttons.HorizontalAlignment = HorizontalAlignment.Right;
        return SetupUi.Page(
            state.ShowsSides && CalibrationState.PositionsText(state.Sides) is { } positions ? FormLook.Text(positions, "FontSizeBody", "AccentTextBrush", true) : null,
            FormLook.Text(CalibrationState.RunningTitle(_runningName), "FontSizeSubtitle", "TextPrimaryBrush", true),
            FormLook.Text(state.HelpText, "FontSizeBody", "TextSecondaryBrush"),
            dial,
            state.ShowsSides ? Sides(state.Sides, state.Px4) : null,
            FormLook.Text(state.StatusText, "FontSizeBody", "TextSecondaryBrush"),
            buttons,
            state.WaitingForCancel ? FormLook.Text(CancelWaitTitle, "FontSizeBody", "TextPrimaryBrush", true) : null,
            state.WaitingForCancel ? FormLook.Text(CancelWaitText, "FontSizeBody", "TextSecondaryBrush") : null);
    }

    private static UIElement Sides(IReadOnlyList<CalibrationSide> sides, bool px4) =>
        FormLook.Fill(new VariableSizedWrapGrid { Orientation = Orientation.Horizontal, ItemWidth = 124, ItemHeight = 148 }, sides.Where(s => s.Visible).Select(side =>
        {
            var current = side.Stage == "inProgress";
            var done = side.Stage == "done";
            var tint = current ? "AccentBrush" : done ? "SuccessBrush" : "TextTertiaryBrush";
            return (UIElement)new Border
            {
                Margin = new Thickness(0, 0, 8, 8),
                Padding = new Thickness(8),
                BorderThickness = new Thickness(2),
                BorderBrush = FormLook.Brush(current || done ? tint : "CardStrokeBrush"),
                CornerRadius = FormLook.Radius("RadiusControl"),
                Background = FormLook.Brush("CardBrush"),
                Child = FormLook.Stack(4,
                    AssetImages.Png(AssetImages.Calibration, CalibrationState.SideImage(side.Key, px4 && current && side.Rotate), 72),
                    FormLook.Text(side.Title, "FontSizeCaption", "TextPrimaryBrush"),
                    FormLook.Text(CalibrationState.SideState(side), "FontSizeBody", tint, current)),
            };
        }));

    private UIElement Idle(CalibrationState state)
    {
        var sensors = MapUiJson.Flag(_health["available"]) && MapUiJson.Text(_health["class"]) == "SensorHealth"
            ? MapUiJson.Objects(_health["sensors"]).Where(s => MapUiJson.Text(s["name"]).Trim().Length > 0).ToList()
            : [];
        var failing = MapUiJson.Texts(_health["failing"]);
        var summary = failing.Count == 1 ? $"{failing[0]} is reporting a fault." : failing.Count > 1 ? $"{failing.Count} sensors are reporting faults." : "";
        return SetupUi.Page(new UIElement?[]
        {
            sensors.Count > 0 ? SetupUi.Heading("Sensor health") : null,
            sensors.Count > 0 ? FormLook.Caption(summary, "CriticalBrush") : null,
            sensors.Count > 0 ? FormLook.Card(sensors.Select(s => SetupUi.Row(
                MapUiJson.Text(s["name"]),
                MapUiJson.Text(s["label"]),
                MapUiJson.Text(s["state"]) switch { "healthy" => SetupState.Done, "unhealthy" => SetupState.NeedsAttention, _ => SetupState.Neutral },
                glyph: ""))) : null,
            SetupUi.Heading("Calibration"),
            _notice is null ? null : FormLook.Caption(_notice, "CriticalBrush"),
            FormLook.Card(state.Routines.Select(routine => SetupUi.Row(
                MapUiJson.SentenceCase(routine.Title),
                routine.SpinsPropeller ? "Spins the motors" : routine.Status,
                routine.Status switch { "Not calibrated" => SetupState.NeedsAttention, "Calibrated" => SetupState.Done, _ => SetupState.Neutral },
                routine.Enabled ? () => Start(routine, state) : null,
                glyph: routine.Id == CalibrationState.CompassRoutine || routine.Id == "compassMot" ? "" : ""))
                .Append(SetupUi.Row(MapUiJson.SentenceCase(state.SettingsTitle), onClick: () => Settings(state), glyph: ""))
                .Concat(state.Px4 ? [SetupUi.Row("Factory reset", state: SetupState.NeedsAttention, onClick: ConfirmFactoryReset, glyph: "")] : [])),
            state.StatusText.Length > 0 ? SetupUi.Heading("Last calibration") : null,
            state.StatusText.Length > 0 ? SetupUi.Monospace(state.StatusText) : null,
            FormLook.Footnote(Footnote),
        });
    }

    private async void Settings(CalibrationState state)
    {
        var dialog = FormLook.Dialog(XamlRoot, state.SettingsDialogTitle.Length > 0 ? state.SettingsDialogTitle : state.SettingsTitle,
            new ScrollViewer { Content = SensorSettingsForm.Block(false, true), MaxHeight = 560, MinWidth = 480 }, null, "OK");
        await FormLook.Show(dialog);
    }

    private async void ConfirmFactoryReset()
    {
        if (!await FormLook.Confirm(XamlRoot, "Factory reset", "Reset every parameter on the vehicle to its factory default?", "Reset")) return;
        _notice = await FormWrites.Off(() => SetupUi.Refusal(FactoryReset));
        Render();
    }

    private async void Start(CalibrationRoutine routine, CalibrationState state)
    {
        var fast = routine.Id == CalibrationState.CompassRoutine ? state.Fast : null;
        var offersSimple = routine.Id == CalibrationState.Accel && routine.Arguments.Count > 0;
        var orientationFirst = routine.Id is CalibrationState.Accel or CalibrationState.CompassRoutine;
        var simple = new Flag();
        var choice = fast is null ? null : new FastChoice { UseGcs = fast.GcsLatitude is not null };
        var dialog = FormLook.Dialog(XamlRoot, routine.DialogTitle.Length > 0 ? routine.DialogTitle : routine.Title, new ScrollViewer
        {
            MaxHeight = 560,
            MinWidth = 480,
            Content = FormLook.Stack(12,
                orientationFirst ? SensorSettingsForm.Block(true, routine.Id == CalibrationState.CompassRoutine, offersSimple ? on => simple.On = on : null) : null,
                FormLook.Text(routine.DialogHelp.Length > 0 ? routine.DialogHelp : routine.Description),
                fast is null || choice is null ? null : FastBlock(fast, choice),
                FormLook.Text(routine.Warning, "FontSizeBody", "CriticalBrush")),
        }, "OK");
        if (await FormLook.Show(dialog) != ContentDialogResult.Primary) return;
        var (invocation, arguments) = fast is not null && choice is { Enabled: true }
            ? (fast.Invocation, choice.Arguments(fast))
            : offersSimple ? (routine.Invocation, new object?[] { simple.On })
            : (routine.Invocation, routine.Arguments.Select(a => (object?)a).ToArray());
        _runningName = routine.Title;
        _ranRoutine = routine.Id;
        _notice = null;
        var started = await Task.Run(async () =>
        {
            var before = CalibrationState.Parse(Bridge.Get(CalibrationState.View))?.StatusText ?? "";
            if (!FormWrites.Ok(invocation, arguments)) return false;
            var deadline = DateTime.UtcNow + StartWait;
            while (DateTime.UtcNow < deadline)
            {
                var now = CalibrationState.Parse(Bridge.Get(CalibrationState.View));
                if (now is not null && (now.InProgress || now.StatusText != before)) return true;
                await Task.Delay(StartPoll);
            }
            return false;
        });
        _notice = started ? null : $"{routine.Title} calibration did not start.";
        Render();
    }

    private static UIElement FastBlock(FastCompass fast, FastChoice choice)
    {
        var asks = FormLook.Caption("Vehicle has no valid position, please provide it");
        var useGcs = FormLook.Check("Use GCS position instead", choice.UseGcs, on => choice.UseGcs = on);
        var latitude = FormLook.Field(choice.Latitude, "Latitude");
        var longitude = FormLook.Field(choice.Longitude, "Longitude");
        latitude.TextChanged += (_, _) => choice.Latitude = latitude.Text;
        longitude.TextChanged += (_, _) => choice.Longitude = longitude.Text;
        void Paint()
        {
            var asking = choice.Enabled && !fast.VehicleHasPosition;
            asks.Visibility = FormLook.Shown(asking);
            useGcs.Visibility = FormLook.Shown(asking && fast.GcsLatitude is not null);
            latitude.Visibility = longitude.Visibility = FormLook.Shown(choice.Enabled && !(choice.UseGcs && fast.GcsLatitude is not null));
        }
        var enable = FormLook.Check("Fast Calibration", choice.Enabled, on =>
        {
            choice.Enabled = on;
            Paint();
        });
        useGcs.Click += (_, _) => Paint();
        Paint();
        return FormLook.Stack(8, FormLook.Caption(fast.Help), enable, asks, useGcs, latitude, longitude);
    }

    private async void RebootPrompt(CalibrationState state, string prompt)
    {
        var dialog = FormLook.Dialog(XamlRoot, CalibrationState.PostTitle(_ranRoutine, state.Px4), new ScrollViewer
        {
            MaxHeight = 560,
            Content = FormLook.Stack(8, new UIElement?[] { FormLook.Text(prompt) }
                .Concat(!state.Px4 && state.Completed == CalibrationState.CompassRoutine ? state.CompassResults.Select(FitnessBar) : [])
                .Append(state.Px4 && _ranRoutine == CalibrationState.CompassRoutine ? SensorSettingsForm.Orientations() : null)),
        }, "Reboot vehicle", "Close");
        if (await FormLook.Show(dialog) == ContentDialogResult.Primary) SetupUi.Detached(() => FormWrites.Ok(SetupModel.RebootVehicle));
    }

    private static UIElement FitnessBar(CompassResult result)
    {
        var range = result.Range > 0 ? result.Range : 1;
        var bands = new Grid
        {
            Height = 16,
            ColumnDefinitions =
            {
                new ColumnDefinition { Width = new GridLength(Math.Max(0, result.Green / range), GridUnitType.Star) },
                new ColumnDefinition { Width = new GridLength(Math.Max(0, (result.Yellow - result.Green) / range), GridUnitType.Star) },
                new ColumnDefinition { Width = new GridLength(Math.Max(0, (range - result.Yellow) / range), GridUnitType.Star) },
            },
        };
        new[] { "SuccessBrush", "CautionBrush", "CriticalBrush" }.Select((brush, at) => (Brush: brush, At: at)).ToList().ForEach(band =>
        {
            var fill = new Border { Background = FormLook.Brush(band.Brush) };
            Grid.SetColumn(fill, band.At);
            bands.Children.Add(fill);
        });
        var position = Math.Clamp(result.Position, 0, 1);
        var marker = new Grid
        {
            Height = 16,
            ColumnDefinitions =
            {
                new ColumnDefinition { Width = new GridLength(position, GridUnitType.Star) },
                new ColumnDefinition { Width = new GridLength(1 - position, GridUnitType.Star) },
            },
        };
        var dot = new Ellipse
        {
            Width = 11,
            Height = 11,
            Fill = FormLook.Brush("TextPrimaryBrush"),
            Stroke = FormLook.Brush("BgBaseBrush"),
            StrokeThickness = 1,
            HorizontalAlignment = HorizontalAlignment.Left,
            Margin = new Thickness(-5.5, 0, 0, 0),
        };
        Grid.SetColumn(dot, 1);
        marker.Children.Add(dot);
        return FormLook.Stack(2,
            FormLook.Text(string.Format(CultureInfo.InvariantCulture, "Compass {0}", result.Compass), "FontSizeCaption", "TextPrimaryBrush"),
            new Grid { Children = { bands, marker } });
    }

    private sealed class Flag
    {
        public bool On { get; set; }
    }

    private sealed class FastChoice
    {
        public bool Enabled { get; set; }
        public bool UseGcs { get; set; }
        public string Latitude { get; set; } = "0.00";
        public string Longitude { get; set; } = "0.00";

        public object?[] Arguments(FastCompass fast) =>
            UseGcs && fast.GcsLatitude is { } lat && fast.GcsLongitude is { } lon
                ? [lat, lon]
                : [Typed(Latitude), Typed(Longitude)];

        private static object Typed(string text) => FormText.Parse(text) is { } number ? number : text;
    }
}
