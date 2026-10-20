using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Shapes;
using Windows.Foundation;

namespace AircastQGC.Controls;

public sealed partial class ObstacleArc : UserControl
{
    private const double ArcSize = 48;
    private const double CardAlpha = 0.8;
    private const double FadedAlpha = 0.45;
    private const double RimAlpha = 0.25;
    private const double HubAlpha = 0.6;

    private readonly Canvas _canvas = new() { Width = ArcSize, Height = ArcSize, IsHitTestVisible = false };
    private readonly TextBlock _lastSeen = FormLook.Caption("");

    public ObstacleArc()
    {
        _lastSeen.HorizontalAlignment = HorizontalAlignment.Center;
        Content = new Border
        {
            Background = OverlayInk.Brush("CardBrush", CardAlpha),
            CornerRadius = FormLook.Radius("RadiusControl"),
            Padding = new Thickness(6),
            Child = FormLook.Stack(2, _canvas, _lastSeen),
        };
        Visibility = Visibility.Collapsed;
        AutomationProperties.SetName(this, "Obstacle distance");
        Live.Watch(this, ObstacleModel.View, Render);
    }

    internal void Render(JsonObject view)
    {
        var ring = ObstacleModel.Ring(view);
        Visibility = FormLook.Shown(ring is not null);
        FormLook.Say(_lastSeen, ring?.Stale == true ? "Last seen" : null);
        FormLook.Fill(_canvas, ring is null ? [] : Shapes(ring, XamlRoot?.RasterizationScale ?? 1));
    }

    private static IEnumerable<UIElement> Shapes(ObstacleRing ring, double scale)
    {
        double Pixels(double devicePixels) => devicePixels / scale;
        var full = ArcSize / 2;
        var centre = new Point(full, full);
        var sweep = ObstacleModel.ArcSweep(ring.IncrementDegrees);
        var rim = new Ellipse { Width = ArcSize, Height = ArcSize, Stroke = OverlayInk.Brush("TextSecondaryBrush", RimAlpha), StrokeThickness = Pixels(2) };
        var hub = OverlayInk.Dot(centre, Pixels(3), OverlayInk.Brush("TextSecondaryBrush", HubAlpha));
        var arcs = ring.Samples.Select(sample =>
        {
            var near = ObstacleModel.SampleIsClose(sample.Metres, ring.FloorMetres);
            var tone = ObstacleModel.Tone(near, ring.Stale) switch
            {
                ArcTone.Alarm => OverlayInk.Brush("CriticalBrush"),
                ArcTone.Calm => OverlayInk.Brush("TextSecondaryBrush"),
                _ => OverlayInk.Brush("TextSecondaryBrush", FadedAlpha),
            };
            var reach = full * ObstacleModel.ArcRadiusFraction(sample.Metres, ring.MaxMetres);
            return (UIElement)OverlayInk.Arc(centre, reach, sample.BearingDegrees - 90 - sweep / 2, sweep, tone, Pixels(near ? 10 : 6));
        });
        return new UIElement[] { rim, hub }.Concat(arcs).ToList();
    }
}
