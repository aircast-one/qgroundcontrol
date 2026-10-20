using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading;
using System.Threading.Tasks;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Shapes;
using Windows.Foundation;

namespace AircastQGC.Controls;

internal sealed record StickSettings(
    bool Enabled,
    bool Show,
    bool Sending,
    bool AutoCenterThrottle,
    bool LeftHandedMode,
    bool LeftPositiveOnly,
    bool RightPositiveOnly,
    int PeriodMs)
{
    public const string ViewPath = "view.virtualJoystick";
    private const int DefaultPeriodMs = 40;

    public bool ThrottleOnLeft => !LeftHandedMode;

    public bool Live => Enabled && Sending;

    public static StickSettings? Of(JsonObject view) =>
        MapUiJson.Text(view["class"]) == "VirtualJoystick"
            ? new StickSettings(
                MapUiJson.Flag(view["enabled"]),
                MapUiJson.Flag(view["show"]),
                MapUiJson.Flag(view["sending"]),
                MapUiJson.Flag(view["autoCenterThrottle"]),
                MapUiJson.Flag(view["leftHandedMode"]),
                MapUiJson.Flag(view["leftPositiveOnly"]),
                MapUiJson.Flag(view["rightPositiveOnly"]),
                (int)(MapUiJson.Number(view["periodMs"]) ?? DefaultPeriodMs))
            : null;
}

internal readonly record struct Stick(double X, double Y)
{
    public static readonly Stick Centred = new(0.5, 0.5);
}

internal sealed record StickFrame(StickSettings? Settings, Stick Left, Stick Right, bool LeftHeld, bool RightHeld, bool Grounded)
{
    public static readonly StickFrame Idle = new(null, Stick.Centred, Stick.Centred, false, false, false);
}

internal static class StickMath
{
    public const double KnobFraction = 0.22;
    private const double PadFraction = 0.42;
    private const double MinPad = 96;
    private const double MaxPad = 200;

    private static readonly IReadOnlySet<string> GroundStates = new HashSet<string> { "disarmed", "armed" };

    public static bool Resets(StickSettings? previous, StickSettings? current) =>
        current?.Enabled != true
        || current.AutoCenterThrottle != previous?.AutoCenterThrottle
        || current.LeftHandedMode != previous?.LeftHandedMode;

    public static bool OnTheGround(JsonObject fly) =>
        MapUiJson.Flag(fly["connected"]) && GroundStates.Contains(MapUiJson.Text(fly["state"]));

    public static bool ThrottleParked(StickSettings settings, bool grounded) =>
        grounded && (settings.ThrottleOnLeft ? settings.LeftPositiveOnly : settings.RightPositiveOnly);

    public static Stick Released(Stick stick, bool throttle, bool autoCenter, bool parked) =>
        new(0.5, !throttle || autoCenter || parked ? 0.5 : stick.Y);

    public static (Stick Left, Stick Right) Shown(StickFrame frame) =>
        frame.Settings is { } settings ? Shown(settings, frame.Left, frame.Right, frame.Grounded, frame.LeftHeld, frame.RightHeld) : (frame.Left, frame.Right);

    public static (Stick Left, Stick Right) Shown(StickSettings settings, Stick left, Stick right, bool grounded, bool leftHeld, bool rightHeld)
    {
        var parked = ThrottleParked(settings, grounded);
        Stick Park(Stick stick, bool held) => parked && !held ? stick with { Y = 1 } : stick;
        return settings.ThrottleOnLeft ? (Park(left, leftHeld), right) : (left, Park(right, rightHeld));
    }

    public static (double X, double Y) Axes(Stick stick, bool positiveOnly)
    {
        var up = 1.0 - Math.Clamp(stick.Y, 0, 1);
        return (Math.Clamp(stick.X, 0, 1) * 2.0 - 1.0, positiveOnly ? up : up * 2.0 - 1.0);
    }

    public static IReadOnlyList<double> Values(StickSettings settings, Stick left, Stick right)
    {
        var l = Axes(left, settings.LeftPositiveOnly);
        var r = Axes(right, settings.RightPositiveOnly);
        return settings.LeftHandedMode ? [l.X, l.Y, r.X, r.Y] : [r.X, r.Y, l.X, l.Y];
    }

    public static double Pad(double shortSide) => Math.Clamp(shortSide * PadFraction, MinPad, MaxPad);

    public static double Travel(double pad) => pad / 2 - pad * KnobFraction;

    public static Point KnobOffset(Stick stick, double travel)
    {
        var x = (stick.X - 0.5) * 2 * travel;
        var y = (stick.Y - 0.5) * 2 * travel;
        var distance = Math.Sqrt(x * x + y * y);
        return distance > travel ? new Point(x * travel / distance, y * travel / distance) : new Point(x, y);
    }

