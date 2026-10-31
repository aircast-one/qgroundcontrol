using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Text.Json.Nodes;
using AircastQGC.Forms;
using AircastQGC.Pages.Analyze;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Shapes;
using Windows.Foundation;

namespace AircastQGC.Controls;

internal sealed record LayoutState(
    ImmutableList<string> Order,
    ImmutableHashSet<string> Hidden,
    ImmutableDictionary<string, Point> Offsets)
{
    public static readonly LayoutState Default = new([], [StatusKeys.Gimbal], ImmutableDictionary<string, Point>.Empty);

    public JsonObject ToJson() => new()
    {
        ["order"] = new JsonArray(Order.Select(k => (JsonNode?)JsonValue.Create(k)).ToArray()),
        ["hidden"] = new JsonArray(Hidden.Order().Select(k => (JsonNode?)JsonValue.Create(k)).ToArray()),
        ["offsets"] = new JsonObject(Offsets.Select(p => KeyValuePair.Create(p.Key, (JsonNode?)new JsonArray(p.Value.X, p.Value.Y)))),
    };

    public static LayoutState FromJson(JsonObject? json) => json is null ? Default : new(
        Strings(json["order"]).ToImmutableList(),
        Strings(json["hidden"]).ToImmutableHashSet(),
        (json["offsets"] as JsonObject ?? [])
            .Select(p => (p.Key, At: (p.Value as JsonArray ?? []).Select(n => MapUiJson.Number(n)).ToList()))
            .Where(p => p.At is [double, double])
            .ToImmutableDictionary(p => p.Key, p => new Point(p.At[0]!.Value, p.At[1]!.Value)));

    private static IEnumerable<string> Strings(JsonNode? node) =>
        (node as JsonArray ?? []).Select(MapUiJson.Text).Where(text => text.Length > 0);
}

internal static class OverlayLayout
{
    public const string LockedNote = "Disarm to change the layout.";
    public const int ResetArmMs = 4000;

    private static readonly string StorePath = System.IO.Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "AircastQGC", "fly-overlay-layout.json");

    private static LayoutState _state = Load();

    public static event Action? Changed;

    public static bool Editing { get; private set; }

    public static bool Locked { get; private set; }

    public static IReadOnlyList<string> Order => _state.Order;

    public static bool IsHidden(string key) => _state.Hidden.Contains(key);

    public static Point Offset(string key) => _state.Offsets.GetValueOrDefault(key);

    public static void StartEditing()
    {
        if (Locked || Editing) return;
        Editing = true;
        Changed?.Invoke();
    }

    public static void StopEditing()
    {
        if (!Editing) return;
        Editing = false;
        Changed?.Invoke();
    }

    public static void LockWhileArmed(bool armed)
    {
        if (armed == Locked) return;
        Locked = armed;
        Editing = Editing && !armed;
        Changed?.Invoke();
    }

    public static void SetHidden(string key, bool hide) =>
        Save(_state with { Hidden = hide ? _state.Hidden.Add(key) : _state.Hidden.Remove(key) });

    public static void SaveOrder(IEnumerable<string> keys) => Save(_state with { Order = keys.ToImmutableList() });

    public static void SaveOffset(string key, Point offset) => Save(_state with { Offsets = _state.Offsets.SetItem(key, offset) });

    public static void Reset() => Save(LayoutState.Default);

    public static IReadOnlyList<string> Ordered(IReadOnlyList<string> available, IReadOnlyList<string> order) =>
        order.Where(available.Contains).Concat(available.Where(key => !order.Contains(key))).ToList();

    public static IReadOnlyList<string>? Moved(IReadOnlyList<string> keys, string key, int delta)
    {
        var from = keys.ToList().IndexOf(key);
        var to = from + delta;
        if (from < 0 || to < 0 || to >= keys.Count) return null;
        var rest = keys.Where((_, at) => at != from).ToList();
        return rest.Take(to).Append(key).Concat(rest.Skip(to)).ToList();
    }

    public static Point Clamped(Rect bounds, Size room, Point offset, Point delta)
    {
        static double Clamp(double move, double low, double high, double size, double space) =>
            size > space ? 0 : Math.Clamp(move, -low, Math.Max(-low, space - high));
        return new Point(
            offset.X + Clamp(delta.X, bounds.Left, bounds.Right, bounds.Width, room.Width),
            offset.Y + Clamp(delta.Y, bounds.Top, bounds.Bottom, bounds.Height, room.Height));
    }

    private static void Save(LayoutState next)
    {
        _state = next;
        try
        {
            Directory.CreateDirectory(System.IO.Path.GetDirectoryName(StorePath)!);
            File.WriteAllText(StorePath, next.ToJson().ToJsonString());
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        {
        }
        Changed?.Invoke();
    }

    private static LayoutState Load()
    {
        try
        {
            return File.Exists(StorePath) ? LayoutState.FromJson(JsonNode.Parse(File.ReadAllText(StorePath)) as JsonObject) : LayoutState.Default;
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException or JsonException)
        {
            return LayoutState.Default;
        }
    }
}

