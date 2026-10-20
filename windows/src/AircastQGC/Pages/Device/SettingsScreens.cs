using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Pages.Device;

internal sealed class DeviceSettingsScreen : DeviceScreen
{
    private const string Services = "Services";

    private static readonly string[] ProvisionSteps =
    [
        "Export this device's settings.",
        "Add your tailnet auth key to the file. It is not exported, because this page answers anyone who can reach the device.",
        "Copy the file onto the new card's boot partition as aircast-provision.json.",
        "Boot it. It joins, picks up the camera and the data caps, then deletes the file so the key does not stay on the card.",
    ];

    public DeviceSettingsScreen(DeviceNav nav) : base("device", nav) { }

    protected override IEnumerable<Node?>? Build(DeviceReading reading, DeviceContext ctx) =>
        Viewed(reading, view => view["health"] is JsonObject, view => new Node?[]
        {
            Health(ctx, DJ.Section(view, "health")),
            Kit.Notice(DJ.MaybeBadge(view, "notice")),
            Kit.Section("Status", null, DJ.Objs(view, "stats").Select(stat => DJ.Text(stat, "label") == Services
                ? Kit.Disclosure(Services, $"{DJ.Text(stat, "value")} · {DJ.Text(stat, "hint")}", new Badge(DJ.Int(stat, "level"), DJ.Text(stat, "value")), false, () =>
                    DJ.Objs(view, "services").Select(service => (Node?)Kit.Fact(DJ.Text(service, "label"), DJ.Text(service, "state"), DJ.Int(service, "level")))
                        .Append(Kit.StatusLine("Supervised by systemd — a unit that stops is restarted automatically.")))
                : Stat(stat))),
            Cloud(ctx, DJ.Section(view, "cloud")),
            Software(ctx, DJ.Section(view, "software")),
            Kit.Section("More devices", null, [
                Kit.Disclosure("Set up another device like this one", "Export these settings for the next card.", null, false, () =>
                    ProvisionSteps.Select((step, at) => Kit.Text($"{at + 1}. {step}"))
                        .Append(Kit.Text("The file carries the camera's own address, sign-in included — that is what makes it worth copying. Treat it like the password it contains."))
                        .Append(Kit.Buttons("export", [Kit.Link(ctx, ctx.Url("/api/provision/export"), "Export these settings", glyph: "")]))
                        .Append(Kit.StatusLine("Airframes two through five are this one with a different serial."))),
            ]),
            Kit.Section("Reset device", null, [
                Kit.Text("Reboots and reverts the device to factory settings — camera, telemetry and all Wi-Fi and added network settings are wiped, and the setup wizard runs again. Tailscale stays connected so the device remains reachable."),
                DeviceTree.Kept("reset", () => new ResetBox(), box => box.Show(ctx, DJ.Text(view, "hostname"))),
            ]),
        });

    private static Node Health(DeviceContext ctx, JsonObject health) =>
        DeviceTree.Leaf("health", DJ.Sig(health), () =>
        {
            var level = DJ.Int(health, "level");
            var check = DJ.Text(health, "href") is { Length: > 0 } href ? FormLook.Button("Check", () => ctx.Open(href)) : null;
            var text = FormLook.Text(DJ.Text(health, "text"), "FontSizeBodyLarge", Kit.LevelBrush(level), true);
            var line = FormLook.Lead(12, Kit.Dot(level, 10), text);
            return FormLook.Margined(check is null ? line : FormLook.Row(line, check), new Thickness(0, 16, 0, 4));
        });

    private static Node Stat(JsonObject stat) =>
        DeviceTree.Leaf($"stat:{DJ.Text(stat, "label")}", DJ.Sig(stat), () => Kit.Padded(FormLook.Stack(2,
            FormLook.Text(DJ.Text(stat, "label"), "FontSizeBody", "TextSecondaryBrush"),
            FormLook.Text(DJ.Text(stat, "value"), "FontSizeBodyLarge", Kit.LevelTextBrush(DJ.Int(stat, "level"))),
            FormLook.Caption(DJ.Text(stat, "hint"), "TextTertiaryBrush"))));

