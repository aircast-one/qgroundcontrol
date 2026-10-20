using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Linq;
using System.Numerics;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.WindowsRuntime;
using System.Text.Json.Nodes;
using System.Threading;
using System.Threading.Tasks;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Windows.Foundation;
using Windows.Graphics.Imaging;
using Windows.Storage.Streams;
using Windows.System;

namespace AircastQGC.Controls;

public sealed partial class Viewer3DView : UserControl
{
    private const string ScenePath = "view.viewer3d";
    private const string PathPath = "view.viewer3dPath";
    private const string VehiclesPath = "view.viewer3dVehicle";
    private const string TrackPath = "view.track";
    private const string Client = "viewer3d.";
    private const float SceneTilt = 60;
    private const double SceneZoom = 16;
    private const double EarthMetres = 40_075_016.686;
    private const double TilePixels = 512;
    private const double CameraHeights = 1.5;
    private const float TurnPerPixel = 0.3f;
    private const float LeanPerPixel = 0.25f;
    private const float ZoomStep = 0.85f;
    private const float KeyTurn = 10;
    private const float KeyLean = 5;
    private const double TapSlop = 4;
    private const double PickRadius = 24;
    private const double MaxPixels = 2560 * 1440;
    private const float RebuildRatio = 1.15f;
    private const int MaxTiles = 256;
    private const int MaxFetches = 8;
    private const VirtualKey PlusKey = (VirtualKey)187;
    private const VirtualKey MinusKey = (VirtualKey)189;
    private const string Help = "Drag to orbit and tilt, right-drag to pan, scroll to zoom. Arrow keys orbit and tilt, plus and minus zoom, Home follows the vehicle.";

    private sealed record Drag(uint Pointer, Point Start, Point Last, bool Pan, bool Moved);

    private readonly Image _image = new() { Stretch = Stretch.Fill, IsHitTestVisible = false };
    private readonly Canvas _labels = new() { IsHitTestVisible = false };
    private readonly TextBlock _reasonText = FormLook.Text("");
    private readonly Border _reason;
    private readonly Raster3D _raster = new();
    private readonly ConcurrentDictionary<(int Z, int X, int Y), byte[]?> _tiles = new();
    private readonly ConcurrentDictionary<(int Z, int X, int Y), int> _tileUse = new();
    private readonly ConcurrentDictionary<(int Z, int X, int Y), bool> _wanted = new();
    private CancellationTokenSource _fetching = new();
    private string _mapType = string.Empty;
    private int _fetches;
    private int _frame;
    private WriteableBitmap? _bitmap;
    private Palette3D? _palette;
    private Scene3D _scene = Scene3D.None;
    private Path3D _path = Path3D.None;
    private IReadOnlyList<Vehicle3D> _vehicles = [];
    private Vehicle3D? _followed;
    private IReadOnlyList<(double Lon, double Lat)> _track = [];
    private ImmutableHashSet<int> _selected = ImmutableHashSet<int>.Empty;
    private Origin3D? _origin;
    private IReadOnlyList<Face3D> _buildings = [];
    private IReadOnlyList<Face3D> _moving = [];
    private IReadOnlyList<(Vector3 At, string Text)> _labelPoints = [];
    private float _movingScale = float.NaN;
    private double _renderScale = 1;
    private Camera3D? _camera;
    private bool _follow = true;
    private bool _queued;
    private bool _live;
    private Drag? _drag;

