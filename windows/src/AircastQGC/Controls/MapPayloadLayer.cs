using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Shapes;
using Windows.Foundation;

namespace AircastQGC.Controls;

public sealed partial class MapPayloadLayer : UserControl
{
    private const string AzimuthView = "view.gimbalAzimuth";
    private const string SyntheticView = "view.syntheticView";
    private const string RadarView = "view.proximityRadar";
    private const double WedgeSize = 48;
    private const double OtherWedgeOpacity = 0.4;
    private const double BeamFillOpacity = 0.18;
    private const double BeamLineOpacity = 0.6;
    private const double RadarOpacity = 0.5;
    private const double RadarLine = 3;
    private const double SectorSweep = 45;
    private static readonly TimeSpan AzimuthPoll = TimeSpan.FromMilliseconds(500);

    private readonly string _client = $"map-payload.{Guid.NewGuid():N}";
    private readonly Canvas _canvas = new();
    private readonly DispatcherQueueTimer _poll;
    private string _azimuthRead = "";
    private MapView? _map;
    private IReadOnlyList<(double Yaw, bool Active)> _azimuths = [];
    private IReadOnlyList<Geo> _beam = [];
    private ProximityRadar? _radar;

    public MapPayloadLayer()
    {
        IsHitTestVisible = false;
        Content = _canvas;
        SizeChanged += (_, e) =>
        {
            _canvas.Clip = new RectangleGeometry { Rect = new Rect(0, 0, e.NewSize.Width, e.NewSize.Height) };
            Draw();
        };
        _poll = DispatcherQueue.GetForCurrentThread().CreateTimer();
        _poll.Interval = AzimuthPoll;
        _poll.Tick += (_, _) => ReadAzimuths();
        Loaded += (_, _) => Start();
        Unloaded += (_, _) => Halt();
    }

    public void Attach(MapView map)
    {
        _map = map;
        map.ViewChanged += (_, _) => Draw();
    }

    private void Start()
    {
        _poll.Start();
        ReadAzimuths();
        Watch("beam", SyntheticView, view => _beam = PayloadJson.Beam(view));
        Watch("radar", RadarView, view => _radar = PayloadJson.Radar(view));
    }

    private void Watch(string name, string path, Action<JsonObject> take)
    {
        void Apply(JsonObject view)
        {
            take(view);
            Draw();
        }
        BridgeWatch.Watch($"{_client}.{name}", [path], Apply);
        Apply(Bridge.Get(path));
    }

    private void Halt()
    {
        if (IsLoaded) return;
        _poll.Stop();
        new[] { "beam", "radar" }.ToList().ForEach(name => BridgeWatch.Stop($"{_client}.{name}"));
    }

    private void ReadAzimuths()
    {
        var read = Bridge.Raw(AzimuthView);
        if (read == _azimuthRead || read.Length == 0) return;
        _azimuthRead = read;
        _azimuths = PayloadJson.Azimuths(Parsed(read));
        Draw();
    }

    private static JsonObject Parsed(string raw)
    {
        try { return JsonNode.Parse(raw) as JsonObject ?? new JsonObject(); }
        catch (System.Text.Json.JsonException) { return new JsonObject(); }
    }

    private void Draw()
    {
        if (_map is not { } map || map.VehicleAt is not { } vehicle)
        {
            _canvas.Children.Clear();
            return;
        }
        var at = map.ScreenOf(vehicle);
        FormLook.Fill(_canvas, Beam(map, at)
            .Concat(Radar(map, vehicle, at, map.VehicleHeading ?? 0))
            .Concat(_azimuths.Select(g => Wedge(at, g.Yaw, g.Active))));
    }

    private IEnumerable<UIElement> Beam(MapView map, Point at)
    {
        if (_beam.Count == 0) return [];
        var reach = 2 * Math.Max(ActualWidth, ActualHeight);
        var colour = ((SolidColorBrush)FormLook.Brush("CautionBrush")).Color;
        return
        [
            new Polygon
            {
                Points = Collect(_beam.Select(map.ScreenOf).Select(point => Within(at, point, reach))),
                Fill = new SolidColorBrush(colour) { Opacity = BeamFillOpacity },
                Stroke = new SolidColorBrush(colour) { Opacity = BeamLineOpacity },
                StrokeThickness = 1.5,
            },
        ];
    }

