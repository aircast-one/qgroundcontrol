using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Collections.ObjectModel;
using System.IO;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Markup;
using Microsoft.UI.Xaml.Media;

namespace AircastQGC.Pages.Settings;

internal sealed record AppLogFilter(int LevelIndex = 0, string Category = "", string Text = "", bool Regex = false)
{
    public string Path(long? after) =>
        $"{AppLogPanel.View}({Math.Max(LevelIndex - 1, 0)},{Uri.EscapeDataString(Category)},{Uri.EscapeDataString(Text)},{(Regex ? 1 : 0)},{after?.ToString() ?? ""})";
}

internal sealed record AppLogEntry(long Sequence, int Level, string Message, string Detail);

internal sealed record AppLogRead(IReadOnlyList<string> Levels, IReadOnlyList<string> Categories, bool RegexValid, long? First, IReadOnlyList<AppLogEntry> Entries)
{
    public static AppLogRead? Parse(JsonObject view) => MapUiJson.Text(view["class"]) != "AppLog"
        ? null
        : new AppLogRead(
            MapUiJson.Texts(view["levels"]),
            MapUiJson.Texts(view["categories"]),
            MapUiJson.Flag(view["regexValid"], true),
            MapUiJson.Number(view["first"]) is { } first ? (long)first : null,
            MapUiJson.Objects(view["entries"]).Select(entry => new AppLogEntry(
                Panels.Whole(entry["sequence"]),
                (int)Panels.Whole(entry["level"]),
                MapUiJson.Text(entry["message"]),
                string.Join("  ", new[] { MapUiJson.Text(entry["timestamp"]), MapUiJson.Text(entry["category"]), MapUiJson.Text(entry["source"]) }
                    .Where(part => part.Length > 0)))).ToList());
}

internal sealed class AppLogPanel : UserControl
{
    public const string View = "view.appLog";
    private const string Clear = "appLog.clear";
    private const string Save = "appLog.save";
    private const string SaveFormat = "settings.logManagerSettings.saveFormat.rawValue";
    private const int SaveFormatCsv = 1;
    private const int LevelWarning = 2;
    private const int LevelCritical = 3;
    private const string XamlNs = "http://schemas.microsoft.com/winfx/2006/xaml/presentation";
    private static readonly TimeSpan Every = TimeSpan.FromMilliseconds(500);
    private static readonly TimeSpan Settle = TimeSpan.FromMilliseconds(200);
    private static readonly FontFamily Mono = new("Cascadia Mono, Consolas");

    private readonly ObservableCollection<AppLogEntry> _entries = [];
    private readonly ListView _list;
    private readonly TextBlock _empty = FormLook.Text("No log entries", "FontSizeBody", "TextSecondaryBrush");
    private readonly Button _latest;
    private readonly ComboBox _levels = FormLook.Combo([], -1, 160);
    private readonly ComboBox _categories = FormLook.Combo([], -1, 200);
    private readonly TextBox _search = FormLook.Field("", "", "Search…");
    private readonly ToggleButton _regex = new() { Content = ".*", FontFamily = Mono, MinWidth = 44 };
    private readonly TextBlock _notice = FormLook.Caption("", "CriticalBrush");
    private readonly TextBlock _regexError = FormLook.Caption("", "CriticalBrush");
    private readonly DispatcherTimer _settle = new() { Interval = Settle };
    private AppLogFilter _filter = new();
    private AppLogRead? _read;
    private int _generation;
    private ScrollViewer? _scroller;
    private bool _following = true;
    private bool _painting;