    public Viewer3DView()
    {
        _reasonText.TextWrapping = TextWrapping.Wrap;
        _reason = new Border
        {
            HorizontalAlignment = HorizontalAlignment.Center,
            VerticalAlignment = VerticalAlignment.Center,
            Margin = new Thickness(24),
            Padding = new Thickness(16, 12, 16, 12),
            MaxWidth = 420,
            CornerRadius = FormLook.Radius("RadiusOverlay"),
            Background = FormLook.Brush("ChipBrush"),
            BorderBrush = FormLook.Brush("ControlStrokeBrush"),
            BorderThickness = new Thickness(1),
            IsHitTestVisible = false,
            Visibility = Visibility.Collapsed,
            Child = _reasonText,
        };
        AutomationProperties.SetLiveSetting(_reason, Microsoft.UI.Xaml.Automation.Peers.AutomationLiveSetting.Polite);
        Content = new Grid
        {
            Background = new SolidColorBrush(Colors.Transparent),
            Children = { _image, _labels, _reason },
        };
        IsTabStop = true;
        UseSystemFocusVisuals = true;
        AutomationProperties.SetName(this, "3D view");
        AutomationProperties.SetHelpText(this, Help);
        Visibility = Visibility.Collapsed;
        PointerPressed += OnPressed;
        PointerMoved += OnMoved;
        PointerReleased += OnReleased;
        PointerCanceled += (_, _) => _drag = null;
        PointerCaptureLost += (_, _) => _drag = null;
        PointerWheelChanged += OnWheel;
        SizeChanged += (_, _) => Invalidate();
        ActualThemeChanged += (_, _) =>
        {
            _palette = null;
            Rebuild();
        };
        RegisterPropertyChangedCallback(VisibilityProperty, (_, _) => Live());
        Loaded += (_, _) => Start();
        Unloaded += (_, _) => Stop();
    }

    public event EventHandler? EnabledChanged;

    public bool Enabled => _scene.Enabled == true;

    public void ZoomIn() => Move(camera => camera.Zoomed(ZoomStep));

    public void ZoomOut() => Move(camera => camera.Zoomed(1 / ZoomStep));

    public void FollowVehicle()
    {
        _follow = true;
        Invalidate();
    }

    private Palette3D Palette => _palette ??= new Palette3D(
        Colour("Viewer3DSkyBrush"), Colour("MapLandBrush"), Colour("Viewer3DGridBrush"), Colour("Viewer3DBuildingBrush"), Colour("CriticalBrush"),
        Colour("Viewer3DPathBrush"), Colour("Viewer3DReturnBrush"), Colour("Viewer3DTakeoffBrush"), Colour("Viewer3DWaypointBrush"), Colour("Viewer3DSelectedBrush"),
        Colour("Viewer3DFrontArmBrush"), Colour("Viewer3DRearArmBrush"), Colour("Viewer3DRotorBrush"), Colour("Viewer3DHubBrush"));

    private static uint Colour(string key) =>
        FormLook.Brush(key) is SolidColorBrush { Color: var c } ? 0xFF000000 | (uint)c.R << 16 | (uint)c.G << 8 | c.B : 0xFF000000;

    private void Start()
    {
        BridgeWatch.Watch(Client + ScenePath, [ScenePath], OnScene);
        OnScene(Bridge.Get(ScenePath));
        Live();
    }

    private void Stop()
    {
        if (IsLoaded) return;
        BridgeWatch.Stop(Client + ScenePath);
        Live();
    }

    private void Live()
    {
        var wanted = IsLoaded && Visibility == Visibility.Visible;
        if (wanted == _live) return;
        _live = wanted;
        var watched = new (string Path, Action<JsonObject> Apply)[] { (PathPath, OnPath), (VehiclesPath, OnVehicles), (TrackPath, OnTrack) }.ToList();
        if (wanted)
        {
            watched.ForEach(entry =>
            {
                BridgeWatch.Watch(Client + entry.Path, [entry.Path], entry.Apply);
                entry.Apply(Bridge.Get(entry.Path));
            });
            return;
        }
        watched.ForEach(entry => BridgeWatch.Stop(Client + entry.Path));
        Forget(string.Empty);
    }

    private void OnScene(JsonObject view)
    {
        var enabled = _scene.Enabled;
        _scene = Scene3DJson.Scene(view);
        FormLook.Say(_reasonText, _scene.Reason);
        _reason.Visibility = FormLook.Shown(!_scene.Available && _scene.Reason.Trim().Length > 0);
        Rebuild();
        if (enabled != _scene.Enabled) EnabledChanged?.Invoke(this, EventArgs.Empty);
    }

    private void OnPath(JsonObject view)
    {
        var path = Scene3DJson.Path(view);
        if (!path.Markers.SequenceEqual(_path.Markers)) _selected = ImmutableHashSet<int>.Empty;
        _path = path;
        Moved();
    }

