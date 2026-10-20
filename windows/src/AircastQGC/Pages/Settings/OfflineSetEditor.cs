using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Shapes;
using Windows.Foundation;

namespace AircastQGC.Pages.Settings;

internal sealed record OfflineRegion(double West, double North, double East, double South)
{
    private const double KmPerDegree = 111.32;

    public OfflineRegion Ordered => new(Math.Min(West, East), Math.Max(North, South), Math.Max(West, East), Math.Min(North, South));

    public Geo Centre => new((North + South) / 2, (West + East) / 2);

    public OfflineRegion Shifted(double latitude, double longitude) => new(West + longitude, North + latitude, East + longitude, South + latitude);

    public string Span => string.Format(CultureInfo.CurrentCulture, "{0:F1} × {1:F1} km",
        Math.Abs(East - West) * KmPerDegree * Math.Cos(Centre.Latitude * Math.PI / 180),
        Math.Abs(North - South) * KmPerDegree);

    public string Args => string.Join(",", new[] { West, North, East, South }.Select(c => c.ToString("F7", CultureInfo.InvariantCulture)));
}

internal sealed record OfflineSeed(IReadOnlyList<string> Maps, string MapType, string Name, IReadOnlyList<string> Taken, int Min, int Max, (Geo Centre, double Zoom)? Camera)
{
    public const string MinZoomSetting = "settings.offlineMapsSettings.minZoomLevelDownload";
    public const string MaxZoomSetting = "settings.offlineMapsSettings.maxZoomLevelDownload";
    private const int DefaultMinZoom = 13;
    private const int DefaultMaxZoom = 19;
    private const double FieldZoom = 15;

    public static OfflineSeed Read()
    {
        var view = Bridge.Get(OfflineMapsPanel.View);
        return new OfflineSeed(
            MapUiJson.Texts(view["mapList"]),
            MapNative.CurrentType(),
            MapUiJson.Text(view["uniqueName"]),
            MapUiJson.Texts(view["takenNames"]),
            (int)(MapUiJson.Number(Bridge.Get(MinZoomSetting)["value"]) ?? DefaultMinZoom),
            (int)(MapUiJson.Number(Bridge.Get(MaxZoomSetting)["value"]) ?? DefaultMaxZoom),
            MapUiJson.VehiclePosition() is { } vehicle ? (new Geo(vehicle.Latitude, vehicle.Longitude), FieldZoom)
            : GcsCentre() is { } gcs ? (gcs, FieldZoom)
            : MapView.FlightCamera);
    }

    private static Geo? GcsCentre()
    {
        var position = Bridge.Get(GcsPositionPanel.Position);
        var coordinate = position["value"] as JsonObject ?? position;
        return MapUiJson.Flag(coordinate["valid"])
            && MapUiJson.Number(coordinate["latitude"]) is { } latitude
            && MapUiJson.Number(coordinate["longitude"]) is { } longitude
                ? new Geo(latitude, longitude)
                : null;
    }
}

internal sealed class OfflineSetEditor : UserControl
{
    private enum Grip { NorthWest, NorthEast, SouthEast, SouthWest, Middle }

    private const string Start = "offlineMaps.startDownload";
    private const string Subject = "The tile cache";
    private const int LowestZoom = 1;
    private const int HighestZoom = 20;
    private const double WorldZoom = 3;
    private const double Inset = 0.15;
    private const double FormWidth = 340;
    private const double StackBelow = 760;
    private const double NarrowMapHeight = 320;
    private const double GripSize = 16;
    private const double MiddleGripSize = 28;
    private const double PreviewSize = 150;