    public AppLogPanel()
    {
        _list = new ListView
        {
            ItemsSource = _entries,
            SelectionMode = ListViewSelectionMode.None,
            IsItemClickEnabled = false,
            ItemTemplate = (DataTemplate)XamlReader.Load(
                $"<DataTemplate xmlns='{XamlNs}'><Grid Padding='0,4,8,4' ColumnSpacing='8'><Grid.ColumnDefinitions><ColumnDefinition Width='3'/><ColumnDefinition/></Grid.ColumnDefinitions>" +
                "<Border/><StackPanel Grid.Column='1'><TextBlock TextWrapping='Wrap' MaxLines='2' TextTrimming='CharacterEllipsis'/><TextBlock TextTrimming='CharacterEllipsis'/></StackPanel></Grid></DataTemplate>"),
        };
        _list.ContainerContentChanging += (_, args) =>
        {
            if (args.ItemContainer.ContentTemplateRoot is not Grid { Children: [Border bar, StackPanel { Children: [TextBlock message, TextBlock detail] }] } || args.Item is not AppLogEntry entry) return;
            var colour = LevelBrush(entry.Level);
            bar.Background = entry.Level >= LevelWarning ? colour : null;
            message.Text = entry.Message;
            message.Foreground = colour;
            message.FontFamily = Mono;
            message.FontSize = FormLook.Size("FontSizeCaption");
            detail.Text = entry.Detail;
            detail.Foreground = FormLook.Brush("TextTertiaryBrush");
            detail.FontFamily = Mono;
            detail.FontSize = FormLook.Size("FontSizeCaption");
            args.Handled = true;
        };
        _latest = FormLook.Button("Show latest", () =>
        {
            _following = true;
            ScrollToEnd();
        }, ButtonLook.Accent);
        _list.Loaded += (_, _) =>
        {
            if (_scroller is not null || Descendants(_list).OfType<ScrollViewer>().FirstOrDefault() is not { } scroller) return;
            _scroller = scroller;
            scroller.ViewChanged += (_, args) =>
            {
                if (args.IsIntermediate) return;
                _following = scroller.VerticalOffset >= scroller.ScrollableHeight - 4;
                _latest.Visibility = FormLook.Shown(!_following && _entries.Count > 0);
            };
        };
        _latest.Visibility = Visibility.Collapsed;
        _latest.HorizontalAlignment = HorizontalAlignment.Center;
        _latest.VerticalAlignment = VerticalAlignment.Top;
        _latest.Margin = new Thickness(0, 8, 0, 0);
        _empty.HorizontalAlignment = HorizontalAlignment.Center;
        _empty.VerticalAlignment = VerticalAlignment.Center;
        AutomationProperties.SetName(_levels, "Level");
        AutomationProperties.SetName(_categories, "Category");
        AutomationProperties.SetName(_search, "Search the log");
        AutomationProperties.SetName(_regex, "Regular expression");
        _levels.SelectionChanged += (_, _) =>
        {
            if (!_painting && _levels.SelectedIndex >= 0) Refilter(_filter with { LevelIndex = _levels.SelectedIndex });
        };
        _categories.SelectionChanged += (_, _) =>
        {
            if (!_painting && _categories.SelectedIndex >= 0 && _read is { } read)
                Refilter(_filter with { Category = _categories.SelectedIndex == 0 ? "" : read.Categories[_categories.SelectedIndex] });
        };
        _search.TextChanged += (_, _) =>
        {
            _settle.Stop();
            _settle.Start();
        };
        _settle.Tick += (_, _) =>
        {
            _settle.Stop();
            Refilter(_filter with { Text = _search.Text });
        };
        FormLook.Toggled(_regex, on => Refilter(_filter with { Regex = on }));
        Content = Layout();
        _ = new Poller(this, Every, async () =>
        {
            var (generation, path) = (_generation, _filter.Path(_entries.LastOrDefault()?.Sequence));
            Apply(generation, await FormWrites.Off(() => AppLogRead.Parse(Bridge.Get(path))));
        });
    }

    private UIElement Layout()
    {
        var log = new Grid { Children = { _list, _empty, _latest } };
        var bar = new Border
        {
            Background = FormLook.Brush("LayerBrush"),
            BorderBrush = FormLook.Brush("DividerBrush"),
            BorderThickness = new Thickness(0, 1, 0, 0),
            Padding = new Thickness(16, 12, 16, 12),
            Child = FormLook.Stack(8,
                FormLook.Line(8,
                    _levels,
                    _categories,
                    FormLook.Button("Categories…", () => new LoggingCategoriesDialog(XamlRoot).Open()),
                    FormLook.Button("Save…", SaveLog),
                    FormLook.Button("Clear", ClearLog)),
                FormLook.Line(8, _search, _regex),
                _regexError,
                _notice),
        };
        _search.Width = 420;
        Grid.SetRow(bar, 1);
        return new Grid
        {
            Padding = new Thickness(24, 0, 24, 0),
            RowDefinitions = { new RowDefinition(), new RowDefinition { Height = GridLength.Auto } },
            Children = { log, bar },
        };
    }

    private void Refilter(AppLogFilter filter)
    {
        if (filter == _filter) return;
        _filter = filter;
        _generation++;
        _entries.Clear();
        _following = true;
        Paint();
    }

