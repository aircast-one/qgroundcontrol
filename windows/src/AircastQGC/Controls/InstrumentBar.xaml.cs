using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Core;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

public sealed partial class InstrumentBar : UserControl
{
    public const string InstrumentsView = "view.instruments";
    public const string GroupsView = "view.instrumentGroups";
    public const string FlyStateView = "view.flyState";
    public const int MaximumSlots = 10;

    private const string Client = "controls.instrumentBar";
    private const string GenericClass = "generic";
    private const string NoticeClient = "controls.instrumentBar.notice";
    private const double StaleOpacity = 0.55;
    private static readonly TimeSpan RereadInterval = TimeSpan.FromMilliseconds(500);

    private readonly DispatcherQueueTimer _reread;

    private IReadOnlyList<InstrumentSelection> _selections = InstrumentSelectionStore.Load();
    private IReadOnlyList<InstrumentValue> _values = Array.Empty<InstrumentValue>();
    private IReadOnlyList<Reading> _readings = Array.Empty<Reading>();
    private string _vehicleClass = GenericClass;
    private IReadOnlyDictionary<string, ValueDisplay> _displays = InstrumentDisplayStore.Load(GenericClass);

    public InstrumentBar()
    {
        InitializeComponent();
        _reread = DispatcherQueue.CreateTimer();
        _reread.Interval = RereadInterval;
        _reread.Tick += (_, _) => Refresh();
        Loaded += (_, _) => Start();
        Unloaded += (_, _) => Stop();
    }

    public event EventHandler? ValuesChanged;

    public event EventHandler? SelectionsChanged;

    public IReadOnlyList<InstrumentValue> Values => _values;

    public IReadOnlyList<InstrumentSelection> Selections => _selections;

    public string Notice => NoticeText.Text;

    public bool CanAdd => _selections.Count < MaximumSlots;

    public bool CanRemove => _selections.Count > 1;

    public string InstrumentsPath => $"{InstrumentsView}({string.Join(",", _selections.Select(s => s.Stored))})";

    public void Show() => Visibility = Visibility.Visible;

    public void Hide() => Visibility = Visibility.Collapsed;

    public void Refresh()
    {
        Apply(Read(InstrumentsPath));
        ApplyNotice(Read(FlyStateView));
    }

    public void AddSlot()
    {
        if (!CanAdd) return;
        Select(_selections.Append(InstrumentSelection.FirstUnused(_selections)).ToList());
    }

    public void RemoveSlot(int slot)
    {
        if (!CanRemove || slot < 0 || slot >= _selections.Count) return;
        Select(_selections.Where((_, i) => i != slot).ToList());
    }

    public void ResetSlots() => Select(InstrumentSelection.Defaults);

    public void Assign(int slot, InstrumentSelection selection)
    {
        if (slot < 0 || slot >= _selections.Count) return;
        if (_displays.TryGetValue(_selections[slot].Id, out var carried) && !_displays.ContainsKey(selection.Id)) Restyle(selection.Id, carried);
        Select(_selections.Select((s, i) => i == slot ? selection : s).ToList());
    }

    public void MoveSlot(int slot, int by)
    {
        var to = slot + by;
        if (slot < 0 || slot >= _selections.Count || to < 0 || to >= _selections.Count) return;
        Select(_selections.Select((s, i) => i == slot ? _selections[to] : i == to ? _selections[slot] : s).ToList());
    }

    public ValueDisplay DisplayOf(InstrumentValue value) =>
        _displays.TryGetValue(value.Id, out var display) ? display : new ValueDisplay { Icon = value.DefaultIcon };

    public void Restyle(string id, ValueDisplay display)
    {
        InstrumentDisplayStore.Save(_vehicleClass, id, display);
        _displays = InstrumentDisplayStore.Load(_vehicleClass);
        Render();
    }

