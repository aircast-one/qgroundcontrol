using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Windows.System;

namespace AircastQGC.Pages.Device;

internal sealed record DeviceStatus(string Name, int Level, string Headline)
{
    public static DeviceStatus? Of(JsonObject? view) =>
        DJ.Text(view, "headline") is { Length: > 0 } headline ? new DeviceStatus(DJ.Text(view, "name"), DJ.Int(view, "level"), headline) : null;

    public bool NeedsAttention => Level != DeviceLevel.Ready;

    public string Chip => Name.Length > 0 ? $"{Name} · {Headline}" : Headline;
}

internal sealed class DeviceHome : DeviceScreen
{
    public DeviceHome(DeviceNav nav) : base(DeviceLink.Home, nav) { }

    protected override IEnumerable<Node?>? Build(DeviceReading reading, DeviceContext ctx) =>
        reading.Origin.Length == 0
            ? [DeviceTree.Leaf("connect", "", () => new ConnectBox("", "Connect", true, () => { }))]
            : (DeviceStatus.Of(reading.View) is null ? [Kit.Hint(Kit.Waiting)] : Status(reading.View!, ctx)).Append(HostedDevice(reading.Host, ctx));

    private static IEnumerable<Node?> Status(JsonObject view, DeviceContext ctx)
    {
        var level = DJ.Int(view, "level");
        var action = DJ.Obj(view, "action") is { } a && DJ.Text(a, "href").Length > 0 ? a : null;
        var setup = DJ.Obj(view, "setup") is { } s && DJ.Text(s, "href").Length > 0 ? s : null;
        var events = DJ.Text(view, "events");
        return new Node?[]
        {
            DeviceTree.Leaf("headline", DJ.Sig(DJ.Text(view, "headline"), DJ.Text(view, "detail"), level, action), () => Headline(view, level, action, ctx)),
            setup is null ? null : Kit.Row(DJ.Text(setup, "label"), "", "", RowState.NeedsAttention, () => ctx.Open(DJ.Text(setup, "href")), DJ.Text(setup, "href")),
            Kit.Section("Before you fly", null, DJ.Objs(view, "checks")
                .Select(check => (Node?)Kit.Row(DJ.Text(check, "label"), DJ.Text(check, "value"), "",
                    DJ.Int(check, "level") == DeviceLevel.Ready ? RowState.Done : RowState.NeedsAttention,
                    DJ.Text(check, "href") is { Length: > 0 } href ? () => ctx.Open(href) : null, DJ.Text(check, "href")))
                .Append(Kit.Row(events.Length > 0 ? events : "Flight log", "", "", events.Length > 0 ? RowState.NeedsAttention : RowState.Neutral, () => ctx.Open("/flights")))),
            DJ.Obj(view, "link") is { } link ? Uplink(link) : null,
            DJ.Obj(view, "health") is { } health ? Health(health) : null,
            Kit.Section(DJ.Text(view, "name") is { Length: > 0 } name ? name : "Device", null,
                DJ.Objs(view, "pages").Select(page => (Node?)Kit.PageRow(ctx, DJ.Page(page)))),
        };
    }

    private static UIElement Headline(JsonObject view, int level, JsonObject? action, DeviceContext ctx)
    {
        var headline = FormLook.Text(DJ.Text(view, "headline"), "FontSizeSubtitle", Kit.LevelBrush(level), true);
        var button = action is null ? null : FormLook.Button(DJ.Text(action, "label"), () => ctx.Open(DJ.Text(action, "href")), ButtonLook.Accent);
        if (button is not null) button.HorizontalAlignment = HorizontalAlignment.Left;
        return FormLook.Margined(FormLook.Stack(4,
            headline,
            FormLook.Text(DJ.Text(view, "detail"), "FontSizeBody", "TextSecondaryBrush"),
            button is null ? null : FormLook.Margined(button, new Thickness(0, 8, 0, 0))), new Thickness(0, 16, 0, 8));
    }

    private static Node Uplink(JsonObject link) => Kit.Section("Link", null, new[]
    {
        DJ.Text(link, "carrier") is { Length: > 0 } carrier ? Kit.Fact("Carrying video", carrier) : null,
        Kit.Text(string.Join(" · ", new[] { DJ.Text(link, "backup"), DJ.Text(link, "network") }.Where(t => t.Length > 0))),
        DJ.Text(link, "rate") is { Length: > 0 } rate ? Kit.Fact("Throughput", rate) : null,
        DJ.Text(link, "standby") is { Length: > 0 } standby ? Kit.Fact("Cellular · standby", standby) : null,
        Kit.Notice(new Badge(DeviceLevel.Warn, DJ.Text(link, "trouble"))),
        DJ.Text(link, "data") is { Length: > 0 } data ? Kit.Fact("Data", data) : null,
    }.Concat(DJ.Charts(link, "charts").Select(Kit.Chart)));

    private static Node Health(JsonObject health) => Kit.Section("Device health", null, new[]
    {
        DJ.Text(health, "memory") is { Length: > 0 } memory ? Kit.Fact("Memory in use", memory) : null,
        Kit.Notice(DJ.MaybeBadge(health, "warning")),
    }.Concat(DJ.Charts(health, "charts").Select(Kit.Chart)));

    private static Node HostedDevice(string host, DeviceContext ctx) => Kit.Section("This app", null, new Node?[]
    {
        Kit.Fact("Connected to", host),
        Kit.Buttons("this-app", [Kit.Button("disconnect", "Disconnect", async () => await DeviceLink.Disconnect(), ButtonLook.Subtle)]),
    });
}

internal sealed class ConnectBox : UserControl
{
    private readonly TextBox _address;
    private readonly TextBlock _refusal = Kit.Refusal();
    private readonly Button _connect;
    private readonly Action _connected;

    public ConnectBox(string address, string label, bool intro, Action connected)
    {
        _connected = connected;
        _address = FormLook.Field(address, "Device address", "192.168.1.100 or falcon.local");
        AutomationProperties.SetName(_address, "Device address");
        _address.IsSpellCheckEnabled = false;
        _address.KeyDown += (_, e) =>
        {
            if (e.Key != VirtualKey.Enter) return;
            e.Handled = true;
            Connect();
        };
        _connect = FormLook.Button(label, Connect, ButtonLook.Accent);
        _connect.HorizontalAlignment = HorizontalAlignment.Left;
        Content = FormLook.Margined(FormLook.Stack(12,
            intro ? FormLook.Text("Connect to an Aircast device", "FontSizeSubtitle", "TextPrimaryBrush", true) : null,
            intro ? FormLook.Text("Type the address of a device on this network. Its status, cameras and settings open here.", "FontSizeBody", "TextSecondaryBrush") : null,
            _address,
            _refusal,
            _connect), intro ? new Thickness(0, 16, 0, 16) : new Thickness(0));
    }

    private async void Connect()
    {
        if (!_connect.IsEnabled) return;
        _connect.IsEnabled = false;
        FormLook.Say(_refusal, null);
        var refused = await DeviceLink.Connect(_address.Text);
        _connect.IsEnabled = true;
        FormLook.Say(_refusal, refused);
        if (refused is null) _connected();
    }
}