    private static Node Cloud(DeviceContext ctx, JsonObject cloud)
    {
        var stage = DJ.Text(cloud, "stage") is { Length: > 0 } s ? s : "connect";
        var error = DJ.Text(cloud, "error");
        return Kit.Section("Cloud account", DJ.Badge(cloud, "status"), stage switch
        {
            "linked" => [
                Kit.Text("This device is linked to your Aircast cloud account. The ID below only matters if support asks for it."),
                Kit.Copy(ctx, "Device ID (for support)", DJ.Text(cloud, "deviceId")),
                DJ.Text(cloud, "watchUrl") is { Length: > 0 } watch ? Kit.Buttons("watch", [Kit.Link(ctx, watch, "Watch in the cloud", LinkLook.Text, true)]) : null,
            ],
            "signing-in" => [
                Kit.Text("Open the sign-in page, log in (or sign up) with Google, and authorize this device. It connects automatically once you approve."),
                Kit.Buttons("sign-in", [Kit.Link(ctx, DJ.Text(cloud, "signInUrl"), "Open sign-in page", external: true)]),
                Kit.Copy(ctx, "Or go to the link and enter this code", DJ.Text(cloud, "userCode")),
            ],
            _ => [
                Kit.Text("Not connected. Linking signs this device in to an Aircast cloud account; it is optional — every local and self-hosted path works without it."),
                Kit.Notice(new Badge(DeviceLevel.Warn, error)),
                Kit.Disclosure("Advanced · environment", "Which Aircast cloud to sign in to", null, false, () => [
                    Kit.Form(ctx, DJ.Text(cloud, "issuer"), [new Field("issuer", "Cloud API", DJ.Text(cloud, "issuer"), "https://api.aircast.one",
                            "Leave this alone unless you were told to use a different Aircast environment.", FieldKind.Uri)],
                        DeviceLink.Post, "/api/cloud/connect", "Connect", "Sign-in started — the code appears here in a moment."),
                ]),
            ],
        });
    }

    private static Node Software(DeviceContext ctx, JsonObject software)
    {
        var candidate = DJ.Obj(software, "candidate");
        return Kit.Section("Software", DJ.Badge(software, "status"), new Node?[]
            {
                Kit.StackedFact("Running", DJ.Text(software, "summary")),
            }
            .Concat(DJ.Objs(software, "notices").SelectMany(notice => new Node?[]
            {
                Kit.Notice(new Badge(DJ.Int(notice, "level"), DJ.Text(notice, "text") is { Length: > 0 } text ? $"{DJ.Text(notice, "title")} {text}" : DJ.Text(notice, "title"))),
                DJ.Text(notice, "detail") is { Length: > 0 } detail
                    ? Kit.Disclosure(DJ.Text(notice, "detailLabel"), "", null, false, () => [Kit.Mono(detail)])
                    : null,
            }))
            .Append(candidate is null ? null : Kit.Group("candidate", [
                Kit.Sub(DJ.Text(candidate, "title")),
                Kit.Buttons("install", [
                    Kit.Action(ctx, DJ.Text(candidate, "install"), new DeviceRequest(DeviceLink.Post, "/api/update"), DJ.Text(candidate, "success"),
                        new Confirm("Install the update?", DJ.Text(candidate, "confirm"), "Install")),
                    Kit.Link(ctx, DJ.Text(candidate, "notesUrl"), "What's new", LinkLook.Text, true),
                ]),
            ]))
            .Concat(DJ.Texts(software, "notes").Select(Kit.Text))
            .Append(Kit.Hint(DJ.Text(software, "checked")))
            .Append(DJ.Flag(software, "canCheck")
                ? Kit.Buttons("check", [Kit.Action(ctx, "Check now", new DeviceRequest(DeviceLink.Post, "/api/update/check"), "Checked for updates.", tonal: false)])
                : null));
    }
}

internal sealed class ResetBox : UserControl
{
    private readonly TextBlock _refusal = Kit.Refusal();
    private readonly Button _reset;
    private DeviceContext? _ctx;
    private string _hostname = "";

    public ResetBox()
    {
        _reset = FormLook.Button("Reset to defaults", Reset);
        _reset.Foreground = FormLook.Brush("CriticalBrush");
        _reset.HorizontalAlignment = HorizontalAlignment.Left;
        Content = Kit.Padded(FormLook.Stack(4, _reset, _refusal));
    }

    public void Show(DeviceContext ctx, string hostname) => (_ctx, _hostname) = (ctx, hostname);

    private async void Reset()
    {
        if (_ctx is not { } ctx) return;
        var typed = await Kit.TypedConfirm(XamlRoot, "Reset to factory settings?",
            "It reboots and wipes the camera, telemetry and all Wi-Fi and added network settings, then runs setup again. Tailscale is preserved.",
            $"Type {_hostname} to confirm", _hostname, "Reset");
        if (typed is null) return;
        _reset.IsEnabled = false;
        var refused = await DeviceLink.Call(new DeviceRequest(DeviceLink.Post, "/api/setup/reset", DeviceLink.Body(("confirm", typed))));
        _reset.IsEnabled = true;
        FormLook.Say(_refusal, refused?.Message);
        if (refused is null) ctx.Say("Resetting — the device is rebooting to factory settings; reconnect in a minute.");
    }
}

