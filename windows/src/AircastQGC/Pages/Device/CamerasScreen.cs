using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Pages.Device;

internal sealed class CamerasScreen : DeviceScreen
{
    private string _picked;
    private bool _adding;
    private ImmutableDictionary<string, bool>? _seen;

    public CamerasScreen(DeviceNav nav) : base("cameras", nav)
    {
        _picked = nav.Query.GetValueOrDefault("camera", "");
        _adding = nav.Query.ContainsKey("add");
    }

    protected override IEnumerable<Node?>? Build(DeviceReading reading, DeviceContext ctx) =>
        Viewed(reading, view => view["add"] is JsonObject, view =>
        {
            var cameras = DJ.Objs(view, "cameras").Where(camera => camera["wizard"] is JsonObject).ToList();
            Announce(ctx, cameras);
            _adding |= cameras.Count == 0;
            var named = cameras.FirstOrDefault(camera => DJ.Text(camera, "name") == _picked);
            var shown = _adding ? null : named ?? (cameras.Count == 1 ? cameras[0] : null);
            var wall = !_adding && shown is null;
            var cloudLink = DJ.Text(view, "cloudLink");
            var video = DJ.Page(DJ.Obj(view, "video"));
            return new Node?[]
            {
                cameras.Count == 0 ? null : Kit.Buttons("tabs", cameras.Select(camera =>
                {
                    var name = DJ.Text(camera, "name");
                    var status = DJ.Badge(camera, "status");
                    return (Node?)ConnectScreen.Chip($"tab:{name}", status.Text.Length > 0 ? $"{name} · {status.Text}" : name, camera == shown, () => Pick(name));
                })),
                !_adding || cloudLink.Length > 0 ? Kit.Buttons("top", [
                    _adding ? null : Kit.Button("add", "+ Add camera", Add, ButtonLook.Subtle),
                    cloudLink.Length > 0 ? Kit.Link(ctx, cloudLink, "Watch in the cloud", LinkLook.Outlined, true) : null,
                ]) : null,
            }
            .Concat(shown is { } camera
                ? new Node?[]
                {
                    DeviceTree.Kept($"detail:{DJ.Text(camera, "name")}",
                        () => new CameraDetailBox(Nav.Query.ContainsKey("scan") && Nav.Query.GetValueOrDefault("camera") == DJ.Text(camera, "name")),
                        box => box.Show(ctx, camera, () => Pick(DJ.Text(camera, "name")))),
                    Kit.PageRow(ctx, DJ.Page(DJ.Obj(camera, "topic"))),
                    Kit.PageRow(ctx, video),
                }
                : wall
                    ? cameras.Select(tile => (Node?)Kit.Group($"tile:{DJ.Text(tile, "name")}", Tile(ctx, tile))).Append(Kit.PageRow(ctx, video))
                    : [Kit.Section("Add a camera", null, [
                        DeviceTree.Kept("add-wizard", () => new CameraWizardBox(DJ.Section(view, "add"), false, null, _ => { }, Pick), box => box.Show(ctx)),
                    ])]);
        });

    private void Announce(DeviceContext ctx, IReadOnlyList<JsonObject> cameras)
    {
        var live = cameras.ToImmutableDictionary(camera => DJ.Text(camera, "name"), camera => DJ.Flag(camera, "live"));
        var back = _seen is null ? [] : live.Where(kv => kv.Value && _seen.GetValueOrDefault(kv.Key, true) == false).Select(kv => kv.Key).ToList();
        _seen = live;
        if (back.Count > 0) ctx.Say($"{string.Join(", ", back)} {(back.Count == 1 ? "is" : "are")} back — sending video.");
    }

    private void Pick(string name)
    {
        _adding = false;
        _picked = name;
        Rerender();
    }

    private void Add()
    {
        _adding = true;
        Rerender();
    }

    public static Node PictureFrame(DeviceContext ctx, JsonObject camera) =>
        DeviceTree.Kept("picture", () =>
        {
            var held = new StackPanel();
            return new Grid { Margin = Kit.Pad, ColumnDefinitions = { new ColumnDefinition { MaxWidth = 576 } }, Children = { held } };
        }, frame => DeviceTree.Sync((StackPanel)frame.Children[0], [Picture(ctx, camera)]));

