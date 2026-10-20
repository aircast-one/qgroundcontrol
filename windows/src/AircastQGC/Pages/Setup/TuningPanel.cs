using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Diagnostics;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Shapes;

namespace AircastQGC.Pages.Setup;

internal sealed record TuningSample(double Seconds, double Value);

internal static class TuningRange
{
    private const double Tick = 5;
    public const double HistorySeconds = 180;

    public static (double Low, double High)? Grown((double Low, double High)? range, IEnumerable<double> values) =>
        values.Aggregate(range, (held, value) => held is { } r ? (GrownLow(r.Low, value), GrownHigh(r.High, value)) : (value, value));

    public static IReadOnlyList<TuningSample> With(IReadOnlyList<TuningSample> series, TuningSample sample) =>
        series.Append(sample).SkipWhile(s => s.Seconds < sample.Seconds - HistorySeconds).ToList();

    private static double GrownLow(double current, double value) =>
        Math.Min(current, value) is var low && low % Tick != 0 ? Math.Floor((low - Tick) / Tick) * Tick : Math.Min(current, value);

    private static double GrownHigh(double current, double value) =>
        Math.Max(current, value) is var high && high % Tick != 0 ? Math.Floor((high + Tick) / Tick) * Tick : Math.Max(current, value);
}

internal sealed class TuningChart : UserControl
{
    private const string FlightMode = "vehicle.flightMode";
    private static readonly TimeSpan Sample = TimeSpan.FromMilliseconds(30);
    private static readonly string[] Colours = ["AccentBrush", "CautionBrush"];

    private readonly IReadOnlyList<(string Name, string Path)> _plots;
    private readonly double _window;
    private readonly (string Stabilized, string Pause)? _modes;
    private readonly Canvas _canvas = new() { Height = 180 };
    private readonly TextBlock _range = FormLook.Caption("");
    private readonly Button _startStop;
    private readonly TextBlock _hints = FormLook.Caption("");
    private readonly Stopwatch _clock = new();
    private IReadOnlyList<IReadOnlyList<TuningSample>> _series;
    private (double Low, double High)? _bounds;
    private double _offset;
    private bool _running = true;
    private bool _autoModeChange;
    private bool _busy;
    private readonly DispatcherTimer _timer = new() { Interval = Sample };

    public TuningChart(string title, IReadOnlyList<(string Name, string Path)> plots, string unit, double window, (string Stabilized, string Pause)? modes)
    {
        _plots = plots;
        _window = window > 0 ? window : 8;
        _modes = modes;
        _series = plots.Select(_ => (IReadOnlyList<TuningSample>)[]).ToList();
        _canvas.Background = FormLook.Brush("CardBrush");
        _canvas.SizeChanged += (_, _) => Draw();
        _startStop = FormLook.Button("Stop", Toggle);
        var legend = FormLook.Line(12, plots.Select((plot, at) => (UIElement?)FormLook.Line(4,
            new Border { Width = 10, Height = 10, Background = FormLook.Brush(Colours[at % Colours.Length]), VerticalAlignment = VerticalAlignment.Center },
            FormLook.Caption(plot.Name, "TextPrimaryBrush"))).Append(_range).ToArray());
        var auto = modes is null ? null : FormLook.Check("Automatic flight mode switching", false, on =>
        {
            _autoModeChange = on;
            if (on) Run(false);
            Paint();
        });
        Content = FormLook.Stack(6,
            FormLook.Text(title, "FontSizeBody", "TextPrimaryBrush", true),
            _canvas,
            legend,
            FormLook.Line(8, FormLook.Button("Clear", Clear), _startStop),
            auto,
            _hints);
        _timer.Tick += (_, _) => Tick();
        Loaded += (_, _) =>
        {
            if (_running) Start();
        };
        Unloaded += (_, _) =>
        {
            _timer.Stop();
            _clock.Stop();
        };
        Live.Watch(this, NoVehicleState.FlyStatePath, view =>
        {
            if (MapUiJson.Flag(view["armed"]) && !_running) Run(true);
        });
        _unit = unit;
        Paint();
    }

    private readonly string _unit;

    private void Start()
    {
        _clock.Start();
        _timer.Start();
    }

    private double Now => _offset + _clock.Elapsed.TotalSeconds;

