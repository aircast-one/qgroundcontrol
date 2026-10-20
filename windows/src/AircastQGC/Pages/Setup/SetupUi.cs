using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Documents;
using Microsoft.UI.Xaml.Media;

namespace AircastQGC.Pages.Setup;

internal static class SetupUi
{
    public const double PagePadding = 24;
    public static readonly FontFamily Mono = new("Cascadia Mono, Consolas");

    private static readonly object LaneGate = new();
    private static Task _lane = Task.CompletedTask;

    public static void InOrder(Action work)
    {
        lock (LaneGate) _lane = _lane.ContinueWith(_ => work(), TaskScheduler.Default);
    }

    public static void Detached(Action work) => Task.Run(work);

    public static string StateBrush(SetupState state) => state switch
    {
        SetupState.NeedsAttention => "CautionBrush",
        SetupState.Done => "SuccessBrush",
        _ => "TextSecondaryBrush",
    };

    public static TextBlock Notice(string text)
    {
        var block = FormLook.Margined(FormLook.Text(text, "FontSizeBodyLarge", "TextPrimaryBrush"), new Thickness(24));
        block.TextAlignment = TextAlignment.Center;
        block.HorizontalAlignment = HorizontalAlignment.Stretch;
        return block;
    }

    public static TextBlock Heading(string text) => FormLook.Heading(text);

    public static TextBlock Error(string text = "") => FormLook.Caption(text, "CriticalBrush");

    public static TextBlock Monospace(string text)
    {
        var block = FormLook.Text(text, "FontSizeCaption", "TextPrimaryBrush");
        block.FontFamily = Mono;
        return block;
    }

    public static ScrollViewer Scrolled(UIElement content) => new()
    {
        Content = FormLook.Margined(new Border { Child = content }, new Thickness(PagePadding, 0, PagePadding, PagePadding)),
        VerticalScrollBarVisibility = ScrollBarVisibility.Auto,
        HorizontalScrollBarVisibility = ScrollBarVisibility.Disabled,
    };

    public static ScrollViewer Page(params UIElement?[] children) => Scrolled(FormLook.Stack(8, children));

    public static ProgressBar Bar(double fraction) => new()
    {
        Minimum = 0,
        Maximum = 1,
        Value = Math.Clamp(fraction, 0, 1),
        Height = 8,
        MinHeight = 8,
        VerticalAlignment = VerticalAlignment.Center,
        HorizontalAlignment = HorizontalAlignment.Stretch,
    };

    public static UIElement Row(
        string title,
        string status = "",
        SetupState state = SetupState.Neutral,
        Action? onClick = null,
        IReadOnlyList<SummaryLine>? summary = null,
        string? glyph = null,
        string subtitle = "",
        bool selected = false)
    {
        var parts = SetupModel.GlanceParts(summary ?? []);
        var titleBlock = FormLook.Text(title, "FontSizeBody", state == SetupState.Unavailable ? "TextSecondaryBrush" : "TextPrimaryBrush");
        var subtitleBlock = OneLine(FormLook.Caption(subtitle));
        var glance = OneLine(Glance(parts));
        var statusBlock = FormLook.Text(status, "FontSizeCaption", StateBrush(state), true);
        statusBlock.VerticalAlignment = VerticalAlignment.Center;
        var grid = new Grid
        {
            Padding = new Thickness(16, 10, 12, 10),
            MinHeight = 64,
            ColumnSpacing = 14,
            ColumnDefinitions =
            {
                new ColumnDefinition { Width = GridLength.Auto },
                new ColumnDefinition(),
                new ColumnDefinition { Width = GridLength.Auto },
                new ColumnDefinition { Width = GridLength.Auto },
            },
        };
        var icon = glyph is null ? null : Badge(glyph, state == SetupState.NeedsAttention);
        var text = FormLook.Stack(2, titleBlock, subtitleBlock, glance);
        text.VerticalAlignment = VerticalAlignment.Center;
        var chevron = onClick is null ? null : FormLook.Glyph("", 12, "TextTertiaryBrush");
        Place(grid, icon, 0);
        Place(grid, text, 1);
        Place(grid, statusBlock, 2);
        Place(grid, chevron, 3);
        if (onClick is null) return grid;
        var button = FormLook.Stretched(grid, title, onClick);
        button.CornerRadius = FormLook.Radius("RadiusControl");
        if (selected) button.Background = FormLook.Brush("SubtleSelectedBrush");
        AutomationProperties.SetName(button, title);
        return button;
    }

    public static Border Badge(string glyph, bool attention) => new()
    {
        Width = 36,
        Height = 36,
        CornerRadius = new CornerRadius(18),
        Background = FormLook.Brush(attention ? "CautionBgBrush" : "ControlBrush"),
        VerticalAlignment = VerticalAlignment.Center,
        Child = new FontIcon
        {
            Glyph = glyph,
            FontFamily = FormLook.Icons,
            FontSize = 16,
            Foreground = FormLook.Brush(attention ? "CautionBrush" : "TextSecondaryBrush"),
            HorizontalAlignment = HorizontalAlignment.Center,
            VerticalAlignment = VerticalAlignment.Center,
        },
    };

    public static TextBlock Glance(IReadOnlyList<SummaryLine> parts)
    {
        var block = FormLook.Caption("");
        parts.Select((part, at) => (Part: part, At: at)).ToList().ForEach(p =>
        {
            if (p.At > 0) block.Inlines.Add(new Run { Text = " · " });
            block.Inlines.Add(new Run { Text = p.Part.Value, Foreground = FormLook.Brush(p.Part.Warn ? "CriticalBrush" : "TextSecondaryBrush") });
        });
        block.Visibility = FormLook.Shown(parts.Count > 0);
        return block;
    }

