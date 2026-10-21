using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading;
using System.Threading.Tasks;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Microsoft.UI.Xaml.Shapes;
using Windows.Foundation;

namespace AircastQGC.Controls;

public sealed partial class MapView : UserControl
{
    private const double TileSize = 256;
    private const double MinZoom = 2;
    private const double MaxZoom = 20;
    private const double VehicleZoom = 17;
    private const double WheelStep = 0.5;
    private const int MaxParentDepth = 4;
    private const double MaxLatitude = 85.05112878;
    private const string FollowSetting = "view.control(settings.flyViewSettings.keepMapCenteredOnVehicle)";
    private const string FlyMissionPath = "view.flyMissionItems";
    private const string FlyFencesPath = "view.flyFences";
    private const string TrackPath = "view.track";
    private const string TrafficPath = "view.adsbTraffic";
    private const string OrbitPath = "view.orbit";
    private const string MapClickPath = "view.mapClick";
    private const string GuidedPath = "view.guidedActions";
    private const string VehiclesPath = "view.vehicles";
    private const string RallyLabel = "R";
    private const string RoiLabel = "Looking here";
    private const int ReturnToLaunch = 20;
    private const double EquatorMetres = 40_075_016.686;
    private const double ScalePixels = 100;
    private const double FenceFillOpacity = 0.12;
    private const double StaleOpacity = 0.5;
    private const double PinSize = 24;
    private const double CurrentPinSize = 30;
    private const double DotPinSize = 8;
    private const double RallySize = 22;
    private const double MarkSize = 18;
    private const double OrbitDotSize = 10;
    private const double GlyphSize = 32;
    private const double LabelBoxWidth = 120;
    private const double LabelBoxHeight = 112;
    private const double LabelGap = 2;
    private const double SideGap = 4;
    private const double SideLabelHalfHeight = 8;
    private static readonly string[] WatchedPaths = [TrackPath, TrafficPath, OrbitPath, MapClickPath, GuidedPath, VehiclesPath];
    private static readonly TimeSpan PanSuspension = TimeSpan.FromSeconds(10);
    private static readonly TimeSpan PollInterval = TimeSpan.FromMilliseconds(500);
    private static readonly TimeSpan PlanPollInterval = TimeSpan.FromSeconds(2);
    private static readonly (double X, double Y)[] AircraftOutline =
    [
        (0.5, 0.06), (0.58, 0.42), (0.94, 0.58), (0.58, 0.6), (0.56, 0.82), (0.7, 0.92),
        (0.3, 0.92), (0.44, 0.82), (0.42, 0.6), (0.06, 0.58), (0.42, 0.42),
    ];

    private readonly record struct Tile(int X, int Y, int Z);
    private sealed record Cell(Canvas View, CancellationTokenSource Cancel);
    private sealed record Drawn(UIElement View, Action<Func<Geo, Point>> Place);

    private sealed class Overlay(Canvas canvas)
    {
        private List<Action<Func<Geo, Point>>> _placers = [];

        public void Draw(IEnumerable<Drawn> drawn, Func<Geo, Point> project)
        {
            var shown = drawn.ToList();
            canvas.Children.Clear();
            shown.ForEach(item => canvas.Children.Add(item.View));
            _placers = shown.Select(item => item.Place).ToList();
            Place(project);
        }

        public void Place(Func<Geo, Point> project) => _placers.ForEach(place => place(project));
    }

    private readonly string _watchPrefix = $"mapview.{Guid.NewGuid():N}.";
    private readonly Dictionary<Tile, Cell> _cells = new();
    private readonly Overlay _fenceLayer;
    private readonly Overlay _trackLayer;
    private readonly Overlay _missionLayer;
    private readonly Overlay _trafficLayer;
    private readonly Overlay _guideLayer;
    private readonly MapFleet _fleet;
    private DispatcherQueueTimer? _poll;
    private DispatcherQueueTimer? _planPoll;
    private DispatcherQueueTimer? _tapWait;
    private (Geo Geo, Point At)? _pendingTap;
    private Geo _center = new(0, 0);
    private double _zoom = 3;
    private string _mapType = string.Empty;
    private Geo? _vehicle;
    private Geo? _home;
    private double? _heading;
    private bool _keepCentered;
    private bool _framed;
    private DateTime? _pannedAt;
    private Point? _dragFrom;
    private CircleMark? _goTo;
    private CircleMark? _orbit;
    private Geo? _roi;
    private Geo? _clickMark;
    private double _scaleAcross = double.NaN;
    private MapScale? _scale;
    private bool _scaleHidden;
    private bool _planMode;
    private (Geo Centre, double Zoom, double Width, double Height) _shown;