    private IEnumerable<UIElement> Radar(MapView map, Geo vehicle, Point at, double heading)
    {
        if (_radar is not { } radar) return [];
        double Pixels(double metres) =>
            MapOverlayJson.Ring(vehicle, metres) is { Count: > 0 } ring ? Distance(at, map.ScreenOf(ring[0])) : 0;
        IEnumerable<UIElement> limit = radar.MaxMeters is double max
            ? [new Polygon { Points = Collect(MapOverlayJson.Ring(vehicle, max).Select(map.ScreenOf)), Stroke = FormLook.Brush("TextPrimaryBrush"), StrokeThickness = RadarLine, Opacity = RadarOpacity }]
            : [];
        var arcs = radar.Sectors
            .Where(s => s.Meters is > 0)
            .Select(s => (UIElement)Arc(at, Pixels(s.Meters!.Value), heading + s.Bearing - SectorSweep / 2));
        return limit.Concat(arcs).ToList();
    }

    private static Microsoft.UI.Xaml.Shapes.Path Arc(Point centre, double radius, double fromNorth)
    {
        Point On(double degrees) => new(centre.X + radius * Math.Sin(Radians(degrees)), centre.Y - radius * Math.Cos(Radians(degrees)));
        var figure = new PathFigure { StartPoint = On(fromNorth), IsClosed = false, IsFilled = false };
        figure.Segments.Add(new ArcSegment { Point = On(fromNorth + SectorSweep), Size = new Size(radius, radius), SweepDirection = SweepDirection.Clockwise });
        var geometry = new PathGeometry();
        geometry.Figures.Add(figure);
        return new Microsoft.UI.Xaml.Shapes.Path
        {
            Data = geometry,
            Stroke = FormLook.Brush("CriticalBrush"),
            StrokeThickness = RadarLine,
            Opacity = RadarOpacity,
        };
    }

    private static Polygon Wedge(Point at, double yaw, bool active)
    {
        var tone = ((SolidColorBrush)FormLook.Brush("TextPrimaryBrush")).Color;
        var wedge = new Polygon
        {
            Width = WedgeSize,
            Height = WedgeSize,
            Points = Collect(new[] { new Point(WedgeSize / 2, WedgeSize), new Point(0, 0), new Point(WedgeSize / 2, WedgeSize / 4), new Point(WedgeSize, 0) }),
            Fill = new LinearGradientBrush
            {
                StartPoint = new Point(0.5, 0),
                EndPoint = new Point(0.5, 1),
                GradientStops =
                {
                    new GradientStop { Color = Windows.UI.Color.FromArgb(0, tone.R, tone.G, tone.B), Offset = 0 },
                    new GradientStop { Color = Windows.UI.Color.FromArgb(128, tone.R, tone.G, tone.B), Offset = 1 },
                },
            },
            Opacity = active ? 1 : OtherWedgeOpacity,
            RenderTransform = new RotateTransform { Angle = yaw, CenterX = WedgeSize / 2, CenterY = WedgeSize },
        };
        Canvas.SetLeft(wedge, at.X - WedgeSize / 2);
        Canvas.SetTop(wedge, at.Y - WedgeSize);
        return wedge;
    }

    private static Point Within(Point from, Point to, double reach)
    {
        var scale = Math.Min(1, reach / Math.Max(Distance(from, to), 1e-9));
        return new Point(from.X + (to.X - from.X) * scale, from.Y + (to.Y - from.Y) * scale);
    }

    private static double Distance(Point a, Point b) => Math.Sqrt((a.X - b.X) * (a.X - b.X) + (a.Y - b.Y) * (a.Y - b.Y));

    private static double Radians(double degrees) => degrees * Math.PI / 180;

    private static PointCollection Collect(IEnumerable<Point> points)
    {
        var collection = new PointCollection();
        points.ToList().ForEach(collection.Add);
        return collection;
    }
}
