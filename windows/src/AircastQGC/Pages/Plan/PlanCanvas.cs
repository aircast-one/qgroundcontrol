using System;
using System.Collections.Generic;
using System.Linq;
using AircastQGC.Controls;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Shapes;
using Windows.Foundation;

namespace AircastQGC.Pages.Plan;

internal enum Ink { Path, Takeoff, Land, Home, Rally, KeepIn, KeepOut, Area, Transect, Collision, Breach, Handle }

internal sealed record Pin(Hit Hit, Geo At, string Label, string Side, Ink Ink, double Size, bool Selected, bool Draggable, bool Warning = false, string Glyph = "");

internal sealed record Trace(IReadOnlyList<Geo> Points, Ink Ink, bool Closed = false, bool Dashed = false, bool Filled = false, double Thickness = 3);

internal sealed record SplitHandle(string Path, string Invokable, int Segment, Geo At);

internal sealed record Scene(IReadOnlyList<Trace> Traces, IReadOnlyList<Pin> Pins, IReadOnlyList<SplitHandle> Splits);

internal sealed class PlanCanvas
{
    private const double Slop = 4;
    private const double SplitSize = 14;
    private const double FillOpacity = 0.12;
    private const double SideGap = 4;
    private const double SideHalfHeight = 8;

    private sealed record Placed(UIElement View, Action Place);

    private sealed class Gesture(Pin pin, FrameworkElement view, Point from)
    {
        public Pin Pin { get; } = pin;
        public FrameworkElement View { get; } = view;
        public Point From { get; } = from;
        public bool Moving { get; set; }
    }

    private readonly Canvas _lines = new() { IsHitTestVisible = false };
    private readonly Canvas _pins = new();
    private readonly MapView _map;
    private IReadOnlyList<Placed> _placed = [];
    private Gesture? _gesture;
    private Scene? _pending;
    private Scene? _scene;

    public PlanCanvas(Canvas host, MapView map)
    {
        _map = map;
        host.Children.Add(_lines);
        host.Children.Add(_pins);
    }

    public event Action<Hit>? Pressed;
    public event Action<Hit, Geo>? Dragged;
    public event Action<Hit, Geo>? Dropped;
    public event Action<SplitHandle>? SplitClicked;

    public void Draw(Scene scene)
    {
        if (_gesture is { Moving: false })
        {
            _pending = scene;
            return;
        }
        _pending = null;
        _scene = scene;
        var held = _gesture;
        _lines.Children.Clear();
        _pins.Children.Where(child => child != held?.View).ToList().ForEach(child => _pins.Children.Remove(child));
        var lines = scene.Traces.Select(TraceView).ToList();
        var pins = scene.Pins.Where(pin => held == null || pin.Hit != held.Pin.Hit).Select(PinView).ToList();
        var splits = scene.Splits.Select(SplitView).ToList();
        lines.ForEach(placed => _lines.Children.Add(placed.View));
        pins.Concat(splits).ToList().ForEach(placed => _pins.Children.Add(placed.View));
        _placed = lines.Concat(pins).Concat(splits).ToList();
        Place();
    }

    public void Place() => _placed.ToList().ForEach(placed => placed.Place());

    private Point Screen(Geo at) => _map.ScreenOf(at);

    private Placed TraceView(Trace trace)
    {
        Shape shape = trace.Closed
            ? new Polygon { Fill = trace.Filled ? Tint(trace.Ink) : null }
            : new Polyline();
        shape.Stroke = Brush(trace.Ink);
        shape.StrokeThickness = trace.Thickness;
        shape.StrokeLineJoin = PenLineJoin.Round;
        shape.StrokeStartLineCap = PenLineCap.Round;
        shape.StrokeEndLineCap = PenLineCap.Round;
        if (trace.Dashed) shape.StrokeDashArray = new DoubleCollection { 4, 3 };
        return new Placed(shape, () =>
        {
            var points = Collect(trace.Points.Select(Screen));
            if (shape is Polygon polygon) polygon.Points = points;
            else ((Polyline)shape).Points = points;
        });
    }

    private Placed PinView(Pin pin)
    {
        var view = new Grid
        {
            Width = pin.Size,
            Height = pin.Size,
            Children =
            {
                new Ellipse
                {
                    Fill = Brush(pin.Ink),
                    Stroke = pin.Warning ? Token("CriticalBrush") : pin.Selected ? Token("TextPrimaryBrush") : Token("CardStrokeBrush"),
                    StrokeThickness = pin.Warning || pin.Selected ? 3 : pin.Ink == Ink.Handle ? 1.5 : 2,
                },
                pin.Glyph.Length > 0
                    ? new FontIcon { Glyph = pin.Glyph, FontSize = pin.Size * 0.5, Foreground = LabelInk(pin.Ink), IsHitTestVisible = false }
                    : Label(pin.Label, LabelInk(pin.Ink), true),
            },
        };
        if (pin.Side.Length > 0)
        {
            var side = Label(pin.Side, Token("TextPrimaryBrush"), false);
            Canvas.SetLeft(side, pin.Size + SideGap);
            Canvas.SetTop(side, pin.Size / 2 - SideHalfHeight);
            view.Children.Add(new Canvas { IsHitTestVisible = false, Children = { side } });
            ToolTipService.SetToolTip(view, pin.Side);
        }
        view.PointerPressed += (_, e) => Press(pin, view, e);
        view.PointerMoved += (_, e) => Move(view, e);
        view.PointerReleased += (_, e) => Release(view, e);
        view.PointerCaptureLost += (_, _) => Lost(view);
        return new Placed(view, () => Centre(view, pin.At, pin.Size));
    }

