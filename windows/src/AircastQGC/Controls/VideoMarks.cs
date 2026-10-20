using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Shapes;
using Windows.Foundation;

namespace AircastQGC.Controls;

public sealed partial class VideoMarks : UserControl
{
    private const string VideoView = "view.video";
    private const string CameraView = "view.camera";
    private const string FitView = "view.control(settings.videoSettings.videoFit)";
    private const string GridView = "view.control(settings.videoSettings.gridLines)";
    private const string StepZoom = "vehicle.cameraManager.currentCameraInstance.stepZoom";
    private const string RouteBrush = "VideoArRouteBrush";
    private const string EdgeBrush = "VideoEdgeBrush";
    private const string BoxBrush = "DetectionBrush";
    private const string TargetBrush = "DetectionTargetBrush";
    private const double RouteWidth = 3;
    private const double EdgeWidth = 2;
    private const double MarkRadius = 7;
    private const double LabelGap = 4;
    private const double LabelWidth = 220;
    private const double GridAlpha = 0.5;
    private const double RingTextSize = 20;
    private const double RingTextAlpha = 0.8;
    private const double TroubleInset = 8;
    private const double TroubleAbovePill = 36;

    private readonly Grid _root = new();
    private readonly Canvas _stage = new() { IsHitTestVisible = false };
    private readonly Grid _picture = new();
    private readonly Canvas _grid = new();
    private readonly Canvas _ar = new();
    private readonly Canvas _obstacle = new();
    private readonly Canvas _boxes = new();
    private readonly TextBlock _troubleText = FormLook.Caption("", "CriticalBrush");
    private readonly Border _trouble;
    private readonly Button _exit;
    private JsonObject _video = new();
    private ArReading? _arReading;
    private ObstacleDrawing? _drawing;
    private Detections? _detections;
    private (double Width, double Height) _shown;
    private int _fit = VideoSurface.FitHeight;
    private double? _aspectSetting;
    private bool _gridOn;
    private bool _zoomable;
    private bool _expanded;
    private bool _fullScreen;
    private UIElement? _area;
    private ImmutableArray<UIElement> _videoParts = [];
    private ImmutableDictionary<uint, Point> _touches = ImmutableDictionary<uint, Point>.Empty;
    private double _pinchScale = 1;
    private int _pinchSent;
    private (bool Before, bool Last) _pressedOnPicture;

    public VideoMarks()
    {
        _trouble = new Border
        {
            HorizontalAlignment = HorizontalAlignment.Left,
            VerticalAlignment = VerticalAlignment.Bottom,
            Padding = new Thickness(4, 2, 4, 2),
            IsHitTestVisible = false,
            Background = FormLook.Brush("CriticalBgBrush"),
            Child = _troubleText,
            Visibility = Visibility.Collapsed,
        };
        _exit = FormLook.IconButton("", "Exit full screen", () => ExitFullScreenRequested?.Invoke(this, EventArgs.Empty));
        PreflightLook.Standard(_exit);
        _exit.HorizontalAlignment = HorizontalAlignment.Right;
        _exit.VerticalAlignment = VerticalAlignment.Top;
        _exit.Margin = new Thickness(16);
        _exit.Visibility = Visibility.Collapsed;
        new UIElement[] { _grid, _ar, _obstacle, _boxes }.ToList().ForEach(_picture.Children.Add);
        _stage.Children.Add(_picture);
        _root.Children.Add(_stage);
        _root.Children.Add(_trouble);
        _root.Children.Add(_exit);
        _root.SizeChanged += (_, _) => Place();
        Content = _root;
        AutomationProperties.SetName(this, "Video marks");

        Live.Watch(this, VideoView, TakeVideo);
        Live.Watch(this, FitView, view => Take(() => _fit = VideoJson.Number(view["value"]) is double mode ? (int)mode : VideoSurface.FitHeight, Place));
        Live.Watch(this, VideoSurface.AspectView, view => Take(() => _aspectSetting = VideoJson.Number(view["value"]), Place));
        Live.Watch(this, GridView, view => Take(() => _gridOn = VideoJson.Flag(view, "value"), DrawGrid));
        Live.Watch(this, CameraView, view => _zoomable = VideoJson.Flag(view, "hasZoom"));
        Live.Watch(this, VideoMarksModel.ArView, view => Take(() => _arReading = VideoMarksModel.Ar(view), DrawAr));
        Live.Watch(this, ObstacleModel.View, view => Take(() => _drawing = ObstacleModel.Drawing(view), DrawObstacle));
        Live.Watch(this, VideoMarksModel.DetectionsView, view => Take(() => _detections = VideoMarksModel.Read(view), DrawBoxes));
    }