public class LayoutSlot : Grid
{
    private const double HiddenOpacity = 0.35;
    private const double BadgeSize = 24;
    private const int DecorationLayer = 1000;
    private const string LockGlyph = "\uE72E";
    private const string ShowGlyph = "\uE710";
    private const string HideGlyph = "\uE711";

    private readonly Rectangle _outline = new()
    {
        StrokeThickness = 1,
        StrokeDashArray = [4, 3],
        RadiusX = 8,
        RadiusY = 8,
        IsHitTestVisible = false,
        Margin = new Thickness(-3),
    };
    private readonly Border _grip = new() { Background = new SolidColorBrush(Microsoft.UI.Colors.Transparent) };
    private readonly Button _badge = new()
    {
        Width = BadgeSize,
        Height = BadgeSize,
        Padding = new Thickness(0),
        MinWidth = 0,
        MinHeight = 0,
        CornerRadius = new CornerRadius(BadgeSize / 2),
        HorizontalAlignment = HorizontalAlignment.Right,
        VerticalAlignment = VerticalAlignment.Top,
        Margin = new Thickness(0, -BadgeSize / 2, -BadgeSize / 2, 0),
    };
    private readonly FontIcon _badgeGlyph = Ui.Glyph(HideGlyph, 12);
    private readonly TranslateTransform _shift = new();
    private readonly MenuFlyoutItem _editItem = new() { Text = "Edit layout" };
    private FrameworkElement? _room;
    private Point _dragged;
    private bool _hooked;

    public LayoutSlot()
    {
        _outline.Stroke = Ui.Token("AccentBrush");
        _badge.Content = _badgeGlyph;
        PreflightLook.Standard(_badge);
        _badge.Click += (_, _) =>
        {
            if (Hideable) OverlayLayout.SetHidden(Key, !OverlayLayout.IsHidden(Key));
        };
        _grip.ManipulationMode = ManipulationModes.TranslateX | ManipulationModes.TranslateY;
        _grip.ManipulationStarted += (_, _) => _dragged = OverlayLayout.Offset(Key);
        _grip.ManipulationDelta += (_, e) => Drag(new Point(e.Delta.Translation.X, e.Delta.Translation.Y));
        _grip.ManipulationCompleted += (_, _) => OverlayLayout.SaveOffset(Key, _dragged);
        new UIElement[] { _outline, _grip, _badge }.ToList().ForEach(part =>
        {
            Canvas.SetZIndex(part, DecorationLayer);
            part.Visibility = Visibility.Collapsed;
            Children.Add(part);
        });
        RenderTransform = _shift;
        _editItem.Click += (_, _) => OverlayLayout.StartEditing();
        var menu = new MenuFlyout { Items = { _editItem } };
        menu.Opening += (_, _) => _editItem.IsEnabled = !OverlayLayout.Locked && !OverlayLayout.Editing;
        ContextFlyout = menu;
        Loaded += (_, _) =>
        {
            OverlayLayout.Changed -= Paint;
            OverlayLayout.Changed += Paint;
            if (_room is not null) _room.SizeChanged -= OnRoomResized;
            _room = Room;
            if (_room is not null) _room.SizeChanged += OnRoomResized;
            if (!_hooked) Content.OfType<FrameworkElement>().ToList().ForEach(child =>
            {
                child.SizeChanged += (_, _) => Paint();
                child.RegisterPropertyChangedCallback(VisibilityProperty, (_, _) => Paint());
            });
            _hooked = true;
            Paint();
        };
        Unloaded += (_, _) =>
        {
            if (IsLoaded) return;
            OverlayLayout.Changed -= Paint;
            if (_room is not null) _room.SizeChanged -= OnRoomResized;
            _room = null;
        };
    }

    public string Key { get; set; } = "";

    public string Label { get; set; } = "";

    public bool Movable { get; set; } = true;

    public bool Hideable { get; set; } = true;

    public void Drag(Point delta)
    {
        if (!OverlayLayout.Editing || !Movable || Room is not { } room) return;
        var bounds = TransformToVisual(room).TransformBounds(new Rect(0, 0, ActualWidth, ActualHeight));
        _dragged = OverlayLayout.Clamped(bounds, new Size(room.ActualWidth, room.ActualHeight), _dragged, delta);
        Shift(_dragged);
    }

    private FrameworkElement? Room => XamlRoot?.Content is FrameworkElement root ? Ancestor(this, root) : null;

    private static FrameworkElement? Ancestor(FrameworkElement from, FrameworkElement root) =>
        VisualTreeHelper.GetParent(from) is FrameworkElement up
            ? up is Grid { Name: "FlyChrome" } || up == root ? up : Ancestor(up, root)
            : null;

    private IEnumerable<UIElement> Content => Children.Where(child => child != _outline && child != _grip && child != _badge);

    private void Shift(Point offset)
    {
        _shift.X = offset.X;
        _shift.Y = offset.Y;
    }

