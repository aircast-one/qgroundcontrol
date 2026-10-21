using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media.Animation;
using Windows.Foundation;
using Windows.System;

namespace AircastQGC.Controls;

public sealed partial class GuidedBar : UserControl
{
    private const string WatchClient = "guidedBar";
    private const string EmergencyStop = "emergencyStop";
    private const string Return = "rtl";
    private const string StartMission = "startMission";
    private const string PayloadLabel = "Payload";
    private const string PayloadGlyph = "";
    private const string HomeLetter = "H";
    private const string ReturnShortcut = "Ctrl+R";
    private const string StopHint = "Hold to stop all motors, or click to confirm first.";
    private const string StopLabel = "Stop motors";
    private const double BlockedOpacity = 0.35;
    private static readonly TimeSpan HoldToCut = TimeSpan.FromMilliseconds(1500);
    private static readonly string[] FlyingDeck = ["pause", Return, "land"];
    private static readonly IReadOnlyDictionary<string, string> RailLabels = new Dictionary<string, string>
    {
        ["arm"] = "Arm",
        ["takeoff"] = "Take off",
        [StartMission] = "Start",
        ["pause"] = "Pause",
        [Return] = "Return",
        ["land"] = "Land",
    };

    private readonly Storyboard _cut = new();
    private readonly IReadOnlyList<FrameworkElement> _fixedLabels;
    private IReadOnlyList<(FrameworkElement Label, bool Primary)> _deckLabels = [];
    private IReadOnlyList<GuidedOffer> _offers = Array.Empty<GuidedOffer>();
    private GuidedOffer? _stop;
    private MenuFlyoutItem? _extra;
    private bool _cutFired;
    private double _room = double.PositiveInfinity;

    public event EventHandler<GuidedOffer>? ActionRequested;

    public event EventHandler<GuidedOffer>? CutRequested;

    public event EventHandler? PayloadRequested;

    public event EventHandler? ChecklistRequested;

    public event EventHandler? DeckChanged;

    public IReadOnlyList<GuidedOffer> Offers => _offers;

    public bool OnFlyingDeck { get; private set; }

    public double Room
    {
        get => _room;
        set
        {
            if (value == _room) return;
            _room = value;
            Fit();
        }
    }

    public GuidedBar()
    {
        InitializeComponent();
        _fixedLabels = [Caption(ChecklistItem, "Checklist"), Caption(MoreItem, "More"), Caption(StopItem, StopLabel)];
        var fill = new DoubleAnimation { From = 0, To = 1, Duration = HoldToCut, EnableDependentAnimation = true };
        Storyboard.SetTarget(fill, StopHold);
        Storyboard.SetTargetProperty(fill, "Value");
        _cut.Children.Add(fill);
        _cut.Completed += (_, _) => Cut();
        StopButton.AddHandler(PointerPressedEvent, new PointerEventHandler(StopPressed), true);
        StopButton.AddHandler(PointerReleasedEvent, new PointerEventHandler(StopReleased), true);
        StopButton.AddHandler(PointerCanceledEvent, new PointerEventHandler(StopLost), true);
        StopButton.AddHandler(PointerCaptureLostEvent, new PointerEventHandler(StopLost), true);
        ChecklistButton.Click += (_, _) => ChecklistRequested?.Invoke(this, EventArgs.Empty);
        Loaded += (_, _) => Start();
        Unloaded += (_, _) => Stop();
    }

    public void Start()
    {
        BridgeWatch.Watch(WatchClient, new[] { GuidedOffer.ViewPath }, Apply);
        Apply(Bridge.Get(GuidedOffer.ViewPath));
    }

    public void Stop() => BridgeWatch.Stop(WatchClient);

    public void ShowExtra((string Label, string Glyph, Action Act)? extra)
    {
        if (_extra is { } shown) MoreMenu.Items.Remove(shown);
        _extra = extra is { } offered ? Item(offered.Label, "", offered.Glyph, true, false, offered.Act) : null;
        if (_extra is { } item) MoreMenu.Items.Insert(0, item);
        MoreItem.Visibility = Seen(MoreMenu.Items.Count > 0);
        Present();
    }

    public void ShowChecklist(bool shown, string tip)
    {
        ChecklistItem.Visibility = Seen(shown);
        ToolTipService.SetToolTip(ChecklistButton, tip);
        Present();
    }

