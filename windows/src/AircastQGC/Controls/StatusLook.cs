using System;
using System.Collections.Generic;
using System.Linq;
using AircastQGC.Forms;
using AircastQGC.Pages.Analyze;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Shapes;
using Windows.Foundation;

namespace AircastQGC.Controls;

internal static class FlyRequests
{
    public static event Action<string>? Guided;
    public static event Action<string>? Setup;
    public static event Action<string>? Settings;
    public static event Action? Messages;
    public static event Action? Connections;
    public static event Action? Fly;

    public static void AskGuided(string offer) => Guided?.Invoke(offer);

    public static void OpenSetup(string page) => Setup?.Invoke(page);

    public static void OpenSettings(string page) => Settings?.Invoke(page);

    public static void OpenMessages() => Messages?.Invoke();

    public static void OpenConnections() => Connections?.Invoke();

    public static void OpenFly() => Fly?.Invoke();
}

internal sealed record StatusPill(Button Button, FontIcon Icon, TextBlock Label, TextBlock Extra);

internal sealed class DropPanel : Panel
{
    private const double Spacing = 6;

    public double Saving { get; private set; }

    private double Full => Width(Children);

    private IReadOnlyList<UIElement> Shown(Size room) =>
        Full <= room.Width + 0.5 ? Children.ToList() : Children.Take(Children.Count - 1).ToList();

    private static double Width(IEnumerable<UIElement> children)
    {
        var widths = children.Select(child => child.DesiredSize.Width).Where(width => width > 0).ToList();
        return widths.Sum() + Spacing * Math.Max(0, widths.Count - 1);
    }

    protected override Size MeasureOverride(Size available)
    {
        Children.ToList().ForEach(child => child.Measure(new Size(double.PositiveInfinity, available.Height)));
        Saving = Full - Width(Children.Take(Children.Count - 1));
        return new Size(Width(Shown(available)), Children.Select(child => child.DesiredSize.Height).DefaultIfEmpty(0).Max());
    }

    protected override Size ArrangeOverride(Size final)
    {
        var shown = Shown(final);
        shown.Where(child => child.DesiredSize.Width > 0).Aggregate(0.0, (x, child) =>
        {
            child.Arrange(new Rect(x, (final.Height - child.DesiredSize.Height) / 2, child.DesiredSize.Width, child.DesiredSize.Height));
            return x + child.DesiredSize.Width + Spacing;
        });
        Children.Except(shown).ToList().ForEach(child => child.Arrange(new Rect(0, 0, 0, 0)));
        return final;
    }
}

internal static class StatusLook
{
    private const double PillHeight = 28;
    private const double SheetWidth = 380;
    private const double SheetHeight = 560;
    private const double SparkWidth = SheetWidth - 40;
    private const double SparkHeight = 48;
    private const double MarkerSize = 12;
    private const double BarHeight = 3;

    public static Brush Tint(StatusTone tone) => Ui.Token(tone switch
    {
        StatusTone.Good => "SuccessBrush",
        StatusTone.Caution => "CautionBrush",
        StatusTone.Critical => "CriticalBrush",
        StatusTone.Muted => "TextTertiaryBrush",
        _ => "TextSecondaryBrush",
    });

    public static Brush Ink(StatusTone tone) => tone == StatusTone.Normal ? Ui.Token("TextPrimaryBrush") : Tint(tone);

    public static Brush CellInk(StatusTone tone) => tone is StatusTone.Caution or StatusTone.Critical ? Tint(tone) : Ui.Token("TextPrimaryBrush");

    public static void Dress(Control control, StatusTone tone)
    {
        control.Background = Bare;
        control.BorderBrush = Bare;
        control.Foreground = CellInk(tone);
    }

    private static readonly SolidColorBrush Bare = new(Microsoft.UI.Colors.Transparent);