    private void OnRoomResized(object sender, SizeChangedEventArgs e)
    {
        if (_room is not { } room || ActualWidth <= 0 || !Movable) return;
        var bounds = TransformToVisual(room).TransformBounds(new Rect(0, 0, ActualWidth, ActualHeight));
        _dragged = OverlayLayout.Clamped(bounds, new Size(room.ActualWidth, room.ActualHeight), _dragged, new Point(0, 0));
        Shift(_dragged);
    }

    private void Paint()
    {
        var editing = OverlayLayout.Editing;
        var hidden = Hideable && OverlayLayout.IsHidden(Key);
        _dragged = Movable ? OverlayLayout.Offset(Key) : new Point(0, 0);
        Shift(_dragged);
        Visibility = Ui.Shown(editing || !hidden);
        Content.ToList().ForEach(child => child.Opacity = editing && hidden ? HiddenOpacity : 1);
        var present = Content.OfType<FrameworkElement>().Any(child => child.Visibility == Visibility.Visible && child.ActualWidth > 0 && child.ActualHeight > 0);
        _outline.Visibility = Ui.Shown(editing && present);
        _grip.Visibility = Ui.Shown(editing && present && Movable);
        _badge.Visibility = Ui.Shown(editing && present);
        _badge.IsEnabled = Hideable;
        _badgeGlyph.Glyph = !Hideable ? LockGlyph : hidden ? ShowGlyph : HideGlyph;
        var name = Label.Length > 0 ? Label : Key;
        AutomationProperties.SetName(_badge, !Hideable ? $"{name} is always shown" : hidden ? $"Show {name}" : $"Hide {name}");
        ToolTipService.SetToolTip(_badge, !Hideable ? "Always shown" : hidden ? "Show" : "Hide");
        AutomationProperties.SetName(_grip, $"Move {name}");
    }
}

public sealed class LayoutControls : UserControl
{
    private readonly Button _edit;
    private readonly Button _reset;
    private readonly Button _done;
    private readonly TextBlock _note = Ui.Caption(OverlayLayout.LockedNote, "TextTertiaryBrush");
    private readonly DispatcherTimer _disarm = new() { Interval = TimeSpan.FromMilliseconds(OverlayLayout.ResetArmMs) };
    private bool _resetArmed;

    public LayoutControls()
    {
        _edit = Ui.Button("Edit layout", OverlayLayout.StartEditing).Named("Edit layout");
        _reset = Ui.Button("Reset layout", Reset).Named("Reset layout");
        _done = Ui.Button("Done", OverlayLayout.StopEditing, Look.Accent).Named("Done editing layout");
        _disarm.Tick += (_, _) =>
        {
            _disarm.Stop();
            _resetArmed = false;
            Paint();
        };
        Content = new StackPanel
        {
            Spacing = 4,
            Children = { new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8, Children = { _edit, _reset, _done } }, _note },
        };
        Loaded += (_, _) =>
        {
            OverlayLayout.Changed += Paint;
            Paint();
        };
        Unloaded += (_, _) =>
        {
            OverlayLayout.Changed -= Paint;
            _disarm.Stop();
        };
    }

    private void Reset()
    {
        if (_resetArmed)
        {
            _disarm.Stop();
            _resetArmed = false;
            OverlayLayout.Reset();
            return;
        }
        _resetArmed = true;
        _disarm.Start();
        Paint();
    }

    private void Paint()
    {
        var editing = OverlayLayout.Editing;
        _edit.Visibility = Ui.Shown(!editing);
        _edit.IsEnabled = !OverlayLayout.Locked;
        _reset.Visibility = Ui.Shown(editing);
        _done.Visibility = Ui.Shown(editing);
        _reset.Content = _resetArmed ? "Tap again to reset" : "Reset layout";
        _reset.Foreground = Ui.Token(_resetArmed ? "CriticalBrush" : "TextPrimaryBrush");
        AutomationProperties.SetName(_reset, _resetArmed ? "Tap again to reset" : "Reset layout");
        _note.Visibility = Ui.Shown(OverlayLayout.Locked && !editing);
    }
}

public sealed class LayoutEditBar : UserControl
{
    public LayoutEditBar()
    {
        Visibility = Visibility.Collapsed;
        Content = Ui.Card(new StackPanel
        {
            Orientation = Orientation.Horizontal,
            Spacing = 16,
            Children =
            {
                Ui.Text("Hide a widget with its badge, drag it to move it. Order the title bar in All status.", "FontSizeBody", "TextSecondaryBrush").With(t => t.MaxWidth = 360),
                new LayoutControls(),
            },
        }).With(card => card.Padding = new Thickness(16, 10, 16, 10));
        AutomationProperties.SetName(this, "Layout editing");
        Loaded += (_, _) =>
        {
            OverlayLayout.Changed += Paint;
            Paint();
        };
        Unloaded += (_, _) => OverlayLayout.Changed -= Paint;
    }

    private void Paint() => Visibility = Ui.Shown(OverlayLayout.Editing);
}