    private readonly OfflineSeed _seed;
    private readonly Action _close;
    private readonly MapView _map = new() { PlanMode = true };
    private readonly Canvas _overlay = new();
    private readonly Polygon _outline;
    private readonly IReadOnlyList<(Grip Grip, FrameworkElement View)> _grips;
    private readonly TextBox _name;
    private readonly ComboBox _mapType;
    private readonly NumberBox _minZoom;
    private readonly NumberBox _maxZoom;
    private readonly CheckBox _elevation = new() { Content = "Fetch elevation data", IsChecked = true, FontFamily = PreflightLook.Font };
    private readonly TextBlock _span = FormLook.Caption("");
    private readonly StackPanel _estimate = new() { Spacing = 4 };
    private readonly TextBlock _error = FormLook.Caption("", "CriticalBrush");
    private readonly Button _download;
    private readonly Button _previewToggle;
    private readonly StackPanel _previews = new() { Orientation = Orientation.Horizontal, Spacing = 8, Visibility = Visibility.Collapsed };
    private readonly MapView _minPreview = Preview();
    private readonly MapView _maxPreview = Preview();
    private readonly Grid _root = new() { Padding = new Thickness(24, 0, 32, 24), ColumnSpacing = 16, RowSpacing = 16 };
    private readonly ScrollViewer _form;
    private OfflineRegion? _edges;
    private (Grip Grip, Geo From)? _drag;
    private Size _fitted;
    private bool _touched;
    private int _version;

