using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text.Json.Nodes;
using System.Threading;
using System.Threading.Tasks;
using AircastQGC.Controls;
using AircastQGC.Core;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;

namespace AircastQGC.Forms;

internal enum ButtonLook { Standard, Accent, Subtle }

internal static class FormLook
{
    public const double ChoiceWidth = 220;
    public const double NumberWidth = 160;
    public const double RowMinHeight = 56;

    public static FontFamily Icons => (FontFamily)Application.Current.Resources["IconFontFamily"];

    public static Brush Brush(string key) => PreflightLook.Token(key);

    public static double Size(string key) => PreflightLook.Size(key);

    public static CornerRadius Radius(string key) => (CornerRadius)Application.Current.Resources[key];

    public static Visibility Shown(bool on) => on ? Visibility.Visible : Visibility.Collapsed;

    public static TextBlock Text(string text, string size = "FontSizeBody", string brush = "TextPrimaryBrush", bool strong = false) => new()
    {
        Text = text,
        FontFamily = PreflightLook.Font,
        FontSize = Size(size),
        Foreground = Brush(brush),
        FontWeight = strong ? FontWeights.SemiBold : FontWeights.Normal,
        TextWrapping = TextWrapping.Wrap,
        Visibility = Shown(text.Length > 0),
    };

    public static TextBlock Caption(string text, string brush = "TextSecondaryBrush") => Text(text, "FontSizeCaption", brush);

    public static TextBlock Heading(string text) => Margined(Text(text, "FontSizeBody", "TextPrimaryBrush", true), new Thickness(0, 24, 0, 8));

    public static TextBlock Footnote(string text) => Margined(Caption(text), new Thickness(4, 8, 4, 8));

    public static void Say(TextBlock block, string? text)
    {
        block.Text = text ?? "";
        block.Visibility = Shown(!string.IsNullOrEmpty(text));
    }

    public static T Margined<T>(T element, Thickness margin) where T : FrameworkElement
    {
        element.Margin = margin;
        return element;
    }

    public static T Fill<T>(T panel, IEnumerable<UIElement?> children) where T : Panel
    {
        panel.Children.Clear();
        children.OfType<UIElement>().ToList().ForEach(panel.Children.Add);
        return panel;
    }

    public static StackPanel Stack(double spacing, IEnumerable<UIElement?> children) =>
        Fill(new StackPanel { Spacing = spacing }, children);

    public static StackPanel Stack(double spacing, params UIElement?[] children) => Stack(spacing, children.AsEnumerable());

    public static StackPanel Line(double spacing, params UIElement?[] children) =>
        Fill(new StackPanel { Spacing = spacing, Orientation = Orientation.Horizontal }, children);

    public static Grid Lead(double spacing, FrameworkElement lead, FrameworkElement body)
    {
        lead.VerticalAlignment = VerticalAlignment.Center;
        Grid.SetColumn(body, 1);
        return new Grid
        {
            ColumnSpacing = spacing,
            ColumnDefinitions = { new ColumnDefinition { Width = GridLength.Auto }, new ColumnDefinition() },
            Children = { lead, body },
        };
    }

    public static Border Card(IEnumerable<UIElement> rows) => new()
    {
        Background = Brush("CardBrush"),
        BorderBrush = Brush("CardStrokeBrush"),
        BorderThickness = new Thickness(1),
        CornerRadius = Radius("RadiusControl"),
        Child = Stack(0, rows.Select((row, at) => (UIElement)new Border
        {
            BorderBrush = Brush("DividerBrush"),
            BorderThickness = new Thickness(0, at == 0 ? 0 : 1, 0, 0),
            Child = row,
        })),
    };

    public static Border Card(params UIElement[] rows) => Card(rows.AsEnumerable());

