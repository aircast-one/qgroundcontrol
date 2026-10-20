using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Documents;

namespace AircastQGC.Pages.Device;

internal sealed class NetworkScreen : DeviceScreen
{
    private static readonly IReadOnlyDictionary<string, string> ConnectionFieldTypes = new Dictionary<string, string>
    {
        ["ssid"] = "wifi",
        ["psk"] = "wifi",
        ["apn"] = "gsm",
        ["pin"] = "gsm",
    };

    public NetworkScreen(DeviceNav nav) : base("network", nav) { }

    protected override IEnumerable<Node?>? Build(DeviceReading reading, DeviceContext ctx) =>
        Viewed(reading, view => view["uplink"] is JsonObject, view =>
        {
            var wifi = DJ.Section(view, "wifi");
            return new Node?[]
            {
                Uplink(ctx, DJ.Section(view, "uplink")),
                DJ.Obj(view, "modem") is { } modem ? Modem(ctx, modem) : null,
                Starlink(ctx, DJ.Section(view, "starlink")),
                Kit.Section("Wi-Fi", DJ.Badge(wifi, "status"), [
                    Kit.Text(DJ.Text(wifi, "summary")),
                    Kit.Notice(DJ.MaybeBadge(wifi, "joined")),
                    WifiJoin(DJ.Section(wifi, "reopen"), false),
                ]),
                Interfaces(ctx, DJ.Section(view, "interfaces")),
                Kit.Hint("The device routes through the best uplink that reaches the internet and moves off one that stops — this is where you say what it may use."),
                Data(ctx, DJ.Section(view, "data")),
                Kit.Hint("Mobile data is metered; the budget here is what keeps a flight from spending a month of it — set it before a SIM goes in."),
                Tailscale(ctx, DJ.Obj(view, "tailscale") ?? new JsonObject()),
                Relay(ctx, DJ.Section(view, "relay")),
                Kit.Hint("Reaching this device from somewhere else — it is not needed to fly."),
            };
        });

    public static Node WifiJoin(JsonObject reopen, bool open) =>
        Kit.Disclosure("Join a Wi-Fi network", "Connect this device to your Wi-Fi. It leaves the recovery hotspot and rejoins on every boot.", null, open, () => [
            DeviceTree.Kept("wifi-wizard", () => new WifiWizardBox(), box => box.Show(DJ.Text(reopen, "host"), DJ.Text(reopen, "hostname"))),
        ]);

    private static IEnumerable<Node?> Problem(JsonObject? problem) => problem is null ? [] :
    [
        Kit.Notice(new Badge(DeviceLevel.Blocked, DJ.Text(problem, "text"))),
        DJ.Text(problem, "detail") is { Length: > 0 } detail ? Kit.Disclosure("Technical detail", "", null, false, () => [Kit.Mono(detail)]) : null,
    ];

    private static string UplinkGlyph(string icon) => icon switch
    {
        "satellite-dish" => "",
        "signal" => "",
        "wifi" => "",
        "ethernet-port" => "",
        _ => "",
    };

