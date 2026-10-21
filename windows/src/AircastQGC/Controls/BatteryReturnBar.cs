using System;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

public sealed class BatteryReturnBar : UserControl
{
    private const string BatteryPath = "view.battery";
    private const string FlyStatePath = "view.flyState";

    private readonly string _client = $"batteryReturnBar.{Guid.NewGuid():N}";
    private JsonObject _battery = new();
    private JsonObject _fly = new();

    public BatteryReturnBar()
    {
        IsHitTestVisible = false;
        VerticalAlignment = VerticalAlignment.Top;
        Loaded += (_, _) =>
        {
            if (!CoreHost.Started) return;
            _battery = Bridge.Get(BatteryPath);
            _fly = Bridge.Get(FlyStatePath);
            BridgeWatch.Watch($"{_client}.battery", [BatteryPath], Live.WhileShown(this, view => Render(_battery = view)));
            BridgeWatch.Watch($"{_client}.fly", [FlyStatePath], Live.WhileShown(this, view => Render(_fly = view)));
            Render(_battery);
        };
        Unloaded += (_, _) =>
        {
            BridgeWatch.Stop($"{_client}.battery");
            BridgeWatch.Stop($"{_client}.fly");
        };
    }

    private void Render(JsonObject _)
    {
        var headline = FlyStatus.Parse(_fly).Connected ? BatteryHeadline.Parse(_battery) : null;
        Content = headline?.Percent is null ? null : StatusLook.ReturnBar(headline, !FlyStatus.Parse(_fly).Stale);
    }
}
