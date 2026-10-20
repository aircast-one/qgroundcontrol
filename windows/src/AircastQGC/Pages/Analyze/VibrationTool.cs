using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;
using AircastQGC.Core;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Windows.Foundation;

namespace AircastQGC.Pages.Analyze;

internal sealed record VibrationAxis(string Axis, double? Value, double Fraction, string? Severity);

internal sealed record VibrationReading(string Units, double ScaleMaximum, double WarningLevel, double DangerLevel,
    IReadOnlyList<VibrationAxis> Axes, IReadOnlyList<int> ClipCounts);

internal sealed class VibrationTool : UserControl
{
    internal const string VibrationPath = "view.vibration";
    private readonly string _client = $"analyzeTool.vibration.{System.Guid.NewGuid():N}";
    private const string ConnectPrompt = "Connect a vehicle from the Fly view to see its vibration levels.";
    private const double BarHeight = 240;
    private const double BarWidth = 56;
    private const double AxisColumn = 120;
    private const double ScaleWidth = 32;

    private string _signature = "";

    public VibrationTool()
    {
        Loaded += (_, _) =>
        {
            BridgeWatch.Watch(_client, [VibrationPath], Apply);
            Apply(Bridge.Group(VibrationPath));
        };
        Unloaded += (_, _) => BridgeWatch.Stop(_client);
    }

    private void Apply(JsonObject view)
    {
        var signature = view.ToJsonString();
        if (signature == _signature) return;
        _signature = signature;
        var reading = Reading(view);
        Content = EmptyState(view, reading) is { } empty
            ? Ui.EmptyState("\uE877", empty.Title, empty.Body)
            : reading is null ? null : Body(reading);
    }

    internal static VibrationReading? Reading(JsonObject view) =>
        !Ui.IsEmpty(view) && MapUiJson.Flag(view["available"]) && view["axes"] is JsonArray axes
            ? new VibrationReading(
                MapUiJson.Text(view["units"]),
                MapUiJson.Number(view["scaleMaximum"]) ?? 0,
                MapUiJson.Number(view["warningLevel"]) ?? 0,
                MapUiJson.Number(view["dangerLevel"]) ?? 0,
                axes.OfType<JsonObject>().Select(axis => new VibrationAxis(
                    MapUiJson.Text(axis["label"]) is { Length: > 0 } label ? label : MapUiJson.Text(axis["axis"]).ToUpperInvariant(),
                    MapUiJson.Number(axis["value"]),
                    MapUiJson.Number(axis["fraction"]) ?? 0,
                    MapUiJson.Text(axis["severity"]) is { Length: > 0 } severity ? severity : null)).ToList(),
                view["clipCounts"] is JsonArray clips ? clips.Select(clip => Ui.Int(clip)).ToList() : [])
            : null;

    internal static string? Glance(VibrationReading reading)
    {
        var parts = reading.Axes
            .Where(axis => axis.Value is double value && double.IsFinite(value))
            .Select(axis => $"{axis.Axis} {Math.Round(axis.Value!.Value, MidpointRounding.AwayFromZero)}")
            .ToList();
        return parts.Count == 0 ? null : string.Join(" · ", parts) + (reading.Units.Length > 0 ? $" {reading.Units}" : "");
    }

    internal static string? Worst(VibrationReading reading) =>
        new[] { "danger", "warning", "normal" }.FirstOrDefault(level => reading.Axes.Any(axis => axis.Severity == level));

    internal static string SeverityLabel(string? severity) => severity switch
    {
        "danger" => "Unsafe",
        "warning" => "Watch",
        "normal" => "Healthy",
        _ => "",
    };

    internal static string? Caveat(JsonObject view) =>
        Ui.IsEmpty(view) || MapUiJson.Flag(view["available"]) ? null
        : MapUiJson.Text(view["silentReason"]) switch
        {
            "notReported" => "This vehicle is not reporting vibration",
            var token when string.IsNullOrWhiteSpace(token) => "This vehicle is reporting only some vibration axes",
            _ => null,
        };

    private static (string Title, string Body)? EmptyState(JsonObject view, VibrationReading? reading) =>
        Silent(view) ?? (reading is null ? Partly(view) : null);

    private static (string Title, string Body)? Silent(JsonObject view)
    {
        if (Ui.IsEmpty(view)) return ("No vehicle connected", ConnectPrompt);
        var token = MapUiJson.Text(view["silentReason"]);
        if (string.IsNullOrWhiteSpace(token)) return null;
        var title = MapUiJson.Text(view["silentText"]) is { Length: > 0 } text ? text : "Vibration is not being reported";
        return (title, token switch
        {
            "noVehicle" => ConnectPrompt,
            "notReported" => "The autopilot has not sent a VIBRATION message. Not all firmware and airframes publish one.",
            _ => "Nothing has reported a vibration level on this vehicle.",
        });
    }

    private static (string Title, string Body) Partly(JsonObject view) =>
        MapUiJson.Flag(view["connected"])
            ? ("Only some vibration axes are reported", "This screen draws all three axes together, and this vehicle is reporting some of them.")
            : ("No vehicle connected", ConnectPrompt);

    private static string Verdict(VibrationReading reading)
    {
        var worst = Worst(reading);
        var axes = string.Join(" and ", reading.Axes.Where(axis => axis.Severity == worst).Select(axis => axis.Axis));
        var level = worst switch
        {
            "danger" => $"Vibration on {axes} is over the unsafe limit of {(int)reading.DangerLevel}.",
            "warning" => $"Vibration on {axes} is above {(int)reading.WarningLevel}; watch it.",
            _ => "Vibration is well under the limit.",
        };
        var clips = reading.ClipCounts.Sum();
        return $"{level} {(clips == 0 ? "No clipping." : $"The accelerometers clipped {clips} times; expect zero in flight.")}";
    }