    private async void Tick()
    {
        if (_busy || !_running) return;
        _busy = true;
        try
        {
            var paths = _plots.Select(p => p.Path).ToList();
            var values = await Task.Run(() => paths.Select(path => MapUiJson.Number(Bridge.Get(path)["value"])).ToList());
            var now = Now;
            _series = _series.Zip(values, (list, value) => value is { } v ? TuningRange.With(list, new TuningSample(now, v)) : list).ToList();
            _bounds = TuningRange.Grown(_bounds, values.OfType<double>());
            Draw();
        }
        finally { _busy = false; }
    }

    private void Draw()
    {
        _canvas.Children.Clear();
        var width = _canvas.ActualWidth;
        var height = _canvas.Height;
        _canvas.Children.Add(new Line { X1 = 0, X2 = width, Y1 = height / 2, Y2 = height / 2, Stroke = FormLook.Brush("DividerBrush"), StrokeThickness = 1 });
        FormLook.Say(_range, _bounds is { } b ? string.Format(CultureInfo.InvariantCulture, "{0:F2} … {1:F2} {2}", b.Low, b.High, _unit) : null);
        if (_bounds is not { } bounds || width <= 0) return;
        var span = bounds.High - bounds.Low > 0 ? bounds.High - bounds.Low : 1;
        var from = Now - _window;
        _series.Select((list, at) => (List: list.Where(s => s.Seconds >= from).ToList(), At: at))
            .Where(s => s.List.Count >= 2)
            .ToList()
            .ForEach(s =>
            {
                var points = new PointCollection();
                s.List.Select(sample => new Windows.Foundation.Point((sample.Seconds - from) / _window * width, height - (sample.Value - bounds.Low) / span * height))
                    .ToList()
                    .ForEach(points.Add);
                _canvas.Children.Add(new Polyline { Points = points, Stroke = FormLook.Brush(Colours[s.At % Colours.Length]), StrokeThickness = 2 });
            });
    }

    private void Clear()
    {
        _series = _plots.Select(_ => (IReadOnlyList<TuningSample>)[]).ToList();
        _bounds = null;
        _offset = 0;
        _clock.Reset();
        if (_running) _clock.Start();
        Draw();
    }

    private void Toggle()
    {
        Run(!_running);
        if (_modes is { } modes && _autoModeChange)
        {
            var mode = _running ? modes.Stabilized : modes.Pause;
            SetupUi.Detached(() => FormWrites.Set(FlightMode, mode));
        }
    }

    private void Run(bool running)
    {
        _running = running;
        if (running)
        {
            Start();
        }
        else
        {
            _offset = Now;
            _clock.Reset();
            _timer.Stop();
        }
        Paint();
    }

    private void Paint()
    {
        if (_startStop.Content is TextBlock label) label.Text = _running ? "Stop" : "Start";
        AutomationProperties.SetName(_startStop, _running ? "Stop" : "Start");
        FormLook.Say(_hints, _modes is { } modes && _autoModeChange ? $"Switches to 'Stabilized' when you click Start.\nSwitches to '{modes.Pause}' when you click Stop." : null);
    }
}

internal sealed class AutotuneSection : UserControl
{
    private const string View = "view.autotune";
    private const string Request = "vehicle.autotune.autotuneRequest";

    public AutotuneSection() => Live.Watch(this, View, view =>
    {
        if (!MapUiJson.Flag(view["available"]))
        {
            Content = null;
            return;
        }
        var start = FormLook.Button("Start AutoTune", () => Ask(MapUiJson.Text(view["warning"])), ButtonLook.Accent);
        start.IsEnabled = MapUiJson.Flag(view["canStart"]);
        Content = FormLook.Stack(8, start, FormLook.Text(MapUiJson.Text(view["status"])), SetupUi.Bar(MapUiJson.Number(view["progress"]) ?? 0));
    });

    private async void Ask(string warning)
    {
        if (await FormLook.Confirm(XamlRoot, "Start AutoTune", warning, "Ok")) SetupUi.Detached(() => FormWrites.Ok(Request));
    }
}

internal sealed class TuningPanel : UserControl
{
    private const string View = "view.px4Tuning";
    private const string Telemetry = "vehicle.setPIDTuningTelemetryMode";