    private static Node Uplink(DeviceContext ctx, JsonObject uplink) => Kit.Section("Primary uplink", DJ.Badge(uplink, "status"), new Node?[]
        {
            DeviceTree.Leaf("uplink", DJ.Sig(DJ.Text(uplink, "icon"), DJ.Text(uplink, "title"), DJ.Obj(uplink, "verdict"), DJ.Text(uplink, "detail"), DJ.Text(uplink, "addr"), DJ.Text(uplink, "meta"), DJ.Text(uplink, "rate")), () =>
            {
                var verdict = DJ.Badge(uplink, "verdict");
                var words = FormLook.Stack(4,
                    FormLook.Text(DJ.Text(uplink, "title"), "FontSizeBodyLarge", "TextPrimaryBrush", true),
                    FormLook.Text(verdict.Text, "FontSizeBody", Kit.LevelBrush(verdict.Level)),
                    FormLook.Text(DJ.Text(uplink, "detail"), "FontSizeBody", "TextSecondaryBrush"),
                    FormLook.Caption(string.Join(" · ", new[] { DJ.Text(uplink, "addr"), DJ.Text(uplink, "meta") }.Where(t => t.Length > 0))));
                var rate = DJ.Text(uplink, "rate");
                var sending = FormLook.Stack(2, FormLook.Caption("Sending now"), Kit.MonoText(rate, "FontSizeBodyLarge"));
                sending.Visibility = FormLook.Shown(rate.Length > 0);
                var icon = FormLook.Glyph(UplinkGlyph(DJ.Text(uplink, "icon")), 20);
                icon.VerticalAlignment = VerticalAlignment.Top;
                Grid.SetColumn(words, 1);
                Grid.SetColumn(sending, 2);
                return new Grid
                {
                    Margin = Kit.Pad,
                    ColumnSpacing = 12,
                    ColumnDefinitions = { new ColumnDefinition { Width = GridLength.Auto }, new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto } },
                    Children = { icon, words, sending },
                };
            }),
            Kit.Notice(new Badge(DeviceLevel.Warn, DJ.Text(uplink, "warning"))),
            DJ.Text(uplink, "warning").Length > 0 ? Kit.Buttons("check-camera", [Kit.Button("camera", "Check the camera →", () => ctx.Open("/streaming"), ButtonLook.Subtle)]) : null,
        }
        .Concat(DJ.Objs(uplink, "failover").Select(row => (Node?)Kit.Fact(DJ.Text(row, "label"), DJ.Badge(row, "status").Text, DJ.Badge(row, "status").Level)))
        .Append(Kit.Text(DJ.Text(uplink, "switch")))
        .Append(Kit.Text(DJ.Text(uplink, "backup"))));

    private static Node Modem(DeviceContext ctx, JsonObject modem)
    {
        var messages = DJ.Section(modem, "messages");
        var items = DJ.Objs(messages, "items");
        var sim = DJ.Obj(modem, "sim");
        return Kit.Section("Cellular modem", DJ.Badge(modem, "status"), DJ.Badges(modem, "notices").Select(Kit.Notice)
            .Append(Kit.StackedFact(DJ.Text(modem, "operator"), DJ.Text(modem, "detail")))
            .Append(DJ.Chart(DJ.Obj(modem, "signal")) is { } signal ? Kit.Chart(signal) : null)
            .Concat(DJ.Objs(modem, "facts").Select(fact => (Node?)Kit.Fact(DJ.Text(fact, "label"), DJ.Text(fact, "value"))))
            .Append(Kit.Disclosure("Text messages", DJ.Text(messages, "summary"), null, false, () => items.Count == 0
                ? [Kit.Text("Messages the operator sends to this SIM — package expiry, top-ups, portal codes — show up here.")]
                : items.Select(item => (Node?)DeviceTree.Leaf($"sms:{DJ.Text(item, "when")}|{DJ.Text(item, "text")}", "", () => Kit.Padded(FormLook.Stack(2,
                    FormLook.Row(FormLook.Text(DJ.Text(item, "from")), FormLook.Caption(DJ.Text(item, "when"))),
                    FormLook.Text(DJ.Text(item, "text"), "FontSizeBody", "TextSecondaryBrush")))))))
            .Append(sim is null ? null : Kit.Disclosure("SIM & APN", DJ.Text(sim, "summary"), null, false, () => [
                Kit.Form(ctx, "sim", [
                        new Field("apn", "APN", DJ.Text(sim, "apn"), DJ.Text(sim, "placeholder")),
                        new Field("pin", "SIM PIN", "", Hint: "Leave blank to keep the current PIN.", Kind: FieldKind.Password),
                        new Field("metered", "Data usage", DJ.Text(sim, "metered"), Options: DJ.Options(sim, "options")),
                    ],
                    DeviceLink.Put, "/api/network/cellular", "Save", "Cellular settings applied.",
                    new Confirm("Re-dial the modem?", "Saving re-dials the modem — the cellular link drops briefly. If you're connected over cellular you may lose access until it reconnects. Continue?", "Save"),
                    "Re-dials the modem on save."),
            ])));
    }

    private static DeviceRequest Demo(string condition) => new(DeviceLink.Post, "/api/starlink/demo", DeviceLink.Body(("condition", condition)));

    private static Node Starlink(DeviceContext ctx, JsonObject starlink)
    {
        var demo = DJ.Obj(starlink, "demo");
        var address = DJ.Section(starlink, "address");
        var fallback = DJ.Text(address, "default");
        var defaultFirst = DJ.Flag(address, "defaultFirst");
        Node? UseDefault() => fallback.Length == 0 ? null : Kit.Buttons("default", [
            Kit.Action(ctx, $"Use default ({fallback})", new DeviceRequest(DeviceLink.Put, "/api/starlink/config", DeviceLink.Body(("addr", fallback))), "Dish address back to the default."),
        ]);
        return Kit.Section("Starlink", DJ.Badge(starlink, "status"), new[] { Kit.Text(DJ.Text(starlink, "summary")) }
            .Concat(demo is not null && DJ.Flag(demo, "first") ? DemoDish(ctx, demo) : [])
            .Concat(DJ.Badges(starlink, "notices").Select(Kit.Notice))
            .Append(Kit.Mono(DJ.Text(starlink, "pollError")))
            .Concat(DJ.Objs(starlink, "facts").Select(fact => (Node?)Kit.Fact(DJ.Text(fact, "label"), DJ.Text(fact, "value"))))
            .Append(DJ.Text(starlink, "firmware") is { Length: > 0 } firmware ? Kit.Fact("Firmware", firmware) : null)
            .Append(Kit.Disclosure("Dish address", "", null, DJ.Flag(address, "open"), () => new[]
            {
                Kit.Text("The network address of the Starlink dish. Leave the default unless the dish is reached at a different address on this device's network. Port 9200 is assumed if you leave it off."),
                defaultFirst ? UseDefault() : null,
                Kit.Form(ctx, "dish", [new Field("addr", "Dish address", DJ.Text(address, "value"), "192.168.100.1:9200", Kind: FieldKind.Uri)],
                    DeviceLink.Put, "/api/starlink/config", "Save", "Dish address saved."),
                defaultFirst ? null : UseDefault(),
            }))
            .Concat(demo is not null && !DJ.Flag(demo, "first") ? DemoDish(ctx, demo) : []));
    }

    private static IEnumerable<Node?> DemoDish(DeviceContext ctx, JsonObject demo) =>
    [
        Kit.Switch(ctx, "Demo dish", DJ.Flag(demo, "on"), Demo("healthy"), Demo("off"), DJ.Text(demo, "note"), "Demo dish on — simulated, not a real dish.", "Demo dish off."),
        DJ.Flag(demo, "on")
            ? DeviceTree.Leaf("condition", DJ.Sig(DJ.Text(demo, "condition"), demo["conditions"]), () =>
            {
                var refusal = Kit.Refusal();
                ComboBox? select = null;
                select = Kit.Select("Condition", DJ.Text(demo, "condition"), DJ.Options(demo, "conditions"), async condition =>
                {
                    select!.IsEnabled = false;
                    var refused = await DeviceLink.Call(Demo(condition));
                    select.IsEnabled = true;
                    FormLook.Say(refusal, refused?.Message);
                    if (refused is null) ctx.Say("Demo dish condition changed.");
                });
                return Kit.Padded(FormLook.Stack(4, select, refusal));
            })
            : null,
    ];

    private static bool Manual(IReadOnlyDictionary<string, string> values) => values.GetValueOrDefault("method") == "manual";

    private static IReadOnlyList<Field> Ipv4Fields(IReadOnlyList<Option> methods, JsonObject ipv4) =>
    [
        new Field("method", "IPv4 method", DJ.Text(ipv4, "method") is { Length: > 0 } method ? method : methods.FirstOrDefault()?.Value ?? "auto", Options: methods),
        new Field("metric", "Route metric (priority)", DJ.Text(ipv4, "metric"), DJ.Text(ipv4, "metricPlaceholder"), Kind: FieldKind.Number),
        new Field("address", "Address (CIDR)", DJ.Text(ipv4, "address"), "192.168.8.2/24", Kind: FieldKind.Uri, ShownWhen: Manual),
        new Field("gateway", "Gateway", DJ.Text(ipv4, "gateway"), "optional", Kind: FieldKind.Uri, ShownWhen: Manual),
        new Field("dns", "DNS", DJ.Text(ipv4, "dns"), "8.8.8.8, 1.1.1.1", Kind: FieldKind.Uri),
    ];

    private static Field NewConnectionField(string name, string label, string placeholder = "", string hint = "", FieldKind kind = FieldKind.Text) =>
        new(name, label, "", placeholder, hint, kind, ShownWhen: values =>
            !ConnectionFieldTypes.TryGetValue(name, out var type) || values.GetValueOrDefault("type") == type);

    private static Node Rename(DeviceContext ctx, string uuid, string name) =>
        Kit.Form(ctx, $"rename:{uuid}", [new Field("name", "Connection name", name, "e.g. Wired connection 2")],
            DeviceLink.Put, $"/api/network/connections/{uuid}/name", "Rename", "Connection renamed.");

    private static Node Interfaces(DeviceContext ctx, JsonObject interfaces)
    {
        var problem = DJ.Obj(interfaces, "problem");
        var links = DJ.Objs(interfaces, "links");
        var saved = DJ.Objs(interfaces, "saved");
        var other = DJ.Objs(interfaces, "other");
        var methods = DJ.Options(interfaces, "methods");
        var add = DJ.Section(interfaces, "add");
        var types = DJ.Options(add, "types");
        var autoconnect = DJ.Options(add, "autoconnect");
        return Kit.Section("Interfaces & priority", DJ.Badge(interfaces, "status"), new[] { Kit.Text(DJ.Text(interfaces, "summary")) }
            .Concat(Problem(problem))
            .Append(problem is null && links.Count + saved.Count + other.Count == 0 ? Kit.Notice(new Badge(DeviceLevel.Neutral, "No connections.")) : null)
            .Concat(links.Select(link => (Node?)NetLink(ctx, link, methods)))
            .Concat(saved.Select(profile => (Node?)Saved(ctx, profile, methods)))
            .Append(problem is null && links.Count + saved.Count > 0
                ? Kit.Text("The primary uplink carries internet traffic; a backup takes over if it drops. Open a connection to edit it — changes to a running connection apply immediately; inactive ones are saved profiles that aren't running.")
                : null)
            .Append(other.Count == 0 ? null : Kit.Sub("Other interfaces"))
            .Concat(other.Select(usage => Kit.StackedFact(DJ.Text(usage, "label"), $"{DJ.Text(usage, "month")} · all-time {DJ.Text(usage, "total")}")))
            .Append(Kit.Disclosure("Add a connection", "Create a new NetworkManager profile — ethernet, Wi-Fi or cellular.", null, false, () => [
                Kit.Form(ctx, "add-connection", new[]
                    {
                        new Field("type", "Type", types.FirstOrDefault()?.Value ?? "ethernet", Options: types),
                        new Field("name", "Name", "", "camera-net"),
                        new Field("interface", "Interface", "", Options: DJ.Options(add, "ports").Prepend(new Option("", "Any interface")).ToList()),
                        new Field("autoconnect", "Autoconnect", autoconnect.FirstOrDefault()?.Value ?? "false", Options: autoconnect),
                        NewConnectionField("ssid", "SSID", "network name"),
                        NewConnectionField("psk", "Passphrase", hint: "Leave blank for an open network.", kind: FieldKind.Password),
                        NewConnectionField("apn", "APN", "internet"),
                        NewConnectionField("pin", "SIM PIN", "optional", kind: FieldKind.Password),
                    }.Concat(Ipv4Fields(methods, new JsonObject { ["metricPlaceholder"] = "automatic" })).ToList(),
                    DeviceLink.Post, "/api/network/connections", "Create", "Connection created.",
                    note: "Created inactive unless autoconnect is on — it won't disturb the current uplink."),
            ])));
    }

    private static string LinkSummary(JsonObject link) =>
        string.Join(" · ", new[]
        {
            DJ.Text(link, "kind"),
            DJ.MaybeBadge(link, "role")?.Text ?? (DJ.Flag(link, "local") ? "local" : ""),
            DJ.Text(link, "where"),
            DJ.Text(link, "usage"),
        }.Where(text => text.Length > 0));

    private static Node NetLink(DeviceContext ctx, JsonObject link, IReadOnlyList<Option> methods)
    {
        var name = DJ.Text(link, "name");
        if (DJ.Obj(link, "editor") is not { } editor) return Kit.StackedFact(name, LinkSummary(link))!;
        var iface = DJ.Text(editor, "interface");
        var uuid = DJ.Text(editor, "uuid");
        var confirm = DJ.Text(editor, "confirm");
        return Kit.Disclosure(name, LinkSummary(link), null, false, () => new Node?[]
        {
            Kit.Form(ctx, $"ipv4:{iface}", Ipv4Fields(methods, DJ.Section(editor, "ipv4")), DeviceLink.Put, $"/api/network/interfaces/{iface}", "Save", $"{iface} updated.",
                confirm.Length > 0 ? new Confirm($"Reconfigure {iface}?", confirm, "Continue") : null, "Applies immediately."),
        }.Concat(uuid.Length == 0 ? [] : new Node?[]
        {
            Rename(ctx, uuid, name),
            Kit.Text("Bring this connection down, or remove the saved profile."),
            Kit.Buttons("manage", [
                Kit.Action(ctx, "Deactivate", new DeviceRequest(DeviceLink.Post, $"/api/network/connections/{uuid}/deactivate"), $"{name} deactivated.",
                    new Confirm($"Deactivate {name}?", DJ.Text(editor, "deactivateConfirm"), "Deactivate")),
                Kit.Action(ctx, "Delete", new DeviceRequest(DeviceLink.Delete, $"/api/network/connections/{uuid}"), $"{name} deleted.",
                    new Confirm($"Delete {name}?", DJ.Text(editor, "deleteConfirm"), "Delete"), tonal: false),
            ]),
        }));
    }

    private static Node Saved(DeviceContext ctx, JsonObject profile, IReadOnlyList<Option> methods)
    {
        var uuid = DJ.Text(profile, "uuid");
        var name = DJ.Text(profile, "name");
        return Kit.Disclosure(name, $"{DJ.Text(profile, "kind")} · inactive", null, false, () => [
            Rename(ctx, uuid, name),
            Kit.Form(ctx, $"saved-ipv4:{uuid}", Ipv4Fields(methods, DJ.Section(profile, "ipv4")), DeviceLink.Put, $"/api/network/connections/{uuid}/ipv4", "Save", $"{name} updated.",
                note: "Saved to the profile (not activated)."),
            Kit.Buttons("manage", [
                Kit.Action(ctx, "Activate", new DeviceRequest(DeviceLink.Post, $"/api/network/connections/{uuid}/activate"), $"{name} activated.",
                    new Confirm($"Activate {name}?", DJ.Text(profile, "activateConfirm"), "Activate")),
                Kit.Action(ctx, "Delete", new DeviceRequest(DeviceLink.Delete, $"/api/network/connections/{uuid}"), $"{name} deleted.",
                    new Confirm($"Delete {name}?", DJ.Text(profile, "deleteConfirm"), "Delete"), tonal: false),
            ]),
        ]);
    }

    private static Node Data(DeviceContext ctx, JsonObject data)
    {
        var budget = DJ.Section(data, "budget");
        return Kit.Section("Cellular data", DJ.Badge(data, "status"), [
            Kit.Text(DJ.Text(data, "summary")),
            DJ.Obj(data, "gauge") is { } gauge
                ? DeviceTree.Leaf("gauge", DJ.Sig(gauge), () => Gauge(gauge))
                : Kit.Notice(new Badge(DeviceLevel.Neutral, "No usage recorded yet — counters accumulate as data flows.")),
            Kit.Disclosure("Data caps & warnings", DJ.Text(budget, "summary"), null, false, () => [
                Kit.Form(ctx, "budget", [
                        new Field("monthlyCapGB", "Monthly plan (GB)", DJ.Text(budget, "monthlyCapGB"), "e.g. 10 — blank for none", Kind: FieldKind.Number),
                        new Field("flightCapMB", "Per-flight budget (MB)", DJ.Text(budget, "flightCapMB"), "e.g. 800 — blank for none", Kind: FieldKind.Number),
                        new Field("warnPercent", "Warn at (%)", DJ.Text(budget, "warnPercent"), "80", Kind: FieldKind.Number),
                    ],
                    DeviceLink.Put, "/api/usage/budget", "Save", "Data caps saved.", note: "Warnings alert the ground station; caps never cut the link."),
            ]),
        ]);
    }

    private static ProgressBar Bar(int percent, string brush) => new()
    {
        Minimum = 0,
        Maximum = 100,
        Value = Math.Clamp(percent, 0, 100),
        Height = 8,
        MinHeight = 8,
        Foreground = FormLook.Brush(brush),
        HorizontalAlignment = HorizontalAlignment.Stretch,
    };

    private static UIElement Gauge(JsonObject gauge)
    {
        var badges = FormLook.Line(8, DJ.Badges(gauge, "badges").Select(Kit.BadgeView).ToArray());
        var hero = new TextBlock { FontFamily = Controls.PreflightLook.Font, TextWrapping = TextWrapping.Wrap };
        hero.Inlines.Add(new Run { Text = DJ.Text(gauge, "hero"), FontSize = FormLook.Size("FontSizeSubtitle"), Foreground = FormLook.Brush("TextPrimaryBrush") });
        hero.Inlines.Add(new Run { Text = $"  {DJ.Text(gauge, "heroCaption")}", FontSize = FormLook.Size("FontSizeBody"), Foreground = FormLook.Brush("TextSecondaryBrush") });
        var stats = new Grid { ColumnSpacing = 8 };
        DJ.Objs(gauge, "stats").Select((stat, at) => (stat, at)).ToList().ForEach(pair =>
        {
            stats.ColumnDefinitions.Add(new ColumnDefinition());
            var cell = FormLook.Stack(2, FormLook.Caption(DJ.Text(pair.stat, "label")), Kit.MonoText(DJ.Text(pair.stat, "value")), FormLook.Caption(DJ.Text(pair.stat, "note"), "TextTertiaryBrush"));
            Grid.SetColumn(cell, pair.at);
            stats.Children.Add(cell);
        });
        return new Border
        {
            Margin = Kit.Pad,
            Padding = new Thickness(16),
            CornerRadius = FormLook.Radius("RadiusOverlay"),
            Background = FormLook.Brush("ControlBrush"),
            Child = FormLook.Stack(12, new UIElement?[]
            {
                FormLook.Row(FormLook.Caption("Cellular · LTE"), badges),
                hero,
                FormLook.Stack(6, Bar(DJ.Int(gauge, "uplinkPercent"), "AccentBrush"),
                    FormLook.Row(FormLook.Caption(DJ.Text(gauge, "uplink")), FormLook.Caption(DJ.Text(gauge, "downlink")))),
            }.Concat(DJ.Objs(gauge, "meters").Select(meter =>
            {
                var level = DJ.Int(meter, "level");
                return (UIElement?)FormLook.Stack(4,
                    FormLook.Row(FormLook.Caption(DJ.Text(meter, "label")), Kit.MonoText(DJ.Text(meter, "used"), "FontSizeCaption", Kit.LevelTextBrush(level))),
                    Bar(DJ.Int(meter, "percent"), level == DeviceLevel.Neutral ? "AccentBrush" : Kit.LevelBrush(level)),
                    FormLook.Caption(DJ.Text(meter, "remaining"), Kit.LevelBrush(level)),
                    FormLook.Caption(DJ.Text(meter, "note"), "TextTertiaryBrush"));
            })).Append(stats)),
        };
    }

    public static Node Tailscale(DeviceContext ctx, JsonObject tailscale)
    {
        var banner = DJ.Obj(tailscale, "banner");
        var signIn = DJ.Obj(tailscale, "signIn");
        var addresses = DJ.Texts(tailscale, "addresses");
        var connect = DJ.Text(tailscale, "connect");
        var logOut = DJ.Text(tailscale, "logOut");
        var cancel = DJ.Flag(tailscale, "cancel");
        var control = DJ.Obj(tailscale, "control");
        return Kit.Section("Tailscale", DJ.Badge(tailscale, "status"), new[] { Kit.Text(DJ.Text(tailscale, "summary")) }
            .Concat(Problem(DJ.Obj(tailscale, "problem")))
            .Append(banner is null ? null : DeviceTree.Leaf("banner", DJ.Sig(banner), () => new Border
            {
                Margin = Kit.Pad,
                Padding = new Thickness(16, 12, 16, 12),
                CornerRadius = FormLook.Radius("RadiusOverlay"),
                Background = FormLook.Brush("ControlBrush"),
                Child = FormLook.Row(
                    FormLook.Stack(2, FormLook.Text(DJ.Text(banner, "title"), "FontSizeBody", "TextPrimaryBrush", true), FormLook.Text(DJ.Text(banner, "subtitle"), "FontSizeBody", "TextSecondaryBrush")),
                    Kit.BadgeView(DJ.Badge(banner, "status"))),
            }))
            .Concat(DJ.Objs(tailscale, "facts").Select(fact => (Node?)Kit.Fact(DJ.Text(fact, "label"), DJ.Text(fact, "value"))))
            .Append(addresses.Count > 0 ? Kit.Fact("This device's addresses", string.Join(", ", addresses)) : null)
            .Append(signIn is null ? null : Kit.Group("sign-in", [
                Kit.Buttons("sign-in", [Kit.Link(ctx, DJ.Text(signIn, "href"), DJ.Text(signIn, "label"), external: true)]),
                Kit.Mono(DJ.Text(signIn, "href")),
            ]))
            .Append(connect.Length > 0 || cancel || logOut.Length > 0 ? Kit.Buttons("tailscale", [
                connect.Length > 0 ? Kit.Action(ctx, connect, new DeviceRequest(DeviceLink.Post, "/api/tailscale/connection")) : null,
                cancel ? Kit.Action(ctx, "Cancel sign-in", new DeviceRequest(DeviceLink.Post, "/api/tailscale/connection/cancel"), "Sign-in cancelled.", tonal: false) : null,
                logOut.Length > 0 ? Kit.Action(ctx, "Log out", new DeviceRequest(DeviceLink.Delete, "/api/tailscale/connection"), "Logged out of Tailscale.",
                    new Confirm("Log out of the tailnet?", logOut, "Log out")) : null,
            ]) : null)
            .Append(Kit.Text(DJ.Text(tailscale, "hint")))
            .Append(control is null ? null : Control(ctx, control)));
    }

    private static Node Control(DeviceContext ctx, JsonObject control)
    {
        var authKeySet = DJ.Flag(control, "authKeySet");
        var switchTo = DJ.Obj(control, "switch");
        var logOutToSwitch = DJ.Obj(control, "logOutToSwitch");
        var error = DJ.Text(control, "switchError");
        return Kit.Disclosure(DJ.Text(control, "title"), "", null, DJ.Flag(control, "open"), () => new Node?[]
        {
            Kit.Form(ctx, $"control:{authKeySet}", [
                    new Field("controlServer", "Control server URL", DJ.Text(control, "server"), "https://headscale.example.com", Kind: FieldKind.Uri),
                    new Field("authKey", "Auth key — device joins automatically, no sign-in", "", DJ.Text(control, "authKeyPlaceholder"), Kind: FieldKind.Password),
                ],
                DeviceLink.Put, "/api/tailscale/control", "Save", "Control server saved."),
            authKeySet ? Kit.Buttons("authkey", [Kit.Action(ctx, "Remove auth key", new DeviceRequest(DeviceLink.Delete, "/api/tailscale/control/authkey"), "Auth key removed.",
                new Confirm("Remove the auth key?", "Remove the saved auth key? Switching will then need an interactive sign-in over the LAN.", "Remove"), tonal: false)]) : null,
            error.Length > 0 ? Kit.Notice(new Badge(DeviceLevel.Blocked, $"Last switch failed (rolled back): {error}")) : null,
            Kit.Text(DJ.Text(control, "pending")),
            switchTo is null && logOutToSwitch is null ? null : Kit.Buttons("switch", [
                switchTo is null ? null : Kit.Action(ctx, DJ.Text(switchTo, "label"), new DeviceRequest(DeviceLink.Post, "/api/tailscale/switch"),
                    "Switching… if it fails it rolls back automatically.", new Confirm($"{DJ.Text(switchTo, "label")}?", DJ.Text(switchTo, "confirm"), "Switch")),
                logOutToSwitch is null ? null : Kit.Action(ctx, DJ.Text(logOutToSwitch, "label"), new DeviceRequest(DeviceLink.Delete, "/api/tailscale/connection"),
                    "Logged out — now connect to finish switching.", new Confirm("Log out to switch?", DJ.Text(logOutToSwitch, "confirm"), "Log out")),
            ]),
            Kit.Text(DJ.Text(control, "hint")),
        });
    }

    private static DeviceRequest RelayMode(bool relay) => new(DeviceLink.Put, "/api/stream/webrtc-mode", DeviceLink.Body(("relay", relay ? "true" : "false")));

    private static Node Relay(DeviceContext ctx, JsonObject relay)
    {
        var configured = DJ.Flag(relay, "configured");
        return Kit.Section("Relay server (TURN)", DJ.Badge(relay, "status"), [
            Kit.Text(DJ.Text(relay, "summary")),
            Kit.Switch(ctx, "Use my relay instead of Tailscale", configured && DJ.Flag(relay, "on"), RelayMode(true), RelayMode(false),
                configured ? "" : DJ.Text(relay, "reason"), "Remote video now goes through your relay.", "Remote video goes over Tailscale again.", configured),
            Kit.Notice(DJ.Badge(relay, "notice")),
            DeviceTree.Leaf("ice", DJ.Sig(relay["servers"]), () => new IceEditorBox(ctx, DJ.Objs(relay, "servers"))),
            Kit.Disclosure("Technical details", "", null, false, () => [Kit.Text(
                "Off (default) advertises the tailscale 100.x and LAN host candidates. On stops advertising the tailscale host candidate and routes WebRTC media through the configured STUN/TURN ICE servers over UDP, so symmetric-CGNAT sessions on cellular relay over UDP instead of falling back to Tailscale DERP (TCP). Same-LAN viewers still use the direct LAN candidate.")]),
        ]);
    }
}

