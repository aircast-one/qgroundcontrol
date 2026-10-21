using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Linq;
using AircastQGC.Pages.Analyze;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Windows.Foundation;

namespace AircastQGC.Controls;

internal sealed record StatusCell(string Key, string Label, FrameworkElement Cell, bool Vehicle);

internal static class StatusFit
{
    public static (IReadOnlyList<int> Fitted, IReadOnlyList<int> Overflow, double Used, bool Squeezed) Fit(
        IReadOnlyList<int> sequence, IReadOnlyList<double> widths, IReadOnlyList<double> compact, Func<int, int> rank, double room, double spacing)
    {
        var present = sequence.Where(i => widths[i] > 0).ToList();
        var pinned = present.Where(i => rank(i) > 0).OrderByDescending(rank).ToList();
        var free = present.Where(i => rank(i) == 0).ToList();
        (ImmutableList<int> Taken, double Used) Pick(IReadOnlyList<double> sized)
        {
            double Add(double used, int i) => used + (used > 0 ? spacing : 0) + sized[i];
            var held = pinned.Aggregate((Taken: ImmutableList<int>.Empty, Used: 0.0),
                (acc, i) => Add(acc.Used, i) <= room ? (acc.Taken.Add(i), Add(acc.Used, i)) : acc);
            var filled = free.Aggregate((held.Taken, held.Used, Open: true),
                (acc, i) => acc.Open && Add(acc.Used, i) <= room ? (acc.Taken.Add(i), Add(acc.Used, i), true) : (acc.Taken, acc.Used, false));
            return (filled.Taken, filled.Used);
        }
        var natural = Pick(widths);
        var squeezed = pinned.Any(i => !natural.Taken.Contains(i)) && present.Any(i => compact[i] < widths[i]);
        var (taken, used) = squeezed ? Pick(compact) : natural;
        var fitted = present.Where(taken.Contains).ToList();
        return (fitted, present.Except(fitted).ToList(), used, squeezed);
    }
}

internal sealed class FitRow : Panel
{
    private const double Spacing = 4;

    private IReadOnlyList<int> _sequence = [];
    private ImmutableHashSet<int> _fitted = ImmutableHashSet<int>.Empty;
    private ImmutableHashSet<int> _shown = ImmutableHashSet<int>.Empty;
    private bool _moreShown;
    private bool _applying;

    public event Action? Refitted;

    public event Action? NeedChanged;

    public double Needed { get; private set; }

    public IReadOnlyList<int> Sequence
    {
        get => _sequence;
        set
        {
            _sequence = value;
            InvalidateMeasure();
        }
    }

    public Func<int, int> Rank { get; init; } = _ => 0;

    public IReadOnlyList<int> Overflow { get; private set; } = [];

    public bool Fits(int index) => _fitted.Contains(index) && _shown.Contains(index);

    private IReadOnlyList<Border> Slots => Children.OfType<Border>().ToList();

    private static double Natural(Border slot, double height)
    {
        if (slot.Child is not FrameworkElement cell) return 0;
        cell.Measure(new Size(double.PositiveInfinity, height));
        return cell.Visibility == Visibility.Visible ? cell.DesiredSize.Width : 0;
    }

    private static double Saving(Border slot) => slot.Child is ContentControl { Content: DropPanel drop } ? drop.Saving : 0;

    protected override Size MeasureOverride(Size available)
    {
        var slots = Slots;
        if (slots.Count == 0) return new Size(0, 0);
        var height = double.IsInfinity(available.Height) ? double.PositiveInfinity : available.Height;
        var widths = slots.Select(slot => Natural(slot, height)).ToList();
        var compact = slots.Select((slot, i) => widths[i] - Saving(slot)).ToList();
        var more = slots.Count - 1;
        var cells = _sequence.Where(i => i >= 0 && i < more).ToList();
        var anyPresent = Enumerable.Range(0, more).Any(i => widths[i] > 0);
        var reserve = anyPresent ? widths[more] + Spacing : 0;
        var (fitted, overflow, used, squeezed) = StatusFit.Fit(cells, widths, compact, Rank, Math.Max(0, available.Width - reserve), Spacing);
        var pinned = cells.Where(i => widths[i] > 0 && Rank(i) > 0).Select(i => compact[i]).ToList();
        Need(pinned.Sum() + Spacing * pinned.Count + (anyPresent ? widths[more] : 0));
        slots.Select((slot, i) => (slot, i)).ToList().ForEach(entry =>
            entry.slot.Measure(new Size(squeezed && compact[entry.i] < widths[entry.i] ? compact[entry.i] : double.PositiveInfinity, height)));
        _fitted = fitted.ToImmutableHashSet();
        Overflow = overflow;
        var shown = Enumerable.Range(0, more).Where(i => cells.Contains(i) && (_fitted.Contains(i) || widths[i] <= 0)).ToImmutableHashSet();
        if (!shown.SetEquals(_shown) || anyPresent != _moreShown) Apply(shown, anyPresent);
        var tallest = slots.Select(slot => slot.Child?.DesiredSize.Height ?? 0).DefaultIfEmpty(0).Max();
        return new Size(used + (anyPresent ? reserve - (fitted.Count == 0 ? Spacing : 0) : 0), tallest);
    }

