using System;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Shapes;

namespace AircastQGC.Controls;

public sealed record Attitude(
    bool Available, double Roll, double Pitch, double Heading, string HeadingText, double? HomeBearing, double? PilotBearing)
{
    public static readonly Attitude None = new(false, 0, 0, 0, "", null, null);

    public static Attitude From(JsonObject view) =>
        InstrumentJson.Flag(view, "available")
            ? new Attitude(true,
                InstrumentJson.Number(view, "roll") ?? 0,
                InstrumentJson.Number(view, "pitch") ?? 0,
                InstrumentJson.Number(view, "heading") ?? 0,
                InstrumentJson.Text(view, "headingText"),
                InstrumentJson.Number(view, "homeBearing") ?? InstrumentJson.Number(view, "headingToHome"),
                InstrumentJson.Number(view, "pilotBearing"))
            : None;
}

public sealed partial class TelemetryPanel : UserControl
{
    public const string AttitudeView = "view.attitude";

    private const string Client = "controls.telemetryPanel";
    private const string Absent = "—";
    private const double Radius = 60;
    private const double HorizonFraction = 0.72;
    private const double PitchSpanDegrees = 45;
    private const double UnavailableOpacity = 0.5;

    private static readonly int[] TickDegrees = Enumerable.Range(0, 12).Select(i => i * 30).ToArray();

    private readonly Line[] _ticks;

    public TelemetryPanel()
    {
        InitializeComponent();
        _ticks = TickDegrees.Select(_ => new Line { Stroke = Ring.Stroke, StrokeThickness = 1.5 }).ToArray();
        _ticks.ToList().ForEach(tick => Ticks.Children.Add(tick));
        Loaded += (_, _) => Start();
        Unloaded += (_, _) => Stop();
        Draw(Reading);
    }

    public event EventHandler? ReadingChanged;

    public Attitude Reading { get; private set; } = Attitude.None;

    public void Show() => Visibility = Visibility.Visible;

    public void Hide() => Visibility = Visibility.Collapsed;

    public void Refresh() => Apply(Read());

    private void Start()
    {
        try
        {
            BridgeWatch.Watch(Client, new[] { AttitudeView }, view => OnUi(() => Apply(view)));
        }
        catch (Exception)
        {
        }
        Refresh();
    }

    private static void Stop()
    {
        try
        {
            BridgeWatch.Stop(Client);
        }
        catch (Exception)
        {
        }
    }

    private static JsonObject Read()
    {
        try
        {
            return Bridge.Get(AttitudeView);
        }
        catch (Exception)
        {
            return new JsonObject();
        }
    }

    private void OnUi(Action act)
    {
        if (DispatcherQueue.HasThreadAccess) act();
        else DispatcherQueue.TryEnqueue(() => act());
    }

    private void Apply(JsonObject view)
    {
        var read = Attitude.From(view);
        if (read == Reading) return;
        Reading = read;
        Draw(read);
        ReadingChanged?.Invoke(this, EventArgs.Empty);
    }

    private void Draw(Attitude reading)
    {
        DrawHorizon(reading);
        _ticks.Zip(TickDegrees).ToList().ForEach(tick =>
            Place(tick.First, tick.Second - reading.Heading, tick.Second % 90 == 0 ? 8 : 5));
        Pin(North, OnRing(Radius - 15, -reading.Heading));
        PinBearing(Home, reading.HomeBearing, reading.Heading);
        PinBearing(Pilot, reading.PilotBearing, reading.Heading);
        DialHeading.Text = reading.HeadingText;
        Dial.Opacity = reading.Available ? 1 : UnavailableOpacity;
        HeadingValue.Text = reading.Available && reading.HeadingText.Length > 0 ? reading.HeadingText : Absent;
        RollValue.Text = reading.Available ? Degrees(reading.Roll) : Absent;
        PitchValue.Text = reading.Available ? Degrees(reading.Pitch) : Absent;
    }

    private void DrawHorizon(Attitude reading)
    {
        var offset = reading.Pitch * (Radius * HorizonFraction * 2) / PitchSpanDegrees;
        var inside = Math.Abs(offset) < Radius;
        var half = inside ? Math.Sqrt(Radius * Radius - offset * offset) : 0;
        var (x1, y1) = Turned(-half, offset, -reading.Roll);
        var (x2, y2) = Turned(half, offset, -reading.Roll);
        Horizon.X1 = x1;
        Horizon.Y1 = y1;
        Horizon.X2 = x2;
        Horizon.Y2 = y2;
        Horizon.Visibility = inside ? Visibility.Visible : Visibility.Collapsed;
    }

    private static (double X, double Y) Turned(double x, double y, double degrees)
    {
        var angle = degrees * Math.PI / 180;
        return (Radius + x * Math.Cos(angle) - y * Math.Sin(angle), Radius + x * Math.Sin(angle) + y * Math.Cos(angle));
    }

    private static (double X, double Y) OnRing(double distance, double degrees)
    {
        var angle = degrees * Math.PI / 180;
        return (Radius + distance * Math.Sin(angle), Radius - distance * Math.Cos(angle));
    }

    private static void Place(Line tick, double degrees, double depth)
    {
        var (x1, y1) = OnRing(Radius - 2, degrees);
        var (x2, y2) = OnRing(Radius - depth, degrees);
        tick.X1 = x1;
        tick.Y1 = y1;
        tick.X2 = x2;
        tick.Y2 = y2;
    }

    private static void Pin(FrameworkElement marker, (double X, double Y) at)
    {
        Canvas.SetLeft(marker, at.X - marker.Width / 2);
        Canvas.SetTop(marker, at.Y - marker.Height / 2);
    }

    private static void PinBearing(FrameworkElement marker, double? bearing, double heading)
    {
        marker.Visibility = bearing.HasValue ? Visibility.Visible : Visibility.Collapsed;
        Pin(marker, OnRing(Radius - 8, (bearing ?? 0) - heading));
    }

    private static string Degrees(double value) =>
        ((int)Math.Round(value)).ToString(CultureInfo.InvariantCulture) + "°";
}