internal sealed record IceServer(string Url = "", string Username = "", string Password = "")
{
    public JsonObject Json => new() { ["url"] = Url, ["username"] = Username, ["password"] = Password };
}

internal sealed class IceEditorBox : UserControl
{
    private const string GoogleStun = "stun:stun.l.google.com:19302";
    private const string IceServersPath = "deviceNetwork.iceServers";

    private readonly DeviceContext _ctx;
    private readonly StackPanel _rows = new() { Spacing = 8 };
    private readonly TextBlock _refusal = Kit.Refusal();
    private readonly Button _save;
    private ImmutableList<IceServer> _servers;
    private int _generation;

    public IceEditorBox(DeviceContext ctx, IReadOnlyList<JsonObject> servers)
    {
        _ctx = ctx;
        _servers = servers.Select(s => new IceServer(DJ.Text(s, "url"), DJ.Text(s, "username"), DJ.Text(s, "password"))).ToImmutableList();
        if (_servers.IsEmpty) _servers = [new IceServer()];
        _save = FormLook.Button("Save servers", Save);
        _save.HorizontalAlignment = HorizontalAlignment.Left;
        Content = Kit.Padded(FormLook.Stack(12,
            FormLook.Text("Relay (TURN / STUN) servers"),
            _rows,
            FormLook.Line(8,
                FormLook.Button("+ Add server", () => Edit(_servers.Add(new IceServer())), ButtonLook.Subtle),
                FormLook.Button("+ Add Google STUN", () => Edit(_servers.Add(new IceServer(GoogleStun))), ButtonLook.Subtle)),
            _refusal,
            _save,
            FormLook.Caption("A TURN server relays video when a direct connection isn't possible — use one on UDP, ideally port 443. STUN-only lines need no username or password.", "TextTertiaryBrush")));
        Paint();
    }