    protected override Size ArrangeOverride(Size final)
    {
        var slots = Slots;
        if (slots.Count == 0) return final;
        var more = slots.Count - 1;
        var placed = _sequence.Where(i => _fitted.Contains(i)).Aggregate(0.0, (x, i) =>
        {
            var width = slots[i].DesiredSize.Width;
            slots[i].Arrange(new Rect(x, 0, width, final.Height));
            return x + width + Spacing;
        });
        Enumerable.Range(0, more).Where(i => !_fitted.Contains(i)).ToList().ForEach(i => slots[i].Arrange(new Rect(0, 0, 0, 0)));
        slots[more].Arrange(_moreShown ? new Rect(placed, 0, slots[more].DesiredSize.Width, final.Height) : new Rect(0, 0, 0, 0));
        return final;
    }

    private void Need(double needed)
    {
        if (Math.Abs(needed - Needed) < 0.5) return;
        Needed = needed;
        DispatcherQueue.TryEnqueue(() => NeedChanged?.Invoke());
    }

    private void Apply(ImmutableHashSet<int> shown, bool moreShown)
    {
        _shown = shown;
        _moreShown = moreShown;
        if (_applying) return;
        _applying = true;
        DispatcherQueue.TryEnqueue(() =>
        {
            _applying = false;
            var slots = Slots;
            slots.Select((slot, i) => (slot, i)).ToList().ForEach(entry =>
                entry.slot.Visibility = Ui.Shown(entry.i == slots.Count - 1 ? _moreShown : _shown.Contains(entry.i)));
            Refitted?.Invoke();
        });
    }
}

internal sealed class StatusRow
{
    private const double PillHeight = 28;
    private const string MoreGlyph = "\uE76C";
    private const int Pinned = 3;

    private readonly IReadOnlyList<StatusCell> _cells;
    private readonly Func<FrameworkElement, StatusTone> _tone;
    private readonly FitRow _row;
    private readonly Button _more;
    private readonly FontIcon _moreIcon;
    private readonly TextBlock _moreText = Ui.Text("", "FontSizeCaption", wrap: false);
    private readonly Sheet _all;
    private string _ranks = "";

