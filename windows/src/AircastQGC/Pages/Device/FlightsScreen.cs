using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading;
using System.Threading.Tasks;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Shapes;

namespace AircastQGC.Pages.Device;

internal sealed record Zoom(double From, double To)
{
    public static Zoom Compose(Zoom? current, double a, double b)
    {
        var window = current ?? new Zoom(0, 1);
        var span = window.To - window.From;
        return new Zoom(window.From + Math.Min(a, b) * span, window.From + Math.Max(a, b) * span);
    }
}

internal sealed class FlightsScreen : DeviceScreen
{
    public const double MinBrush = 0.02;
    private const string LanesKey = "flight-lanes";

    private IReadOnlyList<string>? _lanes = DeviceStore.Read(LanesKey)?.Split(',', StringSplitOptions.RemoveEmptyEntries);

    public FlightsScreen(DeviceNav nav) : base("flights", nav) { }

    protected override IEnumerable<Node?>? Build(DeviceReading reading, DeviceContext ctx) =>
        Viewed(reading, view => view["flights"] is JsonArray, view =>
        {
            var flights = DJ.Objs(view, "flights");
            var linked = Nav.Query.GetValueOrDefault("flight", "");
            return new Node?[]
            {
                Kit.Text("One entry per run of this device. None of it reaches you in the air — the ground station only gets the few lines worth interrupting a flight for."),
                flights.Count == 0 ? Kit.Notice(new Badge(DeviceLevel.Neutral, "No flights recorded yet. This device starts one every time it powers up.")) : null,
            }.Concat(flights.Select(flight => (Node?)DeviceTree.Kept($"flight:{DJ.Text(flight, "id")}",
                () => new FlightBox(DJ.Flag(flight, "open") || linked == DJ.Text(flight, "id")),
                box => box.Show(ctx, flight, _lanes, PickLanes))));
        });

    private void PickLanes(IReadOnlyList<string> keys)
    {
        DeviceStore.Write(LanesKey, string.Join(",", keys));
        _lanes = keys;
        Rerender();
    }
}

internal sealed class FlightBox : UserControl
{
    private readonly StackPanel _root = new();
    private readonly bool _initiallyOpen;
    private bool _alertsOnly;
    private int? _active;
    private DeviceContext? _ctx;
    private JsonObject _flight = new();
    private IReadOnlyList<string>? _lanes;
    private Action<IReadOnlyList<string>> _pickLanes = _ => { };

    public FlightBox(bool open)
    {
        _initiallyOpen = open;
        Content = _root;
        Loaded += (_, _) =>
        {
            if (_initiallyOpen) StartBringIntoView();
        };
    }

    public void Show(DeviceContext ctx, JsonObject flight, IReadOnlyList<string>? lanes, Action<IReadOnlyList<string>> pickLanes)
    {
        (_ctx, _flight, _lanes, _pickLanes) = (ctx, flight, lanes, pickLanes);
        Paint();
    }

    private void Paint()
    {
        if (_ctx is not { } ctx) return;
        DeviceTree.Sync(_root, [
            Kit.Disclosure(DJ.Text(_flight, "title"), DJ.Text(_flight, "summary"), DJ.Badge(_flight, "badge"), _initiallyOpen, () => Body(ctx)),
            DeviceTree.Leaf("rule", "", () => new Border { Height = 1, Background = FormLook.Brush("DividerBrush") }),
        ]);
    }