    private void Edit(ImmutableList<IceServer> servers)
    {
        _servers = servers;
        _generation++;
        Paint();
    }

    private void Paint() => FormLook.Fill(_rows, _servers.Select((server, at) => Row(server, at)));

    private UIElement Row(IceServer server, int at)
    {
        var url = FormLook.Field(server.Url, "Server URL", "turn:turn.example.com:443?transport=udp");
        var user = FormLook.Field(server.Username, "Username", "username");
        var password = new PasswordBox { Header = "Password", PlaceholderText = "password", Password = server.Password };
        AutomationProperties.SetName(url, "Server URL");
        AutomationProperties.SetName(user, "Username");
        AutomationProperties.SetName(password, "Password");
        url.TextChanged += (_, _) => _servers = _servers.SetItem(at, _servers[at] with { Url = url.Text });
        user.TextChanged += (_, _) => _servers = _servers.SetItem(at, _servers[at] with { Username = user.Text });
        password.PasswordChanged += (_, _) => _servers = _servers.SetItem(at, _servers[at] with { Password = password.Password });
        var remove = FormLook.IconButton("", "Remove server", () => Edit(_servers.RemoveAt(at)));
        remove.VerticalAlignment = VerticalAlignment.Bottom;
        Grid.SetColumn(remove, 1);
        return new Border
        {
            Background = FormLook.Brush("ControlBrush"),
            CornerRadius = FormLook.Radius("RadiusOverlay"),
            Padding = new Thickness(12),
            Child = FormLook.Stack(8,
                new Grid { ColumnSpacing = 8, ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto } }, Children = { url, remove } },
                user,
                password),
        };
    }

    private async void Save()
    {
        _save.IsEnabled = false;
        FormLook.Say(_refusal, null);
        var lines = await DeviceLink.Result(IceServersPath, new JsonArray(_servers.Select(s => (JsonNode?)s.Json).ToArray()));
        var refused = lines is JsonValue value && value.TryGetValue<string>(out var text)
            ? await DeviceLink.Call(new DeviceRequest(DeviceLink.Put, "/api/stream/webrtc-ice-servers", DeviceLink.Body(("servers", text))))
            : DeviceLink.NoAnswer;
        _save.IsEnabled = true;
        FormLook.Say(_refusal, refused?.Message);
        if (refused is null) _ctx.Say("Servers saved.");
    }
}