    public MapView()
    {
        InitializeComponent();
        _fenceLayer = new Overlay(FenceLayer);
        _trackLayer = new Overlay(TrackLayer);
        _missionLayer = new Overlay(MissionLayer);
        _trafficLayer = new Overlay(TrafficLayer);
        _guideLayer = new Overlay(GuideLayer);
        _fleet = new MapFleet(FleetLayer);
        Loaded += OnLoaded;
        Unloaded += OnUnloaded;
        Surface.Tapped += OnTapped;
    }

    public event Action<double, double, Point>? MapRightTapped;

    public event Action<double, double, Point>? MapTapped;

    public event Action<int>? VehicleTapped;

    public event EventHandler? ViewChanged;

    public bool PlanMode
    {
        get => _planMode;
        set
        {
            _planMode = value;
            new UIElement[] { FenceLayer, TrackLayer, MissionLayer, TrafficLayer, GuideLayer, FleetLayer }.ToList()
                .ForEach(layer => layer.Visibility = value ? Visibility.Collapsed : Visibility.Visible);
        }
    }

    internal Point ScreenOf(Geo geo) => ToScreen(geo);

    internal Geo GeoOf(Point screen) => ToGeo(screen);

    public Thickness FollowInsets { get; set; } = new(24, 56, 352, 40);

    public Thickness ScaleMargin
    {
        get => ScaleBar.Margin;
        set => ScaleBar.Margin = value;
    }

    public bool ScaleInCorner
    {
        get => ScaleBar.VerticalAlignment == VerticalAlignment.Bottom;
        set
        {
            ScaleBar.HorizontalAlignment = value ? HorizontalAlignment.Right : HorizontalAlignment.Left;
            ScaleBar.VerticalAlignment = value ? VerticalAlignment.Bottom : VerticalAlignment.Top;
        }
    }

    public bool ScaleHidden
    {
        get => _scaleHidden;
        set
        {
            _scaleHidden = value;
            PlaceScale();
        }
    }

    public string? TileType { get; set; }

    internal static (Geo Centre, double Zoom)? FlightCamera { get; private set; }

    public bool Follow { get; set; }

    public double Latitude => _center.Latitude;

    public double Longitude => _center.Longitude;

    public double Zoom => _zoom;

    public bool HasVehicle => _vehicle.HasValue;

    internal Geo? VehicleAt => _vehicle;

    internal double? VehicleHeading => _heading;

    public string MapType => _mapType;

    public bool CenterOnVehicle()
    {
        if (_vehicle is not { } vehicle) return false;
        _center = vehicle;
        _pannedAt = null;
        Render();
        return true;
    }

    public void CenterOn(double latitude, double longitude, double? zoom = null)
    {
        if (!double.IsFinite(latitude) || !double.IsFinite(longitude)) return;
        _center = new Geo(Math.Clamp(latitude, -MaxLatitude, MaxLatitude), Wrap(longitude));
        _zoom = zoom is { } level && double.IsFinite(level) ? Math.Clamp(level, MinZoom, MaxZoom) : _zoom;
        _framed = true;
        Render();
    }

    public void ZoomIn() => ZoomAround(Middle, 1);

    public void ZoomOut() => ZoomAround(Middle, -1);

    public void ShowClickMarker(double latitude, double longitude)
    {
        _clickMark = Valid(latitude, longitude);
        DrawGuides();
    }

    public void HideClickMarker()
    {
        _clickMark = null;
        DrawGuides();
    }

    private Point Middle => new(Surface.ActualWidth / 2, Surface.ActualHeight / 2);

    private IEnumerable<Overlay> Layers => [_fenceLayer, _trackLayer, _missionLayer, _trafficLayer, _guideLayer];