    private void Apply(int generation, AppLogRead? polled)
    {
        if (generation != _generation || polled is not { } read) return;
        _read = read;
        var stale = read.First is { } first ? _entries.TakeWhile(entry => entry.Sequence < first).Count() : _entries.Count;
        Enumerable.Range(0, stale).ToList().ForEach(_ => _entries.RemoveAt(0));
        read.Entries.ToList().ForEach(_entries.Add);
        Paint();
        if (_following && read.Entries.Count > 0) ScrollToEnd();
    }

    private void Paint()
    {
        _painting = true;
        var levels = _read?.Levels.Select(MapUiJson.SentenceCase).ToList() ?? [];
        var categories = _read?.Categories.Select(MapUiJson.SentenceCase).ToList() ?? [];
        if (!levels.SequenceEqual(_levels.ItemsSource as IEnumerable<string> ?? [])) _levels.ItemsSource = levels;
        if (!categories.SequenceEqual(_categories.ItemsSource as IEnumerable<string> ?? [])) _categories.ItemsSource = categories;
        _levels.SelectedIndex = levels.Count > 0 ? Math.Min(_filter.LevelIndex, levels.Count - 1) : -1;
        _categories.SelectedIndex = categories.Count == 0 ? -1 : _filter.Category.Length == 0 ? 0 : _read!.Categories.ToList().IndexOf(_filter.Category);
        _search.BorderBrush = _read?.RegexValid == false ? FormLook.Brush("CriticalBrush") : FormLook.Brush("ControlStrokeBrush");
        FormLook.Say(_regexError, _read?.RegexValid == false ? "Invalid regular expression" : null);
        _empty.Visibility = FormLook.Shown(_entries.Count == 0);
        _painting = false;
    }

    private void ScrollToEnd()
    {
        if (_entries.Count > 0) _list.ScrollIntoView(_entries[^1]);
        _list.UpdateLayout();
        _scroller?.ChangeView(null, _scroller.ScrollableHeight, null, true);
        _latest.Visibility = Visibility.Collapsed;
    }

    private async void ClearLog()
    {
        await FormWrites.Off(() => FormWrites.Ok(Clear));
        _generation++;
        _entries.Clear();
        Paint();
    }

    private async void SaveLog()
    {
        var csv = await FormWrites.Off(() => MapUiJson.Number(Bridge.Get(SaveFormat)["value"]) == SaveFormatCsv);
        var (kind, extension) = csv ? ("CSV", ".csv") : ("Text", ".txt");
        if (await FilePicks.Save(XamlRoot, "QGCConsole", kind, extension) is not { } path) return;
        var notice = await FormWrites.Off(() =>
        {
            if (FormWrites.Result(Save, Path.GetFileName(path)) is not JsonValue saved || !saved.TryGetValue<string>(out var text)) return "The log could not be read.";
            try
            {
                File.WriteAllText(path, text);
                return null;
            }
            catch (Exception error) when (error is IOException or UnauthorizedAccessException)
            {
                return "The file could not be written.";
            }
        });
        FormLook.Say(_notice, notice);
    }

    private static Brush LevelBrush(int level) => FormLook.Brush(level switch
    {
        0 => "TextSecondaryBrush",
        LevelWarning => "CautionBrush",
        >= LevelCritical => "CriticalBrush",
        _ => "TextPrimaryBrush",
    });

    private static IEnumerable<DependencyObject> Descendants(DependencyObject root) =>
        Enumerable.Range(0, VisualTreeHelper.GetChildrenCount(root))
            .Select(at => VisualTreeHelper.GetChild(root, at))
            .SelectMany(child => Descendants(child).Prepend(child));
}

internal sealed record LogCategory(string Name, string ShortName, int Depth, bool Enabled);

internal static class LogCategories
{
    public static LogCategory? ParentOf(IReadOnlyList<LogCategory> all, LogCategory category) =>
        all.FirstOrDefault(parent =>
            parent.Depth == category.Depth - 1
            && category.Name.StartsWith(parent.Name, StringComparison.Ordinal)
            && category.Name.EndsWith(category.ShortName, StringComparison.Ordinal)
            && category.Name.Length > parent.Name.Length + category.ShortName.Length
            && !category.Name[parent.Name.Length..(category.Name.Length - category.ShortName.Length)].Any(c => char.IsLetterOrDigit(c) || c == '_'));

    public static IReadOnlySet<string> Parents(IReadOnlyList<LogCategory> all) =>
        all.Select(category => ParentOf(all, category)?.Name).OfType<string>().ToHashSet();