internal sealed record JoinAttempt(string Target, string Status, bool Instructions);

internal sealed class WifiWizardBox : UserControl
{
    private const string ScanPath = "/api/network/wifi/scan";

    private readonly StackPanel _root = new() { Padding = Kit.Pad, Spacing = 12 };
    private readonly TextBox _ssid = FormLook.Field("", "Network name (SSID)", "your Wi-Fi name");
    private readonly PasswordBox _psk = new() { Header = "Password", Description = "Leave blank for an open network." };
    private readonly TextBlock _problem = Kit.Refusal();
    private readonly Button _scan;
    private readonly StackPanel _networks = new() { Spacing = 6 };
    private readonly TextBlock _scanSaid = FormLook.Caption("", "TextTertiaryBrush");
    private string _host = "";
    private string _hostname = "";
    private string _pickedSecured = "";
    private int _attempt;
    private JoinAttempt? _joining;

    public WifiWizardBox()
    {
        AutomationProperties.SetName(_ssid, "Network name (SSID)");
        AutomationProperties.SetName(_psk, "Password");
        _ssid.IsSpellCheckEnabled = false;
        _ssid.KeyDown += (_, e) =>
        {
            if (e.Key != Windows.System.VirtualKey.Enter) return;
            e.Handled = true;
            Connect();
        };
        _scan = FormLook.Button("Scan", Scan);
        Content = _root;
    }