    public static Stick Dragged(Stick start, Point moved, double travel) =>
        travel <= 0
            ? start
            : new(Math.Clamp(start.X + moved.X / (2 * travel), 0, 1), Math.Clamp(start.Y + moved.Y / (2 * travel), 0, 1));
}

internal static class StickSender
{
    public const string ValuePath = "vehicle.virtualTabletJoystickValue";

    private static readonly SemaphoreSlim Moved = new(0, 1);
    private static StickFrame _frame = StickFrame.Idle;
    private static long _touchedAt;
    private static CancellationTokenSource? _running;

    public static void Publish(StickFrame frame, bool touched)
    {
        Volatile.Write(ref _frame, frame);
        if (touched) Interlocked.CompareExchange(ref _touchedAt, Environment.TickCount64, 0);
        Wake();
        if (frame.Settings?.Live == true) Start();
        else Stop();
    }

    public static void Stop()
    {
        _running?.Cancel();
        _running = null;
    }

    private static void Start()
    {
        if (_running != null) return;
        var running = _running = new CancellationTokenSource();
        Task.Run(() => Loop(running.Token));
    }

    private static void Wake()
    {
        if (Moved.CurrentCount > 0) return;
        try { Moved.Release(); }
        catch (SemaphoreFullException) { }
    }

    private static async Task Loop(CancellationToken stop)
    {
        try
        {
            while (!stop.IsCancellationRequested)
            {
                var frame = Volatile.Read(ref _frame);
                Send(frame);
                await Moved.WaitAsync(frame.Settings?.PeriodMs ?? 40, stop);
            }
        }
        catch (OperationCanceledException)
        {
        }
    }

    private static void Send(StickFrame frame)
    {
        if (frame.Settings is not { } settings || !CoreHost.Started) return;
        var (left, right) = StickMath.Shown(frame);
        var touched = Interlocked.Exchange(ref _touchedAt, 0);
        var age = touched > 0 ? new JsonNode?[] { JsonValue.Create(Math.Max(0, Environment.TickCount64 - touched)) } : [];
        var axes = StickMath.Values(settings, left, right).Select(value => (JsonNode?)JsonValue.Create(value));
        Bridge.Invoke(ValuePath, new JsonArray(axes.Concat(age).ToArray()));
    }
}

internal sealed class ThumbPad : Grid
{
    private const double FillOpacity = 0.35;
    private const double RingOpacity = 0.45;
    private const double HeldRingOpacity = 0.8;
    private const double KnobOpacity = 0.28;
    private const double HeldKnobOpacity = 0.45;

    private readonly Action<bool> _held;
    private readonly Action<Stick> _moved;
    private readonly Action<Stick> _released;
    private readonly Ellipse _ring;
    private readonly Ellipse _knob;
    private readonly TranslateTransform _shift = new();
    private uint? _pointer;
    private Point _origin;
    private Stick _start = Stick.Centred;
    private Stick _shown = Stick.Centred;

    public ThumbPad(string name, Action<bool> held, Action<Stick> moved, Action<Stick> released)
    {
        _held = held;
        _moved = moved;
        _released = released;
        _ring = new Ellipse { Stroke = FormLook.Brush("TextPrimaryBrush"), StrokeThickness = 1.5, Opacity = RingOpacity };
        _knob = new Ellipse
        {
            Fill = FormLook.Brush("TextPrimaryBrush"),
            Opacity = KnobOpacity,
            HorizontalAlignment = HorizontalAlignment.Center,
            VerticalAlignment = VerticalAlignment.Center,
            RenderTransform = _shift,
            IsHitTestVisible = false,
        };
        Children.Add(new Ellipse { Fill = FormLook.Brush("BgSecondaryBrush"), Opacity = FillOpacity });
        Children.Add(_ring);
        Children.Add(_knob);
        AutomationProperties.SetName(this, name);
        PointerPressed += OnPressed;
        PointerMoved += OnMoved;
        PointerReleased += OnEnded;
        PointerCanceled += OnEnded;
        PointerCaptureLost += OnEnded;
    }

    internal double Travel => StickMath.Travel(Width);

    internal bool Held => _pointer != null;

    public void Size(double pad)
    {
        Width = pad;
        Height = pad;
        _knob.Width = pad * StickMath.KnobFraction * 2;
        _knob.Height = pad * StickMath.KnobFraction * 2;
        Draw(_shown);
    }

    public void Draw(Stick stick)
    {
        _shown = stick;
        var offset = StickMath.KnobOffset(stick, Travel);
        _shift.X = offset.X;
        _shift.Y = offset.Y;
    }

    public void Cancel()
    {
        if (_pointer == null) return;
        _pointer = null;
        Paint(false);
        ReleasePointerCaptures();
    }