    public static StatusPill Pill(string name, string glyph)
    {
        var icon = new FontIcon
        {
            Glyph = glyph,
            FontFamily = FormLook.Icons,
            FontSize = 12,
            Foreground = Tint(StatusTone.Muted),
            VerticalAlignment = VerticalAlignment.Center,
            Visibility = Ui.Shown(glyph.Length > 0),
        };
        var label = Ui.Text("", "FontSizeCaption", wrap: false);
        var extra = Ui.Text("", "FontSizeCaption", wrap: false);
        var button = new Button
        {
            Height = PillHeight,
            MinWidth = 0,
            Padding = new Thickness(10, 0, 10, 0),
            CornerRadius = new CornerRadius(PillHeight / 2),
            BorderThickness = new Thickness(1),
            VerticalAlignment = VerticalAlignment.Center,
            Content = new DropPanel { Children = { icon, label, extra } },
        }.Named(name);
        PreflightLook.Chip(button);
        ToolTipService.SetToolTip(button, name);
        return new StatusPill(button, icon, label, extra);
    }

    public static void Paint(StatusPill pill, string text, StatusTone tone, string extra = "")
    {
        pill.Label.Text = text;
        pill.Extra.Text = extra;
        pill.Icon.Foreground = Tint(tone == StatusTone.Normal ? StatusTone.Muted : tone);
        pill.Label.Foreground = CellInk(tone);
        pill.Extra.Foreground = CellInk(tone);
        Dress(pill.Button, tone);
        AutomationProperties.SetHelpText(pill.Button, string.Join("  ", new[] { text, extra }.Where(part => part.Length > 0)));
    }

    public static Style Presenter() => new(typeof(FlyoutPresenter))
    {
        Setters =
        {
            new Setter(Control.BackgroundProperty, Ui.Token("CardBrush")),
            new Setter(Control.BorderBrushProperty, Ui.Token("CardStrokeBrush")),
            new Setter(Control.BorderThicknessProperty, new Thickness(1)),
            new Setter(Control.CornerRadiusProperty, Application.Current.Resources["RadiusOverlay"]),
            new Setter(Control.PaddingProperty, new Thickness(0)),
            new Setter(FrameworkElement.MaxWidthProperty, SheetWidth + 40),
            new Setter(ScrollViewer.VerticalScrollBarVisibilityProperty, ScrollBarVisibility.Disabled),
        },
    };

    public static TextBlock Title(string text) => Ui.Text(text, "FontSizeBodyLarge", semibold: true);

    public static TextBlock Heading(string text) => Ui.Text(text, semibold: true).With(t => t.Margin = new Thickness(0, 8, 0, 0));

    public static TextBlock Line(string text, StatusTone tone = StatusTone.Normal, bool small = false) =>
        Ui.Text(text, small ? "FontSizeCaption" : "FontSizeBody").With(t => t.Foreground = Ink(tone));

    public static TextBlock Footnote(string text) => Ui.Caption(text, "TextTertiaryBrush").With(t => t.Margin = new Thickness(0, 8, 0, 0));

