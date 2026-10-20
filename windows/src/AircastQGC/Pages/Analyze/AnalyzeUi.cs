using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Controls;
using Microsoft.UI;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Markup;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Shapes;
using Microsoft.Windows.Storage.Pickers;
using Windows.Foundation;

namespace AircastQGC.Pages.Analyze;

internal enum Look { Accent, Standard, Subtle, Critical }

internal static class Ui
{
    private const string XamlNs = "http://schemas.microsoft.com/winfx/2006/xaml/presentation";

    internal static readonly FontFamily Mono = new("Cascadia Mono, Consolas");

    internal static Brush Token(string key) => PreflightLook.Token(key);

    internal static double Space(string key) => PreflightLook.Size(key);

    internal static Visibility Shown(bool shown) => shown ? Visibility.Visible : Visibility.Collapsed;

    internal static TextBlock Text(string text, string size = "FontSizeBody", string brush = "TextPrimaryBrush",
        bool semibold = false, bool mono = false, bool wrap = true) => new()
    {
        Text = text,
        FontFamily = mono ? Mono : PreflightLook.Font,
        FontSize = Space(size),
        FontWeight = semibold ? FontWeights.SemiBold : FontWeights.Normal,
        Foreground = Token(brush),
        TextWrapping = wrap ? TextWrapping.Wrap : TextWrapping.NoWrap,
        TextTrimming = wrap ? TextTrimming.None : TextTrimming.CharacterEllipsis,
        VerticalAlignment = VerticalAlignment.Center,
    };

    internal static TextBlock Caption(string text, string brush = "TextSecondaryBrush") =>
        Text(text, "FontSizeCaption", brush);

    internal static TextBlock SectionHeader(string text) => new()
    {
        Text = text.ToUpper(CultureInfo.CurrentCulture),
        Style = (Style)Application.Current.Resources["StatLabel"],
        Margin = new Thickness(16, 20, 16, 8),
    };

    internal static FontIcon Glyph(string glyph, double size = 16, string brush = "TextPrimaryBrush") => new()
    {
        Glyph = glyph,
        FontFamily = new FontFamily("Segoe Fluent Icons"),
        FontSize = size,
        Foreground = Token(brush),
        VerticalAlignment = VerticalAlignment.Center,
    };

    internal static Border Circle(double size, string background, UIElement child) => new()
    {
        Width = size,
        Height = size,
        CornerRadius = new CornerRadius(size / 2),
        Background = Token(background),
        Child = child,
        VerticalAlignment = VerticalAlignment.Center,
    };

    internal static Border Card(UIElement child, string background = "CardBrush", string stroke = "CardStrokeBrush") => new()
    {
        Background = Token(background),
        BorderBrush = Token(stroke),
        BorderThickness = new Thickness(1),
        CornerRadius = (CornerRadius)Application.Current.Resources["RadiusOverlay"],
        Padding = new Thickness(16),
        Child = child,
    };

    internal static Button Button(string text, Action click, Look look = Look.Standard, bool enabled = true) =>
        Styled(new Button
        {
            Content = text,
            IsEnabled = enabled,
            FontFamily = PreflightLook.Font,
            FontSize = Space("FontSizeBody"),
            MinHeight = Space("SizeControl"),
            Padding = new Thickness(12, 4, 12, 4),
            CornerRadius = (CornerRadius)Application.Current.Resources["RadiusControl"],
        }, look, click);

    internal static Button Styled(Button button, Look look, Action click)
    {
        (look switch
        {
            Look.Accent => (Action<Button>)PreflightLook.Accent,
            Look.Subtle => PreflightLook.Subtle,
            _ => PreflightLook.Standard,
        })(button);
        if (look == Look.Critical)
            new[] { "ButtonForeground", "ButtonForegroundPointerOver", "ButtonForegroundPressed" }
                .ToList()
                .ForEach(key => button.Resources[key] = Token("CriticalBrush"));
        button.Click += (_, _) => click();
        return button;
    }

