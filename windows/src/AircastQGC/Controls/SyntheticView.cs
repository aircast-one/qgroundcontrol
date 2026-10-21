using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Windows.Foundation;

namespace AircastQGC.Controls;

internal static class SyntheticAim
{
    public const double DefaultTilt = -15;
    public const double LowestTilt = -90;
    public const double HighestTilt = 30;
    private const double TiltSpan = 90;
    private const double PanSpan = 360;
    private const double OffCentreDegrees = 2;

    public static double Tilt(double from, double draggedPx, double heightPx) =>
        Math.Clamp(from + draggedPx / Math.Max(heightPx, 1) * TiltSpan, -TiltSpan, 0);

    public static double Pan(double from, double draggedPx, double widthPx) =>
        Turned(from + draggedPx / Math.Max(widthPx, 1) * PanSpan + 180) - 180;

    public static bool OffCentre(double tilt, double pan) =>
        Math.Abs(tilt - DefaultTilt) > OffCentreDegrees || Math.Abs(pan) > OffCentreDegrees;

    public static string Degrees(double degrees) => $"{Math.Round(degrees):0}°";

    private static double Turned(double degrees) => (degrees % 360 + 360) % 360;
}

public sealed partial class SyntheticView : UserControl
{
    public const string WaitingHeadline = "Waiting for the aircraft's position";
    public const string WaitingDetail = "The 3D map view draws once the aircraft is connected and has a GPS fix and a home.";
    private const string View = "view.syntheticView";
    private const string GimbalView = "view.gimbal";
    private const string AimPath = "syntheticView.aim";
    private const string GimbalAimPath = "gimbal.aim";
    private const string Drawn = "drawn";
    private const string Failed = "failed";
    private const string LoadingNote = "Loading 3D view…";
    private const string FailedNote = "3D view failed to start";
    private const string RecenterGlyph = "";
    private const double RenderScale = 0.75;
    private const long GimbalEveryMs = 50;
    private const long AimSettlesMs = 1000;
    private static readonly TimeSpan CheckEvery = TimeSpan.FromMilliseconds(500);
    private static readonly Brush TakesInput = new SolidColorBrush(Colors.Transparent);

    private sealed record Drag(uint Pointer, Point Start, Point Last, (double Tilt, double Pan) Aim, long SentAt, bool Gimbal);

    private readonly Grid _root = new();
    private readonly Canvas _stage = new();
    private readonly SwapChainPanel _panel = new();
    private readonly ScaleTransform _scale = new();
    private readonly ProgressRing _spinner = new() { Width = 16, Height = 16, MinWidth = 0, MinHeight = 0 };
    private readonly TextBlock _noteText = FormLook.Caption("", "TextPrimaryBrush");
    private readonly Border _note;
    private readonly FontIcon _recenter = FormLook.Glyph(RecenterGlyph, 12, "TextPrimaryBrush");
    private readonly TextBlock _tiltText = FormLook.Caption("", "TextPrimaryBrush");
    private readonly Button _tiltButton;
    private readonly Slider _tiltSlider;
    private readonly StackPanel _tiltControls;
    private readonly DispatcherQueueTimer _check;
    private JsonObject _view = new();
    private (double Pitch, double Yaw)? _gimbalAt;
    private (double Tilt, double Pan)? _sent;
    private (double Tilt, double Yaw)? _gimbalSent;
    private long _gimbalSentAt;
    private Drag? _drag;
    private IntPtr _surface;
    private (int Width, int Height) _attached;
    private bool _bindFailed;
    private bool _syntheticChosen;
    private bool _painting;
    private bool _wanted;
    private bool _aimable;
    private bool _showing;
    private bool _pictured;

