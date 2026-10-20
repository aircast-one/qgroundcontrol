using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;
using AircastQGC.Core;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Shapes;
using Windows.Foundation;

namespace AircastQGC.Pages.Analyze;

internal sealed record ChartPlot(string Label, string Field, int Colour, IReadOnlyList<(long Age, double Value)> Points);

internal sealed record ChartModel(int RangeX, int RangeY, long WindowMs, double? YMin, double? YMax, bool Room, IReadOnlyList<ChartPlot> Plots);

internal sealed record InspectorCharts(IReadOnlyList<string> TimeScales, IReadOnlyList<string> Ranges,
    IReadOnlyList<ChartModel> Charts, IReadOnlyDictionary<string, int> Charted)
{
    internal const string ChartsPath = "view.inspectorCharts";
    internal static readonly IReadOnlyList<string> Labels = ["Plot 1", "Plot 2"];

    internal static InspectorCharts? Parse(JsonObject view) =>
        view["charts"] is JsonArray charts
            ? new InspectorCharts(
                Strings(view["timeScales"]),
                Strings(view["ranges"]),
                charts.OfType<JsonObject>().Select(chart => new ChartModel(
                    Ui.Int(chart["rangeX"]),
                    Ui.Int(chart["rangeY"]),
                    (long)(MapUiJson.Number(chart["windowMs"]) ?? 5000),
                    MapUiJson.Number(chart["yMin"]),
                    MapUiJson.Number(chart["yMax"]),
                    MapUiJson.Flag(chart["room"], true),
                    MapUiJson.Objects(chart["plots"]).Select(plot => new ChartPlot(
                        MapUiJson.Text(plot["label"]),
                        MapUiJson.Text(plot["field"]),
                        Ui.Int(plot["colour"]),
                        (plot["points"] as JsonArray ?? [])
                            .OfType<JsonArray>()
                            .Select(point => ((long)(MapUiJson.Number(point.ElementAtOrDefault(0)) ?? 0), MapUiJson.Number(point.ElementAtOrDefault(1)) ?? double.NaN))
                            .ToList())).ToList())).ToList(),
                MapUiJson.Objects(view["selectedCharted"])
                    .GroupBy(entry => MapUiJson.Text(entry["field"]))
                    .ToDictionary(group => group.Key, group => Ui.Int(group.First()["chart"])))
            : null;

    private static IReadOnlyList<string> Strings(JsonNode? node) =>
        node is JsonArray array ? array.Select(MapUiJson.Text).ToList() : [];

    internal static bool Chartable(string type) => !type.StartsWith("char", StringComparison.Ordinal);

    internal static bool ToggleEnabled(InspectorCharts? charts, string field, string type, int chart) =>
        charts?.Charted.TryGetValue(field, out var on) == true
            ? on == chart
            : Chartable(type) && (charts?.Charts.ElementAtOrDefault(chart)?.Room ?? true);

    internal static void Toggle(int chart, string field, bool on) =>
        Lane.Run(() => Bridge.Invoke(on ? "mavlinkInspector.chart.add" : "mavlinkInspector.chart.remove",
            new JsonArray(JsonValue.Create(chart), JsonValue.Create(field))));
}

internal sealed class InspectorChart : UserControl
{
    private const double PlotHeight = 220;
    private const double TickGap = 4;
    private const double TimeBand = 18;
    private const int ValueTicks = 4;
    private const int TimeTicks = 3;

    private static readonly string[] Series =
        ["AccentBrush", "AccentTextBrush", "SuccessBrush", "TextSecondaryBrush", "CriticalBrush", "CautionBrush"];

    private readonly int _index;
    private readonly DropDownButton _scale = Ui.Picker("Time scale");
    private readonly DropDownButton _range = Ui.Picker("Value range");
    private readonly ItemsControl _layer = Ui.Layer([]);
    private readonly ItemsControl _legend = Ui.Line([], 16);
    private string _menus = "";
    private ChartModel? _model;

    public InspectorChart(int index)
    {
        _index = index;
        Visibility = Visibility.Collapsed;
        var plot = new Grid { Height = PlotHeight, Children = { _layer } };
        plot.SizeChanged += (_, _) => Draw();
        Content = new StackPanel
        {
            Spacing = 8,
            Padding = new Thickness(0, 12, 0, 0),
            Children =
            {
                new StackPanel
                {
                    Orientation = Orientation.Horizontal,
                    Spacing = 8,
                    Children = { Ui.Text($"Chart {index + 1}", semibold: true), _scale, _range },
                },
                Ui.Card(new StackPanel
                {
                    Spacing = 8,
                    Children = { plot, _legend.With(legend => legend.HorizontalAlignment = HorizontalAlignment.Center) },
                }, "LayerBrush"),
            },
        };
    }

