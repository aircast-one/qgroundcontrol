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

namespace AircastQGC.Controls;

public sealed partial class GuidedBar : UserControl
{
    private const string WatchClient = "guidedBar";
    private const string EmergencyStop = "emergencyStop";
    private const string Return = "rtl";
    private const string PayloadLabel = "Payload";
    private const string PayloadGlyph = "";
    private const string StopHint = "Hold to stop all motors, or click to confirm first.";
    private const double BlockedOpacity = 0.35;
    private static readonly TimeSpan HoldToCut = TimeSpan.FromMilliseconds(1500);
    private static readonly string[] FlyingDeck = ["pause", Return, "land"];

    private readonly Storyboard _cut = new();
    private IReadOnlyList<GuidedOffer> _offers = Array.Empty<GuidedOffer>();
    private GuidedOffer? _stop;
    private bool _cutFired;

    public event EventHandler<GuidedOffer>? ActionRequested;

    public event EventHandler<GuidedOffer>? CutRequested;

    public event EventHandler? PayloadRequested;

    public IReadOnlyList<GuidedOffer> Offers => _offers;

    public GuidedBar()
    {
        InitializeComponent();
        var fill = new DoubleAnimation { From = 0, To = 1, Duration = HoldToCut, EnableDependentAnimation = true };
        Storyboard.SetTarget(fill, StopHold);
        Storyboard.SetTargetProperty(fill, "Value");
        _cut.Children.Add(fill);
        _cut.Completed += (_, _) => Cut();
        StopButton.AddHandler(PointerPressedEvent, new PointerEventHandler(StopPressed), true);
        StopButton.AddHandler(PointerReleasedEvent, new PointerEventHandler(StopReleased), true);
        StopButton.AddHandler(PointerCanceledEvent, new PointerEventHandler(StopLost), true);
        StopButton.AddHandler(PointerCaptureLostEvent, new PointerEventHandler(StopLost), true);
        Loaded += (_, _) => Start();
        Unloaded += (_, _) => Stop();
    }

    public void Start()
    {
        BridgeWatch.Watch(WatchClient, new[] { GuidedOffer.ViewPath }, Apply);
        Apply(Bridge.Get(GuidedOffer.ViewPath));
    }

    public void Stop() => BridgeWatch.Stop(WatchClient);

    private static IReadOnlyList<string> Deck(IReadOnlySet<string> shown) =>
        (Flying(shown) ? FlyingDeck : [shown.Contains("takeoff") ? "takeoff" : "arm"]).Where(shown.Contains).ToList();

    private static string? Primary(IReadOnlyList<string> deck, IReadOnlySet<string> shown) =>
        Flying(shown) ? deck.FirstOrDefault(id => id == Return) : deck.LastOrDefault();

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
        Strip.ItemsSource = deck
            .Select(id => offers.First(o => o.Id == id))
            .Select(offer => Rail(offer.Title, offer.Explanation, offer.Glyph, offer.Id == primary, offer.Ready, offer.Blocked,
                () => ActionRequested?.Invoke(this, offer)))
            .Concat(payloadOnRail ? [Rail(PayloadLabel, "", PayloadGlyph, false, payloadReady, !payloadReady, Payload)] : [])
            .ToList();
        MoreMenu.Items.Clear();
        rest.Select(MoreItem)
            .Concat(gripper.Count > 0 && !payloadOnRail ? [Item(PayloadLabel, "", PayloadGlyph, payloadReady, false, Payload)] : [])
            .ToList()
            .ForEach(MoreMenu.Items.Add);
        MoreButton.Visibility = Seen(MoreMenu.Items.Count > 0);
        PaintStop(offers.FirstOrDefault(o => o.Id == EmergencyStop));
        Root.Visibility = Seen(deck.Count > 0 || payloadOnRail || MoreMenu.Items.Count > 0 || _stop != null);
    }

    private void PaintStop(GuidedOffer? stop)
    {
        _stop = stop;
        StopHost.Visibility = Seen(stop != null);
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

    private Button Rail(string label, string explanation, string glyph, bool primary, bool ready, bool blocked, Action act)
    {
        var button = new Button
        {
            Style = (Style)Application.Current.Resources["FlyRailButton"],
            IsEnabled = ready,
            Opacity = blocked ? BlockedOpacity : 1,
            Content = new FontIcon { Glyph = glyph, Style = Styled(primary ? "RailIconPrimary" : "RailIcon") },
        };
        ToolTipService.SetToolTip(button, Tip(label, explanation));
        ToolTipService.SetPlacement(button, PlacementMode.Right);
        AutomationProperties.SetName(button, label);
        AutomationProperties.SetHelpText(button, explanation);
        button.Click += (_, _) => act();
        return button;
    }

    private MenuFlyoutItem MoreItem(GuidedOffer offer) =>
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

    private Style Styled(string key) => (Style)Resources[key];

    private static Visibility Seen(bool on) => on ? Visibility.Visible : Visibility.Collapsed;
}