    private void OnLoaded(object sender, RoutedEventArgs e)
    {
        _poll ??= CreatePoll(PollInterval, Refresh);
        _planPoll ??= CreatePoll(PlanPollInterval, RefreshPlan);
        _poll.Start();
        Refresh();
        if (_planMode) return;
        _planPoll.Start();
        RefreshPlan();
        StartWatching();
    }

    private void OnUnloaded(object sender, RoutedEventArgs e)
    {
        _poll?.Stop();
        _planPoll?.Stop();
        WatchedPaths.ToList().ForEach(path => BridgeWatch.Stop(_watchPrefix + path));
        _cells.Keys.ToList().ForEach(Drop);
    }

    private DispatcherQueueTimer CreatePoll(TimeSpan interval, Action tick)
    {
        var timer = DispatcherQueue.CreateTimer();
        timer.Interval = interval;
        timer.Tick += (_, _) => tick();
        return timer;
    }

    private void Refresh()
    {
        if (Live.Hidden(this)) return;
        var type = TileType ?? MapNative.CurrentType();
        if (type != _mapType)
        {
            _mapType = type;
            _cells.Keys.ToList().ForEach(Drop);
        }

        var vehicle = Read("vehicle");
        var live = Text(vehicle["kind"]) == "object";
        _vehicle = live ? Usable(vehicle["coordinate"]) : null;
        _home = live ? Usable(vehicle["homePosition"]) : null;
        _heading = live ? Heading(vehicle["facts"]) : null;
        _keepCentered = Flag(Read(FollowSetting)["value"]);
        _fleet.Operator(Read(MapFleet.GcsPositionPath), ToScreen);

        Track();
        Render();
    }

    private void RefreshPlan()
    {
        _missionLayer.Draw(MissionDrawing(MapOverlayJson.Mission(Read(FlyMissionPath))), ToScreen);
        _fenceLayer.Draw(FenceDrawing(MapOverlayJson.Fences(Read(FlyFencesPath))), ToScreen);
        _scaleAcross = double.NaN;
        PlaceScale();
    }

    private void StartWatching()
    {
        Watch(TrackPath, view => _trackLayer.Draw(TrackDrawing(MapOverlayJson.Track(view)), ToScreen));
        Watch(TrafficPath, view => _trafficLayer.Draw(MapOverlayJson.Traffic(view).Select(TrafficDrawing), ToScreen));
        Watch(OrbitPath, view =>
        {
            _orbit = MapOverlayJson.Orbit(view);
            DrawGuides();
        });
        Watch(MapClickPath, view =>
        {
            _goTo = MapOverlayJson.GoTo(view);
            DrawGuides();
        });
        Watch(VehiclesPath, view => _fleet.Fleet(view, ToScreen));
        Watch(GuidedPath, view =>
        {
            _roi = MapOverlayJson.Roi(view);
            DrawGuides();
        });
    }

    private void Watch(string path, Action<JsonObject> apply)
    {
        try { BridgeWatch.Watch(_watchPrefix + path, new[] { path }, Live.WhileShown(this, apply)); }
        catch (Exception) { }
        apply(Read(path));
    }

    private void Track()
    {
        if (_vehicle == null && !_framed && _fleet.OperatorAt is { } operatorAt)
        {
            _framed = true;
            _zoom = Math.Max(_zoom, VehicleZoom);
            _center = operatorAt;
        }
        if (_vehicle is not { } vehicle) return;
        if (!_framed)
        {
            _framed = true;
            _zoom = Math.Max(_zoom, VehicleZoom);
            _center = vehicle;
            return;
        }
        if (_planMode && !Follow) return;
        if (_pannedAt is { } at && DateTime.UtcNow - at < PanSuspension) return;
        if (_keepCentered || Follow)
        {
            _center = vehicle;
            return;
        }
        Nudge(vehicle);
    }

    private void Nudge(Geo vehicle)
    {
        var width = Surface.ActualWidth;
        var height = Surface.ActualHeight;
        var insets = FollowInsets;
        var usableWidth = width - insets.Left - insets.Right;
        var usableHeight = height - insets.Top - insets.Bottom;
        var at = ToScreen(vehicle);
        var inside = at.X >= insets.Left && at.X < insets.Left + usableWidth
            && at.Y >= insets.Top && at.Y < insets.Top + usableHeight;
        if (usableWidth <= 0 || usableHeight <= 0 || inside) return;
        _center = ToGeo(new Point(
            at.X + width / 2 - (insets.Left + usableWidth / 2),
            at.Y + height / 2 - (insets.Top + usableHeight / 2)));
    }

