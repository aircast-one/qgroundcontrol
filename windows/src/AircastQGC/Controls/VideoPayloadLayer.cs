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
using Microsoft.UI.Xaml.Media.Animation;
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
    private const string OrbitView = "view.orbit";
    private const string GroundAt = "videoAr.groundAt";
    private const string PauseVehicle = "vehicle.pauseVehicle";
    private const string StopLooking = "vehicle.stopGuidedModeROI";
    private const string NotGround = "notGround";
    private const string HoldHint = "Press and hold or right-click a spot to look at it or fly there";
    private const string BelowHorizon = "Press and hold on a spot below the horizon";
    private const string NoGimbal = "The vehicle reports no gimbal, so a spot in the picture can't be placed on the map";
    private const string OsdText = "OsdTextBrush";
    private const string OsdBack = "VideoEdgeBrush";
    private const double RingSize = 48;
    private const double ReticleSize = 48;
    private const double ReticleDot = 8;
    private const double HintLift = 0.2;
    private const double SectorSweep = 45;
    private const double SectorGap = 3.6;
    private const double ArcOffset = -112.5;
    private const double RadarOpacity = 0.5;
    private const double LabelWidth = 64;
    private static readonly TimeSpan HoldRepeat = TimeSpan.FromMilliseconds(100);
    private static readonly TimeSpan DragSpacing = TimeSpan.FromMilliseconds(50);
    private static readonly TimeSpan RefusalShown = TimeSpan.FromSeconds(4);
    private static readonly Thickness BelowStatusRow = new(16, 56, 16, 16);
    private static readonly TimeSpan RingDelay = TimeSpan.FromMilliseconds(150);
    private static readonly TimeSpan HoldTime = TimeSpan.FromMilliseconds(800);
    private static readonly TimeSpan HintShown = TimeSpan.FromSeconds(4);
    private static readonly TimeSpan FirstHintShown = TimeSpan.FromSeconds(10);

    private enum Kind { Track, Drag, Point, Hold }

    private sealed record Gesture(Kind Kind, Point From, Point Latest, bool Moved, bool Begun);

    private readonly string _client = $"video-payload.{Guid.NewGuid():N}";
    private readonly Grid _root = new();
    private readonly Canvas _drawing = new() { IsHitTestVisible = false };
    private readonly Canvas _bandLayer = new() { IsHitTestVisible = false };
    private readonly Rectangle _band;
    private readonly Button _aimPill;
    private readonly TextBlock _aimText = FormLook.Caption("", "TextPrimaryBrush");
    private readonly TextBlock _refusalText = FormLook.Caption("", "CriticalBrush");
    private readonly Border _refusal;
    private readonly StackPanel _aimTag;
    private readonly DispatcherQueueTimer _hold;
    private readonly DispatcherQueueTimer _refusalTimer;
    private readonly Stopwatch _sinceDrag = Stopwatch.StartNew();
    private readonly Canvas _ringLayer = new() { IsHitTestVisible = false };
    private readonly ProgressRing _ring;
    private readonly Storyboard _holding = new();
    private readonly TextBlock _hintText = FormLook.Text("", "FontSizeBody", OsdText);
    private readonly Border _hint;
    private readonly DispatcherQueueTimer _hintTimer;
    private readonly TextBlock _lookText = FormLook.Text("", "FontSizeBody", OsdText);
    private readonly Grid _look;
    private Func<object, bool> _onPicture = _ => false;
    private (uint Pointer, Point From)? _press;
    private bool _vehicle;
    private bool _looking;
    private bool _orbiting;
    private bool _hintDue;
    private bool _hintSeen = FlyViewStore.LoadHoldHintSeen();
    private JsonObject _video = new();
    private GimbalState? _gimbal;
    private TrackingState? _tracking;
    private ProximityRadar? _radar;
    private Gesture? _gesture;
    private bool _expanded;
    private bool _pictured;
    private int _fit = VideoSurface.FitHeight;
    private double? _aspectSetting;

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
            Content = _aimText,
        };
        PreflightLook.Chip(_aimPill);
        _aimPill.Click += (_, _) =>
        {
            if (_gimbal?.Aim is { } aim) GimbalCalls.ChooseAim(aim.Next);
        };
        _refusalText.TextWrapping = TextWrapping.Wrap;
        _refusalText.MaxWidth = 320;
        _refusal = new Border
        {
            Padding = new Thickness(10, 4, 10, 4),
            CornerRadius = new CornerRadius(8),
            Background = FormLook.Brush("ChipBrush"),
            IsHitTestVisible = false,
            Child = _refusalText,
            Visibility = Visibility.Collapsed,
        };
        _aimTag = new StackPanel
        {
            Spacing = 6,
            HorizontalAlignment = HorizontalAlignment.Left,
            VerticalAlignment = VerticalAlignment.Top,
            Margin = BelowStatusRow,
            Visibility = Visibility.Collapsed,
            Children = { _aimPill, _refusal },
        };
        _ring = new ProgressRing
        {
            Width = RingSize,
            Height = RingSize,
            IsIndeterminate = false,
            Minimum = 0,
            Maximum = 1,
            Value = 0,
            Foreground = FormLook.Brush(OsdText),
            Background = new SolidColorBrush(Colors.Transparent),
            Visibility = Visibility.Collapsed,
        };
        _ringLayer.Children.Add(_ring);
        var fill = new DoubleAnimation { From = 0, To = 1, BeginTime = RingDelay, Duration = HoldTime - RingDelay, EnableDependentAnimation = true };
        Storyboard.SetTarget(fill, _ring);
        Storyboard.SetTargetProperty(fill, "Value");
        _holding.Children.Add(fill);
        _holding.Completed += (_, _) => Held();
        _hintText.TextWrapping = TextWrapping.Wrap;
        _hintText.TextAlignment = TextAlignment.Center;
        _hint = new Border
        {
            Padding = new Thickness(14, 6, 14, 6),
            CornerRadius = new CornerRadius(16),
            MaxWidth = 480,
            Background = FormLook.Brush(OsdBack),
            HorizontalAlignment = HorizontalAlignment.Center,
            VerticalAlignment = VerticalAlignment.Bottom,
            IsHitTestVisible = false,
            Visibility = Visibility.Collapsed,
            Child = _hintText,
        };
        _look = LookAt();
        _root.Children.Add(_drawing);
        _root.Children.Add(_bandLayer);
        _root.Children.Add(_look);
        _root.Children.Add(_ringLayer);
        _root.Children.Add(_hint);
        _root.Children.Add(_aimTag);
        _root.PointerPressed += OnPressed;
        _root.PointerMoved += OnMoved;
        _root.PointerReleased += OnReleased;
        _root.PointerCanceled += (_, _) => EndGesture();
        _root.PointerCaptureLost += (_, _) => EndGesture();
        _root.SizeChanged += (_, e) =>
        {
            _root.Clip = new RectangleGeometry { Rect = new Rect(0, 0, e.NewSize.Width, e.NewSize.Height) };
            _hint.Margin = new Thickness(16, 16, 16, e.NewSize.Height * HintLift);
            Draw();
        };
        Content = _root;
        _hold = DispatcherQueue.GetForCurrentThread().CreateTimer();
        _hold.Interval = HoldRepeat;
        _hold.Tick += (_, _) => HoldTick();
        _refusalTimer = DispatcherQueue.GetForCurrentThread().CreateTimer();
        _refusalTimer.Interval = RefusalShown;
        _refusalTimer.IsRepeating = false;
        _refusalTimer.Tick += (_, _) => _refusal.Visibility = Visibility.Collapsed;
        _hintTimer = DispatcherQueue.GetForCurrentThread().CreateTimer();
        _hintTimer.IsRepeating = false;
        _hintTimer.Tick += (_, _) => _hint.Visibility = Visibility.Collapsed;
        Loaded += (_, _) => Start();
        Unloaded += (_, _) => Halt();
    }

    public event EventHandler<PictureGround>? GroundPressed;

    public FrameworkElement AimTag => _aimTag;

    public void HoldFrom(UIElement area, Func<object, bool> onPicture)
    {
        _onPicture = onPicture;
        area.AddHandler(PointerPressedEvent, new PointerEventHandler(OnAreaPressed), true);
        area.AddHandler(PointerMovedEvent, new PointerEventHandler(OnAreaMoved), true);
        area.AddHandler(PointerReleasedEvent, new PointerEventHandler(OnAreaEnded), true);
        area.AddHandler(PointerCanceledEvent, new PointerEventHandler(OnAreaEnded), true);
        area.AddHandler(PointerCaptureLostEvent, new PointerEventHandler(OnAreaEnded), true);
    }

    public bool Expanded
    {
        get => _expanded;
        set
        {
            _expanded = value;
            EndGesture();
            CancelHold();
            Sync();
        }
    }

    public bool Pictured
    {
        get => _pictured;
        set
        {
            _pictured = value;
            EndGesture();
            CancelHold();
            Sync();
            Draw();
        }
    }

    private bool Decoding => VideoJson.Flag(_video, "decoding");

    private bool Synthetic => !Decoding && _pictured;

    private bool Holdable => _expanded && _vehicle && (Decoding || Synthetic);

    private bool TakesInput => _expanded && (Decoding ? _tracking?.CanAim == true || _gimbal is not null : Synthetic && _tracking?.CanAim == true);

    private Painted Picture => Synthetic
        ? new Painted(0, 0, _root.ActualWidth, _root.ActualHeight)
        : PayloadJson.Centred(_root.ActualWidth, _root.ActualHeight,
            VideoSurface.ContentSize(_root.ActualWidth, _root.ActualHeight, VideoSurface.PictureAspect(_video, _aspectSetting), _fit));

    private void Start()
    {
        Watch("video", VideoView, view => _video = view);
        Watch("camera", CameraView, view => _tracking = PayloadJson.Tracking(view));
        Watch("gimbal", GimbalCalls.View, view => _gimbal = PayloadJson.Gimbal(view));
        Watch("radar", RadarView, view => _radar = PayloadJson.Radar(view));
        Watch("fit", FitView, view => _fit = VideoJson.Number(view["value"]) is double mode ? (int)mode : VideoSurface.FitHeight);
        Watch("aspect", VideoSurface.AspectView, view => _aspectSetting = VideoJson.Number(view["value"]));
        Watch("orbit", OrbitView, view => _orbiting = PayloadJson.OrbitLabel(view) is not null);
        Watch("guided", GuidedOffer.ViewPath, view =>
        {
            _vehicle = GuidedOffer.List(view).Count > 0;
            _looking = MapOverlayJson.Roi(view) is not null;
        });
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
        new[] { "video", "camera", "gimbal", "radar", "fit", "aspect", "orbit", "guided" }.ToList().ForEach(name => BridgeWatch.Stop($"{_client}.{name}"));
        EndGesture();
    }

    private void Sync()
    {
        _root.Background = TakesInput ? new SolidColorBrush(Colors.Transparent) : null;
        var aim = _expanded && Decoding ? _gimbal?.Aim : null;
        _aimTag.Visibility = VideoJson.Shown(aim is not null);
        _aimText.Text = aim?.Label ?? "";
        AutomationProperties.SetName(_aimPill, aim is null ? "Video gimbal mode" : $"Video gimbal mode, {aim.Label}");
        ToolTipService.SetToolTip(_aimPill, aim is null ? null : $"Switch to {aim.Next.Label}");
        _look.Visibility = VideoJson.Shown(_expanded && (Decoding || Synthetic) && (_looking || _orbiting));
        FormLook.Say(_lookText, _orbiting ? "Orbiting this point" : "Looking at this point");
        var hintDue = Holdable && _gimbal is not null && !_hintSeen;
        if (hintDue && !_hintDue) Hint(HoldHint, FirstHintShown);
        if (!hintDue && _hintDue && _hintText.Text == HoldHint) _hint.Visibility = Visibility.Collapsed;
        _hintDue = hintDue;
        if (!Holdable) CancelHold();
        Draw();
    }

    private Grid LookAt()
    {
        var ink = FormLook.Brush(OsdText);
        var reticle = new Grid
        {
            Width = ReticleSize,
            Height = ReticleSize,
            IsHitTestVisible = false,
            Children =
            {
                new Ellipse { Stroke = ink, StrokeThickness = 3 },
                new Ellipse { Width = ReticleDot, Height = ReticleDot, Fill = ink },
            },
        };
        var chip = new Border
        {
            Margin = new Thickness(0, 8, 0, 0),
            Padding = new Thickness(12, 4, 4, 4),
            CornerRadius = new CornerRadius(8),
            Background = FormLook.Brush(OsdBack),
            HorizontalAlignment = HorizontalAlignment.Center,
            VerticalAlignment = VerticalAlignment.Top,
            Child = FormLook.Line(8, FormLook.Margined(_lookText, new Thickness(0, 0, 0, 2)), FormLook.Button("Stop", StopWatching)),
        };
        _lookText.VerticalAlignment = VerticalAlignment.Center;
        Grid.SetRow(reticle, 1);
        Grid.SetRow(chip, 2);
        return new Grid
        {
            Visibility = Visibility.Collapsed,
            RowDefinitions =
            {
                new RowDefinition { Height = new GridLength(1, GridUnitType.Star) },
                new RowDefinition { Height = GridLength.Auto },
                new RowDefinition { Height = new GridLength(1, GridUnitType.Star) },
            },
            Children = { reticle, chip },
        };
    }

    private void StopWatching() => Send(_orbiting ? PauseVehicle : StopLooking);

    private void Hint(string text, TimeSpan shown)
    {
        FormLook.Say(_hintText, text);
        _hint.Visibility = Visibility.Visible;
        _hintTimer.Interval = shown;
        _hintTimer.Stop();
        _hintTimer.Start();
    }

    private void OnAreaPressed(object sender, PointerRoutedEventArgs e)
    {
        if (!Holdable || !_onPicture(e.OriginalSource)) return;
        var point = e.GetCurrentPoint(_root);
        if (_press is { } held && held.Pointer != e.Pointer.PointerId)
        {
            CancelHold();
            return;
        }
        if (point.Properties.IsRightButtonPressed)
        {
            CancelHold();
            Ask(point.Position);
            return;
        }
        if (!point.Properties.IsLeftButtonPressed || _gimbal?.Aim == VideoAim.Hold) return;
        _press = (e.Pointer.PointerId, point.Position);
        Canvas.SetLeft(_ring, point.Position.X - RingSize / 2);
        Canvas.SetTop(_ring, point.Position.Y - RingSize / 2);
        _ring.Value = 0;
        _ring.Visibility = Visibility.Visible;
        _holding.Begin();
    }

    private void OnAreaMoved(object sender, PointerRoutedEventArgs e)
    {
        if (_press is not { } held || held.Pointer != e.Pointer.PointerId) return;
        var at = e.GetCurrentPoint(_root).Position;
        if (Math.Abs(at.X - held.From.X) >= PayloadJson.TrackSlop || Math.Abs(at.Y - held.From.Y) >= PayloadJson.TrackSlop) CancelHold();
    }

    private void OnAreaEnded(object sender, PointerRoutedEventArgs e)
    {
        if (_press is { } held && held.Pointer == e.Pointer.PointerId) CancelHold();
    }

    private void CancelHold()
    {
        _holding.Stop();
        _ring.Visibility = Visibility.Collapsed;
        _press = null;
    }

    private void Held()
    {
        if (_press is not { } held) return;
        CancelHold();
        EndGesture();
        Ask(held.From);
    }

    private async void Ask(Point at)
    {
        if (_gimbal is null && !Synthetic)
        {
            Hint(NoGimbal, HintShown);
            return;
        }
        var picture = Picture;
        var (pan, tilt) = PayloadJson.ScreenFraction(at.X - picture.Left, at.Y - picture.Top, picture.Width, picture.Height);
        if (picture.Width <= 0 || picture.Height <= 0 || Math.Abs(pan) > 1 || Math.Abs(tilt) > 1)
        {
            Hint(BelowHorizon, HintShown);
            return;
        }
        var synthetic = Synthetic;
        var answer = await Task.Run(() => Bridge.Invoke(GroundAt, new JsonArray(pan, tilt, picture.Width / picture.Height, synthetic)));
        if (MapUiJson.RefusalOf(answer) is { } refusal)
        {
            Hint(MapUiJson.Text(answer["refusal"]) == NotGround ? BelowHorizon : refusal, HintShown);
            return;
        }
        if (MapUiJson.Number(answer["latitude"]) is not double latitude || MapUiJson.Number(answer["longitude"]) is not double longitude) return;
        if (!_hintSeen) FlyViewStore.SaveHoldHintSeen();
        _hintSeen = true;
        if (_hintText.Text == HoldHint) _hint.Visibility = Visibility.Collapsed;
        GroundPressed?.Invoke(this, new PictureGround(latitude, longitude, MapUiJson.Texts(answer["actions"]), MapUiJson.Text(answer["warning"]), at));
    }

    private void Draw()
    {
        var picture = Picture;
        FormLook.Fill(_drawing, Decoding ? RadarShapes(picture).Concat(TrackShapes(picture)) : Synthetic ? TrackShapes(picture) : []);
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
        var pressed = e.GetCurrentPoint(_root);
        if (!TakesInput || e.OriginalSource is not (Grid or Canvas) || pressed.Properties.IsRightButtonPressed) return;
        var at = pressed.Position;
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
        _gesture = Drags(gesture.Kind) && moved && _sinceDrag.Elapsed >= DragSpacing ? SendDrag(latest) : latest;
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
            case Kind.Drag or Kind.Point when finished.Moved:
                SendDrag(finished);
                break;
        }
    }

    private static bool Drags(Kind kind) => kind is Kind.Drag or Kind.Point;

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

    private void Send(string path, params object?[] args)
    {
        var ui = DispatcherQueue;
        Task.Run(() => GimbalCalls.Call(path, args)).ContinueWith(done => ui.TryEnqueue(() => ShowRefusal(done.Result)), TaskScheduler.Default);
    }

    private void ShowRefusal(string? refusal)
    {
        if (refusal is null) return;
        _refusalText.Text = refusal;
        _refusalText.Visibility = Visibility.Visible;
        _refusal.Visibility = Visibility.Visible;
        _refusalTimer.Stop();
        _refusalTimer.Start();
    }

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
