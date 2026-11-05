using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Linq;
using System.Numerics;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Shapes;
using Windows.Foundation;

namespace AircastQGC.Controls;

public sealed record Map3dPlace(double Latitude, double Longitude, Point At);

internal static class Map3dLook
{
    public const string Hint = "Drag to move · Right-drag, Shift-drag or two fingers to turn and tilt";

    public static string ToggleText(bool shown) => shown ? "2D" : "3D";

    public static string ToggleLabel(bool shown) => shown ? "Show the flat map" : "Show the 3D map";

    public static PointCollection Needle(double size, bool north)
    {
        var centre = size / 2;
        var reach = centre * NeedleReach;
        var half = centre * NeedleHalfWidth;
        var tip = north ? centre - reach : centre + reach;
        return new PointCollection { new Point(centre, tip), new Point(centre + half, centre), new Point(centre - half, centre) };
    }

    public static (double X, double Y) Velocity(IReadOnlyList<(Point At, long Ms)> samples, long now)
    {
        var recent = samples.Where(sample => now - sample.Ms <= FlingWindowMs).ToList();
        var span = recent.Count < 2 ? 0 : recent[^1].Ms - recent[0].Ms;
        return span <= 0
            ? (0, 0)
            : ((recent[^1].At.X - recent[0].At.X) * 1000.0 / span, (recent[^1].At.Y - recent[0].At.Y) * 1000.0 / span);
    }

    private const double NeedleReach = 0.8;
    private const double NeedleHalfWidth = 0.22;
    private const long FlingWindowMs = 100;
}

public sealed partial class Map3dView : UserControl
{
    private const string SyntheticPath = "view.syntheticView";
    private const string Map3dPath = "view.map3d";
    private const string PanCommand = "map3d.pan";
    private const string TurnCommand = "map3d.turn";
    private const string TiltCommand = "map3d.tilt";
    private const string ZoomCommand = "map3d.zoom";
    private const string FollowCommand = "map3d.follow";
    private const string NorthCommand = "map3d.north";
    private const string OpenCommand = "map3d.open";
    private const string PickCommand = "map3d.pick";
    private const string FlingCommand = "map3d.fling";
    private const string StopCommand = "map3d.stop";
    private const string Drawn = "drawn";
    private const string Failed = "failed";
    private const string LoadingNote = "Loading 3D view…";
    private const string FailedNote = "3D view failed to start";
    private const string BackToAircraft = "Back to aircraft";
    private const string CompassLabel = "Compass. Drag to turn and tilt the view, tap to face north";
    private const string FollowGlyph = "";
    private const double TurnPerViewWidth = 180;
    private const double WheelZoomStep = 1.25;
    private const double WheelNotch = 120;
    private const double TapSlop = 6;
    private const double ControlSize = 40;
    private const double NeedleSize = 20;
    private const double PillHeight = 36;
    private static readonly TimeSpan CheckEvery = TimeSpan.FromMilliseconds(500);
    private static readonly TimeSpan HintShown = TimeSpan.FromSeconds(8);
    private static readonly Brush TakesInput = new SolidColorBrush(Colors.Transparent);

    private sealed record Drag(uint Pointer, Point Start, Point Last, bool Orbit, bool Moved);

    private readonly Grid _root = new() { Background = TakesInput };
    private readonly SyntheticSurface _surface = new(orbit: true);
    private readonly TextBlock _noteText = FormLook.Caption("", "TextPrimaryBrush");
    private readonly Border _note;
    private readonly Border _hint;
    private readonly Button _back;
    private readonly Button _compass;
    private readonly RotateTransform _needle = new() { CenterX = NeedleSize / 2, CenterY = NeedleSize / 2 };
    private readonly StackPanel _controls;
    private readonly DispatcherQueueTimer _check;
    private readonly DispatcherQueueTimer _hintTimer;
    private JsonObject _view = new();
    private JsonObject _map3d = new();
    private ImmutableSortedDictionary<uint, Vector2> _touches = ImmutableSortedDictionary<uint, Vector2>.Empty;
    private bool _twoFingered;
    private Drag? _drag;
    private Drag? _compassDrag;
    private bool _wanted;
    private bool _showing;
    private bool _dragged;
    private ImmutableList<(Point At, long Ms)> _samples = ImmutableList<(Point At, long Ms)>.Empty;

