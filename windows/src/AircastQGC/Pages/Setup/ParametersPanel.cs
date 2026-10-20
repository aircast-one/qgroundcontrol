using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Markup;

namespace AircastQGC.Pages.Setup;

internal sealed class ParametersPanel : UserControl
{
    private const string XamlNs = "http://schemas.microsoft.com/winfx/2006/xaml/presentation";
    private static readonly TimeSpan Settle = TimeSpan.FromMilliseconds(200);

    private readonly TextBox _search;
    private readonly TextBlock _waiting = FormLook.Margined(FormLook.Text("Waiting for parameters from the vehicle.", "FontSizeBody", "TextSecondaryBrush"), new Thickness(0, 8, 0, 0));
    private readonly TextBlock _count = FormLook.Text("", "FontSizeBody", "TextSecondaryBrush");
    private readonly ToggleButton _all = new() { Content = "All", IsChecked = true };
    private readonly ToggleButton _changed = new() { Content = "Changed" };
    private readonly ComboBox _categories = FormLook.Combo([], -1, 200);
    private readonly ComboBox _groups = FormLook.Combo([], -1, 200);
    private readonly StackPanel _filters;
    private readonly StackPanel _toolbar;
    private readonly ListView _list;
    private readonly DispatcherTimer _settle = new() { Interval = Settle };
    private readonly DispatcherTimer _rewatch = new() { Interval = Settle };
    private readonly Dictionary<SelectorItem, string> _realized = [];
    private readonly string _client = Live.Client("parameterRows");
    private IReadOnlyDictionary<string, string> _watched = new Dictionary<string, string>();
    private readonly ParameterTools _tools;
    private IReadOnlyList<ParamEntry> _entries = [];
    private ObservableCollection<ParamEntry> _shown = [];
    private IReadOnlyList<ParamCategory> _tree = [];
    private string? _category;
    private string? _group;
    private bool _ready;
    private bool _px4;
    private bool _modifiedChosen;
    private bool _painting;
    private int _reads;

    public ParametersPanel(string initialSearch)
    {
        _search = FormLook.Field(initialSearch, "", "Search parameters");
        AutomationProperties.SetName(_search, "Search parameters");
        AutomationProperties.SetName(_categories, "Category");
        AutomationProperties.SetName(_groups, "Group");
        _tools = new ParameterTools(Reload);
        _list = new ListView
        {
            SelectionMode = ListViewSelectionMode.None,
            IsItemClickEnabled = true,
            ItemTemplate = (DataTemplate)XamlReader.Load(
                $"<DataTemplate xmlns='{XamlNs}'><Grid Padding='0,8,0,8' MinHeight='56' ColumnSpacing='12'><Grid.ColumnDefinitions><ColumnDefinition/><ColumnDefinition Width='Auto'/></Grid.ColumnDefinitions>" +
                "<StackPanel VerticalAlignment='Center' Spacing='2'><TextBlock TextTrimming='CharacterEllipsis' MaxLines='1'/><TextBlock TextTrimming='CharacterEllipsis' MaxLines='1'/></StackPanel>" +
                "<TextBlock Grid.Column='1' MaxWidth='200' TextAlignment='Right' TextWrapping='Wrap' MaxLines='2' TextTrimming='CharacterEllipsis' VerticalAlignment='Center'/></Grid></DataTemplate>"),
        };
        _list.ContainerContentChanging += (_, args) =>
        {
            if (args.InRecycleQueue) _realized.Remove(args.ItemContainer);
            else if (args.Item is ParamEntry realized) _realized[args.ItemContainer] = realized.Key;
            _rewatch.Stop();
            _rewatch.Start();
            if (args.InRecycleQueue || args.ItemContainer.ContentTemplateRoot is not Grid { Children: [StackPanel { Children: [TextBlock name, TextBlock description] }, TextBlock value] } || args.Item is not ParamEntry entry) return;
            Paint(name, entry.Name, "FontSizeBody", "TextPrimaryBrush");
            Paint(description, entry.Description, "FontSizeCaption", "TextSecondaryBrush");
            description.Visibility = FormLook.Shown(entry.Description.Length > 0);
            Paint(value, entry.Value, "FontSizeCaption", entry.Changed ? "AccentTextBrush" : "TextPrimaryBrush");
            args.Handled = true;
        };
        _list.ItemClick += (_, args) =>
        {
            if (args.ClickedItem is ParamEntry entry) Edit(entry);
        };
        _all.Click += (_, _) => ChooseModified(false);
        _changed.Click += (_, _) => ChooseModified(true);
        _categories.SelectionChanged += (_, _) =>
        {
            if (_painting || _categories.SelectedIndex < 0) return;
            _category = _tree[_categories.SelectedIndex].Name;
            _group = null;
            Refilter();
        };
        _groups.SelectionChanged += (_, _) =>
        {
            if (_painting || _groups.SelectedIndex < 0 || CurrentCategory() is not { } category) return;
            _group = category.Groups[_groups.SelectedIndex];
            Refilter();
        };
        _search.TextChanged += (_, _) =>
        {
            _settle.Stop();
            _settle.Start();
        };
        _settle.Tick += (_, _) =>
        {
            _settle.Stop();
            Refilter();
        };
        _rewatch.Tick += (_, _) =>
        {
            _rewatch.Stop();
            WatchRows(_realized.Values.ToHashSet());
        };
        Loaded += (_, _) => _rewatch.Start();
        Unloaded += (_, _) =>
        {
            _rewatch.Stop();
            WatchRows(new HashSet<string>());
        };
        _toolbar = FormLook.Line(8, _all, _changed, _tools.Button);
        _toolbar.VerticalAlignment = VerticalAlignment.Center;
        _filters = FormLook.Line(8, _categories, _groups);
        Content = Layout();
        Refilter();
        Live.Watch(this, SetupModel.SetupPath, view =>
        {
            var ready = SetupModel.ParametersReady(view);
            _px4 = SetupModel.Px4(view);
            if (ready == _ready) return;
            _ready = ready;
            Reload();
        });
    }