    private static Node Picture(DeviceContext ctx, JsonObject camera) =>
        DJ.Text(camera, "preview") is { Length: > 0 } preview
            ? DeviceTree.Kept($"preview:{preview}", () => new PreviewBox(ctx.Url(preview)), box => box.Show(ctx, ctx.Url(DJ.Text(camera, "watch")), DJ.Text(camera, "name")))
            : DeviceTree.Leaf("placeholder", DJ.Text(camera, "placeholder"), () => PreviewBox.Placeholder(DJ.Text(camera, "placeholder")));

    private static IEnumerable<Node?> Tile(DeviceContext ctx, JsonObject camera)
    {
        var name = DJ.Text(camera, "name");
        var diagnosis = DJ.MaybeBadge(camera, "diagnosis");
        return
        [
            DeviceTree.Leaf("head", DJ.Sig(name, DJ.Obj(camera, "status")), () => Kit.Padded(FormLook.Row(
                FormLook.Line(12, FormLook.Text(name, "FontSizeBodyLarge", "TextPrimaryBrush", true), Kit.BadgeView(DJ.Badge(camera, "status"))),
                FormLook.Button("Open", () => ctx.Open($"/streaming?camera={DeviceLink.Escaped(name)}"))))),
            PictureFrame(ctx, camera),
            diagnosis is null ? null : Kit.StatusLine(diagnosis.Text, diagnosis.Level),
            Kit.Switch(ctx, "Record to disk", DJ.Flag(camera, "recording"),
                new DeviceRequest(DeviceLink.Put, RecordingsScreen.RecordPath(name)), new DeviceRequest(DeviceLink.Delete, RecordingsScreen.RecordPath(name)),
                success: $"Recording turned on for {name}.", offSuccess: $"Recording turned off for {name} — download it under Settings → Recordings."),
        ];
    }
}

internal sealed class CameraDetailBox : UserControl
{
    private readonly StackPanel _root = new();
    private bool? _wizard;
    private DeviceContext? _ctx;
    private JsonObject _camera = new();
    private Action _done = () => { };

    public CameraDetailBox(bool scan)
    {
        _wizard = scan ? true : null;
        Content = _root;
    }

    public void Show(DeviceContext ctx, JsonObject camera, Action done)
    {
        (_ctx, _camera, _done) = (ctx, camera, done);
        Paint();
    }

    private void Wizard(bool? scan)
    {
        _wizard = scan;
        Paint();
    }

    private void Paint()
    {
        if (_ctx is not { } ctx) return;
        var name = DJ.Text(_camera, "name");
        DeviceTree.Sync(_root, [_wizard is { } scan
            ? Kit.Section($"Set up {name}", DJ.Badge(_camera, "status"), [
                DeviceTree.Kept($"wizard:{scan}", () => new CameraWizardBox(DJ.Section(_camera, "wizard"), scan, () => Wizard(null), _ => { }, _ =>
                {
                    Wizard(null);
                    _done();
                }), box => box.Show(ctx)),
            ])
            : Overview(ctx)]);
    }