internal sealed class EngineeringScreen : DeviceScreen
{
    public EngineeringScreen(DeviceNav nav) : base("engineering", nav) { }

    protected override IEnumerable<Node?>? Build(DeviceReading reading, DeviceContext ctx) =>
        Viewed(reading, view => view["backend"] is JsonObject, view =>
        {
            var backend = DJ.Section(view, "backend");
            var publish = DJ.Section(view, "publish");
            var diagnostics = DJ.Section(view, "diagnostics");
            return new Node?[]
            {
                Kit.Section("What carries video", DJ.Badge(backend, "status"), [
                    Kit.StatusLine("aircastd's own engine, or mediamtx as a separate service — switching restarts the device."),
                    Kit.Text(DJ.Text(backend, "text")),
                    Kit.Buttons("backend", [Kit.Action(ctx, DJ.Text(backend, "label"),
                        new DeviceRequest(DeviceLink.Put, "/api/stream/media-backend", DeviceLink.Body(("backend", DJ.Text(backend, "switch")))),
                        DJ.Text(backend, "success"), new Confirm("Restart the device?", DJ.Text(backend, "confirm"), "Switch"))]),
                    Kit.Disclosure("Technical details", "", null, false, () => [Kit.Text(
                        "Both backends bind the same ports — 8554, 8888, 8889, 1935, 8890 — so only one can run, and the choice is read once at startup. Switching writes the setting and restarts aircastd into it; " +
                        "if the restart fails the setting still stands and the next boot lands on it. Camera paths carry across in both directions. Only the engine can send a camera to a wfb-ng packet radio.")]),
                ]),
                Kit.Section("Publishing sign-in", DJ.Badge(publish, "status"), [
                    Kit.Text("Devices on the Aircast cloud sign in automatically once connected to a cloud account. Custom servers are set up under Advanced; publishing to a specific server — LiveKit, Cloudflare — is configured per camera on the Cameras page."),
                    DJ.Flag(publish, "awaitingCloud") ? Kit.Buttons("await", [Kit.Button("cloud", "Connect the cloud account", () => ctx.Open("/settings/device"))]) : null,
                    DJ.Flag(publish, "awaitingCloud") ? Kit.StatusLine("Nothing to fill in here — the sign-in arrives with the cloud account.") : null,
                    Kit.Buttons("where", [Kit.Button("where", "Choose where cameras publish", () => ctx.Open("/streaming"), ButtonLook.Subtle)]),
                    Kit.Disclosure("Advanced · sign-in credentials", "How this device signs in to video servers.", null, false, () => [
                        DeviceTree.Leaf("sign-in", DJ.Sig(publish), () => new SignInBox(ctx, publish)),
                    ]),
                ]),
                Kit.Section("Device logs", null, [
                    Kit.Text("Everything every service on this device has logged, gzipped."),
                    Kit.Buttons("logs", [Kit.Link(ctx, ctx.Url("/api/logs"), "Download logs", glyph: "")]),
                ]),
                Kit.Section("Remote diagnostics", DJ.Badge(diagnostics, "status"), [
                    Kit.Text(DJ.Text(diagnostics, "hint")),
                    Kit.Form(ctx, "diagnostics", [
                            new Field("endpoint", "OTLP endpoint (host:port)", DJ.Text(diagnostics, "endpoint"), "otel.example.com:443", Kind: FieldKind.Uri),
                            new Field("token", DJ.Text(diagnostics, "tokenLabel"), DJ.Text(diagnostics, "token"), "none", Kind: FieldKind.Password),
                        ],
                        DeviceLink.Post, "/api/diagnostics/config", "Save", "Diagnostics endpoint saved.", note: "Stored on the device only."),
                    Kit.Switch(ctx, "Stream this device's logs", DJ.Flag(diagnostics, "streaming"),
                        new DeviceRequest(DeviceLink.Put, "/api/diagnostics", DeviceLink.Body(("enabled", "true"))),
                        new DeviceRequest(DeviceLink.Put, "/api/diagnostics", DeviceLink.Body(("enabled", "false"))),
                        success: "Streaming logs.", offSuccess: "Stopped streaming logs.", enabled: DJ.Flag(diagnostics, "canStream"),
                        confirmOn: new Confirm("Stream the logs?", DJ.Text(diagnostics, "confirm"), "Stream")),
                    Kit.StatusLine(DJ.Text(diagnostics, "state")),
                    Kit.StatusLine("Ships this device's logs to an endpoint you name, so support can read them without the device in front of them."),
                ]),
            };
        });
}

