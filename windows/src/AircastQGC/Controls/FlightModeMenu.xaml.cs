using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using Microsoft.UI;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Shapes;

namespace AircastQGC.Controls;

public sealed partial class FlightModeMenu : UserControl
{
    private const string ViewPath = "view.flightModes";
    private const string ModePath = "vehicle.flightMode";
    private const string WatchClient = "flightModeMenu";
    private const string AirplaneGlyph = "";
    private static readonly TimeSpan RequestTimeout = TimeSpan.FromSeconds(5);

    private static readonly (string[] Keywords, string Glyph)[] Glyphs =
    {
        (new[] { "rtl", "return" }, ""),
        (new[] { "land", "dock" }, ""),
        (new[] { "takeoff" }, ""),
        (new[] { "auto", "mission" }, ""),
        (new[] { "guided", "offboard" }, ""),
        (new[] { "loiter", "circle", "orbit", "hold", "brake", "position" }, ""),
        (new[] { "follow" }, ""),
        (new[] { "acro", "sport", "flip", "rattitude", "turtle" }, ""),
        (new[] { "altitude", "depth", "surface", "surftrak" }, ""),
        (new[] { "autotune", "systemid", "motor detection" }, ""),
        (new[] { "stabilize", "stabilized", "manual", "training", "steering" }, ""),
    };

    private static readonly (string Key, string Token)[] SubtleButton =
    {
        ("ButtonBackground", ""), ("ButtonBackgroundPointerOver", "SubtleHoverBrush"),
        ("ButtonBackgroundPressed", "SubtleSelectedBrush"), ("ButtonBackgroundDisabled", ""),
        ("ButtonBorderBrush", ""), ("ButtonBorderBrushPointerOver", ""),
        ("ButtonBorderBrushPressed", ""), ("ButtonBorderBrushDisabled", ""),
        ("ButtonForeground", "TextPrimaryBrush"), ("ButtonForegroundPointerOver", "TextPrimaryBrush"),
        ("ButtonForegroundPressed", "TextSecondaryBrush"), ("ButtonForegroundDisabled", "TextDisabledBrush"),
    };

    private static readonly (string Key, string Token)[] StandardButton =
    {
        ("ButtonBackground", "ControlBrush"), ("ButtonBackgroundPointerOver", "ControlHoverBrush"),
        ("ButtonBackgroundPressed", "ControlBrush"), ("ButtonBackgroundDisabled", "ControlBrush"),
        ("ButtonBorderBrush", "ControlStrokeBrush"), ("ButtonBorderBrushPointerOver", "ControlStrokeBrush"),
        ("ButtonBorderBrushPressed", "ControlStrokeBrush"), ("ButtonBorderBrushDisabled", "ControlStrokeBrush"),
        ("ButtonForeground", "TextPrimaryBrush"), ("ButtonForegroundPointerOver", "TextPrimaryBrush"),
        ("ButtonForegroundPressed", "TextSecondaryBrush"), ("ButtonForegroundDisabled", "TextDisabledBrush"),
    };

    private static readonly (string Key, string Token)[] AccentButton =
    {
        ("ButtonBackground", "AccentBrush"), ("ButtonBackgroundPointerOver", "AccentHoverBrush"),
        ("ButtonBackgroundPressed", "AccentHoverBrush"), ("ButtonBackgroundDisabled", "ControlBrush"),
        ("ButtonBorderBrush", "AccentBrush"), ("ButtonBorderBrushPointerOver", "AccentHoverBrush"),
        ("ButtonBorderBrushPressed", "AccentHoverBrush"), ("ButtonBorderBrushDisabled", "ControlStrokeBrush"),
        ("ButtonForeground", "AccentOnBrush"), ("ButtonForegroundPointerOver", "AccentOnBrush"),
        ("ButtonForegroundPressed", "AccentOnBrush"), ("ButtonForegroundDisabled", "TextDisabledBrush"),
    };

    private readonly Flyout _flyout;
    private readonly DispatcherQueueTimer _timeout;
    private JsonObject _view = new();
    private string _rendered = "";
    private string _askedFrom = "";
    private string _confirming = "";
    private string _error = "";
    private bool _showFolded;

    public event EventHandler<string>? ModeChanged;
    public event EventHandler? StateChanged;