    private Node Overview(DeviceContext ctx)
    {
        var camera = _camera;
        var name = DJ.Text(camera, "name");
        var live = DJ.Flag(camera, "live");
        var fix = DJ.Obj(camera, "fix");
        var diagnosis = DJ.MaybeBadge(camera, "diagnosis");
        var links = DJ.Endpoints(camera, "links");
        var chips = DJ.Objs(camera, "chips");
        Node MoreWays() => DeviceTree.Leaf("more-ways", "", () => FormLook.Margined(Kit.Flow(
            FormLook.Caption("Didn't play, or the ground station is another machine?"),
            FormLook.Button("Ground station links →", () => ctx.Open("/connect"), ButtonLook.Subtle)), Kit.Pad));
        return Kit.Section(name, DJ.Badge(camera, "status"), new Node?[]
            {
                DJ.Flag(camera, "shared") ? Kit.Notice(new Badge(DeviceLevel.Warn,
                    "Another camera pulls this same RTSP source. One camera fed to two pulls splits its video and audio across the streams — remove the duplicate camera for a stable feed.")) : null,
                CamerasScreen.PictureFrame(ctx, camera),
                live ? Kit.Buttons("watch", DJ.Objs(camera, "copies").Select(copy => (Node?)CopyButton(ctx, copy))
                    .Prepend(Kit.Link(ctx, ctx.Url(DJ.Text(camera, "watch")), "▶ Watch live", LinkLook.Filled, true))) : null,
            }
            .Concat(DJ.Objs(camera, "facts").Select(fact => (Node?)Kit.Fact(DJ.Text(fact, "label"), DJ.Text(fact, "value"))))
            .Append(DJ.Chart(DJ.Obj(camera, "bitrate")) is { } bitrate ? Kit.Chart(bitrate) : null)
            .Concat(diagnosis is null ? [] : new Node?[]
            {
                Kit.Notice(diagnosis),
                fix is null ? null : Kit.Buttons("fix", [Kit.Button("fix", DJ.Text(fix, "label"), () => Wizard(DJ.Flag(fix, "scan")), ButtonLook.Accent)]),
                Kit.Hint("This page keeps checking — it updates by itself the moment the camera answers again."),
            })
            .Append(fix is null ? Kit.Buttons("reconfigure", [Kit.Button("reconfigure", "Reconfigure", () => Wizard(!live), live ? ButtonLook.Standard : ButtonLook.Accent)]) : null)
            .Concat(links.Count == 0 ? [] : live
                ? Kit.Endpoints(ctx, links).Prepend(Kit.Sub("Every way to watch this camera")).Append(MoreWays())
                : [Kit.Disclosure("Links — they will play once it's back", "", null, false, () => Kit.Endpoints(ctx, links).Append(MoreWays()))])
            .Append(chips.Count == 0 ? null : Kit.Buttons("chips", chips.Select(chip => (Node?)DeviceTree.Leaf($"chip:{DJ.Text(chip, "key")}", DJ.Sig(chip), () =>
            {
                var button = FormLook.Button(DJ.Text(chip, "label"), () => ctx.Open(DJ.Text(chip, "href")));
                button.Content = FormLook.Line(8, Kit.Dot(DJ.Int(chip, "level")), FormLook.Text(DJ.Text(chip, "label")), FormLook.Glyph("", 12));
                return button;
            })))));
    }

    private static Node CopyButton(DeviceContext ctx, JsonObject copy)
    {
        var label = DJ.Text(copy, "label");
        var text = DJ.Text(copy, "text");
        var title = label.StartsWith("Copy ") ? label[5..] : label;
        return DeviceTree.Leaf($"copy:{label}", DJ.Sig(copy), () => FormLook.Line(0,
            FormLook.Button(label, () => Kit.Clip(ctx, text), glyph: ""),
            DJ.Flag(copy, "qr") ? QrButton(title, text) : null));
    }

    private static Button QrButton(string title, string text)
    {
        var button = FormLook.IconButton("", $"Show {title} as a QR code", () => { });
        button.Click += async (_, _) => await Kit.QrDialog(button.XamlRoot, title, text);
        return button;
    }
}

internal sealed class PreviewBox : UserControl
{
    private const double Aspect = 16.0 / 9.0;

    private readonly string _path;
    private readonly string _client = $"device.preview.{Guid.NewGuid():N}";
    private readonly Grid _frame = new() { CornerRadius = new CornerRadius(12), Background = FormLook.Brush("BgSecondaryBrush") };
    private readonly ContentControl _surface = new() { HorizontalContentAlignment = HorizontalAlignment.Stretch, VerticalContentAlignment = VerticalAlignment.Stretch, IsTabStop = false };
    private readonly StackPanel _waiting = new() { Spacing = 8, HorizontalAlignment = HorizontalAlignment.Center, VerticalAlignment = VerticalAlignment.Center };
    private readonly ProgressRing _ring = new() { IsActive = true, Width = 32, Height = 32 };
    private readonly TextBlock _said = FormLook.Caption("", "TextPrimaryBrush");
    private readonly Button _open;
    private DeviceContext? _ctx;
    private string _watch = "";
    private int? _channel;

