using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Linq;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;

namespace AircastQGC.Pages.Settings;

internal sealed record SettingsNav(Action<string, UIElement> Push, Action Pop, Action<string> Open);

public sealed class SettingsPage : UserControl
{
    public const string ConnectionsPage = SettingsCatalog.Connections;

    private const double ListWidth = 300;
    private const double TwoPaneMinWidth = 840;
    private const string SearchTitle = "Search results";
    private static readonly TimeSpan SearchSettle = TimeSpan.FromMilliseconds(250);

    private readonly StackPanel _list = new() { Spacing = 2 };
    private readonly ContentControl _listHost = new() { HorizontalContentAlignment = HorizontalAlignment.Stretch, IsTabStop = false };
    private readonly TextBox _search = new() { PlaceholderText = "Search settings", Margin = new Thickness(8, 16, 8, 8) };
    private readonly ContentControl _detail = new()
    {
        HorizontalContentAlignment = HorizontalAlignment.Stretch,
        VerticalContentAlignment = VerticalAlignment.Stretch,
        IsTabStop = false,
    };
    private readonly TextBlock _title = FormLook.Text("Settings", "FontSizeTitle", "TextPrimaryBrush", true);
    private readonly Button _back;
    private readonly DispatcherTimer _settle = new() { Interval = SearchSettle };
    private readonly SettingsNav _nav;
    private readonly ColumnDefinition _listColumn = new() { Width = new GridLength(ListWidth) };
    private readonly ColumnDefinition _detailColumn = new();
    private bool _wide = true;
    private bool _listShown;
    private IReadOnlyList<SettingsEntry> _pages = [];
    private ImmutableDictionary<string, Button> _items = ImmutableDictionary<string, Button>.Empty;
    private string? _open;
    private ImmutableStack<(string Title, UIElement Body)> _stack = ImmutableStack<(string, UIElement)>.Empty;

    public SettingsPage()
    {
        _nav = new SettingsNav(Push, Pop, Open);
        _back = FormLook.IconButton("", "Back", Pop);
        _back.Visibility = Visibility.Collapsed;
        _search.FontFamily = PreflightLook.Font;
        AutomationProperties.SetName(_search, "Search settings");
        _search.TextChanged += (_, _) =>
        {
            _settle.Stop();
            _settle.Start();
        };
        _settle.Tick += (_, _) =>
        {
            _settle.Stop();
            ShowSearch();
        };
        _listHost.Content = _list;
        Content = Layout();
        Loaded += (_, _) => Load();
        SizeChanged += (_, size) =>
        {
            var wide = size.NewSize.Width >= TwoPaneMinWidth;
            if (wide == _wide) return;
            _wide = wide;
            _listShown = _listShown && !wide;
            ShowSearch();
            Arrange();
        };
    }

    public void Open(string title)
    {
        _open = title;
        _listShown = false;
        _search.Text = "";
        if (_pages.Count > 0) Show(title);
    }

    private void Arrange()
    {
        _listColumn.Width = _wide ? new GridLength(ListWidth) : _listShown ? new GridLength(1, GridUnitType.Star) : new GridLength(0);
        _detailColumn.Width = _wide || !_listShown ? new GridLength(1, GridUnitType.Star) : new GridLength(0);
        _back.Visibility = FormLook.Shown(!_wide || (_title.Text != SearchTitle && !_stack.IsEmpty && !_stack.Pop().IsEmpty));
    }

    private UIElement Layout()
    {
        var scroller = new ScrollViewer
        {
            Content = FormLook.Margined(_listHost, new Thickness(8, 0, 8, 16)),
            VerticalScrollBarVisibility = ScrollBarVisibility.Auto,
        };
        Grid.SetRow(scroller, 1);
        var pane = new Border
        {
            BorderBrush = FormLook.Brush("DividerBrush"),
            BorderThickness = new Thickness(0, 0, 1, 0),
            Child = new Grid
            {
                RowDefinitions = { new RowDefinition { Height = GridLength.Auto }, new RowDefinition() },
                Children = { _search, scroller },
            },
        };
        _title.VerticalAlignment = VerticalAlignment.Center;
        Grid.SetRow(_detail, 1);
        var detail = new Grid
        {
            RowDefinitions = { new RowDefinition { Height = GridLength.Auto }, new RowDefinition() },
            Children = { FormLook.Margined(FormLook.Line(8, _back, _title), new Thickness(24, 24, 24, 12)), _detail },
        };
        Grid.SetColumn(detail, 1);
        return new Grid
        {
            Background = FormLook.Brush("BgBaseBrush"),
            ColumnDefinitions = { _listColumn, _detailColumn },
            Children = { pane, detail },
        };
    }

