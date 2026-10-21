using System;
using System.Collections.Generic;
using System.Linq;
using Microsoft.UI;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Automation.Peers;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Shapes;
using Windows.Foundation;
using Windows.UI;

namespace AircastQGC.Controls;

internal static class OverlayInk
{
    private const string Edge = "VideoEdgeBrush";
    private static readonly (double X, double Y)[] OutlineOffsets = [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, 1), (-1, 1), (1, -1)];

    private static Color Colour(string token) =>
        PreflightLook.Token(token) is SolidColorBrush solid ? solid.Color : Colors.Transparent;

    public static Brush Brush(string token, double alpha = 1) =>
        alpha >= 1 ? PreflightLook.Token(token) : new SolidColorBrush(Colour(token)) { Opacity = alpha };

    public static Microsoft.UI.Xaml.Shapes.Path Radial(Microsoft.UI.Xaml.Shapes.Path path, Point centre, double radiusX, double radiusY, double from, double to, IReadOnlyList<ShadeStop> stops)
    {
        var bounds = path.Data.Bounds;
        var local = new Point(centre.X - bounds.X, centre.Y - bounds.Y);
        var brush = new RadialGradientBrush
        {
            MappingMode = BrushMappingMode.Absolute,
            Center = local,
            GradientOrigin = local,
            RadiusX = radiusX,
            RadiusY = radiusY,
        };
        Color Faded(ShadeStop stop)
        {
            var colour = Colour(stop.Brush);
            return ColorHelper.FromArgb((byte)Math.Round(255 * stop.Alpha), colour.R, colour.G, colour.B);
        }
        stops.Select(stop => new GradientStop { Offset = ObstacleModel.StopOffset(stop.At, from, to), Color = Faded(stop) })
            .ToList().ForEach(brush.GradientStops.Add);
        path.Fill = brush;
        return path;
    }

    public static Line Line(Point from, Point to, Brush stroke, double width) => new()
    {
        X1 = from.X,
        Y1 = from.Y,
        X2 = to.X,
        Y2 = to.Y,
        Stroke = stroke,
        StrokeThickness = width,
        StrokeStartLineCap = PenLineCap.Round,
        StrokeEndLineCap = PenLineCap.Round,
    };

    public static Ellipse Dot(Point centre, double radius, Brush fill)
    {
        var dot = new Ellipse { Width = radius * 2, Height = radius * 2, Fill = fill };
        Canvas.SetLeft(dot, centre.X - radius);
        Canvas.SetTop(dot, centre.Y - radius);
        return dot;
    }

    public static Ellipse Ring(Point centre, double radius, Brush stroke, double width)
    {
        var ring = new Ellipse { Width = radius * 2, Height = radius * 2, Stroke = stroke, StrokeThickness = width };
        Canvas.SetLeft(ring, centre.X - radius);
        Canvas.SetTop(ring, centre.Y - radius);
        return ring;
    }

    public static Microsoft.UI.Xaml.Shapes.Path Figure(Point start, IEnumerable<PathSegment> segments, bool closed)
    {
        var figure = new PathFigure { StartPoint = start, IsClosed = closed, IsFilled = closed };
        segments.ToList().ForEach(figure.Segments.Add);
        var geometry = new PathGeometry();
        geometry.Figures.Add(figure);
        return new Microsoft.UI.Xaml.Shapes.Path { Data = geometry };
    }

    public static Microsoft.UI.Xaml.Shapes.Path Arc(Point centre, double radius, double startDegrees, double sweepDegrees, Brush stroke, double width)
    {
        Point On(double degrees) => new(centre.X + radius * Math.Cos(degrees * Math.PI / 180), centre.Y + radius * Math.Sin(degrees * Math.PI / 180));
        var arc = Figure(On(startDegrees), [new ArcSegment
        {
            Point = On(startDegrees + sweepDegrees),
            Size = new Size(radius, radius),
            SweepDirection = SweepDirection.Clockwise,
            IsLargeArc = sweepDegrees > 180,
        }], closed: false);
        arc.Stroke = stroke;
        arc.StrokeThickness = width;
        return arc;
    }

    public static IEnumerable<UIElement> Outlined(string text, Point topLeft, double size, string fill, double fillAlpha, double outline, double width = double.NaN, bool bold = true)
    {
        TextBlock Copy(Brush brush, double dx, double dy)
        {
            var block = new TextBlock
            {
                Text = text,
                FontFamily = PreflightLook.Font,
                FontSize = size,
                FontWeight = bold ? FontWeights.Bold : FontWeights.SemiBold,
                Foreground = brush,
                LineHeight = LineHeight(size),
                LineStackingStrategy = LineStackingStrategy.BlockLineHeight,
                TextAlignment = double.IsNaN(width) ? TextAlignment.Left : TextAlignment.Center,
                Width = width,
                IsHitTestVisible = false,
            };
            Canvas.SetLeft(block, topLeft.X + dx);
            Canvas.SetTop(block, topLeft.Y + dy);
            return block;
        }
        var edge = Brush(Edge);
        return OutlineOffsets.Select(offset => Hidden(Copy(edge, offset.X * outline, offset.Y * outline)))
            .Append(Copy(Brush(fill, fillAlpha), 0, 0));
    }

    public static double LineHeight(double size) => Math.Ceiling(size * 1.25);

    private static UIElement Hidden(UIElement element)
    {
        AutomationProperties.SetAccessibilityView(element, AccessibilityView.Raw);
        return element;
    }
}