    public StatusRow(IReadOnlyList<StatusCell> cells, Func<FrameworkElement, StatusTone> tone, Func<UIElement> list)
    {
        _cells = cells;
        _tone = tone;
        _row = new FitRow { VerticalAlignment = VerticalAlignment.Center, Rank = Pin };
        _moreIcon = Ui.Glyph(MoreGlyph, 12, "TextSecondaryBrush");
        _more = new Button
        {
            Height = PillHeight,
            MinWidth = 0,
            Padding = new Thickness(8, 0, 8, 0),
            CornerRadius = new CornerRadius(PillHeight / 2),
            BorderThickness = new Thickness(1),
            VerticalAlignment = VerticalAlignment.Center,
            Content = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 6, Children = { _moreText, _moreIcon } },
        }.Named("All status");
        PreflightLook.Chip(_more);
        ToolTipService.SetToolTip(_more, "All status");
        _all = new Sheet("Status", () => null, () => [list(), new Border { Height = 1, Background = Ui.Token("DividerBrush"), Margin = new Thickness(0, 4, 0, 4) }, new LayoutControls()]);
        _more.Click += (_, _) => _all.ShowAt(_more);
        cells.Select(c => new Border { Child = c.Cell, VerticalAlignment = VerticalAlignment.Center })
            .Append(new Border { Child = _more, VerticalAlignment = VerticalAlignment.Center, Visibility = Visibility.Collapsed })
            .ToList()
            .ForEach(_row.Children.Add);
        _row.Refitted += Paint;
        _row.Loaded += (_, _) =>
        {
            OverlayLayout.Changed -= Arrange;
            OverlayLayout.Changed += Arrange;
            Arrange();
        };
        _row.Unloaded += (_, _) =>
        {
            if (!_row.IsLoaded) OverlayLayout.Changed -= Arrange;
        };
        Arrange();
    }

    public UIElement View => _row;

    public double Needed => _row.Needed;

    public event Action? NeedChanged
    {
        add => _row.NeedChanged += value;
        remove => _row.NeedChanged -= value;
    }

    public FrameworkElement Anchor => _more;

    public void HideAll() => _all.Hide();

    public bool Shows(FrameworkElement cell) => _row.Fits(_cells.Select(c => c.Cell).ToList().IndexOf(cell));

    public void Refit() => _row.InvalidateMeasure();

    private void Arrange()
    {
        var keys = _cells.Select(c => c.Key).ToList();
        _row.Sequence = OverlayLayout.Ordered(keys, OverlayLayout.Order)
            .Where(key => !OverlayLayout.IsHidden(key))
            .Select(key => keys.IndexOf(key))
            .ToList();
    }

    private int Pin(int index) => _cells[index].Key == StatusKeys.Battery ? Pinned : Rank(_tone(_cells[index].Cell));

    public void Paint()
    {
        var ranks = string.Join(",", _cells.Select((_, i) => Pin(i)));
        if (ranks != _ranks)
        {
            _ranks = ranks;
            _row.InvalidateMeasure();
        }
        var shown = _row.Sequence.ToHashSet();
        var tucked = _row.Overflow.Concat(Enumerable.Range(0, _cells.Count).Where(i => !shown.Contains(i)))
            .Select(i => _cells[i].Cell)
            .Where(cell => cell.Visibility == Visibility.Visible)
            .ToList();
        var worst = tucked.Select(_tone).OrderByDescending(Rank).DefaultIfEmpty(StatusTone.Normal).First();
        var flagged = tucked.Where(cell => Rank(_tone(cell)) > 0).OrderByDescending(cell => Rank(_tone(cell))).Select(Label).ToList();
        _more.Background = StatusLook.Fill(worst);
        _moreIcon.Foreground = Rank(worst) > 0 ? StatusLook.Tint(worst) : Ui.Token("TextSecondaryBrush");
        _moreText.Text = flagged.Count switch
        {
            0 => "",
            1 => flagged[0],
            _ => $"{flagged[0]} +{flagged.Count - 1}",
        };
        _moreText.Visibility = Ui.Shown(flagged.Count > 0);
        var names = tucked.Select(Label).ToList();
        AutomationProperties.SetHelpText(_more, names.Count == 0 ? "" : $"Not in the title bar: {string.Join(", ", names)}");
        ToolTipService.SetToolTip(_more, names.Count == 0 ? "All status" : $"All status · {string.Join(", ", names)}");
    }

    private string Label(FrameworkElement cell) => _cells.First(c => c.Cell == cell).Label;

    private static int Rank(StatusTone tone) => tone switch
    {
        StatusTone.Critical => 2,
        StatusTone.Caution => 1,
        _ => 0,
    };
}

internal sealed class StatusList
{
    private const string UpGlyph = "\uE70E";
    private const string DownGlyph = "\uE70D";
    private const string NotReporting = "Not reporting";
    private const double HiddenOpacity = 0.5;

    private sealed record Line(StatusCell Cell, TextBlock Label, IReadOnlyList<FrameworkElement> Editors, Button Toggle);

    private readonly Grid _grid = new();
    private readonly ColumnDefinition _cellColumn = new() { Width = GridLength.Auto };
    private readonly IReadOnlyList<Line> _lines;