internal sealed class SignInBox : UserControl
{
    private readonly DeviceContext _ctx;
    private readonly JsonObject _publish;
    private readonly RadioButtons _modes = new();
    private readonly StackPanel _fields = new() { Spacing = 12 };
    private readonly TextBlock _verdict = FormLook.Text("");
    private readonly TextBlock _refusal = Kit.Refusal();
    private readonly Button _save;
    private readonly IReadOnlyList<JsonObject> _modeList;
    private IReadOnlyDictionary<string, string> _edits = new Dictionary<string, string>();
    private string _mode;

    public SignInBox(DeviceContext ctx, JsonObject publish)
    {
        (_ctx, _publish) = (ctx, publish);
        _mode = DJ.Text(publish, "mode");
        _modeList = DJ.Objs(publish, "modes");
        _modes.ItemsSource = _modeList.Select(mode => FormLook.Stack(2,
            FormLook.Text(DJ.Text(mode, "title")),
            FormLook.Caption(DJ.Text(mode, "desc")))).ToList();
        _modes.SelectedIndex = _modeList.ToList().FindIndex(mode => DJ.Text(mode, "value") == _mode);
        AutomationProperties.SetName(_modes, "Sign-in method");
        _modes.SelectionChanged += (_, _) =>
        {
            if (_modes.SelectedIndex < 0) return;
            _mode = DJ.Text(_modeList[_modes.SelectedIndex], "value");
            Paint();
        };
        _save = FormLook.Button("Save", Save);
        var stored = FormLook.Caption("Stored on the device only.", "TextTertiaryBrush");
        Content = Kit.Padded(FormLook.Stack(12, _modes, _fields, _verdict, _refusal, FormLook.Row(stored, _save)));
        Paint();
    }

    private JsonObject? Picked => _modeList.FirstOrDefault(mode => DJ.Text(mode, "value") == _mode);

    private void Paint()
    {
        var names = DJ.Texts(Picked, "fields");
        var environment = DJ.Text(_publish, "environment");
        var fields = DJ.Objs(_publish, "fields").Where(field => names.Contains(DJ.Text(field, "name"))).ToList();
        FormLook.Fill(_fields, fields.Select(Input)
            .Prepend(_mode == "aircast" && environment.Length > 0
                ? FormLook.Stack(2, FormLook.Caption("Environment"), Kit.MonoText(environment))
                : null));
        var dirty = _mode != DJ.Text(_publish, "mode");
        var verdict = DJ.Badge(_publish, "verdict");
        _verdict.Text = dirty ? DJ.Text(Picked, "unsaved") : verdict.Text;
        _verdict.Foreground = FormLook.Brush(!dirty && verdict.Level == DeviceLevel.Blocked ? "CriticalBrush" : "TextSecondaryBrush");
        _verdict.Visibility = FormLook.Shown(_verdict.Text.Length > 0);
        AutomationProperties.SetName(_save, DJ.Text(Picked, "submit") is { Length: > 0 } submit ? submit : "Save");
        _save.Content = AutomationProperties.GetName(_save);
    }

    private UIElement Input(JsonObject field)
    {
        var name = DJ.Text(field, "name");
        var value = _edits.GetValueOrDefault(name, DJ.Text(field, "value"));
        if (DJ.Flag(field, "secret"))
        {
            var secret = new PasswordBox { Header = DJ.Text(field, "label"), PlaceholderText = DJ.Text(field, "placeholder"), Password = value };
            AutomationProperties.SetName(secret, DJ.Text(field, "label"));
            secret.PasswordChanged += (_, _) => _edits = new Dictionary<string, string>(_edits) { [name] = secret.Password };
            return secret;
        }
        var box = FormLook.Field(value, DJ.Text(field, "label"), DJ.Text(field, "placeholder"));
        AutomationProperties.SetName(box, DJ.Text(field, "label"));
        box.TextChanged += (_, _) => _edits = new Dictionary<string, string>(_edits) { [name] = box.Text };
        return box;
    }

    private async void Save()
    {
        var dirty = _mode != DJ.Text(_publish, "mode");
        var confirm = dirty ? DJ.Text(Picked, "confirm") : "";
        if (confirm.Length > 0 && !await Kit.Ask(XamlRoot, new Confirm("Clear the sign-in?", confirm, "Clear"))) return;
        var clears = DJ.Texts(Picked, "clears");
        var body = DJ.Objs(_publish, "fields").Select(field => (Name: DJ.Text(field, "name"), Value: DJ.Text(field, "value")))
            .Select(field => KeyValuePair.Create(field.Name, clears.Contains(field.Name) ? "" : _edits.GetValueOrDefault(field.Name, field.Value)));
        _save.IsEnabled = false;
        var refused = await DeviceLink.Call(new DeviceRequest(DeviceLink.Put, "/api/stream/sfu/oauth", DeviceLink.Body(body)));
        _save.IsEnabled = true;
        FormLook.Say(_refusal, refused?.Message);
        if (refused is null) _ctx.Say("Publishing sign-in saved.");
    }
}