    public static UIElement Row(string label, string value, StatusTone tone = StatusTone.Normal, bool stale = false)
    {
        var name = Ui.Text(label, brush: tone == StatusTone.Muted ? "TextTertiaryBrush" : "TextSecondaryBrush");
        var shown = Ui.Text(value).With(t =>
        {
            t.Foreground = stale ? Ui.Token("TextTertiaryBrush") : Ink(tone == StatusTone.Muted ? StatusTone.Normal : tone);
            t.TextAlignment = TextAlignment.Right;
            t.HorizontalAlignment = HorizontalAlignment.Right;
        });
        Grid.SetColumn(shown, 1);
        return new Grid
        {
            ColumnSpacing = 16,
            ColumnDefinitions = { new ColumnDefinition { Width = GridLength.Auto }, new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) } },
            Children = { name, shown },
        };
    }

    public static UIElement Rows(IReadOnlyList<DetailRow> rows, bool stale) =>
        rows.Count == 0
            ? Line(StatusText.NothingYet, StatusTone.Muted)
            : Ui.Column(rows.Select(r => Row(r.Label, r.Value, r.Tone, stale)), 8);

    public static UIElement Pairs(IEnumerable<(string Label, string Value)> rows) =>
        Ui.Column(rows.Select(r => Row(r.Label, r.Value)), 6);

    public static Button Action(string text, Action click, Look look = Look.Standard) =>
        Ui.Button(text, click, look).With(b => b.HorizontalAlignment = HorizontalAlignment.Left);

    public static Button Link(string text, Action click) =>
        Ui.Button(text, click, Look.Subtle).With(b =>
        {
            b.HorizontalAlignment = HorizontalAlignment.Left;
            b.Padding = new Thickness(0, 4, 0, 4);
            b.Foreground = Ui.Token("AccentTextBrush");
        });

    public static Button ListRow(UIElement content, string name, Action click)
    {
        var row = new Button
        {
            HorizontalAlignment = HorizontalAlignment.Stretch,
            HorizontalContentAlignment = HorizontalAlignment.Stretch,
            Padding = new Thickness(8, 6, 8, 6),
            Content = content,
        }.Named(name);
        PreflightLook.Subtle(row);
        row.Click += (_, _) => click();
        return row;
    }

    public static UIElement Spread(UIElement left, UIElement? right)
    {
        var trailing = right ?? new Border();
        Grid.SetColumn((FrameworkElement)trailing, 1);
        return new Grid
        {
            ColumnSpacing = 12,
            ColumnDefinitions = { new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) }, new ColumnDefinition { Width = GridLength.Auto } },
            Children = { left, trailing },
        };
    }

    public static UIElement Chevron(UIElement left, UIElement? right = null) =>
        Spread(left, Ui.Line([right, Ui.Glyph("\uE76C", 12, "TextTertiaryBrush")], 8));

    public static UIElement Sparkline(IReadOnlyList<double> values, double maximum) =>
        Ui.Layer(AircastLink.SparkRuns(values, maximum)
                .Select(run => Ui.Stroke(run.Select(p => new Point(p.X * SparkWidth, p.Y * (SparkHeight - 1) + 0.5)), Ui.Token("SuccessBrush"), 1.5)))
            .With(layer =>
            {
                layer.Width = SparkWidth;
                layer.Height = SparkHeight;
            });

    private static Grid Share(double share, Brush brush) => new()
    {
        Height = BarHeight,
        VerticalAlignment = VerticalAlignment.Center,
        ColumnDefinitions =
        {
            new ColumnDefinition { Width = new GridLength(Math.Clamp(share, 0, 100), GridUnitType.Star) },
            new ColumnDefinition { Width = new GridLength(100 - Math.Clamp(share, 0, 100), GridUnitType.Star) },
        },
        Children = { new Rectangle { Fill = brush } },
    };

    private static UIElement HomeMarker(double at)
    {
        var share = Math.Clamp(at, 0, 100);
        var marker = new Grid
        {
            Width = MarkerSize,
            Height = MarkerSize,
            Margin = new Thickness(-MarkerSize / 2, 0, -MarkerSize / 2, 0),
            Children =
            {
                new Ellipse { Fill = Ui.Token("SuccessBrush") },
                new TextBlock
                {
                    Text = "H",
                    FontSize = 8,
                    FontWeight = FontWeights.Bold,
                    Foreground = Ui.Token("AccentOnBrush"),
                    HorizontalAlignment = HorizontalAlignment.Center,
                    VerticalAlignment = VerticalAlignment.Center,
                },
            },
        };
        Grid.SetColumn(marker, 1);
        return new Grid
        {
            ColumnDefinitions =
            {
                new ColumnDefinition { Width = new GridLength(share, GridUnitType.Star) },
                new ColumnDefinition { Width = GridLength.Auto },
                new ColumnDefinition { Width = new GridLength(100 - share, GridUnitType.Star) },
            },
            Children = { marker },
        };
    }

    public static UIElement ReturnBar(BatteryHeadline headline, bool live) =>
        Ui.Layer(
        [
            new Rectangle { Height = BarHeight, VerticalAlignment = VerticalAlignment.Center, Fill = Ui.Token("DividerBrush") },
            Share(headline.Percent ?? 0, live ? Tint(headline.BarTone) : Ui.Token("TextTertiaryBrush")),
            Share(headline.Reserve, Ui.Token("CriticalBrush")),
            headline.ReturnAt is double at ? HomeMarker(at) : null,
        ]).With(layer =>
        {
            layer.Height = MarkerSize;
            layer.IsHitTestVisible = false;
            layer.HorizontalContentAlignment = HorizontalAlignment.Stretch;
            AutomationProperties.SetName(layer, headline.BarDescription);
        });
}

