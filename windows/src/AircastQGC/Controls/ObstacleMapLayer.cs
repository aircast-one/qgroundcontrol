using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Windows.Foundation;

namespace AircastQGC.Controls;

public sealed partial class ObstacleMapLayer : UserControl
{
    private const string CoordinatePath = "vehicle.coordinate";
    private const string HeadingPath = "vehicle.heading";
    private const double MapTextSize = 22;
    private const double MapTextAlpha = 0.9;

    private readonly string _client = $"obstacle-map.{Guid.NewGuid():N}";
    private readonly Canvas _canvas = new();
    private MapView? _map;
    private ObstacleDrawing? _drawing;
    private Geo? _vehicle;
    private double? _heading;
    private bool _following;
    private bool _showText = true;

    public ObstacleMapLayer()
    {
        IsHitTestVisible = false;
        Content = _canvas;
        SizeChanged += (_, e) =>
        {
            _canvas.Clip = new RectangleGeometry { Rect = new Rect(0, 0, e.NewSize.Width, e.NewSize.Height) };
            Draw();
        };
        Loaded += (_, _) => Start();
        Unloaded += (_, _) => Halt();
    }

    public bool ShowText
    {
        get => _showText;
        set
        {
            _showText = value;
            Draw();
        }
    }

    public void Attach(MapView map)
    {
        _map = map;
        map.ViewChanged += (_, _) => Draw();
    }

    private void Start()
    {
        BridgeWatch.Watch($"{_client}.view", [ObstacleModel.View], Render);
        Render(Bridge.Get(ObstacleModel.View));
    }

    private void Halt()
    {
        if (IsLoaded) return;
        BridgeWatch.Stop($"{_client}.view");
        Follow(false);
    }

    internal void Render(JsonObject view)
    {
        _drawing = ObstacleModel.Drawing(view);
        Follow(_drawing is not null);
        Draw();
    }

    private void Follow(bool on)
    {
        if (on == _following) return;
        _following = on;
        if (!on)
        {
            BridgeWatch.Stop($"{_client}.pose");
            return;
        }
        BridgeWatch.Watch($"{_client}.pose", [CoordinatePath, HeadingPath], Pose);
        Pose(Bridge.Get(CoordinatePath));
        Pose(Bridge.Get(HeadingPath));
    }

    private void Pose(JsonObject view)
    {
        if (view.ContainsKey("latitude"))
            _vehicle = VideoJson.Flag(view, "valid", true)
                && VideoJson.Number(view["latitude"]) is double latitude
                && VideoJson.Number(view["longitude"]) is double longitude
                && (latitude != 0 || longitude != 0)
                    ? new Geo(latitude, longitude)
                    : null;
        else _heading = VideoJson.Number(view["value"]);
        Draw();
    }

    private void Draw() => FormLook.Fill(_canvas, Shapes());

    private IEnumerable<UIElement> Shapes()
    {
        if (_map is null || _drawing is not { } drawing || _vehicle is not { } vehicle || _heading is not double heading || ActualHeight <= 0) return [];
        var at = _map.ScreenOf(vehicle);
        var centre = new Spot(at.X, at.Y);
        var metres = ObstacleModel.MetresAcross(vehicle.Latitude, _map.Zoom, ObstacleModel.ProbePixels);
        var shape = ObstacleModel.MapShape(drawing, centre, ActualHeight, metres, ObstacleModel.ProbePixels, heading, 0);
        if (shape.Points.Count == 0 || shape.GradientTo <= 0) return [];
        var labels = _showText
            ? ObstacleModel.MapLabels(drawing, shape.Points).SelectMany(point =>
                OverlayInk.Outlined(drawing.Texts[point.Index], new Point(point.Inner.X, point.Inner.Y - MapTextSize), MapTextSize, "TextPrimaryBrush", MapTextAlpha, 1))
            : [];
        return Wedges(shape, at).Concat(labels).ToList();
    }

    private static IEnumerable<UIElement> Wedges(MapObstacleShape shape, Point centre)
    {
        var points = shape.Points;

        Point P(Spot spot) => new(spot.X, spot.Y);
        OverlayPoint Nth(int at) => points[at % points.Count];
        return Enumerable.Range(0, points.Count).Where(at => at % 3 == 0).Select(at =>
        {
            var (a, b, c, d) = (Nth(at), Nth(at + 1), Nth(at + 2), Nth(at + 3));
            var wedge = OverlayInk.Figure(P(a.Inner), new PathSegment[]
            {
                new LineSegment { Point = P(a.Outer) },
                new BezierSegment { Point1 = P(b.Outer), Point2 = P(c.Outer), Point3 = P(d.Outer) },
                new LineSegment { Point = P(d.Inner) },
                new BezierSegment { Point1 = P(d.Inner), Point2 = P(c.Inner), Point3 = P(b.Inner) },
            }, closed: true);
            return (UIElement)OverlayInk.Radial(wedge, centre, shape.GradientTo, shape.GradientTo, shape.GradientFrom, shape.GradientTo, ObstacleModel.MapStops);
        });
    }
}