    private void Render()
    {
        var width = Surface.ActualWidth;
        var height = Surface.ActualHeight;
        if (width <= 0 || height <= 0) return;

        var level = (int)Math.Clamp(Math.Floor(_zoom), MinZoom, MaxZoom);
        var size = TileSize * Math.Pow(2, _zoom - level);
        var centre = Project(_center, level);
        var originX = width / 2 - centre.X / TileSize * size;
        var originY = height / 2 - centre.Y / TileSize * size;
        var count = 1 << level;
        var firstX = (int)Math.Floor(-originX / size);
        var lastX = (int)Math.Floor((width - originX) / size);
        var firstY = Math.Max(0, (int)Math.Floor(-originY / size));
        var lastY = Math.Min(count - 1, (int)Math.Floor((height - originY) / size));

        var wanted = Enumerable.Range(firstX, lastX - firstX + 1)
            .SelectMany(x => Enumerable.Range(firstY, Math.Max(0, lastY - firstY + 1)).Select(y => new Tile(x, y, level)))
            .ToHashSet();

        _cells.Keys.Where(slot => !wanted.Contains(slot)).ToList().ForEach(Drop);
        wanted.Where(slot => !_cells.ContainsKey(slot)).ToList().ForEach(Add);
        _cells.ToList().ForEach(entry => PlaceCell(entry.Value.View, entry.Key, originX, originY, size));

        PlaceMarkers();
        Layers.ToList().ForEach(layer => layer.Place(ToScreen));
        _fleet.Place(ToScreen);
        PlaceScale();
        if (!_planMode && (_framed || _pannedAt.HasValue)) FlightCamera = (_center, _zoom);
        var shown = (_center, _zoom, width, height);
        if (shown == _shown) return;
        _shown = shown;
        ViewChanged?.Invoke(this, EventArgs.Empty);
    }

    private void Add(Tile slot)
    {
        var cell = new Cell(new Canvas
        {
            Width = TileSize,
            Height = TileSize,
            Clip = new RectangleGeometry { Rect = new Rect(0, 0, TileSize, TileSize) },
            RenderTransform = new ScaleTransform(),
            Children = { new Border { Style = Styled("TileFrame") } },
        }, new CancellationTokenSource());
        _cells[slot] = cell;
        TileLayer.Children.Add(cell.View);
        Load(slot, cell);
    }

    private void Drop(Tile slot)
    {
        if (!_cells.Remove(slot, out var cell)) return;
        cell.Cancel.Cancel();
        TileLayer.Children.Remove(cell.View);
    }

    private static void PlaceCell(Canvas view, Tile slot, double originX, double originY, double size)
    {
        Canvas.SetLeft(view, Math.Floor(originX + slot.X * size));
        Canvas.SetTop(view, Math.Floor(originY + slot.Y * size));
        var scale = (ScaleTransform)view.RenderTransform;
        scale.ScaleX = (size + 1) / TileSize;
        scale.ScaleY = (size + 1) / TileSize;
    }

    private async void Load(Tile slot, Cell cell)
    {
        try
        {
            var token = cell.Cancel.Token;
            var fill = await Fill(new Tile(Mod(slot.X, 1 << slot.Z), slot.Y, slot.Z), token);
            if (fill != null && !token.IsCancellationRequested) cell.View.Children.Add(fill);
        }
        catch (Exception)
        {
        }
    }

    private async Task<UIElement?> Fill(Tile tile, CancellationToken token)
    {
        var exact = await Decode(tile, token);
        return exact != null
            ? Picture(exact, 0, 0, TileSize)
            : await FromParent(tile, 1, token) ?? await FromChildren(tile, token);
    }

    private async Task<UIElement?> FromParent(Tile tile, int depth, CancellationToken token)
    {
        if (depth > MaxParentDepth || tile.Z - depth < 0 || token.IsCancellationRequested) return null;
        var parent = new Tile(tile.X >> depth, tile.Y >> depth, tile.Z - depth);
        var image = await Decode(parent, token);
        return image != null
            ? Picture(image,
                -(tile.X - (parent.X << depth)) * TileSize,
                -(tile.Y - (parent.Y << depth)) * TileSize,
                TileSize * (1 << depth))
            : await FromParent(tile, depth + 1, token);
    }