    public async Task StyleSlotAsync(int slot)
    {
        if (slot < 0 || slot >= _values.Count || XamlRoot is null) return;
        var value = _values[slot];
        var at = _selections.ToList().FindIndex(s => s.Id == value.Id);
        var extra = new (bool Shown, string Label, Action Then)[]
        {
            (at >= 0, "Change reading", () => _ = EditSlotAsync(at)),
            (at > 0, "Move left", () => MoveSlot(at, -1)),
            (at >= 0 && at < _selections.Count - 1, "Move right", () => MoveSlot(at, 1)),
            (at >= 0 && CanRemove, "Remove", () => RemoveSlot(at)),
        }.Where(entry => entry.Shown).Select(entry => (entry.Label, entry.Then)).ToList();
        if (await DisplayEditor.Ask(XamlRoot, value.Label, DisplayOf(value), extra) is { } chosen) Restyle(value.Id, chosen);
    }

    public async Task EditSlotAsync(int slot)
    {
        if (slot < 0 || slot >= _selections.Count || XamlRoot is null) return;
        var groups = InstrumentGroup.Discovered(Read(GroupsView));
        var groupList = new StackPanel();
        var factList = new StackPanel();
        var dialog = new ContentDialog
        {
            XamlRoot = XamlRoot,
            Style = Styled("EditorDialog"),
            RequestedTheme = (XamlRoot.Content as FrameworkElement)?.ActualTheme ?? ElementTheme.Default,
            Title = "Change reading",
            CloseButtonText = "Cancel",
            Content = EditorFrame(EditingLabel(slot), groupList, factList),
        };

        void Choose(string chosen)
        {
            Fill(groupList, groups.Select(g => Row(g.Title, g.Facts.Count.ToString(CultureInfo.InvariantCulture), "",
                g.Group == chosen, () => Choose(g.Group))));
            var facts = groups.Where(g => g.Group == chosen).SelectMany(g => g.Facts)
                .Select(f => (UIElement)Row(f.Label.Length == 0 ? f.Name : f.Label, "", f.Name, false, () =>
                {
                    Assign(slot, new InstrumentSelection(chosen, f.Name));
                    dialog.Hide();
                }))
                .ToArray();
            var empty = Note(groups.Count == 0 ? "Connect a vehicle to choose a reading." : "Choose a group.");
            Fill(factList, facts.Length == 0 ? new[] { (UIElement)empty } : facts);
        }

        Choose(_selections[slot].Group);
        try
        {
            await dialog.ShowAsync();
        }
        catch (COMException)
        {
        }
    }

    private void Start()
    {
        Watch();
        Guard(() => BridgeWatch.Watch(NoticeClient, new[] { FlyStateView }, view => OnUi(() => ApplyNotice(view))));
        Refresh();
        _reread.Start();
    }

    private void Stop()
    {
        _reread.Stop();
        Guard(() => BridgeWatch.Stop(Client));
        Guard(() => BridgeWatch.Stop(NoticeClient));
    }

    private void Watch() =>
        Guard(() => BridgeWatch.Watch(Client, new[] { InstrumentsPath }, view => OnUi(() => Apply(view))));

    private void Select(IReadOnlyList<InstrumentSelection> chosen)
    {
        _selections = chosen;
        InstrumentSelectionStore.Save(chosen);
        Watch();
        Apply(Read(InstrumentsPath));
        SelectionsChanged?.Invoke(this, EventArgs.Empty);
    }

    private void Apply(JsonObject view)
    {
        var vehicleClass = InstrumentJson.Text(view, "vehicleClass") is { Length: > 0 } named ? named : _vehicleClass;
        if (vehicleClass != _vehicleClass)
        {
            _vehicleClass = vehicleClass;
            _displays = InstrumentDisplayStore.Load(vehicleClass);
            _values = Array.Empty<InstrumentValue>();
        }
        var read = InstrumentValue.List(view["items"]);
        if (read.SequenceEqual(_values)) return;
        _values = read;
        Render();
        ValuesChanged?.Invoke(this, EventArgs.Empty);
    }

    private void ApplyNotice(JsonObject view)
    {
        var notice = InstrumentJson.Text(view, "staleNotice");
        NoticeText.Text = notice;
        NoticeRow.Visibility = notice.Length == 0 ? Visibility.Collapsed : Visibility.Visible;
        Card.Opacity = notice.Length == 0 ? 1 : StaleOpacity;
    }