    public StatusList(IReadOnlyList<StatusCell> cells)
    {
        _grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        _grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        _grid.ColumnDefinitions.Add(_cellColumn);
        _grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        cells.ToList().ForEach(_ => _grid.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto }));
        _lines = cells.Select(Build).ToList();
        _lines.ToList().ForEach(line =>
        {
            line.Cell.Cell.RegisterPropertyChangedCallback(UIElement.VisibilityProperty, (_, _) => Paint());
            Parts(line).ToList().ForEach(_grid.Children.Add);
        });
        _grid.Loaded += (_, _) =>
        {
            OverlayLayout.Changed -= Paint;
            OverlayLayout.Changed += Paint;
            Paint();
        };
        _grid.Unloaded += (_, _) =>
        {
            if (!_grid.IsLoaded) OverlayLayout.Changed -= Paint;
        };
        Paint();
    }

    public UIElement View => _grid;

    private static IEnumerable<FrameworkElement> Parts(Line line) => line.Editors.Prepend(line.Label).Append(line.Cell.Cell);

    private static Line Build(StatusCell cell)
    {
        var label = Ui.Text(cell.Label, brush: "TextSecondaryBrush", wrap: false);
        label.Margin = new Thickness(0, 6, 0, 6);
        Grid.SetColumn(label, 1);
        cell.Cell.HorizontalAlignment = HorizontalAlignment.Right;
        cell.Cell.Margin = new Thickness(0, 3, 0, 3);
        Grid.SetColumn(cell.Cell, 2);
        var up = Ui.IconButton(UpGlyph, $"Move {cell.Label} up", () => Move(cell.Key, -1));
        var down = Ui.IconButton(DownGlyph, $"Move {cell.Label} down", () => Move(cell.Key, 1));
        var arrows = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 2, Margin = new Thickness(0, 0, 8, 0), Children = { up, down } };
        var toggle = Ui.Button("Hide", () => OverlayLayout.SetHidden(cell.Key, !OverlayLayout.IsHidden(cell.Key)), Look.Subtle);
        toggle.Margin = new Thickness(8, 0, 0, 0);
        Grid.SetColumn(toggle, 3);
        return new Line(cell, label, [arrows, toggle], toggle);
    }

    private IReadOnlyList<string> Keys => _lines.Select(l => l.Cell.Key).ToList();

    private static void Move(string key, int delta)
    {
        var keys = OverlayLayout.Ordered(StatusKeys.All, OverlayLayout.Order);
        if (OverlayLayout.Moved(keys, key, delta) is { } moved) OverlayLayout.SaveOrder(moved);
    }

    private void Paint()
    {
        var editing = OverlayLayout.Editing;
        var order = OverlayLayout.Ordered(Keys, OverlayLayout.Order).ToList();
        _cellColumn.Width = editing ? new GridLength(0) : GridLength.Auto;
        _lines.ToList().ForEach(line =>
        {
            var present = line.Cell.Cell.Visibility == Visibility.Visible;
            var hidden = OverlayLayout.IsHidden(line.Cell.Key);
            Parts(line).ToList().ForEach(part =>
            {
                Grid.SetRow(part, order.IndexOf(line.Cell.Key));
                part.VerticalAlignment = VerticalAlignment.Center;
            });
            line.Label.Visibility = Ui.Shown(editing || present);
            line.Label.Foreground = Ui.Token(editing && !present ? "TextTertiaryBrush" : "TextSecondaryBrush");
            line.Label.Opacity = editing && hidden ? HiddenOpacity : 1;
            ToolTipService.SetToolTip(line.Label, editing && !present ? NotReporting : null);
            line.Editors.ToList().ForEach(editor => editor.Visibility = Ui.Shown(editing));
            line.Toggle.Content = hidden ? "Show" : "Hide";
            AutomationProperties.SetName(line.Toggle, hidden ? $"Show {line.Cell.Label} in the title bar" : $"Hide {line.Cell.Label} from the title bar");
        });
    }
}

internal static class StatusKeys
{
    public const string Battery = "battery";
    public const string FlightTime = "flightTime";
    public const string Gps = "gps";
    public const string Rc = "rc";
    public const string RcOverride = "rcOverride";
    public const string Telemetry = "telemetry";
    public const string AircastLink = "aircastLink";
    public const string Esc = "esc";
    public const string Joystick = "joystick";
    public const string UsbRadio = "usbRadio";
    public const string RemoteId = "remoteId";
    public const string GpsResilience = "gpsResilience";
    public const string Rtk = "rtk";
    public const string GcsBattery = "gcsBattery";
    public const string Gimbal = "gimbal";
    public const string SupportForwarding = "supportForwarding";

    public static readonly IReadOnlyList<string> All =
        [Battery, FlightTime, Gps, Rc, RcOverride, Telemetry, AircastLink, Esc, Joystick, UsbRadio, RemoteId, GpsResilience, Rtk, GcsBattery, Gimbal, SupportForwarding];
}