internal sealed class VideoEngineScreen : DeviceScreen
{
    private const string ModelsPath = "/api/detect/models";
    private static readonly IReadOnlyDictionary<string, string> SharingOff = new Dictionary<string, string> { ["baseUrl"] = "", ["signKey"] = "", ["deviceToken"] = "" };

    public VideoEngineScreen(DeviceNav nav) : base("video", nav) { }

    protected override IEnumerable<Node?>? Build(DeviceReading reading, DeviceContext ctx) =>
        Viewed(reading, view => view["sharing"] is JsonObject, view =>
        {
            var engineering = DJ.Page(DJ.Obj(view, "engineering"));
            return new Node?[]
            {
                DJ.Obj(view, "radio") is { } radio ? Radio(ctx, radio) : null,
                Sharing(ctx, DJ.Section(view, "sharing")),
                Kit.StatusLine("These apply to every camera on this device. Which camera goes where is set on the camera itself."),
                Detection(ctx, DJ.Section(view, "detection")),
                Kit.StatusLine("Detection runs on the device and sends boxes as data — the picture itself is untouched. Pick the model for what the camera is looking at: an aerial model hallucinates trucks on a desk. " +
                               "Draw-a-box tracking works whichever model is active."),
                Kit.Row(engineering.Title, engineering.Summary, "", RowState.Neutral, () => ctx.Open(engineering.Href), engineering.Href),
            };
        });

    private static Node Radio(DeviceContext ctx, JsonObject radio) => Kit.Section("Packet radio", DJ.Badge(radio, "status"), [
        Kit.Text("The channel has to match the ground station exactly — a mismatch looks the same as a dead radio. Saving restarts the radio, so the link drops for a few seconds."),
        Kit.Form(ctx, "radio", [
                new Field("channel", "Channel", DJ.Text(radio, "channel"), Hint: "1-14 on 2.4 GHz, 32-177 on 5 GHz. 165 is 5825 MHz.", Kind: FieldKind.Number),
                new Field("region", "Region", DJ.Text(radio, "region"), Hint: "Two-letter country code for the regulatory domain. BO permits the highest transmit power."),
                new Field("tx_power", "Transmit power", DJ.Text(radio, "txPower"), "driver default",
                    "0-30. Leave empty for the driver default — the legal ceiling and the heat the adapter can shed are both airframe-specific.", FieldKind.Number),
            ],
            DeviceLink.Put, "/api/stream/wfb/radio", "Save", "Radio settings applied.",
            new Confirm("Restart the radio?", "Change the radio and restart it? The link drops for a few seconds — do this on the ground.", "Restart"), "Restarts the radio."),
        Kit.Disclosure("Technical details", "", null, false, () => [Kit.Text(DJ.Text(radio, "details"))]),
    ]);