    public bool Available => Flag(_view["available"]);
    public bool CanSet => Flag(_view["canSet"]);
    public string Current => Text(_view["current"]);
    public string Requested { get; private set; } = "";
    public bool Pending => Requested.Length > 0;
    public string Display => Pending ? Requested : Current;
    public bool IsOpen => _flyout.IsOpen;

    public FlightModeMenu()
    {
        InitializeComponent();
        Paint(Lists, SubtleButton);
        Paint(MoreButton, SubtleButton);
        Paint(Footer, StandardButton);
        Paint(ConfirmButton, AccentButton);

        _flyout = new Flyout
        {
            Content = this,
            Placement = FlyoutPlacementMode.BottomEdgeAlignedRight,
            FlyoutPresenterStyle = (Style)Resources["MenuPresenter"],
        };
        _flyout.Closed += (_, _) =>
        {
            _confirming = "";
            Render();
        };

        _timeout = DispatcherQueue.CreateTimer();
        _timeout.Interval = RequestTimeout;
        _timeout.IsRepeating = false;
        _timeout.Tick += (_, _) =>
        {
            Settle();
            Render();
            StateChanged?.Invoke(this, EventArgs.Empty);
        };

        BridgeWatch.Watch(WatchClient, new[] { ViewPath }, Apply);
        Apply(Read());
    }

    public void Show(FrameworkElement anchor)
    {
        _confirming = "";
        _error = "";
        Apply(Read());
        _flyout.ShowAt(anchor);
    }

    public void Hide() => _flyout.Hide();

    private void Apply(JsonObject view)
    {
        _view = view;
        if (Pending && Current != _askedFrom) Settle();
        Render();
        StateChanged?.Invoke(this, EventArgs.Empty);
    }

    private void Settle()
    {
        _timeout.Stop();
        Requested = "";
    }

    private void Render()
    {
        var everyday = Objects(_view["everyday"]);
        var folded = Objects(_view["folded"]);
        var notice = new[] { Text(_view["unknownModeNotice"]), Text(_view["cannotSetNotice"]) }
            .FirstOrDefault(text => text.Length > 0) ?? "";
        var key = $"{CanSet}|{_view["everyday"]?.ToJsonString()}|{_view["folded"]?.ToJsonString()}";

        if (key != _rendered)
        {
            _rendered = key;
            Fill(EverydayList, everyday);
            Fill(FoldedList, folded);
        }

        SummaryText.Text = Text(_view["currentSummary"]);
        SummaryText.Visibility = Shown(SummaryText.Text.Length > 0);
        NoticeText.Text = notice;
        NoticeBox.Visibility = Shown(notice.Length > 0);
        EmptyText.Visibility = Shown(!Available);
        ListScroll.Visibility = Shown(Available);
        FoldedList.Visibility = Shown(_showFolded && folded.Count > 0);
        MoreButton.Visibility = Shown(Available && folded.Count > 0);
        MoreText.Text = _showFolded ? "Fewer modes" : $"More modes ({folded.Count})";
        MoreGlyph.Glyph = _showFolded ? "" : "";
        ErrorText.Text = _error;
        ErrorText.Visibility = Shown(_error.Length > 0);

        var confirming = _confirming.Length > 0;
        ConfirmText.Text = $"{_confirming} while flying — confirm?";
        ConfirmButton.Content = _confirming;
        ConfirmText.Visibility = Shown(confirming);
        BackButton.Visibility = Shown(confirming);
        ConfirmButton.Visibility = Shown(confirming);
        CancelButton.Visibility = Shown(!confirming);
    }

    private void Fill(Panel panel, IReadOnlyList<JsonObject> modes)
    {
        panel.Children.Clear();
        modes
            .Select((mode, row) => (mode, startsSection: row > 0 && Text(modes[row - 1]["section"]) != Text(mode["section"])))
            .SelectMany(entry => entry.startsSection
                ? new UIElement[] { new Rectangle { Style = Styled("SectionDivider") }, Row(entry.mode) }
                : new UIElement[] { Row(entry.mode) })
            .ToList()
            .ForEach(panel.Children.Add);
    }