    internal static string ClipText(IReadOnlyList<int> counts) =>
        string.Join(" · ", counts.Select((count, index) => $"Accel {index + 1}: {count}"));

    private static string SeverityBrush(string? severity) => severity switch
    {
        "danger" => "CriticalBrush",
        "warning" => "CautionBrush",
        "normal" => "AccentBrush",
        _ => "ControlStrokeBrush",
    };

    private static string Band(VibrationReading reading)
    {
        var warn = (int)reading.WarningLevel;
        var danger = (int)reading.DangerLevel;
        return $"Under {warn} healthy · {warn}-{danger} watch · over {danger} unsafe";
    }

    private static UIElement Body(VibrationReading reading)
    {
        var (background, foreground, glyph) = Worst(reading) switch
        {
            "danger" => ("CriticalBgBrush", "CriticalBrush", "\uE783"),
            "warning" => ("CautionBgBrush", "CautionBrush", "\uE7BA"),
            _ => ("SuccessBgBrush", "SuccessBrush", "\uE73E"),
        };
        return new ScrollViewer
        {
            Content = new StackPanel
            {
                Padding = new Thickness(24, 0, 24, 24),
                Spacing = 12,
                Children =
                {
                    Ui.Card(new StackPanel
                    {
                        Spacing = 12,
                        Children =
                        {
                            Ui.Text(reading.Units.Length == 0 ? "Vibration" : $"Vibration ({reading.Units})", semibold: true),
                            new StackPanel
                            {
                                Orientation = Orientation.Horizontal,
                                HorizontalAlignment = HorizontalAlignment.Center,
                                Children = { ScaleAxis(reading), Ui.Line(reading.Axes.Select(Bar), 0) },
                            },
                            new StackPanel
                            {
                                Orientation = Orientation.Horizontal,
                                HorizontalAlignment = HorizontalAlignment.Center,
                                Margin = new Thickness(ScaleWidth, 0, 0, 0),
                                Children = { Ui.Line(reading.Axes.Select(Readout), 0) },
                            },
                            Ui.Caption(Band(reading)),
                        },
                    }),
                    Ui.Card(new StackPanel
                    {
                        Orientation = Orientation.Horizontal,
                        Spacing = 12,
                        Children =
                        {
                            Ui.Glyph(glyph, 20, foreground),
                            Ui.Text(Verdict(reading), brush: foreground).With(text => text.MaxWidth = 560),
                        },
                    }, background, background),
                    new Grid
                    {
                        Padding = new Thickness(0, 4, 0, 4),
                        ColumnDefinitions =
                        {
                            new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) },
                            new ColumnDefinition { Width = GridLength.Auto },
                        },
                        Children =
                        {
                            Ui.Text("Clipping events"),
                            Ui.Text(ClipText(reading.ClipCounts), brush: "TextSecondaryBrush").With(text => Grid.SetColumn(text, 1)),
                        },
                    },
                },
            },
        };
    }

    private static UIElement ScaleAxis(VibrationReading reading)
    {
        var labels = new[] { reading.ScaleMaximum, reading.DangerLevel, reading.WarningLevel, 0.0 }
            .Select(level => ((int)level).ToString(CultureInfo.CurrentCulture))
            .ToList();
        var last = labels.Count - 1;
        return Ui.Layer(labels.Select((label, at) => Ui.Caption(label).With(text =>
        {
            text.HorizontalAlignment = HorizontalAlignment.Right;
            text.VerticalAlignment = at == last ? VerticalAlignment.Bottom : VerticalAlignment.Top;
            text.Margin = new Thickness(0, at == 0 || at == last ? 0 : at * BarHeight / last - 8, 4, 0);
        }))).With(layer =>
        {
            layer.Width = ScaleWidth;
            layer.Height = BarHeight;
        });
    }

    private static UIElement Bar(VibrationAxis axis) => new Grid
    {
        Width = AxisColumn,
        Height = BarHeight,
        Children =
        {
            new Grid
            {
                Width = BarWidth,
                CornerRadius = new CornerRadius(6),
                Background = Ui.Token("ControlBrush"),
                Children =
                {
                    new Border
                    {
                        VerticalAlignment = VerticalAlignment.Bottom,
                        Height = Math.Clamp(axis.Fraction, 0, 1) * BarHeight,
                        Background = Ui.Token(SeverityBrush(axis.Severity)),
                    },
                    (UIElement?)Ui.Segments(Enumerable.Range(0, 4).Select(step => step * (BarHeight - 1) / 3 + 0.5)
                        .Select(y => (new Point(0, y), new Point(BarWidth, y))), Ui.Token("TextTertiaryBrush"), 1) ?? new Grid(),
                },
            },
        },
    };

    private static UIElement Readout(VibrationAxis axis) => new StackPanel
    {
        Width = AxisColumn,
        Children =
        {
            Ui.Centered(Ui.Text(axis.Value is double value ? value.ToString("0.0", CultureInfo.InvariantCulture) : "--", "FontSizeBodyLarge", semibold: true)),
            Ui.Centered(Ui.Text(axis.Axis)),
            Ui.Centered(Ui.Caption(SeverityLabel(axis.Severity))),
        },
    };
}