    internal static Button IconButton(string glyph, string name, Action click, Look look = Look.Subtle) =>
        Styled(new Button
        {
            Content = Glyph(glyph),
            Width = Space("SizeControl"),
            Height = Space("SizeControl"),
            Padding = new Thickness(0),
            CornerRadius = (CornerRadius)Application.Current.Resources["RadiusControl"],
        }.Named(name), look, click);

    internal static T Named<T>(this T element, string name) where T : FrameworkElement
    {
        Microsoft.UI.Xaml.Automation.AutomationProperties.SetName(element, name);
        return element;
    }

    internal static Flyout Menu(IEnumerable<(string Text, Action Pick)> choices)
    {
        var flyout = new Flyout();
        flyout.Content = new ScrollViewer
        {
            MaxHeight = 360,
            Content = Column(choices.Select(choice => Button(choice.Text, () =>
            {
                flyout.Hide();
                choice.Pick();
            }, Look.Subtle).With(item =>
            {
                item.HorizontalAlignment = HorizontalAlignment.Stretch;
                item.HorizontalContentAlignment = HorizontalAlignment.Left;
            }))),
        };
        return flyout;
    }

    internal static DropDownButton Picker(string name) => new DropDownButton
    {
        FontFamily = PreflightLook.Font,
        FontSize = Space("FontSizeBody"),
        MinHeight = Space("SizeControl"),
        CornerRadius = (CornerRadius)Application.Current.Resources["RadiusControl"],
    }.Named(name).With(PreflightLook.Standard);

    internal static UIElement EmptyState(string glyph, string title, string body) => new StackPanel
    {
        HorizontalAlignment = HorizontalAlignment.Center,
        VerticalAlignment = VerticalAlignment.Center,
        MaxWidth = 420,
        Padding = new Thickness(32, 48, 32, 48),
        Spacing = 16,
        Children =
        {
            new Border
            {
                Width = 64,
                Height = 64,
                CornerRadius = new CornerRadius(32),
                HorizontalAlignment = HorizontalAlignment.Center,
                Background = Token("LayerBrush"),
                Child = new FontIcon
                {
                    Glyph = glyph,
                    FontFamily = new FontFamily("Segoe Fluent Icons"),
                    FontSize = 28,
                    Foreground = Token("TextTertiaryBrush"),
                },
            },
            Centered(Text(MapUiJson.SentenceCase(title), "FontSizeSubtitle", semibold: true)),
            Centered(Text(body, brush: "TextSecondaryBrush")).With(t => t.Visibility = Shown(body.Length > 0)),
        },
    };

    internal static TextBlock Centered(TextBlock text)
    {
        text.TextAlignment = TextAlignment.Center;
        text.HorizontalAlignment = HorizontalAlignment.Center;
        return text;
    }

    internal static T With<T>(this T item, Action<T> change)
    {
        change(item);
        return item;
    }

    internal static ItemsControl Column(IEnumerable<UIElement?> items, double spacing = 0) =>
        Items(items, $"<StackPanel Spacing='{Num(spacing)}' />");

    internal static ItemsControl Line(IEnumerable<UIElement?> items, double spacing = 8) =>
        Items(items, $"<StackPanel Orientation='Horizontal' Spacing='{Num(spacing)}' />");

    internal static ItemsControl Layer(IEnumerable<UIElement?> items) => Items(items, "<Grid />");

    private static ItemsControl Items(IEnumerable<UIElement?> items, string panel) => new()
    {
        ItemsPanel = (ItemsPanelTemplate)XamlReader.Load($"<ItemsPanelTemplate xmlns='{XamlNs}'>{panel}</ItemsPanelTemplate>"),
        ItemsSource = items.OfType<UIElement>().ToList(),
        IsTabStop = false,
    };

    internal static void Fill(this ItemsControl host, IEnumerable<UIElement?> items) =>
        host.ItemsSource = items.OfType<UIElement>().ToList();

