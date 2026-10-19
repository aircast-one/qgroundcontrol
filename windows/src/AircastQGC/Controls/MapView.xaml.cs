using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading;
using System.Threading.Tasks;
using AircastQGC.Core;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
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
    private static readonly TimeSpan PanSuspension = TimeSpan.FromSeconds(10);
    private static readonly TimeSpan PollInterval = TimeSpan.FromMilliseconds(500);

    private readonly record struct Geo(double Latitude, double Longitude);
    private readonly record struct Tile(int X, int Y, int Z);
    private sealed record Cell(Canvas View, CancellationTokenSource Cancel);

    private readonly Dictionary<Tile, Cell> _cells = new();
    private DispatcherQueueTimer? _poll;
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

    public MapView()
    {
        InitializeComponent();
        Loaded += OnLoaded;
        Unloaded += OnUnloaded;
    }

    public event Action<double, double, Point>? MapRightTapped;

    public Thickness FollowInsets { get; set; } = new(24, 56, 352, 40);

    public double Latitude => _center.Latitude;

    public double Longitude => _center.Longitude;

    public double Zoom => _zoom;

    public bool HasVehicle => _vehicle.HasValue;

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

    private Point Middle => new(Surface.ActualWidth / 2, Surface.ActualHeight / 2);

    private void OnLoaded(object sender, RoutedEventArgs e)
    {
        _poll ??= CreatePoll();
        _poll.Start();
        Refresh();
    }

    private void OnUnloaded(object sender, RoutedEventArgs e)
    {
        _poll?.Stop();
        _cells.Keys.ToList().ForEach(Drop);
    }

    private DispatcherQueueTimer CreatePoll()
    {
        var timer = DispatcherQueue.CreateTimer();
        timer.Interval = PollInterval;
        timer.Tick += (_, _) => Refresh();
        return timer;
    }

    private void Refresh()
    {
        var type = MapNative.CurrentType();
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

        Track();
        Render();
    }

    private void Track()
    {
        if (_vehicle is not { } vehicle) return;
        if (!_framed)
        {
            _framed = true;
            _zoom = Math.Max(_zoom, VehicleZoom);
            _center = vehicle;
            return;
        }
        if (_pannedAt is { } at && DateTime.UtcNow - at < PanSuspension) return;
        if (_keepCentered)
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
    }

    private void Add(Tile slot)
    {
        var cell = new Cell(new Canvas
        {
            Width = TileSize,
            Height = TileSize,
            Clip = new RectangleGeometry { Rect = new Rect(0, 0, TileSize, TileSize) },
            RenderTransform = new ScaleTransform(),
            Children = { new Border { Style = (Style)Resources["TileFrame"] } },
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

    private void OnDoubleTapped(object sender, DoubleTappedRoutedEventArgs e)
    {
        ZoomAround(e.GetPosition(Surface), 1);
        e.Handled = true;
    }

    private void OnRightTapped(object sender, RightTappedRoutedEventArgs e)
    {
        var at = e.GetPosition(Surface);
        var geo = ToGeo(at);
        MapRightTapped?.Invoke(geo.Latitude, geo.Longitude, at);
        e.Handled = true;
    }
}