    public static Grid Row(FrameworkElement left, FrameworkElement? right, FrameworkElement? below = null)
    {
        var grid = new Grid
        {
            Padding = new Thickness(16, 12, 16, 12),
            MinHeight = RowMinHeight,
            ColumnSpacing = 16,
            RowSpacing = 8,
            ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto } },
            RowDefinitions = { new RowDefinition { Height = GridLength.Auto }, new RowDefinition { Height = GridLength.Auto } },
        };
        left.VerticalAlignment = VerticalAlignment.Center;
        grid.Children.Add(left);
        if (right is not null)
        {
            right.VerticalAlignment = VerticalAlignment.Center;
            right.HorizontalAlignment = HorizontalAlignment.Right;
            Grid.SetColumn(right, 1);
            grid.Children.Add(right);
        }
        if (below is not null)
        {
            Grid.SetRow(below, 1);
            Grid.SetColumnSpan(below, 2);
            grid.Children.Add(below);
        }
        return grid;
    }

    public static Grid Pair(string label, string value) =>
        Row(Text(label), Text(value, "FontSizeBody", "TextSecondaryBrush"));

    public static Border Pairs(IEnumerable<(string Label, string Value)> rows) =>
        Card(rows.Where(row => row.Value.Length > 0).Select(row => (UIElement)Pair(row.Label, row.Value)));

    public static Border Chip(string text) => new()
    {
        Background = Brush("ControlBrush"),
        CornerRadius = new CornerRadius(12),
        Height = 24,
        Padding = new Thickness(10, 0, 10, 0),
        Margin = new Thickness(0, 0, 6, 6),
        Child = Margined(Caption(text, "TextPrimaryBrush"), new Thickness(0, 3, 0, 0)),
    };

    public static Button Button(string text, Action onClick, ButtonLook look = ButtonLook.Standard, string? glyph = null)
    {
        var label = new TextBlock { Text = text, FontFamily = PreflightLook.Font, FontSize = Size("FontSizeBody") };
        var button = new Button
        {
            Content = glyph is null ? label : Line(8, new FontIcon { Glyph = glyph, FontFamily = Icons, FontSize = 14 }, label),
            MinHeight = Size("SizeControl"),
            Padding = new Thickness(12, 4, 12, 4),
            CornerRadius = Radius("RadiusControl"),
        };
        Paint(button, look);
        AutomationProperties.SetName(button, text);
        button.Click += (_, _) => onClick();
        return button;
    }

    public static Button IconButton(string glyph, string name, Action onClick)
    {
        var button = new Button
        {
            Content = new FontIcon { Glyph = glyph, FontFamily = Icons, FontSize = 14 },
            Width = Size("SizeControl"),
            Height = Size("SizeControl"),
            Padding = new Thickness(0),
            CornerRadius = Radius("RadiusControl"),
        };
        Paint(button, ButtonLook.Subtle);
        AutomationProperties.SetName(button, name);
        ToolTipService.SetToolTip(button, name);
        button.Click += (_, _) => onClick();
        return button;
    }

    public static Button Stretched(FrameworkElement content, string name, Action onClick)
    {
        var button = new Button
        {
            HorizontalAlignment = HorizontalAlignment.Stretch,
            HorizontalContentAlignment = HorizontalAlignment.Stretch,
            Padding = new Thickness(0),
            Content = content,
        };
        Paint(button, ButtonLook.Subtle);
        AutomationProperties.SetName(button, name);
        button.Click += (_, _) => onClick();
        return button;
    }

    public static void Paint(Button button, ButtonLook look)
    {
        switch (look)
        {
            case ButtonLook.Accent: PreflightLook.Accent(button); break;
            case ButtonLook.Subtle: PreflightLook.Subtle(button); break;
            default: PreflightLook.Standard(button); break;
        }
    }

    public static FontIcon Glyph(string glyph, double size = 16, string brush = "TextSecondaryBrush") => new()
    {
        Glyph = glyph,
        FontFamily = Icons,
        FontSize = size,
        Foreground = Brush(brush),
    };

    public static Border Dot(string brush, double size = 8) => new()
    {
        Width = size,
        Height = size,
        CornerRadius = new CornerRadius(size / 2),
        Background = Brush(brush),
        VerticalAlignment = VerticalAlignment.Center,
    };

    public static Border Badge(string glyph, bool good) => new()
    {
        Width = 40,
        Height = 40,
        CornerRadius = new CornerRadius(20),
        Background = Brush(good ? "SuccessBgBrush" : "ControlBrush"),
        Child = new FontIcon
        {
            Glyph = glyph,
            FontFamily = Icons,
            FontSize = 18,
            Foreground = Brush(good ? "SuccessBrush" : "TextSecondaryBrush"),
            HorizontalAlignment = HorizontalAlignment.Center,
            VerticalAlignment = VerticalAlignment.Center,
        },
    };

    public static ComboBox Combo(IEnumerable<string> items, int selected, double width = ChoiceWidth, string placeholder = "") => new()
    {
        ItemsSource = items.ToList(),
        SelectedIndex = selected,
        Width = width,
        PlaceholderText = placeholder,
        FontFamily = PreflightLook.Font,
    };

    public static TextBox Field(string text, string header = "", string placeholder = "") => new()
    {
        Text = text,
        Header = header.Length == 0 ? null : header,
        PlaceholderText = placeholder,
        FontFamily = PreflightLook.Font,
        HorizontalAlignment = HorizontalAlignment.Stretch,
    };

    public static CheckBox Check(string text, bool on, Action<bool> changed)
    {
        var box = new CheckBox { Content = text, IsChecked = on, FontFamily = PreflightLook.Font };
        Toggled(box, changed);
        return box;
    }

    public static void Toggled(Microsoft.UI.Xaml.Controls.Primitives.ToggleButton box, Action<bool> changed)
    {
        box.Checked += (_, _) => changed(true);
        box.Unchecked += (_, _) => changed(false);
    }

    public static UIElement Empty(string glyph, string title, string text) => Margined(Stack(12,
        new Border
        {
            Width = 72,
            Height = 72,
            CornerRadius = new CornerRadius(36),
            Background = Brush("ControlBrush"),
            HorizontalAlignment = HorizontalAlignment.Center,
            Child = new FontIcon
            {
                Glyph = glyph,
                FontFamily = Icons,
                FontSize = 32,
                Foreground = Brush("TextSecondaryBrush"),
                HorizontalAlignment = HorizontalAlignment.Center,
                VerticalAlignment = VerticalAlignment.Center,
            },
        },
        Centered(Text(MapUiJson.SentenceCase(title), "FontSizeBodyLarge", "TextPrimaryBrush", true)),
        Centered(Text(text, "FontSizeBody", "TextSecondaryBrush"))), new Thickness(32, 48, 32, 48));

    public static ContentDialog Dialog(XamlRoot root, string title, UIElement content, string? primary, string close = "Cancel") => new()
    {
        XamlRoot = root,
        Style = Application.Current.Resources["DefaultContentDialogStyle"] as Style,
        RequestedTheme = (root.Content as FrameworkElement)?.ActualTheme ?? ElementTheme.Default,
        Background = Brush("LayerBrush"),
        BorderBrush = Brush("CardStrokeBrush"),
        Foreground = Brush("TextPrimaryBrush"),
        FontFamily = PreflightLook.Font,
        Title = title,
        Content = content,
        PrimaryButtonText = primary ?? "",
        CloseButtonText = close,
        DefaultButton = primary is null ? ContentDialogButton.Close : ContentDialogButton.Primary,
    };

    public static async Task<ContentDialogResult> Show(ContentDialog dialog)
    {
        try { return await dialog.ShowAsync(); }
        catch (COMException) { return ContentDialogResult.None; }
    }

    public static async Task<bool> Confirm(XamlRoot? root, string title, string message, string primary, string close = "Cancel") =>
        root is not null
        && await Show(Dialog(root, title, Text(message, "FontSizeBody", "TextSecondaryBrush"), primary, close)) == ContentDialogResult.Primary;

    public static async Task Tell(XamlRoot? root, string title, string message)
    {
        if (root is null) return;
        await Show(Dialog(root, title, Text(message, "FontSizeBody", "TextSecondaryBrush"), null, "OK"));
    }

    private static TextBlock Centered(TextBlock element)
    {
        element.HorizontalAlignment = HorizontalAlignment.Center;
        element.TextAlignment = TextAlignment.Center;
        return element;
    }
}