    private JsonObject _view = new();
    private bool _loaded;
    private int _tab;
    private int _axis;
    private bool _autoTuning;
    private int? _telemetryMode;
    private ImmutableList<(string Name, string Path, string Value)> _clipboard = ImmutableList<(string, string, string)>.Empty;
    private string _clipboardFor = "";
    private string? _refusal;
    private int _reads;
    private (string Key, TuningChart Chart)? _chart;
    private ContentControl? _chartHost;

    public TuningPanel()
    {
        Content = SetupUi.Notice("Reading parameters from the vehicle.");
        Loaded += (_, _) => Refresh();
        Unloaded += (_, _) =>
        {
            _telemetryMode = null;
            SetupUi.InOrder(() => FormWrites.Ok(Telemetry, 0));
        };
    }

    private async void Refresh()
    {
        var read = ++_reads;
        var view = await FormWrites.Off(() => Bridge.Get(View));
        if (read != _reads) return;
        _view = view;
        _loaded = true;
        Render();
    }

    private IReadOnlyList<JsonObject> Tabs => MapUiJson.Flag(_view["available"]) ? MapUiJson.Objects(_view["tabs"]) : [];

    private void Render()
    {
        if (!_loaded) return;
        var tabs = Tabs;
        var tab = tabs.ElementAtOrDefault(_tab);
        if (tab is null)
        {
            Content = SetupUi.Notice("This vehicle has no PX4 tuning page.");
            return;
        }
        var axes = MapUiJson.Objects(tab["axes"]);
        var axis = axes.ElementAtOrDefault(_axis) ?? axes.FirstOrDefault();
        var mode = (int)(MapUiJson.Number(tab["tuningMode"]) ?? 0);
        if (mode != _telemetryMode)
        {
            _telemetryMode = mode;
            SetupUi.InOrder(() => FormWrites.Ok(Telemetry, mode));
        }
        var parameters = axis is null ? [] : MapUiJson.Objects(axis["params"]).Where(p => p["fact"] is JsonObject).ToList();
        var clipboardKey = $"{_tab}|{_axis}";
        if (clipboardKey != _clipboardFor)
        {
            _clipboardFor = clipboardKey;
            _clipboard = Snapshot(parameters);
        }
        var autoTuning = MapUiJson.Flag(tab["autoTuning"]);
        var modes = MapUiJson.Flag(tab["autoModeChange"]) ? (MapUiJson.Text(_view["stabilizedFlightMode"]), MapUiJson.Text(_view["pauseFlightMode"])) : ((string, string)?)null;
        Content = SetupUi.Page(new UIElement?[]
        {
            Chips(tabs.Select(t => MapUiJson.SentenceCase(MapUiJson.Text(t["name"]))).ToList(), _tab, at =>
            {
                _tab = at;
                _axis = 0;
                Render();
            }),
        }
        .Concat(MapUiJson.Objects(tab["extras"]).Select(extra => SetupUi.Control(extra) is { } control ? (UIElement?)SetupUi.Embedded([control], Refresh) : null))
        .Concat(
        [
            Chips(axes.Select(a => MapUiJson.SentenceCase(MapUiJson.Text(a["name"]))).ToList(), axis is null ? -1 : axes.ToList().IndexOf(axis), at =>
            {
                _axis = at;
                Render();
            }),
            axis is null ? null : Chart(clipboardKey, () => new TuningChart(
                MapUiJson.SentenceCase(MapUiJson.Text(axis["chartTitle"])),
                MapUiJson.Objects(axis["plot"]).Select(p => (MapUiJson.Text(p["name"]), MapUiJson.Text(p["path"]))).ToList(),
                MapUiJson.Text(tab["unit"]),
                MapUiJson.Number(tab["chartSeconds"]) ?? 8,
                modes)),
            autoTuning ? FormLook.Line(16,
                Radio("Use auto-tuning", _autoTuning, () => { _autoTuning = true; Render(); }),
                Radio("Use manual tuning", !_autoTuning, () => { _autoTuning = false; Render(); })) : null,
        ])
        .Concat(autoTuning && _autoTuning
            ? [new AutotuneSection()]
            : parameters.Select(Slider).Append(axis is null ? null : FormLook.Line(8,
                FormLook.Button("Save to clipboard", () =>
                {
                    _clipboard = Snapshot(parameters);
                    Render();
                }),
                Restore())))
        .Concat(
        [
            _refusal is null ? null : FormLook.Text(_refusal, "FontSizeBody", "CriticalBrush"),
            _clipboard.IsEmpty ? null : FormLook.Text("Clipboard values:", "FontSizeBody", "TextPrimaryBrush", true),
        ])
        .Concat(_clipboard.Select(entry => (UIElement?)SetupUi.Monospace($"{entry.Name}  {entry.Value}")))
        .ToArray());
    }

