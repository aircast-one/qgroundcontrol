using System.Collections.Generic;
using System.Collections.Immutable;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Pages.Setup;

internal static class ModeSlots
{
    public const string View = "view.modeSlots";

    public static string? LiveSlot(JsonObject view)
    {
        if (!MapUiJson.Flag(view["available"])) return null;
        var slots = MapUiJson.Objects(view["slots"]);
        var channel = (int)(MapUiJson.Number(view["channel"]) ?? 0);
        if (slots.Count == 0) return null;
        return slots.FirstOrDefault(s => MapUiJson.Flag(s["live"])) is { } live
            ? $"The switch on channel {channel} is on slot {(int)(MapUiJson.Number(live["slot"]) ?? 0)}, {MapUiJson.Text(live["mode"])}."
            : MapUiJson.Text(view["reason"]) is { Length: > 0 } reason ? reason : $"The switch on channel {channel} is not on any mode slot.";
    }

    public static string? LiveSwitches(JsonObject view)
    {
        if (!MapUiJson.Flag(view["available"])) return null;
        var switches = MapUiJson.Texts(view["activeSwitches"]);
        var options = MapUiJson.Objects(view["channelOptions"]).Where(o => MapUiJson.Flag(o["enabled"])).Select(o => (int)(MapUiJson.Number(o["channel"]) ?? 0)).ToList();
        return switches.Count > 0 ? $"Switches on: {string.Join(", ", switches)}"
            : options.Count > 0 ? $"Channel options on: {string.Join(", ", options.Select(c => $"channel {c}"))}"
            : null;
    }

    public static IReadOnlySet<string> Active(JsonObject view) =>
        MapUiJson.Flag(view["available"]) ? MapUiJson.Texts(view["activeParams"]).ToImmutableHashSet() : ImmutableHashSet<string>.Empty;
}

internal sealed class FlightModesPanel : UserControl
{
    private readonly System.Lazy<bool> _opened = SetupForms.OpenedOnce(SetupModel.FlightModes);
    private readonly TextBlock _live = FormLook.Text("", "FontSizeBody", "TextPrimaryBrush");
    private readonly Border _banner;
    private readonly ContentControl _form = new() { HorizontalContentAlignment = HorizontalAlignment.Stretch, IsTabStop = false };
    private readonly ContentControl _monitor = new() { HorizontalContentAlignment = HorizontalAlignment.Stretch, IsTabStop = false };
    private string? _highlighted;

    public FlightModesPanel()
    {
        _banner = new Border
        {
            Background = FormLook.Brush("ControlBrush"),
            Padding = new Thickness(SetupUi.PagePadding, 10, SetupUi.PagePadding, 10),
            Child = _live,
            Visibility = Visibility.Collapsed,
        };
        var scroller = SetupUi.Scrolled(FormLook.Stack(0, _form, _monitor));
        Grid.SetRow(scroller, 1);
        Content = new Grid
        {
            RowDefinitions = { new RowDefinition { Height = GridLength.Auto }, new RowDefinition() },
            Children = { _banner, scroller },
        };
        Live.Watch(this, ModeSlots.View, Apply);
        Rebuild(ImmutableHashSet<string>.Empty);
    }

    private void Apply(JsonObject view)
    {
        var live = string.Join("\n", new[] { ModeSlots.LiveSlot(view), ModeSlots.LiveSwitches(view) }.OfType<string>());
        FormLook.Say(_live, live);
        _banner.Visibility = FormLook.Shown(live.Length > 0);
        Rebuild(ModeSlots.Active(view));
        var monitor = MapUiJson.Flag(view["available"]) && MapUiJson.Flag(view["channelMonitor"]);
        if (monitor && _monitor.Content is null) _monitor.Content = new ChannelMonitor();
        if (!monitor) _monitor.Content = null;
    }

    private void Rebuild(IReadOnlySet<string> highlighted)
    {
        var key = string.Join(",", highlighted.Order());
        if (key == _highlighted) return;
        _highlighted = key;
        _form.Content = SetupForms.Opened(SetupModel.FlightModes, _opened, highlighted);
    }
}

internal sealed class ChannelMonitor : UserControl
{
    private readonly Meters _channels = new(true);

    public ChannelMonitor()
    {
        Content = FormLook.Stack(0, SetupUi.Heading("Channel monitor"), _channels);
        Loaded += (_, _) => SetupUi.Detached(() => FormWrites.Ok("radioCal.start"));
        Live.Watch(this, RadioPanel.View, view => _channels.Show(RadioPanel.Channels(view)));
    }
}
