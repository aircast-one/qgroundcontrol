using System;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Forms;
using AircastQGC.Pages.Analyze;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

internal enum UsbRadioState { Live, Paused, Standby, Ready, Stopped }

internal sealed record UsbRadioBadge(string Text, UsbRadioState State);

internal static class UsbRadio
{
    public const string View = "view.usbRadio";

    public static UsbRadioBadge? Badge(JsonObject view) =>
        view["indicator"] is JsonObject indicator
        && Enum.GetNames<UsbRadioState>().FirstOrDefault(name => name.ToLowerInvariant() == MapUiJson.Text(indicator["state"])) is { } state
            ? new UsbRadioBadge(MapUiJson.Text(indicator["text"]), Enum.Parse<UsbRadioState>(state))
            : null;

    public static StatusTone Tone(UsbRadioState state) => state switch
    {
        UsbRadioState.Stopped => StatusTone.Critical,
        UsbRadioState.Paused => StatusTone.Caution,
        _ => StatusTone.Normal,
    };
}

public sealed class UsbRadioCell : UserControl
{
    private readonly StatusPill _pill = StatusLook.Pill("USB radio", "");

    public UsbRadioCell()
    {
        _pill.Button.IsHitTestVisible = false;
        _pill.Button.IsTabStop = false;
        Content = _pill.Button;
        Visibility = Visibility.Collapsed;
        VerticalAlignment = VerticalAlignment.Center;
        Live.Watch(this, UsbRadio.View, view =>
        {
            var badge = UsbRadio.Badge(view);
            Visibility = Ui.Shown(badge is not null);
            if (badge is not null) StatusLook.Paint(_pill, badge.Text, UsbRadio.Tone(badge.State));
        });
    }
}

public sealed class SupportForwardingCell : UserControl
{
    private const string LinksView = "view.links";
    private const string HostSetting = "view.control(settings.mavlinkSettings.forwardMavlinkAPMSupportHostName)";

    private readonly StatusPill _pill = StatusLook.Pill("Support", "");
    private string _host = "";

    public SupportForwardingCell()
    {
        var sheet = new Sheet("Support forwarding",
            () => Ui.Column([
                StatusLook.Line("Mavlink traffic is being forwarded to a support server"),
                StatusLook.Line($"Server name:  {_host}"),
            ], 8),
            opened: () => _host = FormJson.Shown(Bridge.Get(HostSetting)["valueString"]));
        StatusLook.Paint(_pill, "Support", StatusTone.Good);
        _pill.Label.Foreground = Ui.Token("SuccessBrush");
        _pill.Button.Click += (_, _) => sheet.ShowAt(_pill.Button);
        Content = _pill.Button;
        Visibility = Visibility.Collapsed;
        VerticalAlignment = VerticalAlignment.Center;
        Live.Watch(this, LinksView, view => Visibility = Ui.Shown(MapUiJson.Flag(view["supportForwarding"])));
    }
}