    public Map3dView()
    {
        _noteText.TextWrapping = TextWrapping.Wrap;
        _noteText.TextAlignment = TextAlignment.Center;
        _note = Pill(_noteText);
        _note.HorizontalAlignment = HorizontalAlignment.Center;
        _note.VerticalAlignment = VerticalAlignment.Center;
        _note.Visibility = Visibility.Collapsed;
        _hint = Pill(FormLook.Caption(Map3dLook.Hint, "TextPrimaryBrush"));
        _hint.HorizontalAlignment = HorizontalAlignment.Right;
        _hint.Visibility = Visibility.Collapsed;
        _back = new Button
        {
            Height = PillHeight,
            Padding = new Thickness(12, 0, 12, 0),
            CornerRadius = new CornerRadius(PillHeight / 2),
            Content = FormLook.Line(8, FormLook.Glyph(FollowGlyph, 14, "TextPrimaryBrush"), FormLook.Caption(BackToAircraft, "TextPrimaryBrush")),
            Visibility = Visibility.Collapsed,
        };
        PreflightLook.Chip(_back);
        AutomationProperties.SetName(_back, BackToAircraft);
        _back.Click += (_, _) => FollowAircraft();
        _compass = new Button
        {
            Style = (Style)Application.Current.Resources["FlyRoundButton"],
            Width = ControlSize,
            Height = ControlSize,
            CornerRadius = new CornerRadius(ControlSize / 2),
            Background = FormLook.Brush("ChipBrush"),
            BorderBrush = FormLook.Brush("ControlStrokeBrush"),
            BorderThickness = new Thickness(1),
            Content = new Canvas
            {
                Width = NeedleSize,
                Height = NeedleSize,
                RenderTransform = _needle,
                Children =
                {
                    new Polygon { Points = Map3dLook.Needle(NeedleSize, true), Fill = FormLook.Brush("CriticalBrush") },
                    new Polygon { Points = Map3dLook.Needle(NeedleSize, false), Fill = FormLook.Brush("OsdTextBrush") },
                },
            },
        };
        AutomationProperties.SetName(_compass, CompassLabel);
        ToolTipService.SetToolTip(_compass, "Drag to turn and tilt, click to face north");
        _compass.Click += (_, _) =>
        {
            if (_compassDrag is not { Moved: true }) Send(NorthCommand);
            _compassDrag = null;
        };
        _compass.AddHandler(PointerPressedEvent, new PointerEventHandler(OnCompassPressed), true);
        _compass.AddHandler(PointerMovedEvent, new PointerEventHandler(OnCompassMoved), true);
        var row = FormLook.Line(8, _back, _compass);
        row.HorizontalAlignment = HorizontalAlignment.Right;
        _controls = new StackPanel
        {
            Spacing = 8,
            HorizontalAlignment = HorizontalAlignment.Right,
            VerticalAlignment = VerticalAlignment.Bottom,
            Children = { _hint, row },
        };
        _root.Children.Add(_surface.Stage);
        _root.Children.Add(_note);
        _root.Children.Add(_controls);
        _root.SizeChanged += (_, _) => Fit();
        _root.PointerPressed += OnPressed;
        _root.PointerMoved += OnMoved;
        _root.PointerReleased += OnReleased;
        _root.PointerCanceled += (_, e) => Lifted(e.Pointer.PointerId);
        _root.PointerCaptureLost += (_, e) => Lifted(e.Pointer.PointerId);
        _root.PointerWheelChanged += OnWheel;
        _root.DoubleTapped += (_, e) =>
        {
            FollowAircraft();
            e.Handled = true;
        };
        _root.AddHandler(RightTappedEvent, new RightTappedEventHandler(OnRightTapped), true);
        Content = _root;
        Visibility = Visibility.Collapsed;
        AutomationProperties.SetName(this, "3D map");
        _check = DispatcherQueue.GetForCurrentThread().CreateTimer();
        _check.Interval = CheckEvery;
        _check.Tick += (_, _) => Sync();
        _hintTimer = DispatcherQueue.GetForCurrentThread().CreateTimer();
        _hintTimer.Interval = HintShown;
        _hintTimer.IsRepeating = false;
        _hintTimer.Tick += (_, _) => HideHint();
        Live.Watch(this, SyntheticPath, view =>
        {
            var (native, software) = (Native, Software);
            _view = view;
            if (Software != software) Fit();
            else Sync();
            if (Native != native) OfferedChanged?.Invoke(this, EventArgs.Empty);
        });
        Live.Watch(this, Map3dPath, view =>
        {
            _map3d = view;
            Paint();
        });
        Loaded += (_, _) => Start();
        Unloaded += (_, _) => Halt();
    }

    public event EventHandler? OfferedChanged;

    public event EventHandler<Map3dPlace>? PlaceRequested;

    public bool Offered => Native;

    public bool Wanted
    {
        get => _wanted;
        set
        {
            _wanted = value;
            Sync();
        }
    }

