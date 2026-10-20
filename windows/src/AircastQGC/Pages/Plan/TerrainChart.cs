using System;
using System.Collections.Generic;
using System.Linq;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Shapes;
using Windows.Foundation;

namespace AircastQGC.Pages.Plan;

internal static class TerrainChart
{
    private const double ChartHeight = 132;
    private const double MarkerSize = 18;
    private const double MinimumSpan = 1;

    internal static FrameworkElement View(TerrainProfile profile, string provider, int? selected, Action<int> pick)
    {
        var chart = new Canvas { Height = ChartHeight, Background = Token("CardBrush") };
        chart.SizeChanged += (_, e) => Draw(chart, profile, selected, pick, e.NewSize.Width);
        var header = PlanFacts.Pair(
            PlanFacts.Note(string.Join(" · ", new[] { profile.HeightHeader, profile.Lowest, profile.Highest }.Where(text => text.Length > 0))),
            PlanFacts.Note(profile.DistanceText));
        var panel = new StackPanel { Spacing = 4, Children = { header, new Border { CornerRadius = new CornerRadius(4), Child = chart } } };
        if (profile.Points.Count(point => point.Terrain != null) < 2) panel.Children.Add(PlanFacts.Note("No terrain data for this route yet."));
        if (provider.Length > 0) panel.Children.Add(PlanFacts.Note($"Powered by {provider}"));
        return panel;
    }

    private static void Draw(Canvas chart, TerrainProfile profile, int? selected, Action<int> pick, double width)
    {
        chart.Children.Clear();
        if (!profile.Drawable || width <= 0) return;
        var distance = Math.Max(profile.Points[^1].Distance, MinimumSpan);
        var heights = profile.Points.SelectMany(point => new[] { point.Planned, point.Terrain ?? point.Planned }).Where(double.IsFinite).ToList();
        var (low, high) = profile.Band ?? (heights.DefaultIfEmpty(0).Min(), heights.DefaultIfEmpty(0).Max());
        var span = Math.Max(high - low, MinimumSpan);
        var top = MarkerSize + 4;
        Point At(double along, double height) => new(along / distance * width, top + (1 - (height - low) / span) * (ChartHeight - top - 4));

        var ground = profile.Points.Where(point => point.Terrain != null).ToList();
        if (ground.Count >= 2)
        {
            var outline = ground.Select(point => At(point.Distance, point.Terrain!.Value))
                .Append(new Point(ground[^1].Distance / distance * width, ChartHeight))
                .Append(new Point(ground[0].Distance / distance * width, ChartHeight));
            chart.Children.Add(new Polygon { Points = Collect(outline), Fill = Token("MapRoadBrush"), Stroke = Token("TextTertiaryBrush"), StrokeThickness = 1 });
        }

        var planned = profile.Points.Where(point => double.IsFinite(point.Planned)).ToList();
        chart.Children.Add(new Polyline { Points = Collect(planned.Select(point => At(point.Distance, point.Planned))), Stroke = Token("MapPathBrush"), StrokeThickness = 2 });
        planned.Zip(planned.Skip(1))
            .Where(leg => leg.First.Collision || leg.Second.Collision)
            .Select(leg => new Microsoft.UI.Xaml.Shapes.Line
            {
                X1 = At(leg.First.Distance, leg.First.Planned).X,
                Y1 = At(leg.First.Distance, leg.First.Planned).Y,
                X2 = At(leg.Second.Distance, leg.Second.Planned).X,
                Y2 = At(leg.Second.Distance, leg.Second.Planned).Y,
                Stroke = Token("CriticalBrush"),
                StrokeThickness = 3,
            })
            .ToList()
            .ForEach(chart.Children.Add);

        profile.Markers.Select(marker => Marker(marker, marker.Sequence == selected, pick, marker.Distance / distance * width))
            .ToList()
            .ForEach(chart.Children.Add);
    }

    private static FrameworkElement Marker(ProfileMarker marker, bool chosen, Action<int> pick, double x)
    {
        var view = new Grid
        {
            Width = MarkerSize,
            Height = MarkerSize,
            Children =
            {
                new Ellipse { Fill = Token("MapPathBrush"), Stroke = chosen ? Token("TextPrimaryBrush") : null, StrokeThickness = 2 },
                new TextBlock
                {
                    Text = marker.Label,
                    FontSize = 10,
                    Foreground = Token("AccentOnBrush"),
                    HorizontalAlignment = HorizontalAlignment.Center,
                    VerticalAlignment = VerticalAlignment.Center,
                },
            },
        };
        view.Tapped += (_, _) => pick(marker.Sequence);
        Canvas.SetLeft(view, Math.Max(0, x - MarkerSize / 2));
        Canvas.SetTop(view, 2);
        return view;
    }

    private static PointCollection Collect(IEnumerable<Point> points)
    {
        var collection = new PointCollection();
        points.ToList().ForEach(collection.Add);
        return collection;
    }

    private static Brush Token(string key) => (Brush)Application.Current.Resources[key];
}