    private IEnumerable<Node?> Body(DeviceContext ctx)
    {
        var sections = DJ.Objs(_flight, "sections");
        var hasLines = sections.Any(part => DJ.Objs(part, "lines").Count > 0);
        var trace = DJ.Obj(_flight, "trace");
        var id = DJ.Text(_flight, "id");
        return (hasLines
                ? sections.Select((part, at) => (Node?)Kit.Group($"part:{at}:{DJ.Text(part, "title")}", Part(ctx, part)))
                    .Prepend(Kit.Buttons("alerts", [
                        ConnectScreen.Chip("everything", "Everything", !_alertsOnly, () => Alerts(false)),
                        ConnectScreen.Chip("alerts", "Alerts only", _alertsOnly, () => Alerts(true)),
                    ]))
                : [Kit.Text("Nothing worth recording — the device had no faults to report.")])
            .Append(trace is null ? null : DeviceTree.Kept("lanes", () => new LanesBox(), box => box.Show(trace, _active, _lanes, _pickLanes)))
            .Append(Kit.Notice(new Badge(DeviceLevel.Warn, DJ.Text(_flight, "dropped"))))
            .Append(Kit.Buttons("files", [
                Kit.Link(ctx, ctx.Url(DJ.Text(_flight, "csv")), "Samples as CSV", LinkLook.Outlined),
                Kit.Link(ctx, ctx.Url(DJ.Text(_flight, "json")), "Everything as JSON", LinkLook.Text),
                Kit.Button("copy-flight", "Copy link to this flight", () => Kit.Clip(ctx,
                    $"{ctx.Url("/fly/")}#device={DeviceLink.Escaped($"/flights?flight={id}")}", "Link copied — it opens straight on this flight."), ButtonLook.Subtle, glyph: ""),
            ]))
            .Append(Kit.Hint("The CSV is one row per sample — every lane above and more besides: signal and radio, the aircraft's height, distance, battery and satellites, video per camera, viewers, what was dropped, and the box's own load and memory. " +
                             "The JSON adds every line above."));
    }

    private void Alerts(bool only)
    {
        _alertsOnly = only;
        Paint();
    }

    private void Activate(int index)
    {
        _active = _active == index ? null : index;
        Paint();
    }

    private IEnumerable<Node?> Part(DeviceContext ctx, JsonObject part) =>
        new Node?[]
        {
            DJ.Text(part, "title") is { Length: > 0 } title
                ? DeviceTree.Leaf("part-title", DJ.Sig(title, DJ.Text(part, "span")), () => FormLook.Margined(FormLook.Line(12,
                    FormLook.Text(title.ToUpperInvariant(), "FontSizeCaption", "TextSecondaryBrush", true),
                    Kit.MonoText(DJ.Text(part, "span"), "FontSizeCaption", "TextSecondaryBrush")), new Thickness(16, 16, 16, 4)))
                : null,
        }
        .Concat(DJ.Objs(part, "videos").Select(video => (Node?)Kit.Group($"video:{DJ.Text(video, "camera")}", new Node?[]
        {
            Kit.Buttons("download", [Kit.Link(ctx, ctx.Url(DJ.Text(video, "url")), DJ.Text(video, "label"), LinkLook.Outlined, glyph: "")]),
            DJ.Obj(video, "share") is { } share ? DeviceTree.Kept("share", () => new ShareBox(share), box => box.Show(ctx)) : null,
        })))
        .Concat(DJ.Objs(part, "lines")
            .Where(line => !_alertsOnly || DJ.Flag(line, "alert"))
            .Select(line => (Node?)DeviceTree.Leaf($"line:{DJ.Int(line, "index")}", DJ.Sig(line, _active == DJ.Int(line, "index")), () => Line(line))));