    public Thickness ControlsMargin
    {
        set => _controls.Margin = value;
    }

    public void Open(double latitude, double longitude) => Send(OpenCommand, latitude, longitude);

    public void ZoomBy(double scale) => Send(ZoomCommand, scale);

    public void FollowAircraft() => Send(FollowCommand);

    private bool Native => VideoJson.Flag(_view, "native");

    private bool Software => VideoJson.Flag(_view, "software");

    private bool Following => _map3d["following"] is not JsonValue || VideoJson.Flag(_map3d, "following");

    private bool Attachable => _showing && _surface.Sized && SyntheticSurface.OnScreen(this);

    private static Border Pill(UIElement child) => new()
    {
        MinHeight = PillHeight,
        Padding = new Thickness(14, 8, 14, 8),
        CornerRadius = new CornerRadius(PillHeight / 2),
        Background = FormLook.Brush("ChipBrush"),
        BorderBrush = FormLook.Brush("ControlStrokeBrush"),
        BorderThickness = new Thickness(1),
        IsHitTestVisible = false,
        Child = child,
    };

    private void Start()
    {
        if (XamlRoot is { } root) root.Changed += OnRootChanged;
        _check.Start();
        Fit();
    }

    private void Halt()
    {
        if (IsLoaded) return;
        if (XamlRoot is { } root) root.Changed -= OnRootChanged;
        _check.Stop();
        _hintTimer.Stop();
        _surface.Detach();
    }

    private void OnRootChanged(XamlRoot sender, XamlRootChangedEventArgs args) => Fit();

    private void Fit()
    {
        _surface.Fit(_root.ActualWidth, _root.ActualHeight, Software, XamlRoot?.RasterizationScale ?? 1);
        Sync();
    }

    private void Sync()
    {
        var was = _showing;
        _showing = _wanted && Native;
        Visibility = VideoJson.Shown(_showing);
        _surface.Hold(Attachable);
        _surface.Stage.Opacity = _showing && !_surface.BindFailed && VideoJson.Text(_view, "draw") == Drawn ? 1 : 0;
        if (_showing && !was) OfferHint();
        Paint();
    }

    private void Paint()
    {
        var draw = VideoJson.Text(_view, "draw");
        var note = !VideoJson.Flag(_view, "available") && Following
            ? SyntheticView.WaitingHeadline
            : _surface.BindFailed || draw == Failed
                ? FailedNote
                : draw != Drawn ? LoadingNote : null;
        FormLook.Say(_noteText, note);
        _note.Visibility = VideoJson.Shown(_showing && !string.IsNullOrEmpty(note));
        _needle.Angle = -(VideoJson.Number(_map3d["heading"]) ?? 0);
        _back.Visibility = VideoJson.Shown(VideoJson.Flag(_map3d, "aircraft") && !Following);
    }

    private void OfferHint()
    {
        if (FlyViewStore.LoadMap3dHinted()) return;
        FlyViewStore.SaveMap3dHinted();
        _hint.Visibility = Visibility.Visible;
        _hintTimer.Start();
    }

    private void HideHint()
    {
        _hintTimer.Stop();
        _hint.Visibility = Visibility.Collapsed;
    }

    private void OnPressed(object sender, PointerRoutedEventArgs e)
    {
        HideHint();
        var point = e.GetCurrentPoint(_root);
        var touch = e.Pointer.PointerDeviceType == PointerDeviceType.Touch;
        _touches = touch ? _touches.SetItem(e.Pointer.PointerId, point.Position.ToVector2()) : _touches;
        _twoFingered = _twoFingered || _touches.Count > 1;
        var modified = (e.KeyModifiers & (Windows.System.VirtualKeyModifiers.Control | Windows.System.VirtualKeyModifiers.Shift)) != 0;
        var orbit = !touch && (point.Properties.IsRightButtonPressed || point.Properties.IsMiddleButtonPressed || modified);
        _drag = _twoFingered ? null : new Drag(e.Pointer.PointerId, point.Position, point.Position, orbit, false);
        _dragged = _twoFingered;
        _samples = ImmutableList.Create((point.Position, Environment.TickCount64));
        Send(StopCommand);
        _root.CapturePointer(e.Pointer);
        e.Handled = true;
    }