    private void OnVehicles(JsonObject view)
    {
        _vehicles = Scene3DJson.Vehicles(view);
        var active = MapUiJson.VehiclePosition();
        _followed = active is { } at
            ? _vehicles.MinBy(vehicle => Math.Abs(vehicle.At.Lat - at.Latitude) + Math.Abs(vehicle.At.Lon - at.Longitude))
            : _vehicles.FirstOrDefault();
        if (_origin is null && _followed is not null) Rebuild();
        Moved();
    }

    private void OnTrack(JsonObject view)
    {
        _track = MapOverlayJson.Track(view).Select(at => (at.Longitude, at.Latitude)).ToList();
        Moved();
    }

    private void Moved()
    {
        _movingScale = float.NaN;
        Invalidate();
    }

    private void Rebuild()
    {
        var origin = _scene.Centre is { } centre ? new Origin3D(centre.Lon, centre.Lat)
            : _origin ?? (_followed is { } vehicle ? new Origin3D(vehicle.At.Lon, vehicle.At.Lat) : null);
        if (origin != _origin) _camera = null;
        _origin = origin;
        _buildings = origin is { } at
            ? Geometry3D.Buildings(_scene.Buildings, at, Palette.Building).SelectMany(Geometry3D.Faces).ToList()
            : [];
        Moved();
    }

    private Camera3D Framed(Origin3D origin)
    {
        var target = _scene.Centre is null && _followed is { } vehicle ? origin.Metres(vehicle.At) : Vector3.Zero;
        var distance = CameraHeights * Math.Max(ActualHeight, 1) * EarthMetres * Math.Cos(origin.Lat * Math.PI / 180) / (TilePixels * Math.Pow(2, SceneZoom));
        return new Camera3D(target, 0, SceneTilt, Math.Clamp((float)distance, Camera3D.Nearest, Camera3D.Farthest));
    }

    private Camera3D? Camera()
    {
        if (_origin is not { } origin) return null;
        var camera = _camera ??= Framed(origin);
        return _follow && _followed is { } vehicle ? camera with { Target = origin.Metres(vehicle.At) } : camera;
    }

    private void Move(Func<Camera3D, Camera3D> change)
    {
        if (Camera() is not { } camera) return;
        _camera = change(camera);
        Invalidate();
    }

    private void Invalidate()
    {
        if (_queued || !_live) return;
        _queued = true;
        DispatcherQueue.TryEnqueue(Render);
    }

    private void Render()
    {
        _queued = false;
        if (!_live || XamlRoot is null || ActualWidth < 1 || ActualHeight < 1 || _origin is not { } origin || Camera() is not { } camera)
        {
            _image.Source = null;
            _labels.Children.Clear();
            return;
        }
        _renderScale = Math.Min(XamlRoot.RasterizationScale, Math.Sqrt(MaxPixels / (ActualWidth * ActualHeight)));
        var (width, height) = ((int)(ActualWidth * _renderScale), (int)(ActualHeight * _renderScale));
        if (_bitmap is null || _bitmap.PixelWidth != width || _bitmap.PixelHeight != height)
        {
            _bitmap = new WriteableBitmap(width, height);
            _raster.Resize(width, height);
        }
        _image.Source = _bitmap;
        var metresPerDip = camera.Distance / Camera3D.Focal((int)ActualHeight);
        if (!(_movingScale / metresPerDip is > 1 / RebuildRatio and < RebuildRatio)) Furnish(origin, metresPerDip);
        var type = MapNative.CurrentType();
        if (type != _mapType) Forget(type);
        _frame++;
        _raster.Draw(camera, Palette, _mapType.Length > 0 ? Ground3D.At(origin, Tile) : null, _buildings, _moving);
        using (var stream = _bitmap.PixelBuffer.AsStream()) stream.Write(MemoryMarshal.AsBytes(_raster.Pixels.AsSpan()));
        _bitmap.Invalidate();
        PlaceLabels();
        Fetch();
    }