    public PreviewBox(string endpoint)
    {
        _path = $"view.videoPreview({endpoint})";
        _said.TextAlignment = TextAlignment.Center;
        FormLook.Fill(_waiting, [_ring, _said]);
        _open = FormLook.Button("Open camera", async () =>
        {
            if (_ctx is { } ctx) await Kit.Launch(ctx, _watch);
        }, ButtonLook.Standard, "");
        _open.HorizontalAlignment = HorizontalAlignment.Right;
        _open.VerticalAlignment = VerticalAlignment.Bottom;
        _open.Margin = new Thickness(8);
        FormLook.Fill(_frame, [_surface, _waiting, _open]);
        Content = _frame;
        SizeChanged += (_, size) => _frame.Height = size.NewSize.Width / Aspect;
        Loaded += async (_, _) =>
        {
            BridgeWatch.Watch(_client, [_path], Apply);
            Apply(await Task.Run(() => Bridge.Get(_path)));
        };
        Unloaded += (_, _) => BridgeWatch.Stop(_client);
    }

    public void Show(DeviceContext ctx, string watch, string name)
    {
        (_ctx, _watch) = (ctx, watch);
        AutomationProperties.SetName(_open, $"Open the {name} camera");
    }

    private void Apply(JsonObject view)
    {
        if (DJ.Text(view, "class") != "VideoPreview") return;
        var channel = view["channel"] is null ? (int?)null : DJ.Int(view, "channel");
        if (channel != _channel)
        {
            _channel = channel;
            _surface.Content = channel is { } at ? new VideoSurface { Channel = at, Fit = VideoSurface.NoCrop } : null;
        }
        var playing = DJ.Flag(view, "playing");
        _waiting.Visibility = FormLook.Shown(!playing);
        _ring.Visibility = FormLook.Shown(view["trying"] is null || DJ.Flag(view, "trying"));
        FormLook.Say(_said, DJ.Text(view, "text"));
    }

    public static UIElement Placeholder(string text)
    {
        var frame = new Grid { CornerRadius = new CornerRadius(12), Background = FormLook.Brush("BgSecondaryBrush") };
        var said = FormLook.Caption(text);
        said.TextAlignment = TextAlignment.Center;
        var inner = FormLook.Stack(8, FormLook.Glyph("", 28), said);
        inner.HorizontalAlignment = HorizontalAlignment.Center;
        inner.VerticalAlignment = VerticalAlignment.Center;
        inner.Margin = new Thickness(24);
        frame.Children.Add(inner);
        frame.SizeChanged += (_, size) => frame.Height = size.NewSize.Width / Aspect;
        return frame;
    }
}

internal sealed class CameraSetupScreen : DeviceScreen
{
    private static readonly IReadOnlyDictionary<string, string> StopLabels = new Dictionary<string, string>
    {
        ["whip"] = "the SFU",
        ["livekit"] = "LiveKit",
        ["cloudflare"] = "Cloudflare",
    };

    public CameraSetupScreen(DeviceNav nav) : base("cameras", nav) { }

    protected override IEnumerable<Node?>? Build(DeviceReading reading, DeviceContext ctx) =>
        Viewed(reading, view => view["add"] is JsonObject, view =>
            DJ.Objs(view, "cameras").FirstOrDefault(c => DJ.Text(c, "name") == Nav.Query.GetValueOrDefault("camera", "")) is { } camera
                ? Parts(ctx, camera, Nav.Fragment)
                : [
                    Kit.Notice(new Badge(DeviceLevel.Neutral, "Pick a camera on the Cameras page to set it up.")),
                    Kit.Buttons("cameras", [Kit.Button("cameras", "Cameras →", () => ctx.Open("/streaming"), ButtonLook.Subtle)]),
                ]);