    private static Node Sharing(DeviceContext ctx, JsonObject sharing)
    {
        var off = new DeviceRequest(DeviceLink.Put, "/api/stream/cfbridge", DeviceLink.Body(SharingOff));
        return Kit.Section("Public viewer (Cloudflare)", DJ.Badge(sharing, "status"), DJ.Text(sharing, "stage") switch
        {
            "live" => [
                Kit.Text("Public sharing is on. Every camera that publishes to Cloudflare shows a public share link — hand it to anyone, no login or VPN, and it survives the backend going down."),
                Kit.Copy(ctx, "Bridge Worker", DJ.Text(sharing, "baseUrl")),
                Kit.Buttons("off", [Kit.Action(ctx, "Turn off", off, "Public sharing turned off.",
                    new Confirm("Turn off public sharing?", "Turn off public sharing? Existing share links stop working.", "Turn off"))]),
            ],
            "deploy" => [
                Kit.Step(1, "Deploy the bridge to your Cloudflare account", "One click — Cloudflare copies the bridge into your account and creates its storage. Sign in when asked."),
                Kit.Buttons("deploy", [Kit.Link(ctx, DJ.Text(sharing, "deployUrl"), "Deploy to Cloudflare", external: true)]),
                Kit.Step(2, "Paste these two keys when Cloudflare asks for secrets"),
                Kit.Copy(ctx, "SIGN_KEY", DJ.Text(sharing, "signKey"), "Signs share links (generated on this device)."),
                Kit.Copy(ctx, "DEVICE_TOKEN", DJ.Text(sharing, "deviceToken"), "Lets this device report the live session (generated on this device)."),
                Kit.StatusLine("Your Cloudflare app secret is sent automatically by this device — you don't need to set it."),
                Kit.Step(3, "Paste the Worker URL Cloudflare gives you"),
                Kit.Form(ctx, "worker", [new Field("baseUrl", "Worker URL", DJ.Text(sharing, "baseUrl"), "https://aircast-cf-bridge.<you>.workers.dev", Kind: FieldKind.Uri)],
                    DeviceLink.Put, "/api/stream/cfbridge", "Connect", "Public sharing is on.",
                    extra: new Dictionary<string, string> { ["signKey"] = DJ.Text(sharing, "signKey"), ["deviceToken"] = DJ.Text(sharing, "deviceToken") }),
                Kit.Buttons("over", [Kit.Action(ctx, "Start over", off, "Setup cleared.", new Confirm("Start over?", "Discard the generated keys and start over?", "Discard"), tonal: false)]),
            ],
            _ => [
                Kit.Text("Give a camera that publishes to Cloudflare Realtime a public share link — anyone opens it in a browser, no login and no VPN, and it keeps working even if the cloud backend is down. " +
                         "You host the bridge in your own Cloudflare account (free tier is plenty)."),
                Kit.Buttons("set-up", [Kit.Action(ctx, "Set up public sharing", new DeviceRequest(DeviceLink.Post, "/api/stream/cfbridge/keys"), "Keys generated — follow the deploy steps.")]),
            ],
        });
    }

    private static Node Detection(DeviceContext ctx, JsonObject detection)
    {
        var models = DJ.Objs(detection, "models");
        return Kit.Section("Detection model", null, new Node?[]
            {
                Kit.StackedFact("In use", DJ.Badge(detection, "status").Text),
                DJ.Flag(detection, "unavailable")
                    ? Kit.Notice(new Badge(DeviceLevel.Warn, "Detection is unavailable — this device has no AI accelerator, so the models below can't run. Draw-a-box tracking still works on every camera."))
                    : null,
                models.Count == 0 ? Kit.Text("No models installed yet.") : null,
            }
            .Concat(models.Select(model => (Node?)DeviceTree.Leaf($"model:{DJ.Text(model, "name")}", DJ.Sig(model), () => Model(ctx, model))))
            .Append(Kit.Disclosure("Add a model", "A .hef compiled for Hailo-8, with its class labels.", null, false, () => [
                DeviceTree.Leaf("upload", "", () => new ModelUploadBox(ctx)),
            ])));
    }

    private static UIElement Model(DeviceContext ctx, JsonObject model)
    {
        var name = DJ.Text(model, "name");
        var classes = DJ.Badge(model, "classes");
        var about = FormLook.Stack(2,
            FormLook.Text(name),
            FormLook.Text(DJ.Text(model, "about"), "FontSizeBody", "TextSecondaryBrush"),
            FormLook.Line(12, FormLook.Caption(DJ.Text(model, "size")), Kit.BadgeView(classes)));
        var actions = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 4, VerticalAlignment = VerticalAlignment.Center };
        DeviceTree.Sync(actions, DJ.Flag(model, "active")
            ? [DeviceTree.Leaf("active", "", () => Kit.BadgeView(new Badge(DeviceLevel.Ready, "Active")))]
            : [
                Kit.Action(ctx, "Use", new DeviceRequest(DeviceLink.Put, "/api/stream/detect/settings", DeviceLink.Body(("hef", DJ.Text(model, "path")))), "Model switched — detection restarts on the new model."),
                Kit.Action(ctx, "Delete", new DeviceRequest(DeviceLink.Delete, $"{ModelsPath}/{DeviceLink.Escaped(name)}"), "Model deleted.",
                    new Confirm($"Delete {name}?", $"Delete {name}? Anyone relying on it loses it from this device.", "Delete"), tonal: false),
            ]);
        Grid.SetColumn(actions, 1);
        var row = new Grid
        {
            Margin = Kit.Pad,
            ColumnSpacing = 12,
            ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto } },
            Children = { about, actions },
        };
        AutomationProperties.SetName(row, name);
        return row;
    }
}

internal sealed class ModelUploadBox : UserControl
{
    private const string ModelsPath = "/api/detect/models";