    private void OnMoved(object sender, PointerRoutedEventArgs e)
    {
        var at = e.GetCurrentPoint(_root).Position;
        if (_twoFingered && _touches.ContainsKey(e.Pointer.PointerId))
        {
            var before = Pair(_touches);
            _touches = _touches.SetItem(e.Pointer.PointerId, at.ToVector2());
            if (before is { } was && Pair(_touches) is { } now) Pinched(Pinch3D.Between(was, now));
            e.Handled = true;
            return;
        }
        if (_drag is not { } drag || drag.Pointer != e.Pointer.PointerId) return;
        var moved = drag.Moved || Math.Abs(at.X - drag.Start.X) + Math.Abs(at.Y - drag.Start.Y) > TapSlop;
        var (dx, dy) = (at.X - drag.Last.X, at.Y - drag.Last.Y);
        if (moved && drag.Orbit) Orbited(dx, dy);
        else if (moved) Send(PanCommand, dx, dy, Math.Max(_root.ActualWidth, 1));
        _drag = drag with { Last = at, Moved = moved };
        _dragged = _dragged || moved;
        _samples = _samples.Add((at, Environment.TickCount64));
        e.Handled = true;
    }

    private void OnRightTapped(object sender, RightTappedRoutedEventArgs e)
    {
        if (_dragged) return;
        var at = e.GetPosition(_root);
        var answer = Bridge.Invoke(PickCommand, new JsonArray(at.X, at.Y, Math.Max(_root.ActualWidth, 1), Math.Max(_root.ActualHeight, 1)));
        if (VideoJson.Flag(answer, "ok") && VideoJson.Number(answer["latitude"]) is double latitude && VideoJson.Number(answer["longitude"]) is double longitude)
            PlaceRequested?.Invoke(this, new Map3dPlace(latitude, longitude, at));
        e.Handled = true;
    }

    private void OnReleased(object sender, PointerRoutedEventArgs e)
    {
        if (_drag is { Moved: true, Orbit: false } drag && drag.Pointer == e.Pointer.PointerId && !_twoFingered)
        {
            var now = Environment.TickCount64;
            var (vx, vy) = Map3dLook.Velocity(_samples.Add((e.GetCurrentPoint(_root).Position, now)), now);
            Send(FlingCommand, vx, vy, Math.Max(_root.ActualWidth, 1));
        }
        Lifted(e.Pointer.PointerId);
        _root.ReleasePointerCapture(e.Pointer);
        e.Handled = true;
    }

    private void Lifted(uint pointer)
    {
        _touches = _touches.Remove(pointer);
        _twoFingered = _twoFingered && !_touches.IsEmpty;
        _drag = _drag is { } drag && drag.Pointer == pointer ? null : _drag;
    }

    private void OnWheel(object sender, PointerRoutedEventArgs e)
    {
        var notches = e.GetCurrentPoint(_root).Properties.MouseWheelDelta / WheelNotch;
        ZoomBy(Math.Pow(WheelZoomStep, notches));
        e.Handled = true;
    }

    private void OnCompassPressed(object sender, PointerRoutedEventArgs e)
    {
        HideHint();
        var at = e.GetCurrentPoint(_root).Position;
        _compassDrag = new Drag(e.Pointer.PointerId, at, at, true, false);
    }

    private void OnCompassMoved(object sender, PointerRoutedEventArgs e)
    {
        if (_compassDrag is not { } drag || drag.Pointer != e.Pointer.PointerId || !e.GetCurrentPoint(_root).IsInContact) return;
        var at = e.GetCurrentPoint(_root).Position;
        var moved = drag.Moved || Math.Abs(at.X - drag.Start.X) + Math.Abs(at.Y - drag.Start.Y) > TapSlop;
        if (moved) Orbited(at.X - drag.Last.X, at.Y - drag.Last.Y);
        _compassDrag = drag with { Last = at, Moved = moved };
    }

    private void Orbited(double dx, double dy)
    {
        Send(TurnCommand, dx / Math.Max(_root.ActualWidth, 1) * TurnPerViewWidth);
        Send(TiltCommand, -dy, Math.Max(_root.ActualHeight, 1));
    }

    private void Pinched(Pinch3D pinch)
    {
        if (pinch.Scale != 1) ZoomBy(pinch.Scale);
        if (pinch.Twist != 0) Send(TurnCommand, pinch.Twist);
        if (pinch.Rise != 0) Send(TiltCommand, pinch.Rise, Math.Max(_root.ActualHeight, 1));
    }

    private static (Vector2 A, Vector2 B)? Pair(ImmutableSortedDictionary<uint, Vector2> touches) =>
        touches.Count >= 2 ? (touches.Values.First(), touches.Values.Skip(1).First()) : null;

    private static void Send(string path, params double[] args) =>
        Bridge.Invoke(path, new JsonArray(args.Select(arg => (JsonNode?)JsonValue.Create(arg)).ToArray()));
}