    public OfflineSetEditor(OfflineSeed seed, Action close)
    {
        _seed = seed;
        _close = close;
        _map.TileType = seed.MapType;
        _minPreview.TileType = seed.MapType;
        _maxPreview.TileType = seed.MapType;
        _outline = new Polygon
        {
            Stroke = FormLook.Brush("AccentBrush"),
            StrokeThickness = 2,
            Fill = new SolidColorBrush(((SolidColorBrush)FormLook.Brush("AccentBrush")).Color) { Opacity = 0.12 },
            IsHitTestVisible = false,
        };
        _grips = Enum.GetValues<Grip>().Select(grip => (grip, GripView(grip))).ToList();
        FormLook.Fill(_overlay, _grips.Select(grip => grip.View).Prepend(_outline));
        _name = FormLook.Field(seed.Name, "Name");
        _mapType = FormLook.Combo(seed.Maps, seed.Maps.ToList().IndexOf(seed.MapType), double.NaN, seed.MapType);
        _mapType.Header = "Map type";
        _mapType.HorizontalAlignment = HorizontalAlignment.Stretch;
        _minZoom = Zoom("Min zoom", seed.Min);
        _maxZoom = Zoom("Max zoom", seed.Max);
        _download = FormLook.Button("Download", Download, ButtonLook.Accent);
        _download.IsEnabled = false;
        _previewToggle = FormLook.Button("Show zoom previews", TogglePreviews);
        AutomationProperties.SetName(_name, "Name");
        AutomationProperties.SetName(_mapType, "Map type");
        FormLook.Fill(_previews, [Labelled(_minPreview, "Min zoom"), Labelled(_maxPreview, "Max zoom")]);
        _form = new ScrollViewer
        {
            VerticalScrollBarVisibility = ScrollBarVisibility.Auto,
            Content = FormLook.Stack(10,
                _name,
                _mapType,
                FormLook.Caption("Drag the corners of the rectangle to set the area, or the middle handle to move it."),
                _span,
                FormLook.Line(12, _minZoom, _maxZoom),
                _previewToggle,
                _previews,
                _elevation,
                _estimate,
                _error,
                FormLook.Line(8, _download, FormLook.Button("Cancel", close))),
        };
        _root.ColumnDefinitions.Add(new ColumnDefinition());
        _root.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(FormWidth) });
        _root.RowDefinitions.Add(new RowDefinition());
        _root.RowDefinitions.Add(new RowDefinition { Height = new GridLength(0) });
        _root.Children.Add(MapFrame());
        _root.Children.Add(_form);
        Arrange(true);
        Content = _root;
        _root.SizeChanged += (_, e) => Arrange(e.NewSize.Width >= StackBelow);
        _map.ViewChanged += (_, _) =>
        {
            var size = new Size(_map.ActualWidth, _map.ActualHeight);
            if (!_touched && size != _fitted)
            {
                _fitted = size;
                _edges = InView();
                Estimate();
            }
            Place();
        };
        _overlay.SizeChanged += (_, e) => _overlay.Clip = new RectangleGeometry { Rect = new Rect(0, 0, e.NewSize.Width, e.NewSize.Height) };
        var (centre, zoom) = seed.Camera ?? (new Geo(0, 0), WorldZoom);
        _map.CenterOn(centre.Latitude, centre.Longitude, zoom);
        _name.TextChanged += (_, _) => Estimate();
        _mapType.SelectionChanged += (_, _) =>
        {
            new[] { _map, _minPreview, _maxPreview }.ToList().ForEach(map => map.TileType = Chosen());
            Estimate();
        };
        _minZoom.ValueChanged += (_, _) => ZoomChanged();
        _maxZoom.ValueChanged += (_, _) => ZoomChanged();
        FormLook.Toggled(_elevation, _ => Estimate());
    }

    private OfflineRegion? Region => _edges?.Ordered;

    private (int Low, int High) Levels() => (Math.Min(Level(_minZoom), Level(_maxZoom)), Math.Max(Level(_minZoom), Level(_maxZoom)));

    private string Chosen() => _mapType.SelectedIndex >= 0 ? _seed.Maps[_mapType.SelectedIndex] : _seed.MapType;

    private static int Level(NumberBox box) => (int)Math.Round(double.IsNaN(box.Value) ? LowestZoom : box.Value);

    private void Arrange(bool wide)
    {
        _root.ColumnDefinitions[1].Width = new GridLength(wide ? FormWidth : 0);
        _root.RowDefinitions[0].Height = wide ? new GridLength(1, GridUnitType.Star) : new GridLength(NarrowMapHeight);
        _root.RowDefinitions[1].Height = wide ? new GridLength(0) : new GridLength(1, GridUnitType.Star);
        Grid.SetColumn(_form, wide ? 1 : 0);
        Grid.SetRow(_form, wide ? 0 : 1);
    }

    private UIElement MapFrame()
    {
        var buttons = FormLook.Stack(4,
            MapButton("", "Zoom in", _map.ZoomIn),
            MapButton("", "Zoom out", _map.ZoomOut),
            MapButton("", "Centre on vehicle", () => _map.CenterOnVehicle()),
            MapButton("", "Fit area to view", () =>
            {
                _touched = true;
                _edges = InView();
                Place();
                Estimate();
            }));
        buttons.HorizontalAlignment = HorizontalAlignment.Right;
        buttons.VerticalAlignment = VerticalAlignment.Top;
        buttons.Margin = new Thickness(8);
        return new Border
        {
            CornerRadius = FormLook.Radius("RadiusOverlay"),
            BorderBrush = FormLook.Brush("CardStrokeBrush"),
            BorderThickness = new Thickness(1),
            Child = new Grid { Children = { _map, _overlay, buttons } },
        };
    }

    private static Button MapButton(string glyph, string name, Action onClick)
    {
        var button = new Button { Style = (Style)Application.Current.Resources["MapControl"], Content = glyph };
        AutomationProperties.SetName(button, name);
        ToolTipService.SetToolTip(button, name);
        button.Click += (_, _) => onClick();
        return button;
    }

    private OfflineRegion InView()
    {
        var (width, height) = (_map.ActualWidth, _map.ActualHeight);
        var northWest = _map.GeoOf(new Point(width * Inset, height * Inset));
        var southEast = _map.GeoOf(new Point(width * (1 - Inset), height * (1 - Inset)));
        return new OfflineRegion(northWest.Longitude, northWest.Latitude, southEast.Longitude, southEast.Latitude);
    }

    private FrameworkElement GripView(Grip grip)
    {
        var middle = grip == Grip.Middle;
        var size = middle ? MiddleGripSize : GripSize;
        var view = new Grid
        {
            Width = size,
            Height = size,
            Children =
            {
                new Ellipse
                {
                    Fill = FormLook.Brush(middle ? "AccentBrush" : "TextPrimaryBrush"),
                    Stroke = FormLook.Brush("AccentOnBrush"),
                    StrokeThickness = 2,
                },
            },
        };
        if (middle) view.Children.Add(new FontIcon { Glyph = "", FontFamily = FormLook.Icons, FontSize = 14, Foreground = FormLook.Brush("AccentOnBrush"), IsHitTestVisible = false });
        var name = middle ? "Move the area" : $"Drag the {Words(grip)} corner";
        AutomationProperties.SetName(view, name);
        ToolTipService.SetToolTip(view, name);
        view.PointerPressed += (_, e) =>
        {
            if (!e.GetCurrentPoint(view).Properties.IsLeftButtonPressed) return;
            _drag = (grip, _map.GeoOf(e.GetCurrentPoint(_map).Position));
            _touched = true;
            view.CapturePointer(e.Pointer);
            e.Handled = true;
        };
        view.PointerMoved += (_, e) =>
        {
            if (_drag is not { } drag || drag.Grip != grip || _edges is not { } edges) return;
            var at = _map.GeoOf(e.GetCurrentPoint(_map).Position);
            _edges = Moved(edges, grip, drag.From, at);
            _drag = (grip, at);
            Place();
            e.Handled = true;
        };
        view.PointerReleased += (_, e) =>
        {
            if (_drag?.Grip != grip) return;
            view.ReleasePointerCapture(e.Pointer);
            e.Handled = true;
        };
        view.PointerCaptureLost += (_, _) =>
        {
            if (_drag?.Grip != grip) return;
            _drag = null;
            _edges = _edges?.Ordered;
            Place();
            Estimate();
        };
        return view;
    }

    private static string Words(Grip grip) => grip switch
    {
        Grip.NorthWest => "north-west",
        Grip.NorthEast => "north-east",
        Grip.SouthEast => "south-east",
        _ => "south-west",
    };

    private static OfflineRegion Moved(OfflineRegion edges, Grip grip, Geo from, Geo to) => grip switch
    {
        Grip.NorthWest => edges with { North = to.Latitude, West = to.Longitude },
        Grip.NorthEast => edges with { North = to.Latitude, East = to.Longitude },
        Grip.SouthEast => edges with { South = to.Latitude, East = to.Longitude },
        Grip.SouthWest => edges with { South = to.Latitude, West = to.Longitude },
        _ => edges.Shifted(to.Latitude - from.Latitude, to.Longitude - from.Longitude),
    };

    private void Place()
    {
        _overlay.Visibility = FormLook.Shown(_edges is not null);
        if (_edges is not { } edges) return;
        var corners = new Dictionary<Grip, Geo>
        {
            [Grip.NorthWest] = new(edges.North, edges.West),
            [Grip.NorthEast] = new(edges.North, edges.East),
            [Grip.SouthEast] = new(edges.South, edges.East),
            [Grip.SouthWest] = new(edges.South, edges.West),
            [Grip.Middle] = edges.Centre,
        };
        var screen = corners.ToDictionary(corner => corner.Key, corner => _map.ScreenOf(corner.Value));
        var outline = new PointCollection();
        new[] { Grip.NorthWest, Grip.NorthEast, Grip.SouthEast, Grip.SouthWest }.ToList().ForEach(grip => outline.Add(screen[grip]));
        _outline.Points = outline;
        _grips.ToList().ForEach(grip =>
        {
            Canvas.SetLeft(grip.View, screen[grip.Grip].X - grip.View.Width / 2);
            Canvas.SetTop(grip.View, screen[grip.Grip].Y - grip.View.Height / 2);
        });
        FormLook.Say(_span, $"Area: {edges.Span}");
    }

    private async void Estimate()
    {
        var at = ++_version;
        var (type, region, (low, high), fetch, typed) = (Chosen(), Region, Levels(), _elevation.IsChecked == true, _name.Text.Trim());
        var read = region is null ? null : await FormWrites.Off(() =>
            Bridge.Get($"{OfflineMapsPanel.View}({type},{region.Args},{low},{high},{(fetch ? "true" : "false")})")["estimate"] as JsonObject);
        if (at != _version) return;
        var nameTaken = _seed.Taken.Contains(typed);
        var tooMany = MapUiJson.Flag(read?["tooMany"]);
        FormLook.Fill(_estimate,
        [
            read is null ? null : FormLook.Pair("Tile count", MapUiJson.Text(read["tileCountText"])),
            read is null ? null : FormLook.Pair("Est size", MapUiJson.Text(read["tileSizeText"])),
            tooMany ? FormLook.Caption("Too many tiles", "CriticalBrush") : null,
            nameTaken ? FormLook.Caption("Tile set with this name already exists", "CriticalBrush") : null,
        ]);
        _download.IsEnabled = region is not null && read is not null && !tooMany && !nameTaken && typed.Length > 0;
        if (region is not null) Recentre(region.Centre, low, high);
    }

    private void ZoomChanged()
    {
        var (low, high) = Levels();
        FormWrites.Off(() =>
        {
            FormWrites.Set(OfflineSeed.MinZoomSetting, low);
            FormWrites.Set(OfflineSeed.MaxZoomSetting, high);
        });
        Estimate();
    }

    private void TogglePreviews()
    {
        var shown = _previews.Visibility != Visibility.Visible;
        _previews.Visibility = FormLook.Shown(shown);
        ((TextBlock)_previewToggle.Content).Text = shown ? "Hide zoom previews" : "Show zoom previews";
        AutomationProperties.SetName(_previewToggle, shown ? "Hide zoom previews" : "Show zoom previews");
        if (Region is { } region)
        {
            var (low, high) = Levels();
            Recentre(region.Centre, low, high);
        }
    }

    private void Recentre(Geo centre, int low, int high)
    {
        if (_previews.Visibility != Visibility.Visible) return;
        _minPreview.CenterOn(centre.Latitude, centre.Longitude, low);
        _maxPreview.CenterOn(centre.Latitude, centre.Longitude, high);
    }

    private async void Download()
    {
        if (Region is not { } region) return;
        _download.IsEnabled = false;
        var (low, high) = Levels();
        var (typed, type, fetch) = (_name.Text.Trim(), Chosen(), _elevation.IsChecked == true);
        var refused = await FormWrites.Off(() => FormWrites.Call(Start, Subject,
            typed, type, region.West, region.North, region.East, region.South, low, high, fetch));
        FormLook.Say(_error, refused);
        if (refused is null) _close();
        else _download.IsEnabled = true;
    }

    private static MapView Preview() => new()
    {
        PlanMode = true,
        Width = PreviewSize,
        Height = PreviewSize,
        IsHitTestVisible = false,
    };

    private static UIElement Labelled(MapView preview, string label)
    {
        var caption = FormLook.Caption(label, "TextPrimaryBrush");
        caption.HorizontalAlignment = HorizontalAlignment.Center;
        caption.VerticalAlignment = VerticalAlignment.Bottom;
        caption.Margin = new Thickness(4);
        return new Border
        {
            CornerRadius = FormLook.Radius("RadiusControl"),
            BorderBrush = FormLook.Brush("CardStrokeBrush"),
            BorderThickness = new Thickness(1),
            Child = new Grid { Children = { preview, caption } },
        };
    }

    private static NumberBox Zoom(string header, int value)
    {
        var box = new NumberBox
        {
            Header = header,
            Value = Math.Clamp(value, LowestZoom, HighestZoom),
            Minimum = LowestZoom,
            Maximum = HighestZoom,
            SmallChange = 1,
            LargeChange = 5,
            SpinButtonPlacementMode = NumberBoxSpinButtonPlacementMode.Compact,
            ValidationMode = NumberBoxValidationMode.InvalidInputOverwritten,
            Width = 150,
            FontFamily = PreflightLook.Font,
        };
        AutomationProperties.SetName(box, header);
        return box;
    }
}