    private async Task<UIElement?> FromChildren(Tile tile, CancellationToken token)
    {
        if (tile.Z + 1 > MaxZoom || token.IsCancellationRequested) return null;
        var children = new[] { (0, 0), (1, 0), (0, 1), (1, 1) }
            .Select(offset => new Tile(tile.X * 2 + offset.Item1, tile.Y * 2 + offset.Item2, tile.Z + 1))
            .ToArray();
        var images = await Task.WhenAll(children.Select(child => Decode(child, token)));
        var half = TileSize / 2;
        var pieces = children.Zip(images)
            .Where(pair => pair.Second != null)
            .Select(pair => Picture(pair.Second!, (pair.First.X & 1) * half, (pair.First.Y & 1) * half, half))
            .ToList();
        if (pieces.Count == 0) return null;
        var canvas = new Canvas { Width = TileSize, Height = TileSize };
        pieces.ForEach(canvas.Children.Add);
        return canvas;
    }

    private async Task<BitmapImage?> Decode(Tile tile, CancellationToken token)
    {
        var mapType = _mapType;
        if (mapType.Length == 0 || token.IsCancellationRequested) return null;
        var bytes = await MapNative.FetchAsync(mapType, tile.X, tile.Y, tile.Z, token);
        if (bytes == null || token.IsCancellationRequested) return null;
        try
        {
            var image = new BitmapImage();
            using var stream = new MemoryStream(bytes).AsRandomAccessStream();
            await image.SetSourceAsync(stream);
            return image;
        }
        catch (Exception)
        {
            return null;
        }
    }

    private static Image Picture(ImageSource source, double left, double top, double size)
    {
        var image = new Image { Source = source, Width = size, Height = size, Stretch = Stretch.Fill };
        Canvas.SetLeft(image, left);
        Canvas.SetTop(image, top);
        return image;
    }

    private void PlaceMarkers()
    {
        PlaceMarker(HomeMarker, _home);
        PlaceMarker(VehicleMarker, _vehicle);
        VehicleArrow.Visibility = _heading.HasValue ? Visibility.Visible : Visibility.Collapsed;
        VehicleDot.Visibility = _heading.HasValue ? Visibility.Collapsed : Visibility.Visible;
        VehicleRotation.Angle = _heading ?? 0;
    }

    private void PlaceMarker(FrameworkElement marker, Geo? at)
    {
        marker.Visibility = at.HasValue ? Visibility.Visible : Visibility.Collapsed;
        if (at is not { } geo) return;
        var point = ToScreen(geo);
        Canvas.SetLeft(marker, point.X - marker.Width / 2);
        Canvas.SetTop(marker, point.Y - marker.Height / 2);
    }

    private void PlaceScale()
    {
        var across = Math.Round(EquatorMetres * Math.Cos(_center.Latitude * Math.PI / 180) / (TileSize * Math.Pow(2, _zoom)) * ScalePixels);
        if (across != _scaleAcross)
        {
            _scaleAcross = across;
            _scale = across > 0 ? MapOverlayJson.Scale(Read(FormattableString.Invariant($"view.mapScale({across:F0})"))) : null;
        }
        ScaleBar.Visibility = !_scaleHidden && _scale != null && Surface.ActualWidth > ScalePixels ? Visibility.Visible : Visibility.Collapsed;
        ScaleText.Text = _scale?.Text ?? string.Empty;
        ScaleLine.Width = (_scale?.Fraction ?? 0) * ScalePixels;
    }

    private IEnumerable<Drawn> MissionDrawing(FlownMission mission) =>
        mission.Route.Select(line => Line(line, "RouteLine"))
            .Concat(mission.Pins.OrderBy(pin => pin.Current).Select(pin => Pin(pin.At, MissionPinView(pin))));

    private IEnumerable<Drawn> FenceDrawing(FlownFences fences) =>
        fences.Areas
            .Select(area => Area(area.Ring, area.KeepIn ? "KeepInEdge" : "KeepOutEdge", Tint(area.KeepIn ? "CautionBrush" : "CriticalBrush")))
            .Append(Area(fences.Enforced, "KeepInEdge", dashed: true))
            .Concat(fences.Rally.Select(at => Pin(at, Disc("RallyDisc", RallySize, RallyLabel))));