    private static IReadOnlyList<string> Deck(IReadOnlySet<string> shown) =>
        (Flying(shown) ? FlyingDeck : [shown.Contains("takeoff") ? "takeoff" : "arm", StartMission]).Where(shown.Contains).ToList();

    private static string? Primary(IReadOnlyList<string> deck, IReadOnlySet<string> shown) =>
        Flying(shown) ? deck.FirstOrDefault(id => id == Return) : deck.FirstOrDefault();

    private static bool Flying(IReadOnlySet<string> shown) => shown.Contains(Return) || shown.Contains("land");

    private void Apply(JsonObject view)
    {
        var offers = GuidedOffer.List(view).Where(o => o.Shown).ToList();
        if (offers.SequenceEqual(_offers)) return;
        _offers = offers;
        var shown = offers.Select(o => o.Id).ToHashSet();
        var deck = Deck(shown);
        var primary = Primary(deck, shown);
        var gripper = offers.Where(o => PayloadJson.GripperIds.Contains(o.Id)).ToList();
        var payloadOnRail = gripper.Count > 0 && Flying(shown);
        var payloadReady = gripper.Any(o => o.Ready);
        var rest = offers.Where(o => !deck.Contains(o.Id) && o.Id != EmergencyStop && !gripper.Contains(o)).ToList();
        var items = deck
            .Select(id => offers.First(o => o.Id == id))
            .Select(offer => Rail(offer.Id, RailLabels.GetValueOrDefault(offer.Id, offer.Title), offer.Title, offer.Explanation,
                offer.Glyph, offer.Id == primary, offer.Ready, offer.Blocked, () => ActionRequested?.Invoke(this, offer)))
            .Concat(payloadOnRail ? [Rail("payload", PayloadLabel, PayloadLabel, "", PayloadGlyph, false, payloadReady, !payloadReady, Payload)] : [])
            .ToList();
        _deckLabels = items.Select(item => (item.Label, item.Primary)).ToList();
        Strip.ItemsSource = items.Select(item => item.View).ToList();
        MoreMenu.Items.Clear();
        (_extra is { } extra ? [extra] : Array.Empty<MenuFlyoutItem>())
            .Concat(rest.Select(MoreItemFor))
            .Concat(gripper.Count > 0 && !payloadOnRail ? [Item(PayloadLabel, "", PayloadGlyph, payloadReady, false, Payload)] : [])
            .ToList()
            .ForEach(MoreMenu.Items.Add);
        MoreItem.Visibility = Seen(MoreMenu.Items.Count > 0);
        PaintStop(offers.FirstOrDefault(o => o.Id == EmergencyStop));
        Present();
        if (OnFlyingDeck == Flying(shown)) return;
        OnFlyingDeck = Flying(shown);
        DeckChanged?.Invoke(this, EventArgs.Empty);
    }

    private void Present()
    {
        Root.Visibility = Seen(_deckLabels.Count > 0 || new UIElement[] { ChecklistItem, MoreItem, StopItem }.Any(item => item.Visibility == Visibility.Visible));
        Fit();
    }

    private void Fit()
    {
        ShowLabels(true);
        Root.Measure(new Size(double.PositiveInfinity, double.PositiveInfinity));
        ShowLabels(Root.DesiredSize.Height <= _room);
    }

    private void ShowLabels(bool all) =>
        _fixedLabels.Select(label => (Label: label, Primary: false)).Concat(_deckLabels).ToList()
            .ForEach(entry => entry.Label.Visibility = Seen(all || entry.Primary));

    private void PaintStop(GuidedOffer? stop)
    {
        _stop = stop;
        StopItem.Visibility = Seen(stop != null);
        if (stop == null)
        {
            _cut.Stop();
            return;
        }
        StopButton.IsEnabled = stop.Ready;
        AutomationProperties.SetName(StopButton, stop.Title);
        AutomationProperties.SetHelpText(StopButton, $"{StopHint} {stop.Explanation}".Trim());
        ToolTipService.SetToolTip(StopButton, Tip(stop.Title, StopHint));
    }