    public SyntheticView()
    {
        _panel.RenderTransform = _scale;
        _stage.Children.Add(_panel);
        _noteText.TextWrapping = TextWrapping.Wrap;
        _noteText.TextAlignment = TextAlignment.Center;
        _noteText.MaxLines = 2;
        _note = new Border
        {
            HorizontalAlignment = HorizontalAlignment.Center,
            VerticalAlignment = VerticalAlignment.Center,
            Margin = new Thickness(16),
            Padding = new Thickness(16, 10, 16, 10),
            CornerRadius = new CornerRadius(20),
            Background = FormLook.Brush("ChipBrush"),
            BorderBrush = FormLook.Brush("ControlStrokeBrush"),
            BorderThickness = new Thickness(1),
            IsHitTestVisible = false,
            Visibility = Visibility.Collapsed,
            Child = FormLook.Line(8, _spinner, _noteText),
        };
        _tiltText.Visibility = Visibility.Visible;
        _tiltButton = new Button
        {
            Height = 28,
            Padding = new Thickness(10, 0, 10, 0),
            CornerRadius = new CornerRadius(14),
            HorizontalAlignment = HorizontalAlignment.Center,
            Content = FormLook.Line(4, _recenter, _tiltText),
        };
        PreflightLook.Chip(_tiltButton);
        _tiltButton.Click += (_, _) => SendAim((SyntheticAim.DefaultTilt, 0));
        _tiltSlider = new Slider
        {
            Orientation = Orientation.Vertical,
            Minimum = SyntheticAim.LowestTilt,
            Maximum = SyntheticAim.HighestTilt,
            StepFrequency = 1,
            Height = 140,
            HorizontalAlignment = HorizontalAlignment.Center,
        };
        AutomationProperties.SetName(_tiltSlider, "Camera tilt");
        _tiltSlider.ValueChanged += (_, e) =>
        {
            if (!_painting) SendAim((e.NewValue, Pan));
        };
        _tiltControls = new StackPanel
        {
            Spacing = 8,
            HorizontalAlignment = HorizontalAlignment.Center,
            Visibility = Visibility.Collapsed,
            Children = { _tiltButton, _tiltSlider },
        };
        _root.Children.Add(_stage);
        _root.Children.Add(_note);
        _root.SizeChanged += (_, _) => Fit();
        _root.PointerPressed += OnPressed;
        _root.PointerMoved += OnMoved;
        _root.PointerReleased += OnReleased;
        _root.PointerCanceled += (_, _) => EndDrag();
        _root.PointerCaptureLost += (_, _) => _drag = null;
        Content = _root;
        Visibility = Visibility.Collapsed;
        AutomationProperties.SetName(this, "3D map view");
        _check = DispatcherQueue.GetForCurrentThread().CreateTimer();
        _check.Interval = CheckEvery;
        _check.Tick += (_, _) => Sync();
        Live.Watch(this, View, view =>
        {
            _view = view;
            Sync();
        });
        Live.Watch(this, GimbalView, view => _gimbalAt = GimbalAt(view));
        Live.Watch(this, CameraList.View, view =>
        {
            var chosen = CameraList.Parse(view)?.Cameras.Any(c => c.Active && c.Source == CameraList.SyntheticSource) == true;
            var changed = chosen != _syntheticChosen;
            _syntheticChosen = chosen;
            PaintAim();
            if (changed) Changed?.Invoke(this, EventArgs.Empty);
        });
        Loaded += (_, _) => Start();
        Unloaded += (_, _) => Halt();
    }

    public event EventHandler? Changed;

    public FrameworkElement AimRail => _tiltControls;

    public bool Showing => _showing;

    public bool Chosen => _syntheticChosen;

    public bool Pictured => _pictured;

    public bool Wanted
    {
        get => _wanted;
        set
        {
            _wanted = value;
            Sync();
        }
    }

    public bool Aimable
    {
        get => _aimable;
        set
        {
            _aimable = value;
            _sent = null;
            EndDrag();
            Sync();
        }
    }

    private bool Available => VideoJson.Flag(_view, "available");

    private bool Native => VideoJson.Flag(_view, "native");

    private bool CanAim => _aimable && VideoJson.Flag(_view, "aimable");

