using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Shapes;
using Windows.Foundation;

namespace AircastQGC.Controls;

public sealed partial class VideoPayloadLayer : UserControl
{
    private const string VideoView = "view.video";
    private const string CameraView = "view.camera";
    private const string RadarView = "view.proximityRadar";
    private const string FitView = "view.control(settings.videoSettings.videoFit)";
    private const string StartTracking = "vehicle.cameraManager.currentCameraInstance.startTracking";
    private const double SectorSweep = 45;
    private const double SectorGap = 3.6;
    private const double ArcOffset = -112.5;
    private const double RadarOpacity = 0.5;
    private const double LabelWidth = 64;
    private static readonly TimeSpan HoldRepeat = TimeSpan.FromMilliseconds(100);
    private static readonly TimeSpan DragSpacing = TimeSpan.FromMilliseconds(50);

    private enum Kind { Track, Drag, Point, Hold }

    private sealed record Gesture(Kind Kind, Point From, Point Latest, bool Moved, bool Begun);

    private readonly string _client = $"video-payload.{Guid.NewGuid():N}";
    private readonly Grid _root = new();
    private readonly Canvas _drawing = new() { IsHitTestVisible = false };
    private readonly Canvas _bandLayer = new() { IsHitTestVisible = false };
    private readonly Rectangle _band;
    private readonly Button _aimPill;
    private readonly TextBlock _aimText = FormLook.Caption("", "TextPrimaryBrush");
    private readonly DispatcherQueueTimer _hold;
    private readonly Stopwatch _sinceDrag = Stopwatch.StartNew();
    private JsonObject _video = new();
    private GimbalState? _gimbal;
    private TrackingState? _tracking;
    private ProximityRadar? _radar;
    private Gesture? _gesture;
    private bool _expanded;
    private int _fit = VideoSurface.FitHeight;

    public VideoPayloadLayer()
    {
        _band = new Rectangle { Stroke = FormLook.Brush("AccentBrush"), StrokeThickness = 2, StrokeDashArray = new DoubleCollection { 4, 3 }, Visibility = Visibility.Collapsed };
        _bandLayer.Children.Add(_band);
        _aimText.Visibility = Visibility.Visible;
        _aimPill = new Button
        {
            Height = 28,
            Padding = new Thickness(12, 0, 12, 0),
            CornerRadius = new CornerRadius(14),
            HorizontalAlignment = HorizontalAlignment.Left,
            VerticalAlignment = VerticalAlignment.Top,
            Margin = new Thickness(16),
            Content = _aimText,
            Visibility = Visibility.Collapsed,
        };
        FormLook.Paint(_aimPill, ButtonLook.Standard);
        _aimPill.Click += (_, _) =>
        {
            if (_gimbal?.Aim is { } aim) GimbalCalls.ChooseAim(aim.Next);
        };
        _root.Children.Add(_drawing);
        _root.Children.Add(_bandLayer);
        _root.Children.Add(_aimPill);
        _root.PointerPressed += OnPressed;
        _root.PointerMoved += OnMoved;
        _root.PointerReleased += OnReleased;
        _root.PointerCanceled += (_, _) => EndGesture();
        _root.PointerCaptureLost += (_, _) => EndGesture();
        _root.SizeChanged += (_, e) =>
        {
            _root.Clip = new RectangleGeometry { Rect = new Rect(0, 0, e.NewSize.Width, e.NewSize.Height) };
            Draw();
        };
        Content = _root;
        _hold = DispatcherQueue.GetForCurrentThread().CreateTimer();
        _hold.Interval = HoldRepeat;
        _hold.Tick += (_, _) => HoldTick();
        Loaded += (_, _) => Start();
        Unloaded += (_, _) => Halt();
    }

    public event EventHandler? SwapRequested;

    public bool Expanded
    {
        get => _expanded;
        set
        {
            _expanded = value;
            EndGesture();
            Sync();
        }
    }

    private bool Decoding => VideoJson.Flag(_video, "decoding");

    private bool TakesInput => _expanded && Decoding && (_tracking?.CanAim == true || _gimbal is not null);

    private Painted Picture => PayloadJson.Aspect(_video) is double aspect
        ? PayloadJson.Centred(_root.ActualWidth, _root.ActualHeight, VideoSurface.ContentSize(_root.ActualWidth, _root.ActualHeight, aspect, _fit))
        : new Painted(0, 0, _root.ActualWidth, _root.ActualHeight);

    private void Start()
    {
        Watch("video", VideoView, view => _video = view);
        Watch("camera", CameraView, view => _tracking = PayloadJson.Tracking(view));
        Watch("gimbal", GimbalCalls.View, view => _gimbal = PayloadJson.Gimbal(view));
        Watch("radar", RadarView, view => _radar = PayloadJson.Radar(view));
        Watch("fit", FitView, view => _fit = VideoJson.Number(view["value"]) is double mode ? (int)mode : VideoSurface.FitHeight);
    }