    private UIElement Line(JsonObject line)
    {
        var index = DJ.Int(line, "index");
        var held = DJ.Flag(line, "heldBack");
        var level = DJ.Int(line, "level", DeviceLevel.Neutral);
        var clock = Kit.MonoText(DJ.Text(line, "clock"), "FontSizeCaption", "TextSecondaryBrush");
        clock.Width = 72;
        var text = FormLook.Text(DJ.Text(line, "text"), "FontSizeBody", held ? "TextSecondaryBrush" : Kit.LevelTextBrush(level));
        var told = new Border
        {
            BorderBrush = FormLook.Brush("ControlStrokeBrush"),
            BorderThickness = new Thickness(1),
            CornerRadius = new CornerRadius(10),
            Padding = new Thickness(8, 1, 8, 1),
            Visibility = FormLook.Shown(held),
            Child = FormLook.Caption("pilot not told"),
        };
        var grid = new Grid
        {
            Padding = new Thickness(8, 4, 8, 4),
            ColumnSpacing = 12,
            CornerRadius = FormLook.Radius("RadiusControl"),
            Background = _active == index ? FormLook.Brush("SubtleSelectedBrush") : null,
            ColumnDefinitions =
            {
                new ColumnDefinition { Width = GridLength.Auto },
                new ColumnDefinition(),
                new ColumnDefinition { Width = GridLength.Auto },
                new ColumnDefinition { Width = GridLength.Auto },
            },
        };
        var repeats = Kit.MonoText(DJ.Text(line, "repeats"), "FontSizeCaption", "TextSecondaryBrush");
        new FrameworkElement[] { clock, text, repeats, told }.Select((child, at) => (child, at)).ToList().ForEach(pair =>
        {
            Grid.SetColumn(pair.child, pair.at);
            pair.child.VerticalAlignment = VerticalAlignment.Center;
            grid.Children.Add(pair.child);
        });
        var button = FormLook.Stretched(grid, $"{DJ.Text(line, "clock")} {DJ.Text(line, "text")}", () => Activate(index));
        button.Margin = new Thickness(8, 0, 8, 0);
        return button;
    }
}

internal sealed class LanesBox : UserControl
{
    private const string ChartPath = "deviceFlight.chart";

    private readonly StackPanel _root = new() { Padding = new Thickness(0, 8, 0, 0) };
    private JsonObject? _trace;
    private string _traceSig = "";
    private JsonObject? _chart;
    private int? _active;
    private IReadOnlyList<string>? _lanes;
    private Action<IReadOnlyList<string>> _pick = _ => { };
    private int _asked;

    public LanesBox()
    {
        Content = _root;
    }

    public Zoom? Zoomed { get; private set; }

    public double? Cursor { get; private set; }

    public Zoom? Brush { get; private set; }

    public void Show(JsonObject trace, int? active, IReadOnlyList<string>? lanes, Action<IReadOnlyList<string>> pick)
    {
        (_active, _lanes, _pick) = (active, lanes, pick);
        var sig = DJ.Sig(trace);
        if (sig != _traceSig)
        {
            (_trace, _traceSig) = (trace, sig);
            Fetch();
        }
        Paint();
    }

    public void Down(double at)
    {
        Brush = new Zoom(at, at);
        Cursor = at;
        PaintCharts();
    }

    public void Move(double at)
    {
        if (Brush is not { } brush) return;
        Brush = brush with { To = at };
        PaintCharts();
    }

    public void Up()
    {
        var brush = Brush;
        Brush = null;
        if (brush is not null && Math.Abs(brush.To - brush.From) >= FlightsScreen.MinBrush) ZoomTo(Zoom.Compose(Zoomed, brush.From, brush.To));
        else PaintCharts();
    }

    private void ZoomTo(Zoom? zoom)
    {
        Zoomed = zoom;
        Fetch();
    }

    private async void Fetch()
    {
        if (_trace is not { } trace) return;
        var asked = Interlocked.Increment(ref _asked);
        var answer = await DeviceLink.Result(ChartPath, trace, Zoomed?.From, Zoomed?.To);
        if (asked != _asked || answer is not JsonObject chart) return;
        _chart = chart;
        Paint();
    }

    private IReadOnlyList<string> Keys(IReadOnlyList<JsonObject> lanes) =>
        _lanes ?? lanes.Where(lane => DJ.Flag(lane, "default")).Select(lane => DJ.Text(lane, "key")).ToList();