    internal void Grab(uint pointer)
    {
        _pointer = pointer;
        _start = _shown;
        Paint(true);
        _held(true);
    }

    internal void Drag(Point moved) => _moved(StickMath.Dragged(_start, moved, Travel));

    internal void Lift()
    {
        _pointer = null;
        Paint(false);
        _held(false);
        _released(_shown);
    }

    private void OnPressed(object sender, PointerRoutedEventArgs e)
    {
        if (_pointer != null || !CapturePointer(e.Pointer)) return;
        _origin = e.GetCurrentPoint(this).Position;
        Grab(e.Pointer.PointerId);
        e.Handled = true;
    }

    private void OnMoved(object sender, PointerRoutedEventArgs e)
    {
        if (_pointer != e.Pointer.PointerId) return;
        var at = e.GetCurrentPoint(this).Position;
        Drag(new Point(at.X - _origin.X, at.Y - _origin.Y));
        e.Handled = true;
    }

    private void OnEnded(object sender, PointerRoutedEventArgs e)
    {
        if (_pointer != e.Pointer.PointerId) return;
        Lift();
        e.Handled = true;
    }

    private void Paint(bool held)
    {
        _ring.Opacity = held ? HeldRingOpacity : RingOpacity;
        _knob.Opacity = held ? HeldKnobOpacity : KnobOpacity;
    }
}

public sealed class VirtualJoystick : UserControl
{
    private const string FlyState = "view.flyState";

    private readonly ThumbPad _leftPad;
    private readonly ThumbPad _rightPad;
    private StickSettings? _settings;
    private Stick _left = Stick.Centred;
    private Stick _right = Stick.Centred;
    private bool _leftHeld;
    private bool _rightHeld;
    private bool _grounded;

    public VirtualJoystick()
    {
        _leftPad = new ThumbPad("Left virtual stick",
            held => Hold(ref _leftHeld, held),
            stick => Move(ref _left, stick),
            stick => Move(ref _left, Release(stick, _settings?.ThrottleOnLeft == true)))
        {
            HorizontalAlignment = HorizontalAlignment.Left,
            VerticalAlignment = VerticalAlignment.Bottom,
        };
        _rightPad = new ThumbPad("Right virtual stick",
            held => Hold(ref _rightHeld, held),
            stick => Move(ref _right, stick),
            stick => Move(ref _right, Release(stick, _settings?.ThrottleOnLeft == false)))
        {
            HorizontalAlignment = HorizontalAlignment.Right,
            VerticalAlignment = VerticalAlignment.Bottom,
        };
        Content = new Grid { Children = { _leftPad, _rightPad } };
        Visibility = Visibility.Collapsed;
        AutomationProperties.SetName(this, "Virtual joystick");
        SizeChanged += (_, e) =>
        {
            var pad = StickMath.Pad(e.NewSize.Height);
            _leftPad.Size(pad);
            _rightPad.Size(pad);
        };
        Live.Watch(this, StickSettings.ViewPath, view => Apply(StickSettings.Of(view)));
        Live.Watch(this, FlyState, view =>
        {
            _grounded = StickMath.OnTheGround(view);
            Publish(false);
        });
        Presence.OnHidden(this, LetGo);
        Unloaded += (_, _) => StickSender.Stop();
    }

    internal ThumbPad LeftPad => _leftPad;

    internal ThumbPad RightPad => _rightPad;

    private Stick Release(Stick stick, bool throttle) =>
        _settings is { } settings
            ? StickMath.Released(stick, throttle, settings.AutoCenterThrottle, StickMath.ThrottleParked(settings, _grounded))
            : Stick.Centred;

    private void Apply(StickSettings? next)
    {
        if (_settings?.Show == true && next?.Show != true) LetGo();
        if (StickMath.Resets(_settings, next))
        {
            _left = Stick.Centred;
            _right = Stick.Centred;
        }
        _settings = next;
        Visibility = FormLook.Shown(next?.Show == true);
        Publish(false);
    }

    private void LetGo()
    {
        _leftPad.Cancel();
        _rightPad.Cancel();
        _left = Release(_left, _settings?.ThrottleOnLeft == true);
        _right = Release(_right, _settings?.ThrottleOnLeft == false);
        _leftHeld = false;
        _rightHeld = false;
        Publish(false);
    }

    private void Hold(ref bool side, bool held)
    {
        side = held;
        Publish(false);
    }

    private void Move(ref Stick side, Stick stick)
    {
        side = stick;
        Publish(true);
    }

    private void Publish(bool touched)
    {
        var frame = new StickFrame(_settings, _left, _right, _leftHeld, _rightHeld, _grounded);
        StickSender.Publish(frame, touched);
        var (left, right) = StickMath.Shown(frame);
        _leftPad.Draw(left);
        _rightPad.Draw(right);
    }
}