    private void Watch(string name, string path, Action<JsonObject> take)
    {
        void Apply(JsonObject view)
        {
            take(view);
            Sync();
        }
        BridgeWatch.Watch($"{_client}.{name}", [path], Apply);
        Apply(Bridge.Get(path));
    }

    private void Halt()
    {
        if (IsLoaded) return;
        new[] { "video", "camera", "gimbal", "radar", "fit" }.ToList().ForEach(name => BridgeWatch.Stop($"{_client}.{name}"));
        EndGesture();
    }

    private void Sync()
    {
        _root.Background = TakesInput ? new SolidColorBrush(Colors.Transparent) : null;
        var aim = _expanded && Decoding ? _gimbal?.Aim : null;
        _aimPill.Visibility = VideoJson.Shown(aim is not null);
        _aimText.Text = aim?.Label ?? "";
        AutomationProperties.SetName(_aimPill, aim is null ? "Video gimbal mode" : $"Video gimbal mode, {aim.Label}");
        ToolTipService.SetToolTip(_aimPill, aim is null ? null : $"Switch to {aim.Next.Label}");
        Draw();
    }

    private void Draw()
    {
        var picture = Picture;
        FormLook.Fill(_drawing, Decoding ? RadarShapes(picture).Concat(TrackShapes(picture)) : []);
    }

    private IEnumerable<UIElement> RadarShapes(Painted picture)
    {
        if (_radar is not { } radar || picture.Width <= 0 || picture.Height <= 0) return [];
        var shortest = Math.Min(picture.Width, picture.Height);
        var ratio = shortest / 2 / radar.Range;
        var (scaleX, scaleY) = (picture.Width / shortest, picture.Height / shortest);
        var centre = new Point(picture.Left + picture.Width / 2, picture.Top + picture.Height / 2);
        var placed = radar.Sectors.Where(s => s.Meters is > 0).Select(s => (Sector: s, Radius: s.Meters!.Value * ratio)).ToList();
        var arcs = placed.Select(p => (UIElement)Arc(centre, p.Radius * scaleX, p.Radius * scaleY, ArcOffset + p.Sector.Bearing + SectorGap, SectorSweep - 2 * SectorGap, picture.Width / 100));
        var labels = placed.Select(p => (UIElement)Label(p.Sector.Text, new Point(
            centre.X + Math.Cos(Radians(p.Sector.Bearing - 90)) * p.Radius * scaleX,
            centre.Y + Math.Sin(Radians(p.Sector.Bearing - 90)) * p.Radius * scaleY)));
        return arcs.Concat(labels).ToList();
    }

    private IEnumerable<UIElement> TrackShapes(Painted picture)
    {
        if (_tracking?.Box is not { } box) return [];
        var frame = new Rectangle
        {
            Width = box.Width * picture.Width,
            Height = box.Height * picture.Height,
            Stroke = FormLook.Brush("AccentBrush"),
            StrokeThickness = 3,
        };
        Canvas.SetLeft(frame, picture.Left + box.X * picture.Width);
        Canvas.SetTop(frame, picture.Top + box.Y * picture.Height);
        return [frame];
    }

    private static Microsoft.UI.Xaml.Shapes.Path Arc(Point centre, double radiusX, double radiusY, double startDegrees, double sweepDegrees, double thickness)
    {
        Point On(double degrees) => new(centre.X + radiusX * Math.Cos(Radians(degrees)), centre.Y + radiusY * Math.Sin(Radians(degrees)));
        var figure = new PathFigure { StartPoint = On(startDegrees), IsClosed = false, IsFilled = false };
        figure.Segments.Add(new ArcSegment { Point = On(startDegrees + sweepDegrees), Size = new Size(radiusX, radiusY), SweepDirection = SweepDirection.Clockwise });
        var geometry = new PathGeometry();
        geometry.Figures.Add(figure);
        return new Microsoft.UI.Xaml.Shapes.Path
        {
            Data = geometry,
            Stroke = FormLook.Brush("CriticalBrush"),
            StrokeThickness = thickness,
            Opacity = RadarOpacity,
        };
    }

    private static TextBlock Label(string text, Point at)
    {
        var label = FormLook.Caption(text, "TextPrimaryBrush");
        label.FontWeight = FontWeights.Bold;
        label.Width = LabelWidth;
        label.TextAlignment = TextAlignment.Center;
        Canvas.SetLeft(label, at.X - LabelWidth / 2);
        Canvas.SetTop(label, at.Y - 8);
        return label;
    }

    private static double Radians(double degrees) => degrees * Math.PI / 180;

