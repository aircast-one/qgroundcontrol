using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Core;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Documents;
using Microsoft.UI.Xaml.Media;

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
    private const string ReadingTip = "Telemetry display. Right-click for more readings";
    private static readonly TimeSpan RereadInterval = TimeSpan.FromMilliseconds(500);

    private readonly DispatcherQueueTimer _reread;

    private IReadOnlyList<InstrumentSelection>? _chosen = InstrumentSelectionStore.Load();
    private IReadOnlyList<InstrumentValue> _values = Array.Empty<InstrumentValue>();
    private IReadOnlyList<Reading> _readings = Array.Empty<Reading>();
    private string _vehicleClass = GenericClass;
    private IReadOnlyDictionary<string, ValueDisplay> _displays = InstrumentDisplayStore.Load(GenericClass);
    private string _rows = "";

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

    public event EventHandler? AttitudeChanged;

    public bool AttitudeShown { get; private set; } = AttitudeCardStore.Load();

    public IReadOnlyList<InstrumentValue> Values => _values;

    public IReadOnlyList<InstrumentSelection> Selections => _chosen ?? InstrumentSelection.DefaultsFor(_vehicleClass);

    public string Notice => NoticeText.Text;

    public bool CanAdd => Selections.Count < MaximumSlots;

    public bool CanRemove => Selections.Count > 1;

    public string InstrumentsPath => $"{InstrumentsView}({string.Join(",", Selections.Select(s => s.Stored))})";

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
        Select(Selections.Append(InstrumentSelection.FirstUnused(Selections)).ToList());
    }

    public void RemoveSlot(int slot)
    {
        if (!CanRemove || slot < 0 || slot >= Selections.Count) return;
        Select(Selections.Where((_, i) => i != slot).ToList());
    }

    public void ResetSlots() => Select(null);

    public void ShowAttitude(bool shown)
    {
        AttitudeShown = shown;
        AttitudeCardStore.Save(shown);
        AttitudeChanged?.Invoke(this, EventArgs.Empty);
    }

    public void Assign(int slot, InstrumentSelection selection)
    {
        if (slot < 0 || slot >= Selections.Count) return;
        if (_displays.TryGetValue(Selections[slot].Id, out var carried) && !_displays.ContainsKey(selection.Id)) Restyle(selection.Id, carried);
        Select(Selections.Select((s, i) => i == slot ? selection : s).ToList());
    }

    public void MoveSlot(int slot, int by)
    {
        var to = slot + by;
        if (slot < 0 || slot >= Selections.Count || to < 0 || to >= Selections.Count) return;
        Select(Selections.Select((s, i) => i == slot ? Selections[to] : i == to ? Selections[slot] : s).ToList());
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
        var at = Selections.ToList().FindIndex(s => s.Id == value.Id);
        var extra = new (bool Shown, string Label, Action Then)[]
        {
            (at >= 0, "Change reading", () => _ = EditSlotAsync(at)),
            (at > 0, "Move left", () => MoveSlot(at, -1)),
            (at >= 0 && at < Selections.Count - 1, "Move right", () => MoveSlot(at, 1)),
            (at >= 0 && CanRemove, "Remove", () => RemoveSlot(at)),
            (true, AttitudeShown ? "Hide compass" : "Show compass", () => ShowAttitude(!AttitudeShown)),
        }.Where(entry => entry.Shown).Select(entry => (entry.Label, entry.Then)).ToList();
        if (await DisplayEditor.Ask(XamlRoot, value.Label, DisplayOf(value), extra) is { } chosen) Restyle(value.Id, chosen);
    }

    public async Task EditSlotAsync(int slot)
    {
        if (slot < 0 || slot >= Selections.Count || XamlRoot is null) return;
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

        Choose(Selections[slot].Group);
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

    private void Select(IReadOnlyList<InstrumentSelection>? chosen)
    {
        _chosen = chosen;
        InstrumentSelectionStore.Save(chosen ?? []);
        Watch();
        Apply(Read(InstrumentsPath));
        SelectionsChanged?.Invoke(this, EventArgs.Empty);
    }

    private void Apply(JsonObject view)
    {
        var vehicleClass = InstrumentJson.Text(view, "vehicleClass") is { Length: > 0 } named ? named : _vehicleClass;
        if (vehicleClass != _vehicleClass)
        {
            var path = InstrumentsPath;
            _vehicleClass = vehicleClass;
            _displays = InstrumentDisplayStore.Load(vehicleClass);
            _values = Array.Empty<InstrumentValue>();
            if (InstrumentsPath != path)
            {
                Watch();
                Apply(Read(InstrumentsPath));
                return;
            }
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
        Arrange();
        Card.Visibility = _values.Count == 0 ? Visibility.Collapsed : Visibility.Visible;
    }

    private void Arrange()
    {
        var speeds = _values.Select(v => Osd.IsSpeed(v.Label, v.Name)).ToList();
        var rows = string.Concat(speeds.Select(speed => speed ? 'S' : 'P'));
        if (rows == _rows) return;
        _rows = rows;
        SpeedRow.Children.Clear();
        PlaceRow.Children.Clear();
        _readings.Zip(speeds).ToList().ForEach(pair => (pair.Second ? SpeedRow : PlaceRow).Children.Add(pair.First.Root));
    }

    private IReadOnlyList<Reading> Rebuild(int count)
    {
        _rows = "";
        return Enumerable.Range(0, count).Select(slot => new Reading(Styled, Menu(slot), () => _ = StyleSlotAsync(slot))).ToList();
    }

    private string EditingLabel(int slot) =>
        slot < _values.Count ? $"Currently {_values[slot].Label}" : "";

    private MenuFlyout Menu(int slot)
    {
        var add = Item("Add reading", "", AddSlot);
        var remove = Item("Remove this reading", "", () => RemoveSlot(slot));
        var attitude = new ToggleMenuFlyoutItem { Text = "Compass and attitude", Icon = new FontIcon { Glyph = "\uE707" } };
        attitude.Click += (_, _) => ShowAttitude(attitude.IsChecked);
        var menu = new MenuFlyout();
        new MenuFlyoutItemBase[]
        {
            Item("Change reading…", "", async () => await EditSlotAsync(slot)),
            Item("Telemetry display…", "\uE790", async () => await StyleSlotAsync(slot)),
            add,
            remove,
            new MenuFlyoutSeparator(),
            attitude,
            new MenuFlyoutSeparator(),
            Item("Reset to defaults", "", ResetSlots),
        }.ToList().ForEach(menu.Items.Add);
        menu.Opening += (_, _) =>
        {
            add.IsEnabled = CanAdd;
            remove.IsEnabled = CanRemove;
            attitude.IsChecked = AttitudeShown;
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
        private readonly Image _icon = new() { Width = 16, Height = 16, Stretch = Stretch.Uniform, VerticalAlignment = VerticalAlignment.Center, Visibility = Visibility.Collapsed };
        private readonly Run _label = new();
        private readonly Run _gap = new();
        private readonly Run _value = new();
        private readonly Run _absent = new();
        private readonly TextBlock _text;
        private readonly Func<string, Style?> _styled;

        private string _iconKey = "";

        public Reading(Func<string, Style?> styled, MenuFlyout menu, Action open)
        {
            _styled = styled;
            _text = new TextBlock { Style = styled("OsdPlace"), Inlines = { _label, _gap, _value, _absent } };
            Root = new Button
            {
                Padding = new Thickness(4, 0, 4, 0),
                MinHeight = 0,
                MinWidth = 0,
                VerticalAlignment = VerticalAlignment.Bottom,
                Content = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 4, Children = { _icon, Osd.Shadowed(_text) } },
                ContextFlyout = menu,
            };
            Forms.FormLook.Paint(Root, Forms.ButtonLook.Subtle);
            ToolTipService.SetToolTip(Root, ReadingTip);
            Root.Click += (_, _) => open();
        }

        public Button Root { get; }

        public void Show(InstrumentValue value, ValueDisplay display)
        {
            var colour = value.Missing ? null : InstrumentDisplays.Colour(display, value.Raw);
            var icon = InstrumentDisplays.IconOf(display, value.Raw);
            var label = icon is null && !InstrumentDisplays.IconShown(display)
                ? display.Text.Length > 0 ? display.Text : Osd.Label(value.Label, value.Name)
                : "";
            _text.Style = _styled(Osd.IsSpeed(value.Label, value.Name) ? "OsdSpeed" : "OsdPlace");
            _label.Text = label;
            _label.FontSize = PreflightLook.Size("FontSizeCaption");
            _label.FontWeight = FontWeights.Medium;
            _label.Foreground = PreflightLook.Token("OsdLabelBrush");
            _gap.Text = label.Length > 0 ? " " : "";
            _value.Text = display.ShowUnits ? value.Value + value.Units : value.Value;
            _value.Foreground = colour is { } argb
                ? new SolidColorBrush(InstrumentDisplays.ColorOf(argb))
                : PreflightLook.Token(value.Missing ? "OsdLabelBrush" : "OsdTextBrush");
            _absent.Text = value.AbsentHere ? " " + InstrumentValue.AbsentText : "";
            _absent.FontSize = PreflightLook.Size("FontSizeCaption");
            _absent.Foreground = PreflightLook.Token("CautionBrush");
            _icon.Opacity = InstrumentDisplays.Opacity(display, value.Raw);
            ShowIcon(icon, colour is { } tint ? InstrumentDisplays.Hex(tint) : Forms.AssetImages.Hex("OsdTextBrush"));
            AutomationProperties.SetName(Root, $"{value.Label} {value.Value} {value.Units}".Trim());
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
