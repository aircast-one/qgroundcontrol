using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Pages.Analyze;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Shapes;
using Windows.Foundation;

namespace AircastQGC.Controls;

internal sealed class MapFleet(Canvas canvas)
{
    internal const string GcsPositionPath = "view.gcsPosition";
    private const double OtherSize = 28;
    private const double OtherDotSize = 12;
    private const double OperatorSize = 26;
    private const double LabelWidth = 120;
    private const double LabelOffset = 28;
    private const double StaleOpacity = 0.4;
    private const double TapRadius = 22;
    private const string OperatorGlyph = "\uE77B";

    private sealed record Mark(FrameworkElement View, Geo At, double OffsetY, int? Vehicle);

    private IReadOnlyList<Mark> _vehicles = [];
    private IReadOnlyList<Mark> _operator = [];
    private string _operatorKey = "";

    internal Geo? OperatorAt { get; private set; }

    internal void Fleet(JsonObject view, Func<Geo, Point> project)
    {
        var choices = VehicleChoices.Parse(view);
        var many = choices.Choices.Count > 1;
        _vehicles = choices.Choices
            .Where(choice => choice.At != null)
            .SelectMany(choice => VehicleMarks(choice, many))
            .ToList();
        Redraw(project);
    }

    internal void Operator(JsonObject view, Func<Geo, Point> project)
    {
        var usable = MapUiJson.Flag(view["usable"]);
        OperatorAt = usable ? MapOverlayJson.Plottable(view) : null;
        var heading = usable ? MapUiJson.Number(view["heading"]) : null;
        var key = $"{OperatorAt}|{heading}";
        if (key == _operatorKey) return;
        _operatorKey = key;
        _operator = OperatorAt is { } at ? [new Mark(OperatorMark(heading), at, 0, null)] : [];
        Redraw(project);
    }

    internal void Place(Func<Geo, Point> project) =>
        _vehicles.Concat(_operator).ToList().ForEach(mark =>
        {
            var point = project(mark.At);
            Canvas.SetLeft(mark.View, point.X - mark.View.Width / 2);
            Canvas.SetTop(mark.View, point.Y - mark.View.Height / 2 + mark.OffsetY);
        });

    internal int? VehicleAt(Point at, Func<Geo, Point> project) =>
        _vehicles
            .Where(mark => mark.Vehicle != null)
            .Select(mark => (mark.Vehicle, Point: project(mark.At)))
            .Select(hit => (hit.Vehicle, Distance: Math.Sqrt(Math.Pow(hit.Point.X - at.X, 2) + Math.Pow(hit.Point.Y - at.Y, 2))))
            .Where(hit => hit.Distance <= TapRadius)
            .OrderBy(hit => hit.Distance)
            .Select(hit => hit.Vehicle)
            .FirstOrDefault();

    private void Redraw(Func<Geo, Point> project)
    {
        canvas.Children.Clear();
        _vehicles.Concat(_operator).Select(mark => mark.View).ToList().ForEach(canvas.Children.Add);
        Place(project);
    }

    private static IEnumerable<Mark> VehicleMarks(VehicleChoice choice, bool many)
    {
        var at = choice.At!.Value;
        IEnumerable<Mark> glyph = choice.Active ? [] : [new Mark(OtherVehicle(choice), at, 0, choice.Id)];
        IEnumerable<Mark> label = many ? [new Mark(Label($"Vehicle {choice.Id}", choice.ContactLost), at, LabelOffset, null)] : [];
        return glyph.Concat(label);
    }

    private static FrameworkElement OtherVehicle(VehicleChoice choice) => new Grid
    {
        Width = OtherSize,
        Height = OtherSize,
        Opacity = choice.ContactLost ? StaleOpacity : 1,
        Children =
        {
            new Ellipse
            {
                Width = OtherDotSize,
                Height = OtherDotSize,
                Fill = Ui.Token("TextSecondaryBrush"),
                Stroke = Ui.Token("AccentOnBrush"),
                StrokeThickness = 2,
                Visibility = Ui.Shown(!choice.Heading.HasValue),
            },
            FleetLook.Arrow(OtherSize, "TextSecondaryBrush", "AccentOnBrush", choice.Heading),
        },
    };

    private static FrameworkElement Label(string text, bool stale) => new TextBlock
    {
        Text = text,
        Width = LabelWidth,
        Height = 16,
        TextAlignment = TextAlignment.Center,
        FontFamily = PreflightLook.Font,
        FontSize = PreflightLook.Size("FontSizeCaption"),
        Foreground = Ui.Token("TextPrimaryBrush"),
        Opacity = stale ? StaleOpacity : 1,
    };

    private static FrameworkElement OperatorMark(double? heading) => new Grid
    {
        Width = OperatorSize * 2,
        Height = OperatorSize * 2,
        Children =
        {
            FleetLook.Arrow(OperatorSize * 2, "SuccessBrush", "AccentOnBrush", heading).With(arrow => arrow.Opacity = 0.6),
            Ui.Circle(OperatorSize, "SuccessBrush", Ui.Glyph(OperatorGlyph, 14, "AccentOnBrush").With(icon => icon.HorizontalAlignment = HorizontalAlignment.Center))
                .With(disc =>
                {
                    disc.HorizontalAlignment = HorizontalAlignment.Center;
                    disc.BorderBrush = Ui.Token("AccentOnBrush");
                    disc.BorderThickness = new Thickness(2);
                }),
        },
    }.Named("Ground station position");
}