    public event EventHandler? ExitFullScreenRequested;

    public event EventHandler? FullScreenToggled;

    public bool Expanded
    {
        get => _expanded;
        set
        {
            _expanded = value;
            DrawAll();
        }
    }

    public bool FullScreen
    {
        get => _fullScreen;
        set
        {
            _fullScreen = value;
            _exit.Visibility = FormLook.Shown(value);
            DrawGrid();
        }
    }

    public void ZoomFrom(UIElement area, params UIElement[] video)
    {
        _area = area;
        _videoParts = [this, .. video];
        area.AddHandler(PointerWheelChangedEvent, new PointerEventHandler(OnWheel), false);
        area.AddHandler(PointerPressedEvent, new PointerEventHandler(OnTouchDown), true);
        area.AddHandler(PointerMovedEvent, new PointerEventHandler(OnTouchMove), true);
        area.AddHandler(PointerReleasedEvent, new PointerEventHandler(OnTouchUp), true);
        area.AddHandler(PointerCanceledEvent, new PointerEventHandler(OnTouchUp), true);
        area.AddHandler(PointerCaptureLostEvent, new PointerEventHandler(OnTouchUp), true);
        area.AddHandler(PointerPressedEvent, new PointerEventHandler(OnPress), true);
        area.AddHandler(DoubleTappedEvent, new DoubleTappedEventHandler(OnDoubleTap), true);
    }

    private bool Decoding { get; set; }

    private bool Zooms => _expanded && Decoding && _zoomable;

    private static void Take(Action read, Action draw)
    {
        read();
        draw();
    }

    private void TakeVideo(JsonObject view)
    {
        _video = view;
        Decoding = VideoJson.Flag(view, "decoding");
        Place();
    }

    private double Aspect => VideoSurface.PictureAspect(_video, _aspectSetting);

    private void Place()
    {
        var (width, height) = (_root.ActualWidth, _root.ActualHeight);
        if (width <= 0 || height <= 0) return;
        var (shownWidth, shownHeight) = VideoSurface.ContentSize(width, height, Aspect, _fit);
        _shown = (shownWidth, shownHeight);
        _root.Clip = new RectangleGeometry { Rect = new Rect(0, 0, width, height) };
        _picture.Width = shownWidth;
        _picture.Height = shownHeight;
        _picture.Clip = new RectangleGeometry { Rect = new Rect(0, 0, shownWidth, shownHeight) };
        Canvas.SetLeft(_picture, (width - shownWidth) / 2);
        Canvas.SetTop(_picture, (height - shownHeight) / 2);
        var (sideways, upright) = (Math.Max(0, (width - shownWidth) / 2), Math.Max(0, (height - shownHeight) / 2));
        _trouble.Margin = new Thickness(sideways + TroubleInset, 0, 0, upright + TroubleAbovePill);
        DrawAll();
    }

    private void DrawAll()
    {
        DrawGrid();
        DrawAr();
        DrawObstacle();
        DrawBoxes();
    }

    private void DrawGrid()
    {
        var (width, height) = _shown;
        var brush = OverlayInk.Brush("TextPrimaryBrush", GridAlpha);
        FormLook.Fill(_grid, Decoding && _gridOn && !_fullScreen
            ? VideoMarksModel.GridFractions.SelectMany(at => new UIElement[]
            {
                OverlayInk.Line(new Point(width * at, 0), new Point(width * at, height), brush, 1),
                OverlayInk.Line(new Point(0, height * at), new Point(width, height * at), brush, 1),
            })
            : []);
    }

    private void DrawAr() =>
        FormLook.Fill(_ar, Decoding && _expanded && _arReading is { } reading ? ArShapes(reading) : []);