    public void Show(string host, string hostname)
    {
        if (_root.Children.Count > 0 && (host, hostname) == (_host, _hostname)) return;
        (_host, _hostname) = (host, hostname);
        Paint();
    }

    private void Paint()
    {
        if (_joining is { } joining)
        {
            FormLook.Fill(_root, Joining(joining));
            return;
        }
        var connect = FormLook.Button("Connect", Connect);
        FormLook.Fill(_root, new UIElement?[]
        {
            Kit.Split(FormLook.Caption("Pick a nearby network or type its name. Scanning is limited while the hotspot is on, so typing is usually fastest.", "TextTertiaryBrush"), _scan),
            _scanSaid,
            _networks,
            _ssid,
            _psk,
            _problem,
            Kit.Split(FormLook.Caption("Saved with autoconnect, ahead of the hotspot.", "TextTertiaryBrush"), connect),
        });
    }

    private IEnumerable<UIElement?> Joining(JoinAttempt joining)
    {
        var join = new TextBlock { FontFamily = Controls.PreflightLook.Font, FontSize = FormLook.Size("FontSizeBody"), Foreground = FormLook.Brush("TextSecondaryBrush"), TextWrapping = TextWrapping.Wrap };
        join.Inlines.Add(new Run { Text = "1. On your phone or laptop, join " });
        join.Inlines.Add(new Run { Text = joining.Target, FontWeight = Microsoft.UI.Text.FontWeights.SemiBold, Foreground = FormLook.Brush("TextPrimaryBrush") });
        join.Inlines.Add(new Run { Text = "." });
        var back = FormLook.Button("← Back", Back);
        back.HorizontalAlignment = HorizontalAlignment.Left;
        return new UIElement?[]
        {
            FormLook.Text(joining.Status, "FontSizeBodyLarge"),
            joining.Instructions
                ? new Border
                {
                    Padding = new Thickness(16, 12, 16, 12),
                    CornerRadius = FormLook.Radius("RadiusOverlay"),
                    Background = FormLook.Brush("ControlBrush"),
                    Child = FormLook.Stack(8,
                        FormLook.Text("The recovery hotspot is turning off. To reach the device again:", "FontSizeBody", "TextSecondaryBrush"),
                        join,
                        FormLook.Text("2. Reconnect this app to the device on that network.", "FontSizeBody", "TextSecondaryBrush"),
                        new ConnectBox(_host, "Reconnect", false, Back),
                        _host.Length > 0
                            ? FormLook.Caption($"Doesn't connect? Windows can't always reach .local names — find {_hostname} in your router's list of connected devices and use its address instead.", "TextTertiaryBrush")
                            : null,
                        FormLook.Caption("The app reaches the device over the link that's going away, so it can't follow it on its own.", "TextTertiaryBrush")),
                }
                : null,
            back,
        };
    }