    private async void Load()
    {
        var pages = SettingsCatalog.Pages(await FormWrites.Off(() => Bridge.Group(SettingsCatalog.View)));
        _pages = pages;
        _items = ImmutableDictionary<string, Button>.Empty;
        var wanted = _open is { } open && pages.Any(p => p.Title == open) ? open : pages.FirstOrDefault()?.Title;
        if (wanted is null)
        {
            RenderList();
            _detail.Content = FormLook.Margined(FormLook.Footnote("The settings tree is empty — the core is not answering."), new Thickness(24, 0, 24, 0));
            return;
        }
        if (wanted != _open || _stack.IsEmpty) Show(wanted);
        else RenderList();
    }

    private void Show(string title)
    {
        if (_pages.FirstOrDefault(p => p.Title == title) is not { } page) return;
        _open = title;
        _stack = ImmutableStack<(string, UIElement)>.Empty.Push((page.Shown, SettingsBodies.Body(page, _nav)));
        if (_items.Count == 0) RenderList();
        else MarkSelected();
        RenderDetail();
    }

    private void Push(string title, UIElement body)
    {
        _stack = _stack.Push((title, body));
        RenderDetail();
    }

    private void Pop()
    {
        if (_title.Text == SearchTitle || _stack.IsEmpty || _stack.Pop().IsEmpty)
        {
            _listShown = !_wide;
            Arrange();
            return;
        }
        _stack = _stack.Pop();
        RenderDetail();
    }

    private void RenderDetail()
    {
        if (_stack.IsEmpty) return;
        var (title, body) = _stack.Peek();
        _title.Text = title;
        _detail.Content = body;
        Arrange();
    }

    private void ShowSearch()
    {
        var query = _search.Text;
        _listHost.Content = query.Trim().Length > 0 && !_wide ? SettingsBodies.Search(query, _pages, Open) : _list;
        if (query.Trim().Length == 0 || !_wide)
        {
            RenderDetail();
            return;
        }
        _title.Text = SearchTitle;
        _detail.Content = SettingsBodies.Search(query, _pages, Open);
        Arrange();
    }

    private void RenderList()
    {
        _items = _pages.ToImmutableDictionary(page => page.Title, Item);
        FormLook.Fill(_list,
            SettingsCatalog.Grouped(_pages).SelectMany(group => new UIElement[]
            {
                FormLook.Margined(FormLook.Text(SettingsCatalog.GroupTitle(group.Key), "FontSizeCaption", "TextTertiaryBrush", true), new Thickness(12, 16, 12, 4)),
            }.Concat(group.Select(page => _items[page.Title]))));
        MarkSelected();
    }

    private void MarkSelected() => _items.ToList().ForEach(item => Mark(item.Value, item.Key == _open));

    private static void Mark(Button item, bool selected)
    {
        item.Background = selected ? FormLook.Brush("SubtleSelectedBrush") : new SolidColorBrush(Colors.Transparent);
        if (item.Content is not StackPanel { Children: [FontIcon glyph, TextBlock label] }) return;
        glyph.Foreground = FormLook.Brush(selected ? "AccentBrush" : "TextSecondaryBrush");
        label.Foreground = FormLook.Brush(selected ? "TextPrimaryBrush" : "TextSecondaryBrush");
    }

    private Button Item(SettingsEntry page)
    {
        var item = new Button
        {
            HorizontalAlignment = HorizontalAlignment.Stretch,
            HorizontalContentAlignment = HorizontalAlignment.Left,
            Height = FormLook.Size("SizeNavItem"),
            Padding = new Thickness(12, 0, 12, 0),
            CornerRadius = FormLook.Radius("RadiusControl"),
            Content = FormLook.Line(12, FormLook.Glyph(page.Look.Glyph, 16), FormLook.Text(page.Shown, "FontSizeBody", "TextSecondaryBrush")),
        };
        FormLook.Paint(item, ButtonLook.Subtle);
        AutomationProperties.SetName(item, page.Shown);
        item.Click += (_, _) => Open(page.Title);
        return item;
    }
}