internal static class Live
{
    private static int _next;

    public static string Client(string name) => $"settings.{name}.{Interlocked.Increment(ref _next)}";

    public static void Watch(FrameworkElement owner, string path, Action<JsonObject> apply)
    {
        var client = Client(path);
        owner.Loaded += (_, _) =>
        {
            BridgeWatch.Watch(client, new[] { path }, apply);
            apply(Bridge.Group(path));
        };
        owner.Unloaded += (_, _) => BridgeWatch.Stop(client);
    }

    public static Action<JsonObject> WhileShown(FrameworkElement owner, Action<JsonObject> apply) => new ShownGate(owner, apply).Take;

    public static bool Hidden(DependencyObject element) =>
        VisualTreeHelper.GetParent(element) is { } parent && (parent is UIElement { Visibility: Visibility.Collapsed } || Hidden(parent));

    public static Poller Poll<T>(FrameworkElement owner, TimeSpan every, Func<T> read, Action<T> apply) =>
        new(owner, every, async () => apply(await Task.Run(read)));

    public static Poller Changes(FrameworkElement owner, TimeSpan every, Func<JsonObject> read, Action<JsonObject> apply)
    {
        var last = "";
        return Poll(owner, every, read, view =>
        {
            var signature = view.ToJsonString();
            if (signature == last) return;
            last = signature;
            apply(view);
        });
    }
}