    private static Node Part(JsonObject? holder, string id, string opened, Func<IEnumerable<Node?>> content)
    {
        var part = DJ.Section(holder, "part");
        return DeviceTree.Kept($"part:{id}", () => new PartBox(DJ.Flag(part, "open") || opened == id, opened == id),
            box => box.Show(DJ.Text(part, "title"), DJ.Text(part, "summary"), DJ.MaybeBadge(part, "badge"), content));
    }

    private static Node LinkedNotice(DeviceContext ctx, JsonObject? notice) =>
        Kit.Group($"linked:{DJ.Text(notice, "text")}", notice is null || DJ.Text(notice, "text").Length == 0 ? [] : new Node?[]
        {
            Kit.Notice(DJ.Badge(notice)),
            DJ.Obj(notice, "link") is { } link ? Kit.Buttons("link", [Kit.Button("link", $"{DJ.Text(link, "label")} →", () => ctx.Open(DJ.Text(link, "href")), ButtonLook.Subtle)]) : null,
        });

    private static Node NoteLink(string key, string text, string label, Action open) =>
        DeviceTree.Leaf($"note:{key}", DJ.Sig(text, label), () => FormLook.Margined(Kit.Flow(FormLook.Caption(text), FormLook.Button(label, open, ButtonLook.Subtle)), Kit.Pad));

