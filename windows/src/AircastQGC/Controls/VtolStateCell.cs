using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Forms;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

internal sealed record VtolState(bool Forward, bool Flying)
{
    public string Label => Forward ? "FW(vtol)" : "MR(vtol)";

    public string Transition => Forward ? "vtolTransitionToMultiRotor" : "vtolTransitionToFixedWing";

    public string Name => Forward ? "Transition to Multi-Rotor" : "Transition to Fixed Wing";

    public static VtolState? Of(JsonObject view) =>
        MapUiJson.Flag(view["vtol"]) ? new VtolState(MapUiJson.Flag(view["vtolInFwdFlight"]), MapUiJson.Flag(view["flying"])) : null;
}

public sealed class VtolStateCell : UserControl
{
    private const double PillHeight = 28;

    private readonly Button _pill;
    private readonly TextBlock _label = FormLook.Caption("", "TextPrimaryBrush");
    private VtolState? _state;

    public VtolStateCell()
    {
        _label.TextWrapping = TextWrapping.NoWrap;
        _pill = new Button
        {
            Height = PillHeight,
            Padding = new Thickness(10, 0, 10, 0),
            CornerRadius = new CornerRadius(PillHeight / 2),
            BorderThickness = new Thickness(1),
            Content = _label,
        };
        FormLook.Paint(_pill, ButtonLook.Standard);
        _pill.Click += (_, _) =>
        {
            if (_state is { } state) FlyRequests.AskGuided(state.Transition);
        };
        Content = _pill;
        Visibility = Visibility.Collapsed;
        Live.Watch(this, GuidedOffer.ViewPath, view => Paint(VtolState.Of(view), GuidedOffer.List(view)));
    }

    private void Paint(VtolState? state, IReadOnlyList<GuidedOffer> offers)
    {
        _state = state;
        Visibility = FormLook.Shown(state != null);
        if (state == null) return;
        var offer = offers.FirstOrDefault(o => o.Id == state.Transition);
        FormLook.Say(_label, state.Label);
        _label.FontSize = FormLook.Size(state.Flying ? "FontSizeBody" : "FontSizeCaption");
        _label.FontWeight = state.Flying ? FontWeights.SemiBold : FontWeights.Normal;
        _pill.IsEnabled = state.Flying && offer?.Ready == true;
        AutomationProperties.SetName(_pill, state.Flying ? $"{state.Label}, {state.Name}" : state.Label);
        ToolTipService.SetToolTip(_pill, !state.Flying ? "VTOL state" : offer is { Ready: false } held ? held.Explanation : state.Name);
    }
}