    private void Render()
    {
        _readings = _readings.Count == _values.Count ? _readings : Rebuild(_values.Count);
        _readings.Zip(_values).ToList().ForEach(pair => pair.First.Show(pair.Second, DisplayOf(pair.Second)));
        Card.Visibility = _values.Count == 0 ? Visibility.Collapsed : Visibility.Visible;
    }

    private IReadOnlyList<Reading> Rebuild(int count)
    {
        var built = Enumerable.Range(0, count).Select(slot => new Reading(Styled, Menu(slot), () => _ = StyleSlotAsync(slot))).ToList();
        Readings.Children.Clear();
        built.ForEach(r => Readings.Children.Add(r.Root));
        return built;
    }

    private string EditingLabel(int slot) =>
        slot < _values.Count ? $"Currently {_values[slot].Label}" : "";

    private MenuFlyout Menu(int slot)
    {
        var add = Item("Add reading", "", AddSlot);
        var remove = Item("Remove this reading", "", () => RemoveSlot(slot));
        var menu = new MenuFlyout();
        new MenuFlyoutItemBase[]
        {
            Item("Change reading…", "", async () => await EditSlotAsync(slot)),
            Item("Telemetry display…", "\uE790", async () => await StyleSlotAsync(slot)),
            add,
            remove,
            new MenuFlyoutSeparator(),
            Item("Reset to defaults", "", ResetSlots),
        }.ToList().ForEach(menu.Items.Add);
        menu.Opening += (_, _) =>
        {
            add.IsEnabled = CanAdd;
            remove.IsEnabled = CanRemove;
        };
        return menu;
    }

    private static MenuFlyoutItem Item(string text, string glyph, Action act)
    {
        var item = new MenuFlyoutItem { Text = text, Icon = new FontIcon { Glyph = glyph } };
        item.Click += (_, _) => act();
        return item;
    }