    private readonly DeviceContext _ctx;
    private readonly Button _hef;
    private readonly Button _labels;
    private readonly TextBox _about = FormLook.Field("", "What it's for (shown next to the model — optional)", "Aerial — vehicles at 400 ft");
    private readonly CheckBox _use;
    private readonly TextBlock _refusal = Kit.Refusal();
    private readonly Button _upload;
    private string? _hefPath;
    private string? _labelsPath;

    public ModelUploadBox(DeviceContext ctx)
    {
        _ctx = ctx;
        _hef = FormLook.Button("Choose a file", () => { });
        _hef.Click += async (_, _) => _hefPath = await Pick(_hef, _hefPath);
        AutomationProperties.SetName(_hef, "Model file");
        _labels = FormLook.Button("Choose a file", () => { });
        _labels.Click += async (_, _) => _labelsPath = await Pick(_labels, _labelsPath);
        AutomationProperties.SetName(_labels, "Class labels file");
        _use = FormLook.Check("Switch to this model after upload", true, _ => { });
        _upload = FormLook.Button("Upload model", Upload);
        _upload.HorizontalAlignment = HorizontalAlignment.Left;
        Content = Kit.Padded(FormLook.Stack(12,
            FormLook.Stack(4, FormLook.Caption("Model (.hef, compiled for Hailo-8)"), _hef),
            FormLook.Stack(4, FormLook.Caption("Class labels (.labels, one per line — optional). Without labels, boxes are named by class number."), _labels),
            _about,
            _use,
            _refusal,
            _upload));
    }

    private async System.Threading.Tasks.Task<string?> Pick(Button button, string? held)
    {
        var picked = await FilePicks.Open(XamlRoot, [FilePicks.Any]) ?? held;
        if (picked is not null) button.Content = System.IO.Path.GetFileName(picked);
        return picked;
    }

    private async void Upload()
    {
        if (_hefPath is not { } hef)
        {
            FormLook.Say(_refusal, "Choose a .hef model file.");
            return;
        }
        var use = _use.IsChecked == true;
        var parts = new JsonArray(new JsonNode?[]
        {
            DeviceLink.FilePart("hef", hef),
            _labelsPath is { } labels ? DeviceLink.FilePart("labels", labels) : null,
            _about.Text.Trim() is { Length: > 0 } about ? DeviceLink.ValuePart("about", about) : null,
            use ? DeviceLink.ValuePart("use", "1") : null,
        }.OfType<JsonNode>().ToArray());
        _upload.IsEnabled = false;
        _upload.Content = "Uploading…";
        FormLook.Say(_refusal, null);
        var answer = await DeviceLink.Upload(DeviceLink.Post, ModelsPath, parts);
        _upload.IsEnabled = true;
        _upload.Content = "Upload model";
        var switched = DJ.Flag(answer.Result, "switched");
        var refusal = answer.Refusal?.Message ?? (!switched && use ? "Model uploaded, but switching to it failed — pick it from the list." : null);
        FormLook.Say(_refusal, refusal);
        if (answer.Refusal is null && (switched || !use))
            _ctx.Say(switched ? "Model uploaded and switched — detection restarts on it." : "Model uploaded — use it from the list when you want it.");
    }
}

internal sealed class RecordingsScreen : DeviceScreen
{
    public RecordingsScreen(DeviceNav nav) : base("recordings", nav) { }

    protected override IEnumerable<Node?>? Build(DeviceReading reading, DeviceContext ctx) =>
        Viewed(reading, view => view["limit"] is JsonObject, view =>
        {
            var cameras = DJ.Objs(view, "cameras");
            var recorded = DJ.Objs(view, "recorded");
            return new Node?[]
            {
                Kit.Section("Storage", null, [
                    DeviceTree.Leaf("limit", DJ.Sig(DJ.Obj(view, "limit")), () => new LimitBox(ctx, DJ.Section(view, "limit"))),
                    Kit.StatusLine("Recordings share the card this device boots from. Past the limit the oldest go first, so recording can never fill the card."),
                ]),
                cameras.Count == 0 ? null : Kit.Section("Cameras", null, cameras.Select(camera =>
                {
                    var name = DJ.Text(camera, "name");
                    return (Node?)Kit.Switch(ctx, name, DJ.Flag(camera, "recording"),
                        new DeviceRequest(DeviceLink.Put, RecordPath(name)), new DeviceRequest(DeviceLink.Delete, RecordPath(name)),
                        DJ.Text(camera, "note"), $"Recording turned on for {name}.", $"Recording turned off for {name} — its recordings stay below.");
                })),
            }
            .Concat(recorded.Count > 0
                ? recorded.Select(camera => (Node?)Recorded(ctx, camera))
                : [Kit.Section("On this device", null, [Kit.Text("Nothing recorded yet. Turn recording on for a camera above.")])]);
        });