    private static IEnumerable<Node?> Parts(DeviceContext ctx, JsonObject camera, string opened)
    {
        var name = DJ.Text(camera, "name");
        var setup = DJ.Section(camera, "setup");
        var watch = DJ.Section(setup, "watch");
        var radio = DJ.Section(setup, "radio");
        var servers = DJ.Section(setup, "servers");
        var shares = DJ.Section(setup, "shares");
        var detect = DJ.Section(setup, "detect");
        var record = DJ.Section(setup, "record");
        var gimbal = DJ.Obj(setup, "gimbal");
        var hours = DJ.Options(shares, "hours");
        return new Node?[]
        {
            Part(watch, "cam-watch", opened, () => Watch(ctx, name, watch)),
            Part(radio, "cam-radio", opened, () => [
                LinkedNotice(ctx, DJ.Obj(radio, "notice")),
                DJ.Flag(radio, "form") ? Kit.Form(ctx, "wfb", [new Field("address", "Address wfb_tx reads from", DJ.Text(radio, "address"),
                        Hint: "The default is where a stock wfb-ng install reads from.", Kind: FieldKind.Uri)],
                    DeviceLink.Put, $"/api/streams/{name}/wfb", "Save", $"{name} is now going over the radio.", note: "Takes effect immediately — no restart.") : null,
                DJ.Flag(radio, "stop") ? Kit.Buttons("stop", [Kit.Action(ctx, "Stop", new DeviceRequest(DeviceLink.Delete, $"/api/streams/{name}/wfb"), $"Stopped sending {name} over the radio.",
                    new Confirm("Stop the radio", $"Stop sending {name} over the radio link?", "Stop"), tonal: false)]) : null,
            ]),
            Part(servers, "cam-servers", opened, () => DJ.Objs(servers, "dests").Select(dest =>
                (Node?)Part(dest, $"cam-dest-{DJ.Text(dest, "kind")}", opened, () => Server(ctx, name, dest)))),
            Part(shares, "cam-share", opened, () => DJ.Objs(shares, "links").Select(link => (Node?)Kit.Group($"share:{DJ.Text(link, "id")}", Share(ctx, link)))
                .Append(Kit.Form(ctx, "share", [
                        new Field("label", "Who is it for", "", "e.g. command post"),
                        new Field("hours", "Stops working after", hours.FirstOrDefault()?.Value ?? "24", Options: hours),
                    ],
                    DeviceLink.Post, "/api/share", "Create link", "Share link created.",
                    note: "The link opens on a page that is the picture and nothing else — no settings, no addresses.",
                    extra: new Dictionary<string, string> { ["camera"] = name }))
                .Append(Kit.Text("A link stops working when it expires or when you turn it off. It is not a lock: anyone who can already reach this device on your network can watch the camera anyway."))),
            Part(detect, "cam-detect", opened, () => [
                Kit.Switch(ctx, $"Draw detection boxes for {name}", DJ.Flag(detect, "on"),
                    new DeviceRequest(DeviceLink.Put, $"/api/streams/{name}/detect"), new DeviceRequest(DeviceLink.Delete, $"/api/streams/{name}/detect"),
                    success: $"Detection turned on for {name}.", offSuccess: $"Detection turned off for {name}."),
                Kit.Text("Draw object-detection boxes over this camera. The video is untouched — boxes are sent as data and drawn by viewers, so there's no added latency. Needs the detection sidecar running on the device."),
                DJ.Text(detect, "model") is { Length: > 0 } model ? NoteLink("model", model, "Change model →", () => ctx.Open("/settings/video")) : null,
                DJ.Obj(detect, "notice") is { } notice ? LinkedNotice(ctx, notice) : null,
            ]),
            Part(record, "cam-record", opened, () => [
                Kit.Switch(ctx, $"Record {name} to disk", DJ.Flag(record, "on"),
                    new DeviceRequest(DeviceLink.Put, $"/api/streams/{name}/record"), new DeviceRequest(DeviceLink.Delete, $"/api/streams/{name}/record"),
                    success: $"Recording turned on for {name}.", offSuccess: $"Recording turned off for {name} — download it under Settings → Recordings."),
                Kit.Text("Recorded in ten-minute segments to this device's storage, and playable as soon as a segment closes. The oldest go when the size limit is reached, so recording can never fill the card and take the device down with it."),
                DJ.Flag(record, "full") ? Kit.Notice(new Badge(DeviceLevel.Warn, "Recordings have reached the size limit, so the oldest are being deleted to make room for new ones.")) : null,
                DJ.Text(record, "usage") is { Length: > 0 } usage ? NoteLink("usage", usage, "Open recordings →", () => ctx.Open("/settings/recordings")) : null,
            ]),
            gimbal is null ? null : Part(gimbal, "cam-gimbal", opened, () => DJ.Flag(gimbal, "owner")
                ? [
                    Kit.Text("Tracking from QGroundControl steers the gimbal from this camera's picture. Only one camera can be on the gimbal — naming another one moves tracking there."),
                    Kit.Buttons("gimbal", [Kit.Action(ctx, "Not this camera", new DeviceRequest(DeviceLink.Delete, $"/api/streams/{name}/gimbal/owner"), $"Tracking turned off for {name}.", tonal: false)]),
                ]
                : [
                    Kit.Text("This device has a gimbal, but it is not this camera's, so QGroundControl cannot track from this picture. Say it is the gimbal camera to track from it."),
                    Kit.Buttons("gimbal", [Kit.Action(ctx, "This is the gimbal camera", new DeviceRequest(DeviceLink.Put, $"/api/streams/{name}/gimbal/owner"), $"{name} is now the gimbal camera.")]),
                ]),
            Kit.Buttons("remove", [Kit.Action(ctx, "Remove this camera", new DeviceRequest(DeviceLink.Delete, $"/api/streams/{name}"), $"Camera {name} removed.",
                new Confirm("Remove camera", $"Remove camera {name}? Its stream URLs stop working, and once no other camera uses its port the port returns to the rescue network.", "Remove"),
                tonal: false, done: ctx.Close)]),
        };
    }