    private IEnumerable<Drawn> TrackDrawing(IReadOnlyList<Geo> track) =>
        track.Count > 1 ? [Line(track, "TrackLine")] : [];

    private Drawn TrafficDrawing(TrafficMark mark)
    {
        var glyph = new Polygon
        {
            Style = Styled(mark.Alert ? "AlertTrafficGlyph" : "TrafficGlyph"),
            Width = GlyphSize,
            Height = GlyphSize,
            Points = Collect(AircraftOutline.Select(at => new Point(at.X * GlyphSize, at.Y * GlyphSize))),
            RenderTransform = new RotateTransform { Angle = mark.Heading ?? 0, CenterX = GlyphSize / 2, CenterY = GlyphSize / 2 },
        };
        var view = Labelled(glyph, GlyphSize, mark.Label);
        view.Opacity = mark.Stale ? StaleOpacity : 1;
        return Pin(mark.At, view);
    }

    private void DrawGuides() => _guideLayer.Draw(Guides(), ToScreen);

    private IEnumerable<Drawn> Guides()
    {
        IEnumerable<Drawn> orbit = _orbit is { } circle
            ? [Area(circle.Ring, "OrbitEdge"), Pin(circle.Centre, Disc("OrbitDisc", OrbitDotSize))]
            : [];
        IEnumerable<Drawn> going = _goTo is { } destination
            ? [Area(destination.Ring, "LoiterEdge", dashed: true), Pin(destination.Centre, Disc("GoToDisc", MarkSize))]
            : [];
        IEnumerable<Drawn> looking = _roi is { } roi
            ? [Pin(roi, Labelled(Disc("RoiDisc", MarkSize), MarkSize, RoiLabel))]
            : [];
        IEnumerable<Drawn> clicked = _clickMark is { } click ? [Pin(click, ClickMark())] : [];
        return orbit.Concat(going).Concat(looking).Concat(clicked);
    }

    private Drawn Line(IEnumerable<Geo> points, string style)
    {
        var line = new Polyline { Style = Styled(style) };
        return new Drawn(line, project => line.Points = Collect(points.Select(project)));
    }

    private Drawn Area(IReadOnlyList<Geo> ring, string style, Brush? fill = null, bool dashed = false)
    {
        var area = new Polygon { Style = Styled(style), Fill = fill };
        if (dashed) area.StrokeDashArray = Dashes();
        return new Drawn(area, project => area.Points = Collect(ring.Select(project)));
    }

    private static Drawn Pin(Geo at, FrameworkElement view) =>
        new(view, project =>
        {
            var point = project(at);
            Canvas.SetLeft(view, point.X - view.Width / 2);
            Canvas.SetTop(view, point.Y - view.Height / 2);
        });

    private FrameworkElement MissionPinView(MissionPin pin)
    {
        var size = pin.Current ? CurrentPinSize : pin.Dot ? DotPinSize : PinSize;
        var side = new TextBlock { Style = Styled("MapLabel"), Text = pin.Side };
        Canvas.SetLeft(side, size + SideGap);
        Canvas.SetTop(side, size / 2 - SideLabelHalfHeight);
        return new Grid
        {
            Width = size,
            Height = size,
            Children =
            {
                new Ellipse { Style = Styled(pin.Current ? "CurrentPinDisc" : pin.Dot ? "DotPinDisc" : "PinDisc"), Fill = Token(MissionTone(pin)) },
                new TextBlock { Style = Styled("PinLabel"), Text = pin.Label },
                new Canvas { Children = { side } },
            },
        };
    }

    private static string MissionTone(MissionPin pin) => (pin.Kind, pin.Command) switch
    {
        ("takeoff", _) => "SuccessBrush",
        ("land", _) or (_, ReturnToLaunch) => "CriticalBrush",
        _ => "MapPathBrush",
    };

    private FrameworkElement Disc(string style, double size, string label = "") => new Grid
    {
        Width = size,
        Height = size,
        Children =
        {
            new Ellipse { Style = Styled(style) },
            new TextBlock { Style = Styled("PinLabel"), Text = label },
        },
    };