    private byte[]? Tile(int z, int x, int y, bool ask)
    {
        var span = 1 << z;
        if (y < 0 || y >= span) return null;
        var key = (z, (x % span + span) % span, y);
        if (_tiles.TryGetValue(key, out var texels))
        {
            if (texels is not null) _tileUse[key] = _frame;
            return texels;
        }
        if (ask && _tiles.TryAdd(key, null)) _wanted.TryAdd(key, true);
        return null;
    }

    private void Forget(string type)
    {
        _mapType = type;
        _fetching.Cancel();
        _fetching = new CancellationTokenSource();
        _tiles.Clear();
        _tileUse.Clear();
        _wanted.Clear();
    }

    private void Fetch()
    {
        if (_tileUse.Count > MaxTiles)
            _tileUse.Where(use => use.Value < _frame).Select(use => use.Key).ToList()
                .ForEach(key =>
                {
                    _tiles.TryRemove(key, out _);
                    _tileUse.TryRemove(key, out _);
                });
        _wanted.Keys.OrderByDescending(key => key.Z).Take(MaxFetches - _fetches).ToList().ForEach(key =>
        {
            _wanted.TryRemove(key, out _);
            _fetches++;
            _ = Load(key, _mapType, _fetching.Token);
        });
    }

    private async Task Load((int Z, int X, int Y) key, string type, CancellationToken token)
    {
        try
        {
            var bytes = await MapNative.FetchAsync(type, key.X, key.Y, key.Z, token);
            var texels = bytes is null || token.IsCancellationRequested ? null : await Decode(bytes);
            if (texels is not null && !token.IsCancellationRequested)
            {
                _tiles[key] = texels;
                Invalidate();
            }
        }
        catch (OperationCanceledException)
        {
        }
        finally
        {
            _fetches--;
            Fetch();
        }
    }

    private static async Task<byte[]?> Decode(byte[] bytes)
    {
        try
        {
            using var stream = new InMemoryRandomAccessStream();
            await stream.WriteAsync(bytes.AsBuffer());
            stream.Seek(0);
            var decoder = await BitmapDecoder.CreateAsync(stream);
            var size = (uint)Ground3D.TileSize;
            var pixels = await decoder.GetPixelDataAsync(BitmapPixelFormat.Bgra8, BitmapAlphaMode.Ignore,
                new BitmapTransform { ScaledWidth = size, ScaledHeight = size }, ExifOrientationMode.IgnoreExifOrientation, ColorManagementMode.DoNotColorManage);
            return pixels.DetachPixelData();
        }
        catch (Exception)
        {
            return null;
        }
    }

    private void Furnish(Origin3D origin, float metresPerPixel)
    {
        var palette = Palette;
        _moving = Geometry3D.Path(_path, origin, palette, _selected, metresPerPixel)
            .Concat(Geometry3D.Track(_track, origin, palette.Track, metresPerPixel))
            .Concat(_vehicles.SelectMany(vehicle => Geometry3D.Vehicle(vehicle, origin, palette, metresPerPixel)))
            .SelectMany(Geometry3D.Faces)
            .ToList();
        _labelPoints = Geometry3D.Labels(_path, origin).ToList();
        _movingScale = metresPerPixel;
    }

    private Point? OnScreen(Vector3 at) => _raster.Project(at) is { } seen ? new Point(seen.X / _renderScale, seen.Y / _renderScale) : null;

    private void PlaceLabels()
    {
        var placed = _labelPoints
            .Select(label => (label.Text, At: OnScreen(label.At) is { X: >= 0, Y: >= 0 } at && at.X < ActualWidth && at.Y < ActualHeight ? at : (Point?)null))
            .ToList();
        if (_labels.Children.Count != placed.Count)
        {
            _labels.Children.Clear();
            placed.ForEach(_ => _labels.Children.Add(FormLook.Text("", "FontSizeBody", "TextPrimaryBrush", true)));
        }
        placed.Select((label, index) => (label, View: (TextBlock)_labels.Children[index])).ToList().ForEach(entry =>
        {
            entry.View.Text = entry.label.Text;
            entry.View.Visibility = FormLook.Shown(entry.label.At is not null);
            Canvas.SetLeft(entry.View, entry.label.At?.X ?? 0);
            Canvas.SetTop(entry.View, entry.label.At?.Y ?? 0);
        });
    }