    private static IEnumerable<Node?> Watch(DeviceContext ctx, string name, JsonObject watch) =>
        new Node?[] { Kit.Sub("From anywhere") }
            .Concat(DJ.Flag(watch, "cloud")
                ? Kit.Endpoints(ctx, DJ.Endpoints(watch, "cloudLinks")).Concat(new Node?[]
                {
                    Kit.Text(DJ.Text(watch, "cloudCopy")),
                    DJ.Flag(watch, "signIn") ? LinkedNotice(ctx, new JsonObject
                    {
                        ["level"] = DeviceLevel.Warn,
                        ["text"] = "Add cloud credentials in Engineering → Publishing sign-in to finish authorizing the stream.",
                        ["link"] = new JsonObject { ["label"] = "Engineering", ["href"] = "/settings/engineering" },
                    }) : null,
                    Kit.Buttons("disconnect", [Kit.Action(ctx, "Disconnect", new DeviceRequest(DeviceLink.Delete, $"/api/stream/sfu/{name}/cloud"), $"{name} disconnected from the cloud.",
                        new Confirm("Disconnect from the cloud", $"Disconnect {name} from the Aircast cloud? Its cloud watch page stops working. Share links keep working on this network.", "Disconnect"), tonal: false)]),
                })
                : DJ.Flag(watch, "canConnect")
                    ? [
                        Kit.Text("Optional: a watch page on the Aircast cloud, reachable from outside this network."),
                        Kit.Buttons("connect", [Kit.Action(ctx, "Connect to cloud", new DeviceRequest(DeviceLink.Post, $"/api/stream/sfu/{name}/cloud"), $"Connecting {name} to the Aircast cloud…")]),
                    ]
                    : [LinkedNotice(ctx, new JsonObject
                    {
                        ["level"] = DeviceLevel.Neutral,
                        ["text"] = "Needs a cloud account on this device — Device → Cloud account.",
                        ["link"] = new JsonObject { ["label"] = "Device", ["href"] = "/settings/device" },
                    })])
            .Append(Kit.Sub("On this network"))
            .Concat(Kit.Endpoints(ctx, DJ.Endpoints(watch, "links")))
            .Append(NoteLink("ways", "Didn't play, or the ground station is another machine?", "Ground station links →", () => ctx.Open("/connect")));

    private static IEnumerable<Node?> Server(DeviceContext ctx, string name, JsonObject dest)
    {
        var kind = DJ.Text(dest, "kind");
        var path = $"/api/stream/sfu/{name}/{kind}";
        var on = DJ.Flag(dest, "on");
        var label = StopLabels.GetValueOrDefault(kind, kind);
        return new Node?[]
        {
            kind switch
            {
                "whip" => Kit.Form(ctx, "whip", [new Field("url", "WHIP URL", DJ.Text(dest, "url"), "https://sfu.example.com/whip/device", Kind: FieldKind.Uri)],
                    DeviceLink.Put, path, "Save", $"Publishing {name} to the SFU."),
                "livekit" => Kit.Form(ctx, "livekit", [
                        new Field("url", "Server URL", DJ.Text(dest, "url"), "wss://livekit.example.com", "The room is this camera's name.", FieldKind.Uri),
                        new Field("livekitKey", "API key", DJ.Text(dest, "key"), "APIxxxxxxxx"),
                        new Field("livekitSecret", "API secret", DJ.Text(dest, "secret"), "LiveKit API secret", Kind: FieldKind.Password),
                    ],
                    DeviceLink.Put, path, "Save", $"Publishing {name} to LiveKit.",
                    note: "One key pair is shared by every camera published to LiveKit — changing it here changes it everywhere."),
                "cloudflare" => DeviceTree.Leaf("cloudflare", DJ.Sig(DJ.Text(dest, "app"), DJ.Text(dest, "track"), DJ.Text(dest, "secret")), () => new CloudflareBox(ctx, name, dest)),
                _ => null,
            },
            kind == "whip" ? NoteLink("whip", "Signs in with the device's sign-in method.", "Publishing sign-in →", () => ctx.Open("/settings/engineering")) : null,
            on ? Kit.Buttons("stop", [Kit.Action(ctx, "Stop publishing", new DeviceRequest(DeviceLink.Delete, path), $"{name} stopped publishing to {label}.",
                new Confirm("Stop publishing", $"Stop publishing {name} to {label}?", "Stop"), tonal: false)]) : null,
            kind == "cloudflare" && on && DJ.Text(dest, "watch") is { Length: > 0 } watch
                ? Kit.Buttons("cf-watch", [Kit.Link(ctx, ctx.Url(watch), "▶ Watch via Cloudflare", LinkLook.Filled, true)])
                : null,
            kind == "cloudflare" && on
                ? Kit.Text($"This player pulls the stream back from Cloudflare — proof of what your viewers get. There's no public watch URL: a viewer app pulls track {DJ.Text(dest, "track")} from app {DJ.Text(dest, "app")} through Cloudflare's Sessions API, authorized with the app secret.")
                : null,
        };
    }