    private void Paint()
    {
        if (_chart is null) return;
        var lanes = DJ.Objs(_chart, "lanes");
        var spans = DJ.Objs(_chart, "spans");
        var keys = Keys(lanes);
        var shown = lanes.Where(lane => keys.Contains(DJ.Text(lane, "key"))).ToList();
        DeviceTree.Sync(_root, new Node?[]
        {
            DeviceTree.Leaf("rule", "", () => new Border { Height = 1, Background = FormLook.Brush("DividerBrush") }),
            Kit.Buttons("lane-chips", lanes.Select(lane =>
            {
                var key = DJ.Text(lane, "key");
                var label = DJ.Text(lane, "peak") is { Length: > 0 } peak ? $"{DJ.Text(lane, "chip")} {peak}" : DJ.Text(lane, "chip");
                return (Node?)ConnectScreen.Chip($"lane:{key}", label, keys.Contains(key), () =>
                    _pick(keys.Contains(key) ? keys.Where(k => k != key).ToList() : keys.Append(key).ToList()));
            })),
            spans.Count > 0 || Zoomed is not null
                ? Kit.Buttons("spans", spans.Select(span =>
                    {
                        var zoom = new Zoom(DJ.Number(span, "from"), DJ.Number(span, "to", 1));
                        return (Node?)ConnectScreen.Chip($"span:{DJ.Text(span, "label")}", DJ.Text(span, "label"), Zoomed == zoom, () => ZoomTo(zoom));
                    })
                    .Append(Zoomed is null ? null : Kit.Button("whole", "Whole flight", () => ZoomTo(null), ButtonLook.Subtle)))
                : null,
        }
        .Concat(shown.Select((lane, at) => (Node?)DeviceTree.Kept($"lane:{DJ.Text(lane, "key")}", () => new LaneChart(this),
            chart => chart.Show(lane, _active, at == shown.Count - 1))))
        .Append(Kit.Hint("Pick the lanes you want above. One cursor across them: point at any lane to read every sample at that moment. Drag across a lane to zoom them all, or jump to one sortie with the chips above. " +
                         "Dashed lines are the moments listed above, in their colour; point at a line to pick out its mark."))
        .OfType<Node>());
    }

    private void PaintCharts() => _root.Children.OfType<LaneChart>().ToList().ForEach(chart => chart.Draw());
}

internal sealed class LaneChart : UserControl
{
    private const double ChartHeight = 112;

    private readonly LanesBox _plot;
    private readonly TextBlock _label = FormLook.Caption("");
    private readonly TextBlock _reading = Kit.MonoText("", "FontSizeCaption");
    private readonly Canvas _canvas = new() { Height = ChartHeight, Background = new SolidColorBrush(Microsoft.UI.Colors.Transparent) };
    private readonly Grid _ticks = new();
    private readonly TextBlock _span = FormLook.Caption("");
    private JsonObject _lane = new();
    private int? _active;
    private bool _pressed;