    public static TextBlock OneLine(TextBlock block)
    {
        block.TextWrapping = TextWrapping.NoWrap;
        block.TextTrimming = TextTrimming.CharacterEllipsis;
        block.MaxLines = 1;
        return block;
    }

    public static FrameworkElement Choice(string label, IReadOnlyList<string> options, int index, Action<int> onPick, bool enabled = true, string detail = "")
    {
        var combo = FormLook.Combo(options, index < options.Count ? index : -1);
        combo.IsEnabled = enabled;
        AutomationProperties.SetName(combo, label);
        combo.SelectionChanged += (_, _) =>
        {
            if (combo.SelectedIndex >= 0 && combo.SelectedIndex != index) onPick(combo.SelectedIndex);
        };
        return FormLook.Row(FormLook.Stack(2, FormLook.Text(label), FormLook.Caption(detail)), combo);
    }

    public static FrameworkElement Entry(string label, string value, Action<string> onDone, bool enabled = true, string hint = "", string units = "", Func<string, bool>? accept = null)
    {
        var box = FormLook.Field(value);
        box.Width = 180;
        box.IsEnabled = enabled;
        AutomationProperties.SetName(box, label);
        var held = value;
        box.BeforeTextChanging += (_, args) => args.Cancel = accept is not null && !accept(args.NewText);
        void Commit()
        {
            if (box.Text == held) return;
            held = box.Text;
            onDone(box.Text);
        }
        box.LostFocus += (_, _) => Commit();
        box.KeyDown += (_, e) =>
        {
            if (e.Key == Windows.System.VirtualKey.Enter) Commit();
        };
        return FormLook.Row(FormLook.Stack(2, FormLook.Text(label), FormLook.Caption(hint)), FormLook.Line(8, box, FormLook.Text(units, "FontSizeBody", "TextSecondaryBrush")));
    }

    public static FrameworkElement Toggle(string label, bool on, Action<bool> onChange, bool enabled = true, string detail = "")
    {
        var toggle = new ToggleSwitch { IsOn = on, OnContent = "", OffContent = "", MinWidth = 0, IsEnabled = enabled };
        AutomationProperties.SetName(toggle, label);
        toggle.Toggled += (_, _) =>
        {
            if (toggle.IsOn != on) onChange(toggle.IsOn);
        };
        return FormLook.Row(FormLook.Stack(2, FormLook.Text(label), FormLook.Caption(detail)), toggle);
    }

    public static FrameworkElement Pair(string label, string value) => FormLook.Pair(label, value);

    public static string Fixed(double value, int decimals) => value.ToString($"F{decimals}", CultureInfo.InvariantCulture);

    public static double? Typed(string text) => FormText.Parse(text);

    public static JsonObject Read(string path) => Bridge.Get(path);

    public static string? Refusal(string path, params object?[] args) => FormWrites.Call(path, FormWrites.VehicleSubject, args);

    public static string? Write(string path, object? value) => FormWrites.Set(path, value);

    public static string? ForcedWrite(string path, object? value) =>
        FormWrites.Refusal(Decode(NativeMethods.Take(NativeMethods.qgc_bridge_set(path,
            new JsonObject { ["value"] = FormJson.Node(value)?.DeepClone(), ["force"] = true }.ToJsonString()))));

    public static async void Run<T>(Func<T> work, Action<T> then) => then(await Task.Run(work));

    public static void Say(TextBlock block, string? text) => FormLook.Say(block, text);

    public static FormControl? Control(JsonNode? node) => node is JsonObject json ? FormControl.Parse(json) : null;

    public static FormControl Titled(FormControl control, string title) => control with { Label = title, ShortLabel = title };

    public static FormSection Section(string title, string note, params FormBlock[] blocks) => new(title, "", note, "", [], blocks);

    public static FormBlock Block(string title, params FormControl?[] controls) => new(title, controls.OfType<FormControl>().ToList());

    public static FormView Form(Func<JsonObject> read, Func<JsonObject, IReadOnlyList<FormSection>> parse, FormHooks? hooks = null, string loading = "") => new(read, parse)
    {
        Hooks = hooks ?? new FormHooks(),
        LoadingText = loading,
        EmptyText = "",
        Empty = () => new Border(),
        NotesFirst = true,
    };

    public static FormView Embedded(IReadOnlyList<FormControl> controls, Action written, string title = "")
    {
        var dispatcher = Microsoft.UI.Dispatching.DispatcherQueue.GetForCurrentThread();
        var reads = new Counter();
        return Form(() =>
        {
            if (reads.Next() > 0) dispatcher?.TryEnqueue(() => written());
            return new JsonObject();
        }, _ => controls.Count == 0 ? [] : [Section("", "", new FormBlock(title, controls))]);
    }

    private sealed class Counter
    {
        private int _count;

        public int Next() => System.Threading.Interlocked.Increment(ref _count) - 1;
    }

    private static JsonObject Decode(string raw)
    {
        try { return JsonNode.Parse(raw) as JsonObject ?? new JsonObject(); }
        catch (System.Text.Json.JsonException) { return new JsonObject(); }
    }

    private static void Place(Grid grid, FrameworkElement? element, int column)
    {
        if (element is null) return;
        Grid.SetColumn(element, column);
        grid.Children.Add(element);
    }
}