    private UIElement Chart(string key, Func<TuningChart> build)
    {
        var chart = _chart is { } held && held.Key == key ? held.Chart : build();
        _chart = (key, chart);
        if (_chartHost is not null) _chartHost.Content = null;
        _chartHost = new ContentControl { Content = chart, HorizontalContentAlignment = HorizontalAlignment.Stretch, IsTabStop = false };
        return _chartHost;
    }

    private static ImmutableList<(string Name, string Path, string Value)> Snapshot(IReadOnlyList<JsonObject> parameters) =>
        parameters.Select(p => p["fact"] as JsonObject).OfType<JsonObject>()
            .Select(fact => (MapUiJson.Text(fact["name"]), MapUiJson.Text(fact["path"]), FormJson.Shown(fact["valueString"])))
            .ToImmutableList();

    private Button Restore()
    {
        var button = FormLook.Button("Restore from clipboard", () =>
        {
            var entries = _clipboard;
            SetupUi.Run(() => entries
                .Select(entry => FormText.Parse(entry.Value) is { } number ? SetupUi.Write(entry.Path, number) : null)
                .FirstOrDefault(refused => refused is not null), refused =>
            {
                _refusal = refused;
                Refresh();
            });
        });
        button.IsEnabled = !_clipboard.IsEmpty;
        return button;
    }

    private UIElement Slider(JsonObject parameter)
    {
        var fact = (JsonObject)parameter["fact"]!;
        var min = MapUiJson.Number(parameter["min"]) ?? 0;
        var max = MapUiJson.Number(parameter["max"]) ?? 1;
        var step = MapUiJson.Number(parameter["step"]) ?? 0;
        var path = MapUiJson.Text(fact["path"]);
        var current = MapUiJson.Number(fact["value"]) ?? FormText.Parse(FormJson.Shown(fact["valueString"])) ?? min;
        var slider = new Slider
        {
            Minimum = min,
            Maximum = max > min ? max : min + 1,
            StepFrequency = Math.Max(1e-6, step > 0 && max > min ? (max - min) / Math.Max(1, Math.Round((max - min) / step)) : (max - min) / 1000),
            Value = Math.Clamp(current, min, max),
            IsThumbToolTipEnabled = false,
        };
        var title = MapUiJson.SentenceCase(MapUiJson.Text(parameter["title"]));
        AutomationProperties.SetName(slider, title);
        void Save()
        {
            var chosen = slider.Value;
            if (Math.Abs(chosen - current) < 1e-12) return;
            SetupUi.Run(() => SetupUi.Write(path, chosen), refused =>
            {
                _refusal = refused;
                Refresh();
            });
        }
        slider.AddHandler(PointerReleasedEvent, new Microsoft.UI.Xaml.Input.PointerEventHandler((_, _) => Save()), true);
        slider.KeyUp += (_, _) => Save();
        var value = FormLook.Text(FormJson.Shown(fact["valueString"]), "FontSizeBody", "TextSecondaryBrush");
        value.VerticalAlignment = VerticalAlignment.Center;
        var row = new Grid { ColumnSpacing = 8, ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto } } };
        row.Children.Add(slider);
        Grid.SetColumn(value, 1);
        row.Children.Add(value);
        return FormLook.Stack(2,
            FormLook.Text(title),
            FormLook.Caption(MapUiJson.Text(parameter["description"])),
            row);
    }

    private static UIElement Chips(IReadOnlyList<string> labels, int selected, Action<int> pick) =>
        FormLook.Line(8, labels.Select((label, at) =>
        {
            var chip = new ToggleButton { Content = label, IsChecked = at == selected };
            chip.Click += (_, _) => pick(at);
            return (UIElement?)chip;
        }).ToArray());

    private static RadioButton Radio(string text, bool on, Action pick)
    {
        var radio = new RadioButton { Content = text, IsChecked = on, GroupName = "tuningMode" };
        radio.Checked += (_, _) =>
        {
            if (!on) pick();
        };
        return radio;
    }
}
