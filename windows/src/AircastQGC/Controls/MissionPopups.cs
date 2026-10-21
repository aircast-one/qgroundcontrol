using System;
using System.Collections.Immutable;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Forms;

namespace AircastQGC.Controls;

internal sealed class MissionPopups
{
    private const string LandAbort = "landAbort";
    private const string EnabledPath = "view.control(settings.flyViewSettings.enableAutomaticMissionPopups)";
    private static readonly ImmutableArray<string> PopupActions = [LandAbort, "startMission", "continueMission"];
    private static readonly TimeSpan PopupDelay = TimeSpan.FromSeconds(1);

    private readonly GuidedConfirm _card;
    private ImmutableDictionary<string, GuidedOffer> _offers = ImmutableDictionary<string, GuidedOffer>.Empty;
    private ImmutableHashSet<string>? _wasReady;
    private bool _enabled = true;
    private int _due;

    public MissionPopups(GuidedConfirm card)
    {
        _card = card;
        Live.Watch(card, EnabledPath, view => _enabled = FormJson.Truthy(view["value"]));
        Live.Watch(card, GuidedOffer.ViewPath, Apply);
    }

    private static GuidedOffer? Popup(ImmutableHashSet<string> wasReady, ImmutableDictionary<string, GuidedOffer> offers, bool enabled) =>
        PopupActions
            .Select(id => offers.GetValueOrDefault(id))
            .OfType<GuidedOffer>()
            .FirstOrDefault(offer => (enabled || offer.Id == LandAbort) && offer.Ready && !wasReady.Contains(offer.Id));

    private void Apply(JsonObject view)
    {
        _offers = GuidedOffer.List(view).DistinctBy(offer => offer.Id).ToImmutableDictionary(offer => offer.Id);
        var popup = _wasReady is { } wasReady ? Popup(wasReady, _offers, _enabled) : null;
        _wasReady = PopupActions.Where(id => _offers.GetValueOrDefault(id) is { Ready: true }).ToImmutableHashSet();
        if (popup is not null) ShowSoon(popup.Id);
    }

    private async void ShowSoon(string id)
    {
        var due = ++_due;
        await Task.Delay(PopupDelay);
        if (due != _due || _offers.GetValueOrDefault(id) is not { Ready: true } settled) return;
        if (!_card.IsOpen || id == LandAbort) _card.Show(settled, focus: false);
    }
}