    private FrameworkElement Labelled(FrameworkElement glyph, double glyphSize, string label) => new Grid
    {
        Width = LabelBoxWidth,
        Height = LabelBoxHeight,
        Children =
        {
            glyph,
            new TextBlock
            {
                Style = Styled("MapLabel"),
                Text = label,
                HorizontalAlignment = HorizontalAlignment.Center,
                VerticalAlignment = VerticalAlignment.Top,
                Margin = new Thickness(0, LabelBoxHeight / 2 + glyphSize / 2 + LabelGap, 0, 0),
            },
        },
    };

    private FrameworkElement ClickMark() => new Grid
    {
        Width = MarkSize,
        Height = MarkSize,
        Children =
        {
            new Ellipse { Style = Styled("MarkHalo") },
            new Ellipse { Style = Styled("MarkRing") },
            new Ellipse { Style = Styled("MarkDot") },
        },
    };

    private Style Styled(string key) => (Style)Resources[key];

    private static Brush Token(string key) => (Brush)Application.Current.Resources[key];

    private static Brush Tint(string key) =>
        new SolidColorBrush(((SolidColorBrush)Token(key)).Color) { Opacity = FenceFillOpacity };

    private static DoubleCollection Dashes() => new() { 4, 3 };

    private static PointCollection Collect(IEnumerable<Point> points)
    {
        var collection = new PointCollection();
        points.ToList().ForEach(collection.Add);
        return collection;
    }

    private static Geo? Valid(double latitude, double longitude) =>
        double.IsFinite(latitude) && double.IsFinite(longitude) ? new Geo(latitude, longitude) : null;

    private void Pan(double dx, double dy)
    {
        var centre = Project(_center, _zoom);
        _center = Unproject(centre.X - dx, centre.Y - dy, _zoom);
        Render();
    }

    private void ZoomAround(Point at, double change)
    {
        var anchor = ToGeo(at);
        _zoom = Math.Clamp(_zoom + change, MinZoom, MaxZoom);
        var world = Project(anchor, _zoom);
        _center = Unproject(world.X - (at.X - Surface.ActualWidth / 2), world.Y - (at.Y - Surface.ActualHeight / 2), _zoom);
        Render();
    }

    private Point ToScreen(Geo geo)
    {
        var centre = Project(_center, _zoom);
        var at = Project(geo, _zoom);
        return new Point(at.X - centre.X + Surface.ActualWidth / 2, at.Y - centre.Y + Surface.ActualHeight / 2);
    }

    private Geo ToGeo(Point screen)
    {
        var centre = Project(_center, _zoom);
        return Unproject(centre.X + screen.X - Surface.ActualWidth / 2, centre.Y + screen.Y - Surface.ActualHeight / 2, _zoom);
    }

    private static Point Project(Geo geo, double zoom)
    {
        var world = TileSize * Math.Pow(2, zoom);
        var latitude = Math.Clamp(geo.Latitude, -MaxLatitude, MaxLatitude) * Math.PI / 180;
        return new Point(
            (geo.Longitude + 180) / 360 * world,
            (1 - Math.Log(Math.Tan(latitude) + 1 / Math.Cos(latitude)) / Math.PI) / 2 * world);
    }

    private static Geo Unproject(double x, double y, double zoom)
    {
        var world = TileSize * Math.Pow(2, zoom);
        var latitude = Math.Atan(Math.Sinh(Math.PI * (1 - 2 * y / world))) * 180 / Math.PI;
        return new Geo(Math.Clamp(latitude, -MaxLatitude, MaxLatitude), Wrap(x / world * 360 - 180));
    }

    private static double Wrap(double longitude) => ((longitude + 180) % 360 + 360) % 360 - 180;

    private static int Mod(int value, int count) => (value % count + count) % count;

    private static JsonObject Read(string path)
    {
        try { return Bridge.Group(path); }
        catch (Exception) { return new JsonObject(); }
    }

    private static string? Text(JsonNode? node) =>
        node is JsonValue value && value.TryGetValue<string>(out var text) ? text : null;

    private static double? Number(JsonNode? node) =>
        node is JsonValue value && value.TryGetValue<double>(out var number) && double.IsFinite(number) ? number : null;

    private static bool Flag(JsonNode? node) =>
        node is JsonValue value && value.TryGetValue<bool>(out var flag) && flag;