    private bool SteersGimbal => _aimable && Available && !VideoJson.Flag(_view, "aimable", true);

    private double Tilt => VideoJson.Number(_view["tilt"]) ?? SyntheticAim.DefaultTilt;

    private double Pan => VideoJson.Number(_view["pan"]) ?? 0;

    private (int Width, int Height) SurfaceSize =>
        _panel.Width >= 1 && _panel.Height >= 1 ? ((int)_panel.Width, (int)_panel.Height) : (0, 0);

    private bool Attachable =>
        IsLoaded && _showing && SurfaceSize.Width > 0 && SurfaceSize.Height > 0 && XamlRoot?.IsHostVisible == true && Lineage(this).All(e => e.Visibility == Visibility.Visible);

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
        EndDrag();
        Detach();
    }

    private void OnRootChanged(XamlRoot sender, XamlRootChangedEventArgs args) => Fit();

    private void Fit()
    {
        var (width, height) = (_root.ActualWidth, _root.ActualHeight);
        var scale = RenderScale * (XamlRoot?.RasterizationScale ?? 1);
        _stage.Clip = new RectangleGeometry { Rect = new Rect(0, 0, Math.Max(width, 0), Math.Max(height, 0)) };
        _panel.Width = Math.Max(1, Math.Round(width * scale));
        _panel.Height = Math.Max(1, Math.Round(height * scale));
        _scale.ScaleX = width > 0 ? width / _panel.Width : 1;
        _scale.ScaleY = height > 0 ? height / _panel.Height : 1;
        Sync();
    }

    private void Sync()
    {
        var was = (_showing, _pictured);
        _showing = _wanted && Available && Native;
        Visibility = VideoJson.Shown(_showing);
        if (Attachable) Attach();
        else Detach();
        _pictured = _showing && !_bindFailed && VideoJson.Text(_view, "draw") == Drawn;
        _stage.Opacity = _pictured ? 1 : 0;
        if (was != (_showing, _pictured)) Changed?.Invoke(this, EventArgs.Empty);
        _root.Background = CanAim || SteersGimbal ? TakesInput : null;
        PaintNote();
        PaintAim();
    }

    private void Attach()
    {
        var size = SurfaceSize;
        if (_surface != IntPtr.Zero && size == _attached) return;
        _surface = _surface != IntPtr.Zero ? _surface : CompositionSurface.Bind(_panel);
        _bindFailed = _surface == IntPtr.Zero;
        if (_bindFailed) return;
        _attached = size;
        NativeMethods.qgc_synthetic_attach(_surface, size.Width, size.Height);
    }

    private void Detach()
    {
        if (_surface == IntPtr.Zero) return;
        var surface = _surface;
        _surface = IntPtr.Zero;
        _attached = default;
        NativeMethods.qgc_synthetic_release(surface);
        CompositionSurface.Close(surface);
    }

    private void PaintNote()
    {
        var draw = VideoJson.Text(_view, "draw");
        var failed = _bindFailed || draw == Failed;
        var loading = !failed && draw != Drawn;
        var note = failed ? FailedNote : loading ? LoadingNote : null;
        _spinner.IsActive = loading && _showing;
        _spinner.Visibility = VideoJson.Shown(loading);
        FormLook.Say(_noteText, note);
        _note.Visibility = VideoJson.Shown(!string.IsNullOrEmpty(note));
    }

    private void PaintAim()
    {
        var shown = CanAim && _syntheticChosen;
        _tiltControls.Visibility = VideoJson.Shown(shown);
        if (!shown) return;
        var (tilt, pan) = (Tilt, Pan);
        var off = SyntheticAim.OffCentre(tilt, pan);
        var text = SyntheticAim.Degrees(tilt);
        _tiltText.Text = $"Tilt {text}";
        _recenter.Visibility = VideoJson.Shown(off);
        AutomationProperties.SetName(_tiltButton, off ? $"Tilt {text}, off centre" : $"Tilt {text}");
        ToolTipService.SetToolTip(_tiltButton, off ? "Recenter the view" : null);
        _painting = true;
        _tiltSlider.Value = Math.Clamp(tilt, SyntheticAim.LowestTilt, SyntheticAim.HighestTilt);
        _painting = false;
    }

    private void OnPressed(object sender, PointerRoutedEventArgs e)
    {
        if (!CanAim && !SteersGimbal) return;
        var at = e.GetCurrentPoint(_root).Position;
        var gimbal = !CanAim;
        _drag = new Drag(e.Pointer.PointerId, at, at, gimbal ? GimbalStart() : _sent ?? (Tilt, Pan), 0, gimbal);
        _root.CapturePointer(e.Pointer);
        e.Handled = true;
    }

    private void OnMoved(object sender, PointerRoutedEventArgs e)
    {
        if (_drag is not { } drag || e.Pointer.PointerId != drag.Pointer) return;
        e.Handled = true;
        var at = e.GetCurrentPoint(_root).Position;
        var now = Environment.TickCount64;
        var due = drag.Gimbal && now - drag.SentAt >= GimbalEveryMs;
        var moved = drag.Gimbal
            ? drag with { Last = at, SentAt = due ? now : drag.SentAt }
            : drag with { Last = at, Aim = Dragged(drag.Aim, at.X - drag.Last.X, at.Y - drag.Last.Y) };
        _drag = moved;
        if (!moved.Gimbal) SendAim(moved.Aim);
        else if (due) AimGimbal(GimbalTarget(moved));
    }

    private void OnReleased(object sender, PointerRoutedEventArgs e)
    {
        if (_drag is not { } drag || e.Pointer.PointerId != drag.Pointer) return;
        e.Handled = true;
        if (drag.Gimbal && drag.Last != drag.Start) AimGimbal(GimbalTarget(drag));
        EndDrag();
    }

    private void EndDrag()
    {
        _drag = null;
        _root.ReleasePointerCaptures();
    }

    private (double Tilt, double Pan) Dragged((double Tilt, double Pan) from, double dx, double dy) =>
        (SyntheticAim.Tilt(from.Tilt, dy, _root.ActualHeight), SyntheticAim.Pan(from.Pan, dx, _root.ActualWidth));

    private (double Tilt, double Pan) GimbalTarget(Drag drag) =>
        Dragged(drag.Aim, drag.Last.X - drag.Start.X, drag.Last.Y - drag.Start.Y);

    private (double Tilt, double Pan) GimbalStart() =>
        _gimbalSent is { } sent && Environment.TickCount64 - _gimbalSentAt < AimSettlesMs ? sent : _gimbalAt ?? (0, 0);

    private void SendAim((double Tilt, double Pan) aim)
    {
        _sent = aim;
        Bridge.Invoke(AimPath, new JsonArray(aim.Tilt, aim.Pan));
    }

    private void AimGimbal((double Tilt, double Yaw) aim)
    {
        _gimbalSent = aim;
        _gimbalSentAt = Environment.TickCount64;
        GimbalCalls.Call(GimbalAimPath, aim.Tilt, aim.Yaw);
    }

    private static (double Pitch, double Yaw)? GimbalAt(JsonObject view) =>
        (view["gimbals"] as JsonArray ?? new JsonArray()).OfType<JsonObject>()
            .Where(g => VideoJson.Flag(g, "active"))
            .Select(g => (Pitch: VideoJson.Number(g["pitch"]), Yaw: VideoJson.Number(g["bodyYaw"])))
            .Where(g => g.Pitch is not null && g.Yaw is not null)
            .Select(g => ((double Pitch, double Yaw)?)(g.Pitch!.Value, g.Yaw!.Value))
            .FirstOrDefault();

    private static IEnumerable<UIElement> Lineage(DependencyObject? from) =>
        from is null ? [] : (from is UIElement element ? [element] : Array.Empty<UIElement>()).Concat(Lineage(VisualTreeHelper.GetParent(from)));
}