    private void OnPressed(object sender, PointerRoutedEventArgs e)
    {
        if (!TakesInput || e.OriginalSource is not (Grid or Canvas)) return;
        var at = e.GetCurrentPoint(_root).Position;
        var kind = _tracking?.CanAim == true ? Kind.Track
            : _gimbal?.Aim == VideoAim.Hold ? Kind.Hold
            : _gimbal?.Aim == VideoAim.Point ? Kind.Point
            : Kind.Drag;
        _gesture = new Gesture(kind, at, at, false, false);
        _root.CapturePointer(e.Pointer);
        if (kind == Kind.Hold) _hold.Start();
        e.Handled = true;
    }

    private void OnMoved(object sender, PointerRoutedEventArgs e)
    {
        if (_gesture is not { } gesture) return;
        var at = e.GetCurrentPoint(_root).Position;
        var moved = gesture.Moved || Math.Abs(at.X - gesture.From.X) >= PayloadJson.TrackSlop || Math.Abs(at.Y - gesture.From.Y) >= PayloadJson.TrackSlop;
        var latest = gesture with { Latest = at, Moved = moved };
        if (gesture.Kind == Kind.Track && moved) ShowBand(gesture.From, at);
        _gesture = gesture.Kind == Kind.Drag && moved && _sinceDrag.Elapsed >= DragSpacing ? SendDrag(latest) : latest;
        e.Handled = true;
    }

    private void OnReleased(object sender, PointerRoutedEventArgs e)
    {
        if (_gesture is not { } gesture) return;
        var at = e.GetCurrentPoint(_root).Position;
        var finished = gesture with { Latest = at };
        EndGesture();
        _root.ReleasePointerCapture(e.Pointer);
        e.Handled = true;
        switch (finished.Kind)
        {
            case Kind.Track:
                SendTracking(PayloadJson.Track((finished.From.X, finished.From.Y), (at.X, at.Y), Picture));
                break;
            case Kind.Point when !finished.Moved:
                var (pan, tilt) = Fraction(at);
                Send("gimbal.onScreen", pan, tilt, true);
                break;
            case Kind.Drag when finished.Moved:
                SendDrag(finished);
                break;
            case Kind.Drag:
                SwapRequested?.Invoke(this, EventArgs.Empty);
                break;
        }
    }

    private void EndGesture()
    {
        _gesture = null;
        _hold.Stop();
        _band.Visibility = Visibility.Collapsed;
    }

    private void ShowBand(Point from, Point to)
    {
        _band.Width = Math.Abs(to.X - from.X);
        _band.Height = Math.Abs(to.Y - from.Y);
        Canvas.SetLeft(_band, Math.Min(from.X, to.X));
        Canvas.SetTop(_band, Math.Min(from.Y, to.Y));
        _band.Visibility = Visibility.Visible;
    }

    private (double Pan, double Tilt) Fraction(Point at)
    {
        var picture = Picture;
        var (pan, tilt) = PayloadJson.ScreenFraction(at.X - picture.Left, at.Y - picture.Top, picture.Width, picture.Height);
        return (Math.Clamp(pan, -1, 1), Math.Clamp(tilt, -1, 1));
    }

    private Gesture SendDrag(Gesture gesture)
    {
        var picture = Picture;
        var pan = Math.Clamp(PayloadJson.AimFraction(gesture.Latest.X - gesture.From.X, picture.Width), -2, 2);
        var tilt = Math.Clamp(-PayloadJson.AimFraction(gesture.Latest.Y - gesture.From.Y, picture.Height), -2, 2);
        Send("gimbal.drag", pan, tilt, !gesture.Begun);
        _sinceDrag.Restart();
        return gesture with { Begun = true };
    }

    private void HoldTick()
    {
        if (_gesture is not { Kind: Kind.Hold } gesture) return;
        var start = Fraction(gesture.From);
        var now = Fraction(gesture.Latest);
        Send("gimbal.onScreen", Math.Clamp(now.Pan - start.Pan, -1, 1), Math.Clamp(now.Tilt - start.Tilt, -1, 1), false);
    }

    private static void Send(string path, params object?[] args) => Task.Run(() => GimbalCalls.Call(path, args));

    private static void SendTracking(TrackRequest? request)
    {
        JsonArray? args = request switch
        {
            TrackArea area => FormJson.Args(new JsonObject { ["x"] = area.Rect.X, ["y"] = area.Rect.Y, ["width"] = area.Rect.Width, ["height"] = area.Rect.Height }),
            TrackSpot spot => FormJson.Args(new JsonObject { ["x"] = spot.X, ["y"] = spot.Y }, spot.Radius),
            _ => null,
        };
        if (args is not null) Task.Run(() => Bridge.Invoke(StartTracking, args));
    }
}