    private static bool Refused(JsonNode? node) =>
        node is JsonValue value && value.TryGetValue<bool>(out var flag) && !flag;

    private static Geo? Usable(JsonNode? node) =>
        node is JsonObject point
        && !Refused(point["valid"])
        && Number(point["latitude"]) is { } latitude
        && Number(point["longitude"]) is { } longitude
        && (latitude != 0 || longitude != 0)
        && Math.Abs(latitude) <= 90
        && Math.Abs(longitude) <= 180
            ? new Geo(latitude, longitude)
            : null;

    private static double? Heading(JsonNode? facts) =>
        (facts as JsonArray)?.OfType<JsonObject>()
            .Where(fact => Text(fact["name"]) == "heading")
            .Select(fact => Number(fact["value"]))
            .FirstOrDefault();

    private void OnSizeChanged(object sender, SizeChangedEventArgs e)
    {
        Surface.Clip = new RectangleGeometry { Rect = new Rect(0, 0, e.NewSize.Width, e.NewSize.Height) };
        Render();
    }

    private void OnPointerPressed(object sender, PointerRoutedEventArgs e)
    {
        var point = e.GetCurrentPoint(Surface);
        if (point.Properties.IsRightButtonPressed) return;
        if (_dragFrom.HasValue)
        {
            _dragFrom = null;
            return;
        }
        _dragFrom = point.Position;
        Surface.CapturePointer(e.Pointer);
    }

    private void OnPointerMoved(object sender, PointerRoutedEventArgs e)
    {
        if (_dragFrom is not { } from) return;
        var at = e.GetCurrentPoint(Surface).Position;
        _dragFrom = at;
        _pannedAt = DateTime.UtcNow;
        Pan(at.X - from.X, at.Y - from.Y);
    }

    private void OnPointerReleased(object sender, PointerRoutedEventArgs e)
    {
        _dragFrom = null;
        Surface.ReleasePointerCapture(e.Pointer);
    }

    private void OnPointerCaptureLost(object sender, PointerRoutedEventArgs e) => _dragFrom = null;

    private void OnPointerWheelChanged(object sender, PointerRoutedEventArgs e)
    {
        var point = e.GetCurrentPoint(Surface);
        if (point.Properties.IsHorizontalMouseWheel) return;
        ZoomAround(point.Position, point.Properties.MouseWheelDelta / 120.0 * WheelStep);
        e.Handled = true;
    }

    private void OnManipulationDelta(object sender, ManipulationDeltaRoutedEventArgs e)
    {
        var scale = e.Delta.Scale;
        if (scale <= 0 || scale == 1) return;
        _dragFrom = null;
        _pannedAt = DateTime.UtcNow;
        ZoomAround(e.Position, Math.Log2(scale));
        e.Handled = true;
    }

    private void OnDoubleTapped(object sender, DoubleTappedRoutedEventArgs e)
    {
        _tapWait?.Stop();
        _pendingTap = null;
        ZoomAround(e.GetPosition(Surface), 1);
        e.Handled = true;
    }

    private void OnTapped(object sender, TappedRoutedEventArgs e)
    {
        var at = e.GetPosition(Surface);
        if (!_planMode && _fleet.VehicleAt(at, ToScreen) is int vehicle)
        {
            VehicleTapped?.Invoke(vehicle);
            return;
        }
        _pendingTap = (ToGeo(at), at);
        if (!_planMode)
        {
            ReleaseTap();
            return;
        }
        _tapWait ??= CreatePoll(TimeSpan.FromMilliseconds(new Windows.UI.ViewManagement.UISettings().DoubleClickTime), ReleaseTap);
        _tapWait.Stop();
        _tapWait.Start();
    }

    private void ReleaseTap()
    {
        _tapWait?.Stop();
        if (_pendingTap is not { } tap) return;
        _pendingTap = null;
        MapTapped?.Invoke(tap.Geo.Latitude, tap.Geo.Longitude, tap.At);
    }

    private void OnRightTapped(object sender, RightTappedRoutedEventArgs e)
    {
        var at = e.GetPosition(Surface);
        var geo = ToGeo(at);
        MapRightTapped?.Invoke(geo.Latitude, geo.Longitude, at);
        e.Handled = true;
    }
}