    public static string RecordPath(string camera) => $"/api/streams/{DeviceLink.Escaped(camera)}/record";

    private static string RecordingsPath(string camera) => $"/api/recordings/{DeviceLink.Escaped(camera)}";

    private static Node Recorded(DeviceContext ctx, JsonObject camera)
    {
        var name = DJ.Text(camera, "name");
        return Kit.Section(name, DJ.Badge(camera, "status"), new Node?[]
            {
                DJ.Flag(camera, "full") ? Kit.Notice(new Badge(DeviceLevel.Warn, "Recordings have reached the size limit, so the oldest are being deleted to make room for new ones.")) : null,
                Kit.Group("summary", [
                    Kit.Hint($"{DJ.Text(camera, "summary")} · {DJ.Text(camera, "usage")}"),
                    Kit.Buttons("delete-all", [Kit.Action(ctx, "Delete all", new DeviceRequest(DeviceLink.Delete, RecordingsPath(name)), $"Deleted every recording of {name}.",
                        new Confirm("Delete every recording?", $"Delete every recording of {name}? This cannot be undone.", "Delete all"), tonal: false)]),
                ]),
            }
            .Concat(DJ.Objs(camera, "segments").Select(segment => (Node?)DeviceTree.Leaf($"segment:{DJ.Text(segment, "at")}", DJ.Sig(segment), () => Segment(ctx, name, segment))))
            .Append(Kit.StatusLine("Each download carries the aircraft's height, speed, battery and position as a subtitle track. Players keep subtitles off until asked — in QuickTime choose View → Subtitles, " +
                                   "in VLC Subtitle → Sub Track.")));
    }

    private static UIElement Segment(DeviceContext ctx, string camera, JsonObject segment)
    {
        var started = DJ.Text(segment, "started");
        var open = DJ.Flag(segment, "open");
        var detail = string.Join(" · ", new[]
        {
            DJ.Text(segment, "duration"),
            DJ.Text(segment, "size"),
            DJ.Flag(segment, "noTelemetry") ? "no flight data" : "",
            open ? "downloadable once it closes" : "",
        }.Where(text => text.Length > 0));
        var actions = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 4, VerticalAlignment = VerticalAlignment.Center };
        DeviceTree.Sync(actions, new Node?[]
        {
            open ? null : Kit.Link(ctx, ctx.Url(DJ.Text(segment, "url")), "Download", LinkLook.Text, glyph: ""),
            Kit.Action(ctx, "Delete", new DeviceRequest(DeviceLink.Delete, $"{RecordingsPath(camera)}?start={DeviceLink.Escaped(DJ.Text(segment, "at"))}"), "Recording deleted.",
                new Confirm("Delete this recording?", $"Delete the recording from {started}?", "Delete"), tonal: false),
        }.OfType<Node>());
        Grid.SetColumn(actions, 1);
        return new Grid
        {
            Margin = Kit.Pad,
            ColumnSpacing = 8,
            ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto } },
            Children = { FormLook.Stack(2, FormLook.Text(started), FormLook.Caption(detail)), actions },
        };
    }
}

internal sealed class LimitBox : UserControl
{
    private readonly TextBlock _refusal = Kit.Refusal();

    public LimitBox(DeviceContext ctx, JsonObject limit)
    {
        var value = DJ.Text(limit, "value");
        var options = DJ.Options(limit, "options");
        ComboBox? select = null;
        select = Kit.Select("Recordings may use", value, options, async picked =>
        {
            if (picked == value) return;
            var lowers = long.TryParse(picked, out var next) && long.TryParse(value, out var now) && next < now;
            if (lowers && !await Kit.Ask(XamlRoot, new Confirm("Lower the limit?", "Lowering the limit deletes the oldest recordings straight away.", "Lower it")))
            {
                select!.SelectedIndex = options.ToList().FindIndex(o => o.Value == value);
                return;
            }
            select!.IsEnabled = false;
            var refused = await DeviceLink.Call(new DeviceRequest(DeviceLink.Put, "/api/recordings/limits", DeviceLink.Body(("maxBytes", picked))));
            select.IsEnabled = true;
            FormLook.Say(_refusal, refused?.Message);
            if (refused is null) ctx.Say("Recording limit saved.");
            else select.SelectedIndex = options.ToList().FindIndex(o => o.Value == value);
        });
        select.MaxWidth = 320;
        select.HorizontalAlignment = HorizontalAlignment.Left;
        Content = Kit.Padded(FormLook.Stack(4, select, FormLook.Caption(DJ.Text(limit, "used")), _refusal));
    }
}