    private (UIElement View, FrameworkElement Label, bool Primary) Rail(string id, string label, string title, string explanation,
        string glyph, bool primary, bool ready, bool blocked, Action act)
    {
        var returning = id == Return;
        var button = new Button
        {
            Style = Styled(primary ? "RailPrimaryButton" : "FlyRailButton"),
            HorizontalAlignment = HorizontalAlignment.Center,
            IsEnabled = ready,
            Opacity = blocked ? BlockedOpacity : 1,
            Content = returning ? HomeIcon(glyph) : new FontIcon { Glyph = glyph, Style = Styled(primary ? "RailIconPrimary" : "RailIcon") },
        };
        if (primary) PreflightLook.Accent(button);
        if (returning) Shortcut(button);
        ToolTipService.SetToolTip(button, Tip(returning ? $"{title} ({ReturnShortcut})" : title, explanation));
        ToolTipService.SetPlacement(button, PlacementMode.Right);
        AutomationProperties.SetName(button, title);
        AutomationProperties.SetHelpText(button, explanation);
        button.Click += (_, _) => act();
        var view = new StackPanel { Spacing = (double)Application.Current.Resources["SpaceXs"], Children = { button } };
        return (view, Caption(view, label), primary);
    }

    private static void Shortcut(Button button)
    {
        button.KeyboardAccelerators.Add(new KeyboardAccelerator { Key = VirtualKey.R, Modifiers = VirtualKeyModifiers.Control });
        button.KeyboardAcceleratorPlacementMode = KeyboardAcceleratorPlacementMode.Hidden;
        AutomationProperties.SetAcceleratorKey(button, ReturnShortcut);
    }

    private Grid HomeIcon(string glyph) => new()
    {
        Children =
        {
            new FontIcon { Glyph = glyph, Style = Styled("RailIconPrimary") },
            new TextBlock { Text = HomeLetter, Style = Styled("HomeMark") },
        },
    };

    private FrameworkElement Caption(Panel item, string text)
    {
        var label = Osd.Shadowed(new TextBlock { Text = text, Style = Styled("RailLabel") });
        label.HorizontalAlignment = HorizontalAlignment.Center;
        label.IsHitTestVisible = false;
        item.Children.Add(label);
        return label;
    }

    private MenuFlyoutItem MoreItemFor(GuidedOffer offer) =>
        Item(offer.Title, offer.Explanation, offer.Glyph, offer.Ready, offer.Destructive, () => ActionRequested?.Invoke(this, offer));

    private static MenuFlyoutItem Item(string label, string explanation, string glyph, bool ready, bool destructive, Action act)
    {
        var item = new MenuFlyoutItem { Text = label, IsEnabled = ready, Icon = new FontIcon { Glyph = glyph } };
        if (destructive) item.Foreground = PreflightLook.Token("CriticalBrush");
        AutomationProperties.SetHelpText(item, explanation);
        item.Click += (_, _) => act();
        return item;
    }

    private static object Tip(string label, string explanation) =>
        explanation.Length == 0 || explanation == label
            ? label
            : new StackPanel
            {
                Spacing = 2,
                MaxWidth = 280,
                Children =
                {
                    new TextBlock { Text = label, FontWeight = FontWeights.SemiBold },
                    new TextBlock { Text = explanation, TextWrapping = TextWrapping.Wrap },
                },
            };

    private void Payload() => PayloadRequested?.Invoke(this, EventArgs.Empty);

    private void StopPressed(object sender, PointerRoutedEventArgs e)
    {
        _cutFired = false;
        if (_stop is { Ready: true }) _cut.Begin();
    }

    private void StopReleased(object sender, PointerRoutedEventArgs e)
    {
        _cut.Stop();
        _cutFired = false;
    }

    private void StopLost(object sender, PointerRoutedEventArgs e) => _cut.Stop();

    private void Cut()
    {
        _cutFired = true;
        if (_stop is { Ready: true } stop) CutRequested?.Invoke(this, stop);
    }

    private void StopButton_Click(object sender, RoutedEventArgs e)
    {
        if (_cutFired || _stop is not { } stop) return;
        _cut.Stop();
        ActionRequested?.Invoke(this, stop);
    }

    private Style Styled(string key) => (Style)(Resources.TryGetValue(key, out var local) ? local : Application.Current.Resources[key]);

    private static Visibility Seen(bool on) => on ? Visibility.Visible : Visibility.Collapsed;
}