    public static IReadOnlyList<LogCategory> Tree(IReadOnlyList<LogCategory> all, IReadOnlySet<string> expanded)
    {
        IEnumerable<LogCategory> ChildrenOf(LogCategory? parent) => all.Where(category => ParentOf(all, category) == parent);
        IEnumerable<LogCategory> Walk(LogCategory category) =>
            ChildrenOf(category).Where(_ => expanded.Contains(category.Name)).SelectMany(Walk).Prepend(category);
        return ChildrenOf(null).SelectMany(Walk).ToList();
    }
}

internal sealed class LoggingCategoriesDialog(XamlRoot? root)
{
    private const string View = "view.logCategories";
    private const string SetCategory = "appLog.setCategory";
    private const string ResetCategories = "appLog.resetCategories";
    private const double Indent = 12;

    private readonly StackPanel _body = new() { Spacing = 8 };
    private readonly TextBox _filter = FormLook.Field("", "", "Filter categories…");
    private ImmutableHashSet<string> _expanded = ImmutableHashSet<string>.Empty;
    private IReadOnlyList<string> _active = [];
    private IReadOnlyList<LogCategory> _categories = [];

    public async void Open()
    {
        if (root is null) return;
        _filter.TextChanged += (_, _) => Render();
        AutomationProperties.SetName(_filter, "Filter categories");
        await Reload();
        await FormLook.Show(FormLook.Dialog(root, "Logging categories", FormLook.Stack(8,
            FormLook.Line(8, _filter, FormLook.Button("Clear", () => _filter.Text = "", ButtonLook.Subtle)),
            new ScrollViewer { MaxHeight = 520, Content = _body }), null, "Close"));
    }

    private async System.Threading.Tasks.Task Reload()
    {
        var view = await FormWrites.Off(() => Bridge.Get(View));
        _active = MapUiJson.Texts(view["active"]);
        _categories = MapUiJson.Objects(view["categories"])
            .Select(c => new LogCategory(MapUiJson.Text(c["name"]), MapUiJson.Text(c["shortName"]), (int)Panels.Whole(c["depth"]), MapUiJson.Flag(c["enabled"])))
            .ToList();
        Render();
    }

    private async void Act(string path, params object?[] args)
    {
        await FormWrites.Off(() => FormWrites.Ok(path, args));
        await Reload();
    }

    private void Render()
    {
        var search = _filter.Text;
        var parents = LogCategories.Parents(_categories);
        var found = _categories.Where(c => c.Name.Contains(search, StringComparison.OrdinalIgnoreCase)).ToList();
        FormLook.Fill(_body, new UIElement?[]
            {
                FormLook.Text("Active categories", "FontSizeBody", "TextPrimaryBrush", true),
            }
            .Concat(_active.Select(name => Switch(name, true, 0, null, on => Act(SetCategory, name, on))))
            .Append(FormLook.Button("Reset all", () => Act(ResetCategories)))
            .Append(FormLook.Text(search.Length == 0 ? "Categories" : "Search results", "FontSizeBody", "TextPrimaryBrush", true))
            .Concat(search.Length == 0
                ? LogCategories.Tree(_categories, _expanded).Select(c => Switch(c.ShortName, c.Enabled, c.Depth,
                    parents.Contains(c.Name) ? c.Name : null, on => Act(SetCategory, c.Name, on)))
                : found.Select(c => Switch(c.Name, c.Enabled, 0, null, on => Act(SetCategory, c.Name, on))))
            .Append(search.Length > 0 && found.Count == 0 ? FormLook.Caption("No matching categories") : null));
    }

    private UIElement Switch(string label, bool on, int depth, string? expandable, Action<bool> changed)
    {
        var toggle = new ToggleSwitch { IsOn = on, OnContent = "", OffContent = "", MinWidth = 0 };
        AutomationProperties.SetName(toggle, label);
        toggle.Toggled += (_, _) =>
        {
            if (toggle.IsOn != on) changed(toggle.IsOn);
        };
        var opened = expandable is not null && _expanded.Contains(expandable);
        UIElement expander = expandable is null ? new Border { Width = FormLook.Size("SizeControl") } : FormLook.IconButton(opened ? "\uE70D" : "\uE76C", opened ? "Collapse" : "Expand", () =>
        {
            _expanded = opened ? _expanded.Remove(expandable) : _expanded.Add(expandable);
            Render();
        });
        return FormLook.Row(FormLook.Margined(FormLook.Line(4, expander, FormLook.Margined(FormLook.Text(label), new Thickness(0, 6, 0, 0))),
            new Thickness(depth * Indent, 0, 0, 0)), toggle);
    }
}