    private IEnumerable<UIElement> ArShapes(ArReading reading)
    {
        var (width, height) = _shown;
        Point At(ArSpot spot)
        {
            var (x, y) = VideoMarksModel.ArOffset(spot, width, height);
            return new Point(x, y);
        }
        var edge = OverlayInk.Brush(EdgeBrush);
        var route = OverlayInk.Brush(RouteBrush);
        var legs = reading.Route.SelectMany(leg => new UIElement[]
        {
            OverlayInk.Line(At(leg.From), At(leg.To), edge, RouteWidth + EdgeWidth * 2),
            OverlayInk.Line(At(leg.From), At(leg.To), route, RouteWidth),
        });
        IEnumerable<UIElement> Marked(ArMark mark, string fill)
        {
            var at = At(mark.At);
            var caption = VideoMarksModel.ArCaption(mark);
            var size = PreflightLook.Size("FontSizeCaption");
            var label = caption.Length == 0
                ? []
                : OverlayInk.Outlined(caption, new Point(at.X - LabelWidth / 2, at.Y - MarkRadius - LabelGap - OverlayInk.LineHeight(size)), size, "TextPrimaryBrush", 1, 1, LabelWidth, bold: false);
            return new UIElement[] { OverlayInk.Dot(at, MarkRadius + EdgeWidth, edge), OverlayInk.Dot(at, MarkRadius, OverlayInk.Brush(fill)) }.Concat(label);
        }
        var marks = reading.Waypoints.SelectMany(mark => Marked(mark, "TextPrimaryBrush"));
        var home = reading.Home is { } shown ? Marked(shown, "SuccessBrush") : [];
        return legs.Concat(marks).Concat(home).ToList();
    }

    private void DrawObstacle()
    {
        var (width, height) = _shown;
        var rings = Decoding && _drawing is { } drawing ? ObstacleModel.VideoRings(drawing, height, _expanded) : null;
        FormLook.Fill(_obstacle, rings is null ? [] : RingShapes(rings, new Point(width / 2, height / 2)));
    }

    private static IEnumerable<UIElement> RingShapes(VideoObstacleRings rings, Point centre)
    {
        Point Wide(double radius, double radians) => new(centre.X + 2 * radius * Math.Cos(radians), centre.Y + radius * Math.Sin(radians));
        Point Plain(double radius, double radians) => new(centre.X + radius * Math.Cos(radians), centre.Y + radius * Math.Sin(radians));
        UIElement Ring(VideoObstacleSegment segment)
        {
            var (from, to) = (segment.RadFrom - Math.PI / 2, segment.RadTo - Math.PI / 2);
            var ring = OverlayInk.Figure(Wide(segment.From, from), new PathSegment[]
            {
                new ArcSegment { Point = Wide(segment.From, to), Size = new Size(2 * segment.From, segment.From), SweepDirection = SweepDirection.Clockwise },
                new LineSegment { Point = Wide(segment.To, to) },
                new ArcSegment { Point = Wide(segment.To, from), Size = new Size(2 * segment.To, segment.To), SweepDirection = SweepDirection.Counterclockwise },
            }, closed: true);
            return OverlayInk.Radial(ring, centre, 2 * rings.GradientTo, rings.GradientTo, rings.GradientFrom, rings.GradientTo, ObstacleModel.VideoStops);
        }
        IEnumerable<UIElement> Label(VideoObstacleSegment segment)
        {
            if (segment.Label is not { Length: > 0 } text) return [];
            var (top, end) = (Plain(segment.From, segment.RadFrom - Math.PI / 2), Plain(segment.From, segment.RadTo - Math.PI / 2));
            var middle = new Point((top.X + end.X) / 2, (top.Y + end.Y) / 2);
            return OverlayInk.Outlined(text, new Point(2 * middle.X - centre.X, middle.Y - RingTextSize), RingTextSize, "TextPrimaryBrush", RingTextAlpha, 2);
        }
        return rings.Segments.Select(Ring).Concat(rings.Segments.SelectMany(Label)).ToList();
    }

    private void DrawBoxes()
    {
        var (width, height) = _shown;
        var boxes = Decoding ? VideoMarksModel.VisibleBoxes(_detections) : [];
        FormLook.Fill(_boxes, boxes.SelectMany(box => BoxShapes(box, width, height)));
        FormLook.Say(_troubleText, Decoding && VideoMarksModel.Trouble(_detections) is { } reason ? $"Detections: {reason}" : null);
        _trouble.Visibility = _troubleText.Visibility;
    }