    private static TextBlock Label(string text, Brush ink, bool centred) => new()
    {
        Text = text,
        FontFamily = (FontFamily)Application.Current.Resources["AppFontFamily"],
        FontSize = (double)Application.Current.Resources["FontSizeCaption"],
        FontWeight = centred ? Microsoft.UI.Text.FontWeights.SemiBold : Microsoft.UI.Text.FontWeights.Normal,
        Foreground = ink,
        HorizontalAlignment = centred ? HorizontalAlignment.Center : HorizontalAlignment.Left,
        VerticalAlignment = centred ? VerticalAlignment.Center : VerticalAlignment.Top,
        IsHitTestVisible = false,
    };

    private Placed SplitView(SplitHandle split)
    {
        var view = new Grid
        {
            Width = SplitSize,
            Height = SplitSize,
            Children =
            {
                new Ellipse { Fill = Token("CardBrush"), Stroke = Token("TextPrimaryBrush"), StrokeThickness = 1.5 },
                new FontIcon { Glyph = "", FontSize = 8, Foreground = Token("TextPrimaryBrush"), IsHitTestVisible = false },
            },
        };
        ToolTipService.SetToolTip(view, "Add a corner here");
        var downAt = default(Point?);
        view.PointerPressed += (_, e) =>
        {
            if (!e.GetCurrentPoint(view).Properties.IsLeftButtonPressed) return;
            downAt = e.GetCurrentPoint(_pins).Position;
            view.CapturePointer(e.Pointer);
            e.Handled = true;
        };
        view.PointerReleased += (_, e) =>
        {
            var at = e.GetCurrentPoint(_pins).Position;
            var clicked = downAt is { } from && !Far(from, at);
            downAt = null;
            view.ReleasePointerCapture(e.Pointer);
            e.Handled = true;
            if (clicked) SplitClicked?.Invoke(split);
        };
        return new Placed(view, () => Centre(view, split.At, SplitSize));
    }

    private void Press(Pin pin, FrameworkElement view, PointerRoutedEventArgs e)
    {
        if (!e.GetCurrentPoint(view).Properties.IsLeftButtonPressed) return;
        e.Handled = true;
        _gesture = new Gesture(pin, view, e.GetCurrentPoint(_pins).Position);
        view.CapturePointer(e.Pointer);
        Pressed?.Invoke(pin.Hit);
    }

    private void Move(FrameworkElement view, PointerRoutedEventArgs e)
    {
        if (_gesture is not { } gesture || gesture.View != view || !gesture.Pin.Draggable) return;
        var at = e.GetCurrentPoint(_pins).Position;
        if (!gesture.Moving && !Far(gesture.From, at)) return;
        gesture.Moving = true;
        e.Handled = true;
        Canvas.SetLeft(view, at.X - view.Width / 2);
        Canvas.SetTop(view, at.Y - view.Height / 2);
        Dragged?.Invoke(gesture.Pin.Hit, _map.GeoOf(at));
    }

    private void Release(FrameworkElement view, PointerRoutedEventArgs e)
    {
        if (_gesture is not { } gesture || gesture.View != view) return;
        e.Handled = true;
        var at = e.GetCurrentPoint(_pins).Position;
        Finish(gesture, gesture.Moving ? _map.GeoOf(at) : null);
        view.ReleasePointerCapture(e.Pointer);
    }

    private void Lost(FrameworkElement view)
    {
        if (_gesture is not { } gesture || gesture.View != view) return;
        Finish(gesture, null);
    }

    private void Finish(Gesture gesture, Geo? droppedAt)
    {
        _gesture = null;
        var pending = _pending;
        _pending = null;
        if (droppedAt is { } at) Dropped?.Invoke(gesture.Pin.Hit, at);
        if ((pending ?? (gesture.Moving ? _scene : null)) is { } scene) Draw(scene);
    }

    private static bool Far(Point from, Point to) => Math.Abs(to.X - from.X) > Slop || Math.Abs(to.Y - from.Y) > Slop;

    private void Centre(FrameworkElement view, Geo at, double size)
    {
        if (_gesture is { Moving: true } gesture && gesture.View == view) return;
        var point = Screen(at);
        Canvas.SetLeft(view, point.X - size / 2);
        Canvas.SetTop(view, point.Y - size / 2);
    }

    private static PointCollection Collect(IEnumerable<Point> points)
    {
        var collection = new PointCollection();
        points.ToList().ForEach(collection.Add);
        return collection;
    }

    private static Brush Token(string key) => (Brush)Application.Current.Resources[key];

    private static string Key(Ink ink) => ink switch
    {
        Ink.Takeoff or Ink.Rally => "SuccessBrush",
        Ink.Land or Ink.KeepOut or Ink.Collision => "CriticalBrush",
        Ink.KeepIn or Ink.Breach => "CautionBrush",
        Ink.Home => "CardBrush",
        Ink.Area => "AccentBrush",
        Ink.Transect => "AccentTextBrush",
        Ink.Handle => "TextPrimaryBrush",
        _ => "MapPathBrush",
    };

    private static Brush Brush(Ink ink) => Token(Key(ink));

    private static Brush LabelInk(Ink ink) => ink == Ink.Home ? Token("TextPrimaryBrush") : Token("AccentOnBrush");

    private static Brush Tint(Ink ink) => new SolidColorBrush(((SolidColorBrush)Brush(ink)).Color) { Opacity = FillOpacity };
}