    private void Pick(Point tap)
    {
        if (_origin is not { } origin) return;
        var markers = _path.Markers;
        var hit = markers
            .Select((marker, index) => (Index: index, At: OnScreen(origin.Metres(marker.At))))
            .Where(seen => seen.At is not null)
            .Select(seen => (seen.Index, Apart: Math.Sqrt(Math.Pow(seen.At!.Value.X - tap.X, 2) + Math.Pow(seen.At!.Value.Y - tap.Y, 2))))
            .Where(seen => seen.Apart <= PickRadius)
            .OrderBy(seen => seen.Apart)
            .Select(seen => (int?)seen.Index)
            .FirstOrDefault();
        _selected = hit is int picked
            ? _selected.Where(index => markers[index].Mission != markers[picked].Mission).ToImmutableHashSet().Add(picked)
            : ImmutableHashSet<int>.Empty;
        Moved();
    }

    private void OnPressed(object sender, PointerRoutedEventArgs e)
    {
        var point = e.GetCurrentPoint(this);
        var modified = (e.KeyModifiers & (VirtualKeyModifiers.Control | VirtualKeyModifiers.Shift)) != 0;
        var pan = point.Properties.IsRightButtonPressed || point.Properties.IsMiddleButtonPressed || modified;
        _drag = new Drag(e.Pointer.PointerId, point.Position, point.Position, pan, false);
        CapturePointer(e.Pointer);
        Focus(FocusState.Pointer);
        e.Handled = true;
    }

    private void OnMoved(object sender, PointerRoutedEventArgs e)
    {
        if (_drag is not { } drag || drag.Pointer != e.Pointer.PointerId) return;
        var at = e.GetCurrentPoint(this).Position;
        var moved = drag.Moved || Math.Abs(at.X - drag.Start.X) + Math.Abs(at.Y - drag.Start.Y) > TapSlop;
        var (dx, dy) = ((float)(at.X - drag.Last.X), (float)(at.Y - drag.Last.Y));
        if (moved && drag.Pan) Pan(dx, dy);
        else if (moved) Move(camera => camera.Orbited(-dx * TurnPerPixel, -dy * LeanPerPixel));
        _drag = drag with { Last = at, Moved = moved };
        e.Handled = true;
    }

    private void OnReleased(object sender, PointerRoutedEventArgs e)
    {
        if (_drag is { Moved: false, Pan: false } drag && drag.Pointer == e.Pointer.PointerId) Pick(drag.Start);
        _drag = null;
        ReleasePointerCapture(e.Pointer);
        e.Handled = true;
    }

    private void OnWheel(object sender, PointerRoutedEventArgs e)
    {
        var notches = e.GetCurrentPoint(this).Properties.MouseWheelDelta / 120f;
        Move(camera => camera.Zoomed(MathF.Pow(ZoomStep, notches)));
        e.Handled = true;
    }

    private void Pan(float dx, float dy)
    {
        if (Camera() is not { } camera) return;
        _follow = false;
        _camera = camera.Panned(new Vector2(dx, dy), Camera3D.Focal((int)ActualHeight));
        Invalidate();
    }

    protected override void OnKeyDown(KeyRoutedEventArgs e)
    {
        Func<Camera3D, Camera3D>? change = e.Key switch
        {
            VirtualKey.Left => camera => camera.Orbited(KeyTurn, 0),
            VirtualKey.Right => camera => camera.Orbited(-KeyTurn, 0),
            VirtualKey.Up => camera => camera.Orbited(0, KeyLean),
            VirtualKey.Down => camera => camera.Orbited(0, -KeyLean),
            VirtualKey.Add or VirtualKey.PageUp or PlusKey => camera => camera.Zoomed(ZoomStep),
            VirtualKey.Subtract or VirtualKey.PageDown or MinusKey => camera => camera.Zoomed(1 / ZoomStep),
            _ => null,
        };
        if (e.Key == VirtualKey.Home) FollowVehicle();
        else if (change is not null) Move(change);
        else
        {
            base.OnKeyDown(e);
            return;
        }
        e.Handled = true;
    }
}