    private Button Row(JsonObject mode)
    {
        var name = Text(mode["name"]);
        var summary = Text(mode["summary"]);
        var caution = Text(mode["caution"]);
        var current = Flag(mode["current"]);
        var needsConfirm = Flag(mode["needsConfirm"]);

        var words = new StackPanel { Spacing = 2, VerticalAlignment = VerticalAlignment.Center };
        words.Children.Add(new TextBlock { Text = name, Style = Styled("ModeName") });
        if (summary.Length > 0) words.Children.Add(new TextBlock { Text = summary, Style = Styled("ModeSummary") });

        var trailing = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8, VerticalAlignment = VerticalAlignment.Center };
        if (caution.Length > 0)
            trailing.Children.Add(new Border { Style = Styled("CautionChip"), Child = new TextBlock { Text = caution, Style = Styled("CautionChipText") } });
        if (current) trailing.Children.Add(new FontIcon { Glyph = "", Style = Styled("ModeCheck") });

        var layout = new Grid { ColumnSpacing = 12 };
        layout.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        layout.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        layout.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        layout.Children.Add(new FontIcon { Glyph = GlyphFor(name), Style = Styled(current ? "ModeGlyphCurrent" : "ModeGlyph") });
        Grid.SetColumn(words, 1);
        Grid.SetColumn(trailing, 2);
        layout.Children.Add(words);
        layout.Children.Add(trailing);

        var button = new Button { Content = layout, Style = Styled("ModeRow"), IsEnabled = CanSet && !current };
        AutomationProperties.SetName(button, name);
        button.Click += (_, _) => Request(name, needsConfirm);
        return button;
    }

    private void Request(string name, bool needsConfirm)
    {
        if (!CanSet || name == Current) return;
        _error = "";
        if (needsConfirm)
        {
            _confirming = name;
            Render();
            return;
        }
        Send(name);
    }

    private void Send(string name)
    {
        _confirming = "";
        var answer = Write(name);
        var ok = Flag(answer["ok"]);
        _error = ok ? "" : Refusal(Text(answer["reason"]));
        if (ok && !Flag(answer["unchanged"]))
        {
            _askedFrom = Current;
            Requested = name;
            _timeout.Stop();
            _timeout.Start();
        }
        if (ok)
        {
            _flyout.Hide();
            ModeChanged?.Invoke(this, name);
        }
        Apply(Read());
    }

    private void OnMoreClick(object sender, RoutedEventArgs e)
    {
        _showFolded = !_showFolded;
        Render();
    }

    private void OnCancelClick(object sender, RoutedEventArgs e) => _flyout.Hide();

    private void OnBackClick(object sender, RoutedEventArgs e)
    {
        _confirming = "";
        Render();
    }

    private void OnConfirmClick(object sender, RoutedEventArgs e)
    {
        if (_confirming.Length > 0) Send(_confirming);
    }

    private static JsonObject Read()
    {
        try { return Bridge.Get(ViewPath); }
        catch (Exception) { return new JsonObject(); }
    }

    private static JsonObject Write(string name)
    {
        try { return Bridge.Set(ModePath, JsonValue.Create(name)); }
        catch (Exception) { return new JsonObject(); }
    }

    private static string Refusal(string reason) =>
        reason.Length == 0
            ? "Could not change the flight mode. It is unchanged."
            : $"Could not change the flight mode: {reason.TrimEnd('.')}. It is unchanged.";

    private static string GlyphFor(string mode)
    {
        var name = mode.ToLowerInvariant();
        return Glyphs
            .Where(entry => entry.Keywords.Any(keyword => name.Contains(keyword)))
            .Select(entry => entry.Glyph)
            .FirstOrDefault() ?? AirplaneGlyph;
    }

    private Style Styled(string key) => (Style)Resources[key];

    private static void Paint(FrameworkElement scope, IEnumerable<(string Key, string Token)> map) =>
        map.ToList().ForEach(entry => scope.Resources[entry.Key] = Token(entry.Token));

    private static object Token(string key) =>
        key.Length > 0 && Application.Current.Resources.TryGetValue(key, out var value) && value != null
            ? value
            : new SolidColorBrush(Colors.Transparent);

    private static bool Flag(JsonNode? node) =>
        node is JsonValue value && value.TryGetValue<bool>(out var flag) && flag;

    private static string Text(JsonNode? node) =>
        node is JsonValue value && value.TryGetValue<string>(out var text) ? text ?? "" : "";

    private static List<JsonObject> Objects(JsonNode? node) =>
        (node as JsonArray)?.OfType<JsonObject>().ToList() ?? new List<JsonObject>();

    private static Visibility Shown(bool on) => on ? Visibility.Visible : Visibility.Collapsed;
}