    private UIElement Layout()
    {
        var counts = new Grid
        {
            ColumnSpacing = 12,
            Margin = new Thickness(0, 8, 0, 8),
            ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto } },
        };
        _count.VerticalAlignment = VerticalAlignment.Center;
        counts.Children.Add(_count);
        Grid.SetColumn(_toolbar, 1);
        counts.Children.Add(_toolbar);
        _filters.Margin = new Thickness(0, 0, 0, 8);
        var grid = new Grid
        {
            Padding = new Thickness(SetupUi.PagePadding, 0, SetupUi.PagePadding, 0),
            RowDefinitions =
            {
                new RowDefinition { Height = GridLength.Auto },
                new RowDefinition { Height = GridLength.Auto },
                new RowDefinition { Height = GridLength.Auto },
                new RowDefinition { Height = GridLength.Auto },
                new RowDefinition(),
            },
        };
        new (FrameworkElement Element, int Row)[] { (_search, 0), (_waiting, 1), (counts, 2), (_filters, 3), (_list, 4) }
            .ToList()
            .ForEach(cell =>
            {
                Grid.SetRow(cell.Element, cell.Row);
                grid.Children.Add(cell.Element);
            });
        return grid;
    }

    private async void Reload()
    {
        var read = ++_reads;
        if (!_ready)
        {
            _entries = [];
            Refilter();
            return;
        }
        var entries = await Task.Run(() => ParameterModel.Keys().Select(ParameterModel.Read).OfType<ParamEntry>().ToList());
        if (read != _reads) return;
        _entries = entries;
        _tree = ParameterModel.Tree(entries);
        Refilter();
    }

    private void ChooseModified(bool chosen)
    {
        _modifiedChosen = chosen;
        Refilter();
    }

    private ParamCategory? CurrentCategory() =>
        _tree.FirstOrDefault(c => c.Name == _category) ?? _tree.FirstOrDefault();

    private void Refilter()
    {
        var modifiedOnly = _modifiedChosen && _px4;
        var search = _search.Text;
        var browsing = search.Trim().Length == 0 && !modifiedOnly;
        var category = CurrentCategory();
        var group = category?.Groups.FirstOrDefault(g => g == _group) ?? category?.Groups.FirstOrDefault();
        var matches = _entries
            .Where(entry => ParameterModel.Shown(entry, search, modifiedOnly))
            .Where(entry => !browsing || category is null || group is null || (entry.Category == category.Name && entry.Group == group))
            .ToList();
        _waiting.Visibility = FormLook.Shown(!_ready);
        _count.Text = ParameterModel.CountLine(matches.Count, _entries.Count(e => e.Changed));
        _count.Visibility = FormLook.Shown(_ready);
        _all.Visibility = _changed.Visibility = FormLook.Shown(_px4);
        _all.IsChecked = !modifiedOnly;
        _changed.IsChecked = modifiedOnly;
        _toolbar.Visibility = FormLook.Shown(_ready);
        _filters.Visibility = FormLook.Shown(_ready && browsing && category is not null);
        _painting = true;
        _categories.ItemsSource = _tree.Select(c => c.Name).ToList();
        _categories.SelectedIndex = category is null ? -1 : _tree.ToList().IndexOf(category);
        _groups.ItemsSource = category?.Groups.ToList() ?? [];
        _groups.SelectedIndex = category is null || group is null ? -1 : category.Groups.ToList().IndexOf(group);
        _groups.Visibility = FormLook.Shown(group is not null);
        _painting = false;
        Show(matches);
        _list.Visibility = FormLook.Shown(_ready);
    }

    private void Show(IReadOnlyList<ParamEntry> matches)
    {
        if (!_shown.Select(e => e.Key).SequenceEqual(matches.Select(e => e.Key)))
        {
            _shown = new ObservableCollection<ParamEntry>(matches);
            _list.ItemsSource = _shown;
            return;
        }
        matches.Select((entry, at) => (Entry: entry, At: at))
            .Where(pair => pair.Entry != _shown[pair.At])
            .ToList()
            .ForEach(pair => _shown[pair.At] = pair.Entry);
    }

    private void WatchRows(IReadOnlySet<string> keys)
    {
        _watched.Where(pair => !keys.Contains(pair.Key)).ToList().ForEach(pair => BridgeWatch.Stop(pair.Value));
        var added = keys.Where(key => !_watched.ContainsKey(key)).ToDictionary(key => key, key => $"{_client}.{key}");
        added.ToList().ForEach(pair => BridgeWatch.Watch(pair.Value, [ParameterModel.PathOf(pair.Key)], _ => Pushed(pair.Key)));
        _watched = _watched.Where(pair => keys.Contains(pair.Key)).Concat(added).ToDictionary(pair => pair.Key, pair => pair.Value);
    }

    private void Pushed(string key) => SetupUi.Run(() => ParameterModel.Read(key), updated =>
    {
        if (updated is null || _entries.FirstOrDefault(e => e.Key == key) is not { } held || held == updated) return;
        _entries = _entries.Select(e => e.Key == key ? updated : e).ToList();
        _shown.Select((entry, at) => (Entry: entry, At: at)).Where(pair => pair.Entry.Key == key).ToList().ForEach(pair => _shown[pair.At] = updated);
        _count.Text = ParameterModel.CountLine(_shown.Count, _entries.Count(e => e.Changed));
    });

    private void Edit(ParamEntry entry) =>
        ParameterEditor.Show(XamlRoot, entry.Key, _px4 && !entry.ReadOnly && !entry.Key.Contains(':'), () =>
            SetupUi.Run(() => ParameterModel.Read(entry.Key), updated =>
            {
                if (updated is null) return;
                _entries = _entries.Select(e => e.Key == updated.Key ? updated : e).ToList();
                Refilter();
            }));

    private static void Paint(TextBlock block, string text, string size, string brush)
    {
        block.Text = text;
        block.FontFamily = PreflightLook.Font;
        block.FontSize = FormLook.Size(size);
        block.Foreground = FormLook.Brush(brush);
    }
}
