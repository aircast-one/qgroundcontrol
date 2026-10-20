using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Forms;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Shapes;
using Windows.Foundation;

namespace AircastQGC.Controls;

internal sealed record TapeReading(double Heading, double? NextWaypoint, double? Home)
{
    public static TapeReading? Of(JsonObject view) =>
        MapUiJson.Flag(view["available"])
            ? new TapeReading(
                MapUiJson.Number(view["heading"]) ?? 0,
                MapUiJson.Number(view["headingToNextWaypoint"]),
                MapUiJson.Number(view["homeBearing"]) ?? MapUiJson.Number(view["headingToHome"]))
            : null;
}

internal static class TapeMath
{
    public const double Span = 120;
    public const int TickStep = 10;
    public const int LabelStep = 30;
    private const double ReadoutClearance = 15;
    private const double EdgeMargin = 5;
    public const double EdgeReach = Span / 2 - EdgeMargin;

    private static readonly IReadOnlyDictionary<int, string> Cardinals = new Dictionary<int, string>
    {
        [0] = "N",
        [90] = "E",
        [180] = "S",
        [270] = "W",
    };

    public static double Relative(double bearing, double heading) => ((bearing - heading) % 360 + 540) % 360 - 180;

    public static double X(double relative, double width) => width / 2 + relative / Span * width;

    public static double Clamped(double bearing, double heading, double width) =>
        X(Math.Clamp(Relative(bearing, heading), -EdgeReach, EdgeReach), width);

    public static string Label(int degrees) =>
        Cardinals.TryGetValue(Wrap(degrees), out var cardinal) ? cardinal : Wrap(degrees).ToString(CultureInfo.InvariantCulture);

    public static IReadOnlyList<int> Ticks(double heading)
    {
        var first = (int)(Math.Ceiling((heading - Span / 2) / TickStep) * TickStep);
        var last = (int)(Math.Floor((heading + Span / 2) / TickStep) * TickStep);
        return Enumerable.Range(0, (last - first) / TickStep + 1).Select(step => first + step * TickStep).ToList();
    }

    public static bool LabelShown(double relative) => Math.Abs(relative) > ReadoutClearance && Math.Abs(relative) < EdgeReach;

    public static string Readout(double heading) => $"{Wrap((int)Math.Round(heading))}°";

    private static int Wrap(int degrees) => ((degrees % 360) + 360) % 360;
}

public sealed class HeadingTape : UserControl
{
    private const double TapeWidth = 320;
    private const double TapeHeight = 40;
    private const double Pointer = 6;
    private const double PointerWing = 5;
    private const double LabelTick = 10;
    private const double MinorTick = 5;
    private const double Gap = 2;
    private const double HomeRadius = 6;
    private const double EdgeFade = 0.6;
    private const double BackdropOpacity = 0.65;

    private readonly Canvas _canvas = new() { Width = TapeWidth, Height = TapeHeight, IsHitTestVisible = false };

    public HeadingTape()
    {
        Content = new Grid
        {
            Width = TapeWidth,
            Height = TapeHeight,
            Children =
            {
                new Border
                {
                    Background = FormLook.Brush("BgSecondaryBrush"),
                    Opacity = BackdropOpacity,
                    CornerRadius = FormLook.Radius("RadiusControl"),
                },
                _canvas,
            },
        };
        IsHitTestVisible = false;
        Visibility = Visibility.Collapsed;
        Live.Watch(this, TelemetryPanel.AttitudeView, view => Draw(TapeReading.Of(view)));
    }

    private void Draw(TapeReading? reading)
    {
        Visibility = FormLook.Shown(reading != null);
        if (reading == null) return;
        var readout = TapeMath.Readout(reading.Heading);
        AutomationProperties.SetName(this, $"Heading {readout}");
        var line = Pointer + Gap;
        var labelTop = line + LabelTick + Gap;
        var ink = FormLook.Brush("TextPrimaryBrush");
        var ticks = TapeMath.Ticks(reading.Heading).SelectMany(degrees =>
        {
            var relative = TapeMath.Relative(degrees, reading.Heading);
            var x = TapeMath.X(relative, TapeWidth);
            var fade = 1 - Math.Abs(relative) / (TapeMath.Span / 2) * EdgeFade;
            var labelled = degrees % TapeMath.LabelStep == 0;
            var tick = new Line { X1 = x, X2 = x, Y1 = line, Y2 = line + (labelled ? LabelTick : MinorTick), Stroke = ink, StrokeThickness = 1.5, Opacity = fade };
            return labelled && TapeMath.LabelShown(relative)
                ? new UIElement[] { tick, Centred(Label(TapeMath.Label(degrees), "FontSizeCaption", ink, fade), x, labelTop) }
                : [tick];
        });
        var mission = reading.NextWaypoint is double next
            ? new UIElement[] { Triangle(TapeMath.Clamped(next, reading.Heading, TapeWidth), FormLook.Brush("MapPathBrush")) }
            : [];
        var home = reading.Home is double bearing ? Home(TapeMath.Clamped(bearing, reading.Heading, TapeWidth), line) : [];
        var centre = new UIElement[]
        {
            Triangle(TapeWidth / 2, ink),
            Centred(Label(readout, "FontSizeBody", ink, 1, FontWeights.Bold), TapeWidth / 2, labelTop - Gap - 2),
        };
        FormLook.Fill(_canvas, ticks.Concat(mission).Concat(home).Concat(centre));
    }

    private static IEnumerable<UIElement> Home(double x, double y)
    {
        var disc = new Ellipse { Width = HomeRadius * 2, Height = HomeRadius * 2, Fill = FormLook.Brush("SuccessBrush") };
        Canvas.SetLeft(disc, x - HomeRadius);
        Canvas.SetTop(disc, y - HomeRadius);
        var letter = new TextBlock
        {
            Text = "H",
            Width = HomeRadius * 2,
            TextAlignment = TextAlignment.Center,
            FontFamily = PreflightLook.Font,
            FontSize = 9,
            FontWeight = FontWeights.Bold,
            Foreground = FormLook.Brush("AccentOnBrush"),
        };
        Canvas.SetLeft(letter, x - HomeRadius);
        Canvas.SetTop(letter, y - HomeRadius);
        return [disc, letter];
    }

    private static Polygon Triangle(double x, Brush fill) => new()
    {
        Points = new PointCollection { new Point(x - PointerWing, 0), new Point(x + PointerWing, 0), new Point(x, Pointer) },
        Fill = fill,
    };

    private static TextBlock Label(string text, string size, Brush ink, double opacity, Windows.UI.Text.FontWeight? weight = null) => new()
    {
        Text = text,
        FontFamily = PreflightLook.Font,
        FontSize = FormLook.Size(size),
        FontWeight = weight ?? FontWeights.SemiBold,
        Foreground = ink,
        Opacity = opacity,
    };

    private static TextBlock Centred(TextBlock text, double x, double top)
    {
        text.Measure(new Size(double.PositiveInfinity, double.PositiveInfinity));
        Canvas.SetLeft(text, x - text.DesiredSize.Width / 2);
        Canvas.SetTop(text, top);
        return text;
    }
}
