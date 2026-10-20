using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

public sealed partial class GuidedBar : UserControl
{
    private const string WatchClient = "guidedBar";
    private const double BlockedOpacity = 0.35;

    private IReadOnlyList<GuidedOffer> _offers = Array.Empty<GuidedOffer>();

    public event EventHandler<GuidedOffer>? ActionRequested;

    public event EventHandler? PayloadRequested;

    public IReadOnlyList<GuidedOffer> Offers => _offers;

    public GuidedBar()
    {
        InitializeComponent();
        Loaded += (_, _) => Start();
        Unloaded += (_, _) => Stop();
    }

    public void Start()
    {
        BridgeWatch.Watch(WatchClient, new[] { GuidedOffer.ViewPath }, Apply);
        Apply(Bridge.Get(GuidedOffer.ViewPath));
    }

    public void Stop() => BridgeWatch.Stop(WatchClient);

    private void Apply(JsonObject view)
    {
        var offers = GuidedOffer.List(view).Where(o => o.Shown).ToList();
        if (offers.SequenceEqual(_offers)) return;
        _offers = offers;
        var gripper = offers.Where(o => PayloadJson.GripperIds.Contains(o.Id)).ToList();
        Strip.ItemsSource = offers.Except(gripper).Select(Build)
            .Concat(gripper.Count > 0 ? [PayloadTile(gripper.Any(o => o.Ready))] : [])
            .ToList();
        Root.Visibility = offers.Count > 0 ? Visibility.Visible : Visibility.Collapsed;
    }

    private Button Build(GuidedOffer offer)
    {
        var tone = offer.Destructive ? "Destructive" : "";
        var button = new Button
        {
            Style = Styled("GuidedButton"),
            IsEnabled = offer.Ready,
            Opacity = offer.Blocked ? BlockedOpacity : 1,
            Content = new StackPanel
            {
                Spacing = 2,
                Children =
                {
                    new FontIcon { Glyph = offer.Glyph, Style = Styled("GuidedIcon" + tone) },
                    new TextBlock { Text = offer.Title, Style = Styled("GuidedLabel" + tone) },
                },
            },
        };
        ToolTipService.SetToolTip(button, offer.Explanation);
        AutomationProperties.SetName(button, offer.Title);
        AutomationProperties.SetHelpText(button, offer.Explanation);
        button.Click += (_, _) => ActionRequested?.Invoke(this, offer);
        return button;
    }

    private Button PayloadTile(bool ready)
    {
        var button = new Button
        {
            Style = Styled("GuidedButton"),
            IsEnabled = ready,
            Opacity = ready ? 1 : BlockedOpacity,
            Content = new StackPanel
            {
                Spacing = 2,
                Children =
                {
                    new FontIcon { Glyph = "", Style = Styled("GuidedIcon") },
                    new TextBlock { Text = "Payload", Style = Styled("GuidedLabel") },
                },
            },
        };
        AutomationProperties.SetName(button, "Payload");
        button.Click += (_, _) => PayloadRequested?.Invoke(this, EventArgs.Empty);
        return button;
    }

    private Style Styled(string key) => (Style)Resources[key];
}