    private static IEnumerable<Node?> Share(DeviceContext ctx, JsonObject link) =>
        DJ.Text(link, "dead") is { Length: > 0 } dead
            ? [Kit.StackedFact(DJ.Text(link, "label"), dead)]
            : [
                Kit.Copy(ctx, DJ.Text(link, "label"), DJ.Text(link, "url"), DJ.Text(link, "expires"), true),
                Kit.Buttons("off", [Kit.Action(ctx, "Turn off", new DeviceRequest(DeviceLink.Delete, $"/api/share/{DJ.Text(link, "id")}"), "Link turned off.",
                    new Confirm("Turn off this link", "Turn off this link? Whoever has it stops being able to open the picture.", "Turn off"), tonal: false)]),
            ];
}

internal sealed class PartBox : UserControl
{
    private readonly DisclosureBox _disclosure;
    private readonly bool _bring;

    public PartBox(bool open, bool bring)
    {
        _disclosure = new DisclosureBox(open);
        _bring = bring;
        Content = _disclosure;
        Loaded += (_, _) =>
        {
            if (_bring) StartBringIntoView();
        };
    }

    public void Show(string title, string summary, Badge? badge, Func<IEnumerable<Node?>> content)
    {
        AutomationProperties.SetName(this, title);
        _disclosure.Show(title, summary, badge, content);
    }
}

internal sealed class CloudflareBox : UserControl
{
    public CloudflareBox(DeviceContext ctx, string name, JsonObject dest)
    {
        var app = FormLook.Field(DJ.Text(dest, "app"), "App ID", "Realtime → SFU → your app");
        var secret = new PasswordBox { Header = "App secret", PlaceholderText = "Realtime app secret", Password = DJ.Text(dest, "secret") };
        var track = FormLook.Field(DJ.Text(dest, "track"), "Track name");
        track.Description = "Not from Cloudflare — a name you pick. Viewers subscribe to this stream by this name. The camera name works fine.";
        AutomationProperties.SetName(app, "App ID");
        AutomationProperties.SetName(secret, "App secret");
        AutomationProperties.SetName(track, "Track name");
        var appSaid = Kit.Refusal();
        var secretSaid = Kit.Refusal();
        var refusal = Kit.Refusal();
        Button? save = null;
        save = FormLook.Button("Save", async () =>
        {
            var target = app.Text.Trim().Length > 0 && track.Text.Trim().Length > 0 ? $"cloudflare://{app.Text.Trim()}/{track.Text.Trim()}" : "";
            save!.IsEnabled = false;
            var refused = await DeviceLink.Call(new DeviceRequest(DeviceLink.Put, $"/api/stream/sfu/{name}/cloudflare", DeviceLink.Body(("url", target), ("secret", secret.Password))));
            save.IsEnabled = true;
            FormLook.Say(appSaid, refused?.Field == "url" ? refused.Message : null);
            FormLook.Say(secretSaid, refused?.Field == "secret" ? refused.Message : null);
            FormLook.Say(refusal, refused is not null && refused.Field != "url" && refused.Field != "secret" ? refused.Message : null);
            if (refused is null) ctx.Say($"Publishing {name} to Cloudflare.");
        });
        save.HorizontalAlignment = HorizontalAlignment.Left;
        Content = Kit.Padded(FormLook.Stack(12,
            Kit.Flow(
                FormLook.Caption("In the Cloudflare dashboard, open Realtime → SFU and create an app — it gives you the App ID and App secret."),
                Kit.LinkView(ctx, "https://dash.cloudflare.com/", "Cloudflare dashboard", LinkLook.Outlined, true)),
            FormLook.Stack(4, app, appSaid),
            FormLook.Stack(4, secret, secretSaid),
            track,
            refusal,
            save));
    }
}