    internal static Path? Stroke(IEnumerable<Point> points, Brush brush, double thickness)
    {
        var listed = points.ToList();
        return listed.Count < 2
            ? null
            : StrokePath("M" + string.Join(" L", listed.Select(p => $"{Num(p.X)},{Num(p.Y)}")), brush, thickness);
    }

    internal static Path? Segments(IEnumerable<(Point From, Point To)> segments, Brush brush, double thickness)
    {
        var listed = segments.ToList();
        return listed.Count == 0
            ? null
            : StrokePath(string.Join(" ", listed.Select(s => $"M{Num(s.From.X)},{Num(s.From.Y)} L{Num(s.To.X)},{Num(s.To.Y)}")), brush, thickness);
    }

    private static Path StrokePath(string data, Brush brush, double thickness)
    {
        var path = (Path)XamlReader.Load($"<Path xmlns='{XamlNs}' Data='{data}' />");
        path.Stroke = brush;
        path.StrokeThickness = thickness;
        path.StrokeLineJoin = PenLineJoin.Round;
        path.IsHitTestVisible = false;
        return path;
    }

    internal static string Num(double value) => value.ToString("0.##", CultureInfo.InvariantCulture);

    internal static void Off<T>(DependencyObject owner, Func<T> work, Action<T> then)
    {
        var ui = owner.DispatcherQueue;
        Task.Run(work).ContinueWith(done =>
        {
            if (done.IsCompletedSuccessfully) ui.TryEnqueue(() => then(done.Result));
        }, TaskScheduler.Default);
    }

    internal static void Off(Action work) => Task.Run(work);

    internal static async Task<bool> Confirm(XamlRoot root, string title, string text, string confirm, string cancel = "Cancel") =>
        await Show(new ContentDialog
        {
            XamlRoot = root,
            Title = title,
            Content = new TextBlock { Text = text, TextWrapping = TextWrapping.Wrap },
            PrimaryButtonText = confirm,
            CloseButtonText = cancel,
            DefaultButton = ContentDialogButton.Close,
        }) == ContentDialogResult.Primary;

    internal static async Task Tell(XamlRoot root, string title, string text) =>
        await Show(new ContentDialog
        {
            XamlRoot = root,
            Title = title,
            Content = new TextBlock { Text = text, TextWrapping = TextWrapping.Wrap },
            CloseButtonText = "OK",
            DefaultButton = ContentDialogButton.Close,
        });

    private static bool _asking;

    private static async Task<ContentDialogResult> Show(ContentDialog dialog)
    {
        if (_asking || VisualTreeHelper.GetOpenPopupsForXamlRoot(dialog.XamlRoot).Any(popup => popup.Child is ContentDialog))
            return ContentDialogResult.None;
        _asking = true;
        try
        {
            return await dialog.ShowAsync();
        }
        finally
        {
            _asking = false;
        }
    }

    internal static async Task<string?> PickFile(UIElement owner, Func<WindowId, FileOpenPicker> picker) =>
        (await picker(owner.XamlRoot.ContentIslandEnvironment.AppWindowId).PickSingleFileAsync())?.Path is { Length: > 0 } path ? path : null;

    internal static async Task<string?> PickFolder(UIElement owner) =>
        (await new FolderPicker(owner.XamlRoot.ContentIslandEnvironment.AppWindowId).PickSingleFolderAsync())?.Path is { Length: > 0 } path
            ? path
            : null;

    internal static bool IsEmpty(JsonObject view) => view.Count == 0;

    internal static int Int(JsonNode? node, int fallback = 0) =>
        MapUiJson.Number(node) is double number ? (int)number : fallback;

    internal static string? Refusal(JsonObject answer, string subject = "The vehicle") =>
        answer.Count == 0 ? $"{subject} did not answer." : VideoJson.Refusal(answer, $"{subject} refused.");
}

internal static class Lane
{
    private static readonly object Gate = new();
    private static Task _tail = Task.CompletedTask;

    internal static void Run(Action work)
    {
        lock (Gate) _tail = _tail.ContinueWith(_ => work(), TaskScheduler.Default);
    }
}