    internal void Update(InspectorCharts charts)
    {
        _model = charts.Charts.ElementAtOrDefault(_index) is { Plots.Count: > 0 } model ? model : null;
        Visibility = Ui.Shown(_model != null);
        if (_model is not { } shown) return;
        var menus = string.Join("|", charts.TimeScales.Concat(charts.Ranges));
        if (menus != _menus)
        {
            _menus = menus;
            _scale.Flyout = Ui.Menu(charts.TimeScales.Select((label, at) => (label, (Action)(() => Pick("mavlinkInspector.chart.rangeX", at)))));
            _range.Flyout = Ui.Menu(charts.Ranges.Select((label, at) => (label, (Action)(() => Pick("mavlinkInspector.chart.rangeY", at)))));
        }
        _scale.Content = $"Scale: {charts.TimeScales.ElementAtOrDefault(shown.RangeX) ?? ""}";
        _range.Content = $"Range: {charts.Ranges.ElementAtOrDefault(shown.RangeY) ?? ""}";
        _legend.Fill(shown.Plots.Select(Legend));
        Draw();
    }

    private void Pick(string path, int chosen) =>
        Ui.Off(() => Bridge.Invoke(path, new JsonArray(JsonValue.Create(_index), JsonValue.Create(chosen))));

    private static string Colour(ChartPlot plot) => Series[Math.Abs(plot.Colour) % Series.Length];

    private static UIElement Legend(ChartPlot plot) => new StackPanel
    {
        Orientation = Orientation.Horizontal,
        Spacing = 6,
        Children =
        {
            new Ellipse { Width = 8, Height = 8, Fill = Ui.Token(Colour(plot)), VerticalAlignment = VerticalAlignment.Center },
            Ui.Caption(string.Join("  ", new[] { plot.Label, Latest(plot.Points) }.OfType<string>()), "TextPrimaryBrush"),
        },
    };

    internal static string? Latest(IReadOnlyList<(long Age, double Value)> points) =>
        points.Count == 0 ? null : Format(points.MinBy(point => point.Age).Value);

    private static string Format(double value) => value.ToString("G4", CultureInfo.InvariantCulture);

    internal static Point ChartPoint(long age, double value, long windowMs, double low, double high) => new(
        1 - (double)age / Math.Max(windowMs, 1),
        1 - Math.Clamp((value - low) / (high - low > 0 ? high - low : 1), 0, 1));

    private void Draw()
    {
        var width = _layer.ActualWidth;
        if (_model is not { YMin: double low, YMax: double high } model)
        {
            _layer.Fill([]);
            return;
        }
        var plotHeight = PlotHeight - TimeBand;
        var values = Enumerable.Range(0, ValueTicks + 1).Select(step => (
            Y: (1 - (double)step / ValueTicks) * plotHeight,
            Text: Tick(Format(low + (high - low) * step / ValueTicks), TextAlignment.Right))).ToList();
        var gutter = values.Max(tick => Measured(tick.Text)) + TickGap * 2;
        var plotWidth = width - gutter;
        if (plotWidth <= 0)
        {
            _layer.Fill([]);
            return;
        }
        var now = DateTime.Now;
        var times = Enumerable.Range(0, TimeTicks + 1).Select(step => (
            X: gutter + (double)step / TimeTicks * plotWidth,
            Text: Tick(now.AddMilliseconds(-(1 - (double)step / TimeTicks) * model.WindowMs).ToString("mm:ss", CultureInfo.InvariantCulture), TextAlignment.Center)));
        var grid = Ui.Segments(values.Select(tick => (new Point(gutter, tick.Y), new Point(width, tick.Y))), Ui.Token("DividerBrush"), 1);
        var valueLabels = values.Select(tick => (UIElement)tick.Text.With(text =>
        {
            text.Width = gutter - TickGap;
            text.Margin = new Thickness(0, Math.Clamp(tick.Y - 8, 0, plotHeight - 16), 0, 0);
        }));
        var timeLabels = times.Select(tick => (UIElement)tick.Text.With(text =>
        {
            text.Width = 48;
            text.Margin = new Thickness(Math.Clamp(tick.X - 24, gutter, width - 48), plotHeight + 2, 0, 0);
        }));
        var lines = model.Plots.Select(plot => (UIElement?)Ui.Stroke(
            plot.Points
                .Where(point => double.IsFinite(point.Value))
                .Select(point => ChartPoint(point.Age, point.Value, model.WindowMs, low, high))
                .Where(at => at.X >= 0)
                .Select(at => new Point(gutter + at.X * plotWidth, at.Y * plotHeight)),
            Ui.Token(Colour(plot)), 2));
        _layer.Fill(new UIElement?[] { grid }.Concat(valueLabels).Concat(timeLabels).Concat(lines));
    }

    private static double Measured(TextBlock text)
    {
        text.Measure(new Size(double.PositiveInfinity, double.PositiveInfinity));
        return text.DesiredSize.Width;
    }

    private static TextBlock Tick(string label, TextAlignment alignment) => new()
    {
        Text = label,
        FontFamily = Ui.Mono,
        FontSize = Ui.Space("FontSizeCaption"),
        Foreground = Ui.Token("TextTertiaryBrush"),
        TextAlignment = alignment,
        HorizontalAlignment = HorizontalAlignment.Left,
        VerticalAlignment = VerticalAlignment.Top,
    };
}