    private void Back()
    {
        _joining = null;
        _attempt++;
        Paint();
    }

    private async void Scan()
    {
        _scan.IsEnabled = false;
        _scan.Content = "Scanning…";
        var found = await DeviceLink.Read(ScanPath);
        _scan.IsEnabled = true;
        _scan.Content = "Scan";
        var networks = found is null ? null : DJ.Objs(found, "networks");
        FormLook.Say(_scanSaid, networks is null ? "Scan failed — type your network name below."
            : networks.Count == 0 ? "No networks found — type your network name below." : null);
        FormLook.Fill(_networks, (networks ?? []).Select(Network));
    }

    private UIElement Network(JsonObject network)
    {
        var ssid = DJ.Text(network, "ssid");
        var secured = DJ.Flag(network, "secured");
        var name = FormLook.Text(ssid);
        name.TextWrapping = TextWrapping.NoWrap;
        name.TextTrimming = TextTrimming.CharacterEllipsis;
        var line = new Grid
        {
            Padding = new Thickness(16, 0, 16, 0),
            MinHeight = 48,
            ColumnSpacing = 12,
            ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto }, new ColumnDefinition { Width = GridLength.Auto } },
        };
        var words = FormLook.Line(8, name, DJ.Flag(network, "active") ? FormLook.Caption("connected", "SuccessBrush") : null);
        var signal = FormLook.Caption($"{DJ.Int(network, "signal")}%");
        var lockGlyph = FormLook.Glyph("", 14);
        lockGlyph.Visibility = FormLook.Shown(secured);
        AutomationProperties.SetName(lockGlyph, "secured");
        new FrameworkElement[] { words, signal, lockGlyph }.Select((child, at) => (child, at)).ToList().ForEach(pair =>
        {
            pair.child.VerticalAlignment = VerticalAlignment.Center;
            Grid.SetColumn(pair.child, pair.at);
            line.Children.Add(pair.child);
        });
        var button = FormLook.Stretched(line, ssid, () =>
        {
            _ssid.Text = ssid;
            _psk.Password = "";
            _pickedSecured = secured ? ssid : "";
        });
        button.BorderBrush = FormLook.Brush("ControlStrokeBrush");
        button.BorderThickness = new Thickness(1);
        button.CornerRadius = FormLook.Radius("RadiusOverlay");
        return button;
    }

    private DeviceRefusal? Problem(string ssid, string psk)
    {
        var name = ssid.Trim();
        return name.Length == 0 ? new DeviceRefusal("Enter your network name first.", "ssid")
            : psk.Length == 0 && _pickedSecured.Length > 0 && _pickedSecured == name ? new DeviceRefusal($"“{name}” is a secured network — enter its password.", "psk")
            : psk.Length > 0 && (psk.Length < 8 || psk.Length > 63) ? new DeviceRefusal("Password must be 8–63 characters, or blank for an open network.", "psk")
            : null;
    }

    private async void Connect()
    {
        var refused = Problem(_ssid.Text, _psk.Password);
        FormLook.Say(_problem, refused?.Message);
        if (refused is not null) return;
        var target = _ssid.Text.Trim();
        var attempt = ++_attempt;
        _joining = new JoinAttempt(target, $"Switching to “{target}”…", true);
        Paint();
        var refusal = await DeviceLink.Call(new DeviceRequest(DeviceLink.Post, "/api/network/wifi", DeviceLink.Body(("ssid", target), ("psk", _psk.Password))));
        if (attempt != _attempt || _joining is not { } joining) return;
        _joining = refusal is null ? joining with { Status = $"✓ Connected to “{target}” — it's now the active link." }
            : refusal == DeviceLink.NoAnswer ? joining with { Status = "Lost contact with the device — it may be switching networks. If it doesn't come back, reconnect using the steps below." }
            : joining with { Instructions = false, Status = $"✗ {(refusal.Message.Length > 0 ? refusal.Message : "Couldn't join that network.")}" };
        Paint();
    }
}