    public LaneChart(LanesBox plot)
    {
        _plot = plot;
        _label.Visibility = Visibility.Visible;
        _reading.Visibility = Visibility.Visible;
        Grid.SetColumn(_reading, 1);
        _canvas.SizeChanged += (_, _) => Draw();
        _canvas.PointerPressed += (_, e) =>
        {
            _pressed = _canvas.CapturePointer(e.Pointer);
            _plot.Down(At(e));
        };
        _canvas.PointerMoved += (_, e) =>
        {
            if (_pressed) _plot.Move(At(e));
        };
        _canvas.PointerReleased += (_, e) =>
        {
            _pressed = false;
            _canvas.ReleasePointerCapture(e.Pointer);
            _plot.Up();
        };
        _canvas.PointerCaptureLost += (_, _) =>
        {
            if (!_pressed) return;
            _pressed = false;
            _plot.Up();
        };
        Content = new StackPanel
        {
            Margin = new Thickness(16, 16, 16, 0),
            Spacing = 4,
            Children =
            {
                new Grid { ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto } }, Children = { _label, _reading } },
                _canvas,
                _ticks,
                _span,
            },
        };
    }

    public void Show(JsonObject lane, int? active, bool last)
    {
        (_lane, _active) = (lane, active);
        _label.Text = DJ.Text(lane, "label").ToUpperInvariant();
        AutomationProperties.SetName(_canvas, $"{DJ.Text(lane, "label")} over the flight");
        _ticks.Visibility = FormLook.Shown(last);
        FormLook.Say(_span, last ? DJ.Text(lane, "span") : "");
        var ticks = DJ.Objs(lane, "ticks");
        _ticks.ColumnDefinitions.Clear();
        _ticks.Children.Clear();
        ticks.Select((tick, at) => (tick, at)).ToList().ForEach(pair =>
        {
            _ticks.ColumnDefinitions.Add(new ColumnDefinition());
            var label = Kit.MonoText(DJ.Text(pair.tick, "label"), "FontSizeCaption", "TextSecondaryBrush");
            label.HorizontalAlignment = pair.at == 0 ? HorizontalAlignment.Left : pair.at == ticks.Count - 1 ? HorizontalAlignment.Right : HorizontalAlignment.Center;
            Grid.SetColumn(label, pair.at);
            _ticks.Children.Add(label);
        });
        Draw();
    }

    public void Draw()
    {
        var width = _canvas.ActualWidth;
        var height = ChartHeight;
        var across = width / 100;
        var down = height / 100;
        var readings = DJ.Texts(_lane, "readings");
        var sample = _plot.Cursor is { } cursor && readings.Count > 0
            ? (int)Math.Clamp(Math.Round(cursor * (readings.Count - 1)), 0, readings.Count - 1)
            : (int?)null;
        _reading.Text = sample is { } index ? readings[index] : "";
        var ink = FormLook.Brush("AccentBrush");
        var rule = FormLook.Brush("TextPrimaryBrush");
        var grid = DJ.Objs(_lane, "grid").SelectMany(line =>
        {
            var y = DJ.Number(line, "y") * down;
            var text = Kit.MonoText(DJ.Text(line, "label"), "FontSizeCaption", "TextSecondaryBrush");
            text.IsTextSelectionEnabled = false;
            var label = new Border { Child = text, Background = Backdrop, Padding = new Thickness(4, 0, 4, 0), CornerRadius = FormLook.Radius("RadiusControl") };
            Canvas.SetTop(label, y - 8);
            label.Loaded += (_, _) => Canvas.SetLeft(label, Math.Max(0, width - label.ActualWidth));
            return new UIElement[] { new Line { X1 = 0, X2 = width, Y1 = y, Y2 = y, Stroke = rule, Opacity = 0.12, StrokeThickness = 1 }, label };
        });
        var markers = DJ.Objs(_lane, "markers").Select(marker =>
        {
            var x = DJ.Number(marker, "x") * across;
            var picked = DJ.Int(marker, "index") == _active;
            var level = DJ.Int(marker, "level", DeviceLevel.Neutral);
            var line = new Line
            {
                X1 = x, X2 = x, Y1 = 0, Y2 = height,
                StrokeThickness = 1,
                Opacity = picked ? 1 : 0.7,
                Stroke = FormLook.Brush(Kit.LevelBrush(level)),
            };
            if (!picked) line.StrokeDashArray = new DoubleCollection { 2, 2 };
            ToolTipService.SetToolTip(line, DJ.Text(marker, "label"));
            return (UIElement)line;
        });
        var runs = (_lane["lines"] as JsonArray ?? []).OfType<JsonArray>().Select(run =>
        {
            var points = new PointCollection();
            run.OfType<JsonArray>()
                .Select(point => new Windows.Foundation.Point((Coordinate(point, 0)) * across, Coordinate(point, 1) * down))
                .ToList().ForEach(points.Add);
            return (UIElement)new Polyline { Points = points, Stroke = ink, StrokeThickness = 1.5 };
        });
        var brush = _plot.Brush is { } b
            ? new UIElement[] { Placed(new Rectangle { Width = Math.Abs(b.To - b.From) * width, Height = height, Fill = ink, Opacity = 0.15 }, Math.Min(b.From, b.To) * width) }
            : [];
        var cursorLine = sample is { } s && readings.Count > 0
            ? new UIElement[] { CursorLine(readings.Count > 1 ? s / (double)(readings.Count - 1) * width : 0, height, rule) }
            : [];
        _canvas.Children.Clear();
        grid.Concat(markers).Concat(runs).Concat(brush).Concat(cursorLine).ToList().ForEach(_canvas.Children.Add);
    }

    private static Brush Backdrop => new SolidColorBrush(((SolidColorBrush)FormLook.Brush("BgBaseBrush")).Color) { Opacity = 0.8 };

    private static double Coordinate(JsonArray point, int at) => AircastQGC.Controls.MapUiJson.Number(point.ElementAtOrDefault(at)) ?? 0;

    private static UIElement CursorLine(double x, double height, Brush rule) =>
        new Line { X1 = x, X2 = x, Y1 = 0, Y2 = height, Stroke = rule, Opacity = 0.4, StrokeThickness = 1 };

    private static UIElement Placed(FrameworkElement element, double left)
    {
        Canvas.SetLeft(element, left);
        return element;
    }

    private double At(PointerRoutedEventArgs e) =>
        _canvas.ActualWidth > 0 ? Math.Clamp(e.GetCurrentPoint(_canvas).Position.X / _canvas.ActualWidth, 0, 1) : 0;
}