internal sealed class ShownGate(FrameworkElement owner, Action<JsonObject> apply)
{
    private JsonObject? _held;

    public void Take(JsonObject view)
    {
        var hidden = Live.Hidden(owner);
        if (hidden && _held is null) owner.LayoutUpdated += Replay;
        if (!hidden && _held is not null) owner.LayoutUpdated -= Replay;
        _held = hidden ? view : null;
        if (!hidden) apply(view);
    }

    private void Replay(object? sender, object e)
    {
        if (_held is not { } view || Live.Hidden(owner)) return;
        _held = null;
        owner.LayoutUpdated -= Replay;
        apply(view);
    }
}

internal sealed class Poller
{
    private readonly DispatcherTimer _timer;
    private readonly Func<Task> _tick;
    private bool _busy;

    public Poller(FrameworkElement owner, TimeSpan every, Func<Task> tick)
    {
        _tick = tick;
        _timer = new DispatcherTimer { Interval = every };
        _timer.Tick += (_, _) => Now();
        owner.Loaded += (_, _) =>
        {
            Now();
            _timer.Start();
        };
        owner.Unloaded += (_, _) => _timer.Stop();
    }

    public async void Now()
    {
        if (_busy) return;
        _busy = true;
        try { await _tick(); }
        finally { _busy = false; }
    }
}

internal static class FilePicks
{
    public const string Any = "*";

    public static async Task<string?> Open(XamlRoot? root, IReadOnlyList<string> types)
    {
        if (root?.ContentIslandEnvironment is not { } island) return null;
        var picker = new Microsoft.Windows.Storage.Pickers.FileOpenPicker(island.AppWindowId);
        types.ToList().ForEach(picker.FileTypeFilter.Add);
        var picked = await picker.PickSingleFileAsync();
        return picked?.Path;
    }

    public static async Task<string?> Save(XamlRoot? root, string suggested, string kind, string extension)
    {
        if (root?.ContentIslandEnvironment is not { } island) return null;
        var picker = new Microsoft.Windows.Storage.Pickers.FileSavePicker(island.AppWindowId) { SuggestedFileName = suggested };
        picker.FileTypeChoices.Add(kind, new List<string> { extension });
        var picked = await picker.PickSaveFileAsync();
        return picked?.Path;
    }
}