    private static IEnumerable<UIElement> BoxShapes(DetectionBox box, double width, double height)
    {
        var colour = FormLook.Brush(box.Target ? TargetBrush : BoxBrush);
        var frame = new Rectangle
        {
            Width = Math.Max(0, box.W * width),
            Height = Math.Max(0, box.H * height),
            Stroke = colour,
            StrokeThickness = box.Target ? 4 : 2,
        };
        Canvas.SetLeft(frame, box.X * width);
        Canvas.SetTop(frame, box.Y * height);
        var caption = VideoMarksModel.BoxCaption(box);
        if (caption.Length == 0) return [frame];
        var tag = new Border { Background = colour, Padding = new Thickness(3, 0, 3, 0), Child = FormLook.Caption(caption, "AccentOnBrush") };
        Canvas.SetLeft(tag, box.X * width);
        Canvas.SetTop(tag, box.Y * height);
        return [frame, tag];
    }

    private bool OverVideo(object source) => Lineage(source as DependencyObject).Any(IsVideoPart);

    private bool OnPicture(object source) =>
        _expanded && OverVideo(source) && !Lineage(source as DependencyObject).TakeWhile(node => !IsVideoPart(node)).Any(node => node is Control { IsTabStop: true });

    private bool IsVideoPart(DependencyObject node) => node is UIElement element && _videoParts.Contains(element);

    private static IEnumerable<DependencyObject> Lineage(DependencyObject? node) =>
        node is null ? [] : Lineage(VisualTreeHelper.GetParent(node)).Prepend(node);

    private void OnPress(object sender, PointerRoutedEventArgs e) => _pressedOnPicture = (_pressedOnPicture.Last, OnPicture(e.OriginalSource));

    private void OnDoubleTap(object sender, DoubleTappedRoutedEventArgs e)
    {
        if (_pressedOnPicture != (true, true)) return;
        e.Handled = true;
        FullScreenToggled?.Invoke(this, EventArgs.Empty);
    }

    private static void Zoom(int step) => Task.Run(() => Bridge.Invoke(StepZoom, new JsonArray(step)));

    private void OnWheel(object sender, PointerRoutedEventArgs e)
    {
        if (_area is null || !Zooms || !OverVideo(e.OriginalSource)) return;
        var point = e.GetCurrentPoint(_area);
        if (point.Properties.IsHorizontalMouseWheel) return;
        Zoom(VideoMarksModel.WheelStep(point.Properties.MouseWheelDelta));
        e.Handled = true;
    }

    private void OnTouchDown(object sender, PointerRoutedEventArgs e)
    {
        if (_area is null || e.Pointer.PointerDeviceType != PointerDeviceType.Touch || !Zooms || !OverVideo(e.OriginalSource)) return;
        if (_touches.IsEmpty) (_pinchScale, _pinchSent) = (1, 0);
        _touches = _touches.SetItem(e.Pointer.PointerId, e.GetCurrentPoint(_area).Position);
    }

    private void OnTouchMove(object sender, PointerRoutedEventArgs e)
    {
        if (_area is null || !_touches.ContainsKey(e.Pointer.PointerId)) return;
        var before = Spread(_touches);
        _touches = _touches.SetItem(e.Pointer.PointerId, e.GetCurrentPoint(_area).Position);
        if (_touches.Count < 2) return;
        var after = Spread(_touches);
        _pinchScale = before > 0 && after > 0 ? _pinchScale * after / before : _pinchScale;
        var step = VideoMarksModel.PinchStep(_pinchScale);
        if (step != _pinchSent && Zooms) Zoom(step);
        _pinchSent = step;
    }

    private void OnTouchUp(object sender, PointerRoutedEventArgs e) => _touches = _touches.Remove(e.Pointer.PointerId);

    private static double Spread(ImmutableDictionary<uint, Point> touches) =>
        touches.Values.Take(2).ToList() is [var first, var second] ? Math.Sqrt(Math.Pow(first.X - second.X, 2) + Math.Pow(first.Y - second.Y, 2)) : 0;
}