    private UIElement EditorFrame(string currently, UIElement groups, UIElement facts)
    {
        var frame = new Grid { Width = 480, RowSpacing = 12, ColumnSpacing = 12 };
        frame.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        frame.RowDefinitions.Add(new RowDefinition { Height = new GridLength(320) });
        frame.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(180) });
        frame.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        var caption = Note(currently);
        var groupCard = EditorCard(groups);
        var factCard = EditorCard(facts);
        Grid.SetColumnSpan(caption, 2);
        Grid.SetRow(groupCard, 1);
        Grid.SetRow(factCard, 1);
        Grid.SetColumn(factCard, 1);
        new UIElement[] { caption, groupCard, factCard }.ToList().ForEach(frame.Children.Add);
        return frame;
    }

    private Border EditorCard(UIElement list) =>
        new() { Style = Styled("EditorCard"), Child = new ScrollViewer { Content = list } };

    private TextBlock Note(string text) => new() { Style = Styled("EditorNote"), Text = text };

    private Button Row(string title, string value, string description, bool current, Action chosen)
    {
        var words = new StackPanel { Spacing = 2 };
        words.Children.Add(new TextBlock { Style = Styled("EditorTitle"), Text = title });
        words.Children.Add(new TextBlock
        {
            Style = Styled("EditorDetail"),
            Text = description,
            Visibility = description.Length == 0 ? Visibility.Collapsed : Visibility.Visible,
        });
        var count = new TextBlock
        {
            Style = Styled("EditorDetail"),
            Text = value,
            VerticalAlignment = VerticalAlignment.Center,
        };
        var layout = new Grid { ColumnSpacing = 8 };
        layout.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        layout.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        Grid.SetColumn(count, 1);
        layout.Children.Add(words);
        layout.Children.Add(count);
        var row = new Button { Style = Styled(current ? "EditorRowCurrent" : "EditorRow"), Content = layout };
        row.Click += (_, _) => chosen();
        return row;
    }

    private static void Fill(Panel panel, IEnumerable<UIElement> children)
    {
        panel.Children.Clear();
        children.ToList().ForEach(panel.Children.Add);
    }

    private Style? Styled(string key) => Resources.TryGetValue(key, out var found) ? found as Style : null;

    private void OnUi(Action act)
    {
        if (DispatcherQueue.HasThreadAccess) act();
        else DispatcherQueue.TryEnqueue(() => act());
    }

    private static JsonObject Read(string path)
    {
        try
        {
            return Bridge.Get(path);
        }
        catch (Exception)
        {
            return new JsonObject();
        }
    }

    private static void Guard(Action act)
    {
        try
        {
            act();
        }
        catch (Exception)
        {
        }
    }

    private sealed class Reading
    {
        private readonly TextBlock _label;
        private readonly Image _icon = new() { Width = 16, Height = 16, Stretch = Microsoft.UI.Xaml.Media.Stretch.Uniform, HorizontalAlignment = HorizontalAlignment.Left, Visibility = Visibility.Collapsed };
        private readonly TextBlock _value;
        private readonly TextBlock _units;
        private readonly TextBlock _absent;
        private readonly Style? _present;
        private readonly Style? _missing;

        private string _iconKey = "";

        public Reading(Func<string, Style?> styled, MenuFlyout menu, Action open)
        {
            _present = styled("ReadingValue");
            _missing = styled("ReadingMissing");
            _label = new TextBlock { Style = styled("ReadingLabel") };
            _value = new TextBlock { Style = _present };
            _units = new TextBlock { Style = styled("ReadingUnits") };
            _absent = new TextBlock
            {
                Style = styled("ReadingAbsent"),
                Text = InstrumentValue.AbsentText,
                Visibility = Visibility.Collapsed,
            };
            var figure = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 4 };
            figure.Children.Add(_value);
            figure.Children.Add(_units);
            var column = new StackPanel { Spacing = 2 };
            column.Children.Add(_label);
            column.Children.Add(_icon);
            column.Children.Add(figure);
            column.Children.Add(_absent);
            Root = new Button
            {
                Padding = new Thickness(0),
                HorizontalContentAlignment = HorizontalAlignment.Stretch,
                Content = new Border { Style = styled("ReadingCell"), Child = column },
                ContextFlyout = menu,
            };
            Forms.FormLook.Paint(Root, Forms.ButtonLook.Subtle);
            Root.Click += (_, _) => open();
        }

        public Button Root { get; }

        public void Show(InstrumentValue value, ValueDisplay display)
        {
            var colour = value.Missing ? null : InstrumentDisplays.Colour(display, value.Raw);
            var icon = InstrumentDisplays.IconOf(display, value.Raw);
            _label.Text = display.Text.Length > 0 ? display.Text : value.Label.ToUpperInvariant();
            _label.Visibility = icon is null && !InstrumentDisplays.IconShown(display) ? Visibility.Visible : Visibility.Collapsed;
            _label.Opacity = _icon.Opacity = InstrumentDisplays.Opacity(display, value.Raw);
            Paint(_label, colour);
            Paint(_value, colour);
            ShowIcon(icon, colour is { } tint ? InstrumentDisplays.Hex(tint) : Forms.AssetImages.Hex("TextSecondaryBrush"));
            _value.Text = value.Value;
            _value.Style = value.Missing ? _missing : _present;
            _units.Text = value.Units;
            _units.Visibility = value.Units.Length == 0 || !display.ShowUnits ? Visibility.Collapsed : Visibility.Visible;
            _absent.Visibility = value.AbsentHere ? Visibility.Visible : Visibility.Collapsed;
            Microsoft.UI.Xaml.Automation.AutomationProperties.SetName(Root, $"{value.Label} {value.Value} {value.Units}".Trim());
        }

        private static void Paint(TextBlock text, long? colour)
        {
            if (colour is { } argb) text.Foreground = new Microsoft.UI.Xaml.Media.SolidColorBrush(InstrumentDisplays.ColorOf(argb));
            else text.ClearValue(TextBlock.ForegroundProperty);
        }

        private void ShowIcon(string? icon, string tint)
        {
            _icon.Visibility = icon is null ? Visibility.Collapsed : Visibility.Visible;
            var key = icon is null ? "" : $"{icon}|{tint}";
            if (icon is null || key == _iconKey) return;
            _iconKey = key;
            InstrumentIcons.Load(_icon, icon, tint);
        }
    }
}