internal sealed class Sheet
{
    private const double MaxHeight = 560;
    private const double Width = 380;

    private readonly Flyout _flyout;
    private readonly Func<UIElement?> _render;
    private readonly Func<string>? _key;
    private readonly Action? _refreshed;
    private ContentControl? _live;
    private string? _shownKey;

    public Sheet(string title, Func<UIElement?> live, Func<IEnumerable<UIElement?>>? fixedPart = null, Action? opened = null, Action? closed = null,
        Func<string>? key = null, Action? refreshed = null)
    {
        _render = live;
        _key = key;
        _refreshed = refreshed;
        _flyout = new Flyout { FlyoutPresenterStyle = StatusLook.Presenter(), Placement = FlyoutPlacementMode.BottomEdgeAlignedRight };
        _flyout.Opening += (_, _) =>
        {
            IsOpen = true;
            opened?.Invoke();
            _shownKey = _key?.Invoke();
            _live = new ContentControl { HorizontalContentAlignment = HorizontalAlignment.Stretch, IsTabStop = false, Content = _render() };
            _flyout.Content = new ScrollViewer
            {
                MaxHeight = MaxHeight,
                VerticalScrollBarVisibility = ScrollBarVisibility.Auto,
                Content = new StackPanel
                {
                    Width = Width,
                    Padding = new Thickness(20, 16, 20, 20),
                    Spacing = 8,
                    Children = { StatusLook.Title(title), _live, Ui.Column(fixedPart?.Invoke() ?? [], 8) },
                },
            };
            _refreshed?.Invoke();
        };
        _flyout.Closed += (_, _) =>
        {
            IsOpen = false;
            _live = null;
            _shownKey = null;
            _flyout.Content = null;
            closed?.Invoke();
        };
    }

    public FlyoutPlacementMode Placement
    {
        get => _flyout.Placement;
        init => _flyout.Placement = value;
    }

    public bool IsOpen { get; private set; }

    public void ShowAt(FrameworkElement anchor) => _flyout.ShowAt(anchor);

    public void Hide() => _flyout.Hide();

    public void Refresh()
    {
        if (!IsOpen || _live is null) return;
        _refreshed?.Invoke();
        var key = _key?.Invoke();
        if (key is not null && key == _shownKey) return;
        _shownKey = key;
        _live.Content = _render();
    }
}

internal sealed class HoldButton
{
    private readonly DispatcherTimer _timer = new();
    private readonly Action _held;

    public HoldButton(string name, Func<long> holdMs, Action held)
    {
        _held = held;
        Label = Ui.Text("", "FontSizeBodyLarge", "CriticalBrush", semibold: true).With(t => t.HorizontalAlignment = HorizontalAlignment.Center);
        Button = new Button
        {
            HorizontalAlignment = HorizontalAlignment.Stretch,
            HorizontalContentAlignment = HorizontalAlignment.Center,
            Padding = new Thickness(16),
            Background = Ui.Token("CriticalBgBrush"),
            Content = Label,
        }.Named(name);
        Button.Resources["ButtonBackgroundPointerOver"] = Ui.Token("CriticalBgBrush");
        Button.Resources["ButtonBackgroundPressed"] = Ui.Token("CriticalBgBrush");
        _timer.Tick += (_, _) =>
        {
            _timer.Stop();
            _held();
        };
        Button.AddHandler(UIElement.PointerPressedEvent, new PointerEventHandler((_, _) =>
        {
            _timer.Interval = TimeSpan.FromMilliseconds(Math.Max(1, holdMs()));
            _timer.Start();
        }), true);
        new[] { UIElement.PointerReleasedEvent, UIElement.PointerCaptureLostEvent, UIElement.PointerExitedEvent, UIElement.PointerCanceledEvent }
            .ToList()
            .ForEach(routed => Button.AddHandler(routed, new PointerEventHandler((_, _) => _timer.Stop()), true));
    }

    public Button Button { get; }

    public TextBlock Label { get; }
}