internal sealed class ShareBox : UserControl
{
    private const string RenderPath = "deviceFlight.render";

    private readonly JsonObject _share;
    private readonly string _path;
    private readonly string _client = $"device.render.{Guid.NewGuid():N}";
    private readonly StackPanel _root = new() { Margin = new Thickness(16, 0, 16, 8), Spacing = 4 };
    private DeviceContext? _ctx;
    private JsonObject _render = new();

    public ShareBox(JsonObject share)
    {
        _share = share;
        _path = $"view.deviceRender({DJ.Text(share, "camera")},{DJ.Text(share, "from")},{DJ.Text(share, "to")})";
        Content = _root;
        Loaded += async (_, _) =>
        {
            BridgeWatch.Watch(_client, [_path], Apply);
            Apply(await Task.Run(() => Bridge.Get(_path)));
        };
        Unloaded += (_, _) => BridgeWatch.Stop(_client);
    }

    public void Show(DeviceContext ctx)
    {
        _ctx = ctx;
        Paint();
    }

    private void Apply(JsonObject render)
    {
        _render = render;
        Paint();
    }

    private void Paint()
    {
        if (_ctx is not { } ctx) return;
        var canStart = _render["canStart"] is null || DJ.Flag(_render, "canStart");
        var url = DJ.Text(_render, "url");
        DeviceTree.Sync(_root, new Node?[]
        {
            canStart ? Kit.Button("start", DJ.Text(_share, "label"), Start) : null,
            Kit.Hint(DJ.Text(_render, "progress")),
            Kit.StatusLine(DJ.Text(_render, "error"), DeviceLevel.Blocked),
            url.Length == 0 ? null : Kit.Buttons("shared", [
                Kit.Button("qr", "QR code", async () => await Kit.QrDialog(XamlRoot, "Share version", url), ButtonLook.Subtle, glyph: ""),
                Kit.Link(ctx, url, "Open the share version", LinkLook.Text, true),
                Kit.Button("copy", "Copy link", () => Kit.Clip(ctx, url), ButtonLook.Subtle),
            ]),
            Kit.Hint(DJ.Text(_render, "expiry")),
        }.OfType<Node>());
    }

    private async void Start()
    {
        if (!await Kit.Ask(XamlRoot, new Confirm(DJ.Text(_share, "label"), DJ.Text(_share, "confirm"), "Send"))) return;
        await DeviceLink.Invoke(RenderPath, DJ.Text(_share, "camera"), DJ.Text(_share, "from"), DJ.Text(_share, "to"));
    }
}
