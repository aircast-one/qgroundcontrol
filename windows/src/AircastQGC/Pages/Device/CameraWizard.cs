using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Pages.Device;

internal sealed record WizardRefusal(string Text, bool Warn, string? Field);

internal sealed record WizardForm(string Next, bool SkipsPort, string ApplyNote)
{
    public static readonly WizardForm None = new("Continue", false, "");
}

internal sealed record PortInput(bool Open, string Text);

internal sealed record Bound(string Value, Action<string> Changed);

internal sealed class CameraWizardBox : UserControl
{
    public const int Applying = 3;

    private const string WizardView = "view.deviceWizard";
    private const string PiCamera = "rpiCamera";
    private static readonly string[] StepNames = ["Camera", "Port", "Picture"];
    private static readonly string[] CableChecks =
    [
        "The cable goes from the camera to this device's camera port",
        "The camera has power — its light is on",
        "Give it 30 seconds after power-on",
    ];

    private readonly JsonObject _view;
    private readonly bool _scan;
    private readonly Action? _cancel;
    private readonly Action<int> _onStep;
    private readonly Action<string> _done;
    private readonly string _client = $"device.wizard.{Guid.NewGuid():N}";
    private readonly StackPanel _root = new() { Padding = Kit.Pad, Spacing = 12 };
    private DeviceContext? _ctx;
    private int _step = 1;
    private string _rtsp;
    private string _name;
    private string _iface;
    private string _ip = "";
    private string _transport;
    private string _dupAck = "";
    private string _base = "";
    private string _user = "";
    private string _pass = "";
    private bool _addressOpen;
    private bool _namingOpen;
    private bool _techOpen;
    private bool _portOpen;
    private bool _fileOpen;
    private bool _restOpen;
    private bool _checkedOpen;
    private bool _editRtsp;
    private bool _ifaceTouched;
    private bool _uploading;
    private bool _signInShown;
    private WizardRefusal? _refusal;
    private ImmutableDictionary<string, PortInput> _inputs = ImmutableDictionary<string, PortInput>.Empty;
    private JsonObject? _picked;
    private WizardForm _form = WizardForm.None;
    private long _session = -1;
    private string? _file;
    private JsonObject _wizard = new();
    private int _formAsked;

    public CameraWizardBox(JsonObject view, bool scan, Action? cancel, Action<int> onStep, Action<string> done)
    {
        (_view, _scan, _cancel, _onStep, _done) = (view, scan, cancel, onStep, done);
        _rtsp = DJ.Text(view, "rtsp");
        _name = DJ.Text(view, "suggestedName");
        _iface = DJ.Text(view, "interface");
        _transport = DJ.Text(view, "transport");
        _addressOpen = DJ.Flag(view, "addressOpen");
        AutomationProperties.SetName(this, IsNew ? "New camera" : $"Set up {DJ.Text(view, "name")}");
        Content = _root;
        Loaded += async (_, _) =>
        {
            BridgeWatch.Watch(_client, [WizardView], json => _ = Apply(json));
            _session = MapUiJsonLong(await DeviceLink.Result("deviceWizard.open", _view, _scan) as JsonObject);
            await Apply(await Task.Run(() => Bridge.Get(WizardView)));
        };
        Unloaded += (_, _) => BridgeWatch.Stop(_client);
    }

    public int Step => _step;

    private bool IsNew => DJ.Flag(_view, "new");

    private string CameraName => IsNew ? _name.Trim().ToLowerInvariant() : DJ.Text(_view, "name");

    private string CurrentAddress => DJ.Objs(_view, "interfaces").FirstOrDefault(port => DJ.Text(port, "value") == _iface) is { } port ? DJ.Text(port, "address") : "";

    private JsonObject ApplyState => DJ.Section(_wizard, "apply");

    private string ApplyField => DJ.Text(ApplyState, "state") == "failed" ? DJ.Text(ApplyState, "field") : "";

    private bool Locked => _picked is not null && DJ.Flag(_picked, "pathFound") && !_editRtsp;

    private static long MapUiJsonLong(JsonObject? result) => (long)(Controls.MapUiJson.Number(result?["session"]) ?? -1);

    public void Show(DeviceContext ctx)
    {
        _ctx = ctx;
        Paint();
    }

    private async Task Apply(JsonObject shown)
    {
        var now = (long)(Controls.MapUiJson.Number(shown["session"]) ?? -1) == _session && _session >= 0 ? shown : new JsonObject();
        var seen = _wizard;
        _wizard = now;
        if (DJ.Int(now, "scans") > DJ.Int(seen, "scans"))
        {
            (_picked, _base, _signInShown) = (null, "", false);
            _inputs = ImmutableDictionary<string, PortInput>.Empty;
            if (DJ.Obj(now, "adopt") is { } adopt) await Pick(adopt);
        }
        var template = DJ.Section(now, "template");
        if (DJ.Int(template, "n") > DJ.Int(DJ.Obj(seen, "template"), "n"))
        {
            if (DJ.Obj(template, "camera") is not { } found) _refusal = new WizardRefusal(DJ.Text(template, "refusal"), false, null);
            else
            {
                var url = await Pick(found);
                var camera = DJ.Text(template, "name") is { Length: > 0 } named ? named : _name;
                _name = camera;
                await Advance(url, camera);
            }
        }
        var notice = DJ.Section(now, "notice");
        if (DJ.Int(notice, "n") > DJ.Int(DJ.Obj(seen, "notice"), "n")) _ctx?.Say(DJ.Text(notice, "text"));
        var apply = DJ.Section(now, "apply");
        if (DJ.Int(apply, "n") > DJ.Int(DJ.Obj(seen, "apply"), "n"))
        {
            if (DJ.Text(apply, "state") == "live") _done(DJ.Text(apply, "camera"));
            else if (DJ.Text(apply, "state") == "signIn") AskSignIn(DJ.Text(apply, "url"), DJ.Text(apply, "text"));
        }
        RefreshForm();
        Paint();
    }

    private async void RefreshForm()
    {
        var asked = ++_formAsked;
        var result = await DeviceLink.Result("deviceWizard.form", _rtsp, _iface, _ip, CurrentAddress) as JsonObject;
        if (asked != _formAsked) return;
        _form = result is null ? WizardForm.None : new WizardForm(DJ.Text(result, "next"), DJ.Flag(result, "skipsPort"), DJ.Text(result, "applyNote"));
        Paint();
    }

    private void GoTo(int step)
    {
        _step = step;
        _onStep(step);
        Paint();
    }

    private async Task<string> Pick(JsonObject cam)
    {
        _picked = cam;
        _editRtsp = false;
        if (!DJ.Flag(cam, "pathFound")) _addressOpen = true;
        _base = DJ.Text(cam, "url");
        _signInShown = DJ.Flag(cam, "needsAuth");
        if (!_signInShown) (_user, _pass) = ("", "");
        var url = DJ.Text(await DeviceLink.Invoke("deviceWizard.signIn", DJ.Text(cam, "url"), _user, _pass), "result") is { Length: > 0 } signed ? signed : DJ.Text(cam, "url");
        if (DJ.Text(cam, "url").Length > 0) _rtsp = url;
        RefreshForm();
        Paint();
        return url;
    }

    private void AskSignIn(string url, string reason)
    {
        _base = url;
        _signInShown = true;
        _refusal = new WizardRefusal(reason, false, null);
        GoTo(1);
    }

    private void ApplySetup(string url, string camera, string port, string portIp, string how)
    {
        GoTo(Applying);
        _ = DeviceLink.Invoke("deviceWizard.apply", url, camera, port, portIp, how);
    }

    private async Task Advance(string typed, string typedName)
    {
        if (await DeviceLink.Result("deviceWizard.next", typed, typedName, _dupAck) is not JsonObject next) return;
        _rtsp = DJ.Text(next, "rtsp");
        if (next["name"] is JsonValue) _name = DJ.Text(next, "name");
        _dupAck = DJ.Text(next, "dupAck");
        if (DJ.Obj(next, "refusal") is { } refused)
        {
            _refusal = new WizardRefusal(DJ.Text(refused, "text"), DJ.Flag(refused, "warn"), refused["field"] is JsonValue ? DJ.Text(refused, "field") : null);
            if (_refusal.Field == "rtsp") (_editRtsp, _addressOpen) = (true, true);
            if (_refusal.Field == "name") _namingOpen = true;
            Paint();
        }
        else if (DJ.Text(next, "kind") == "apply")
        {
            (_refusal, _iface, _ip) = (null, "", "");
            ApplySetup(DJ.Text(next, "rtsp"), DJ.Text(next, "camera"), "", "", _transport);
        }
        else
        {
            _refusal = null;
            if (DJ.Text(next, "iface").Length > 0 && !_ifaceTouched) _iface = DJ.Text(next, "iface");
            if (_ip.Length == 0) _ip = DJ.Text(next, "ip");
            RefreshForm();
            GoTo(2);
        }
    }

    private void Restart()
    {
        _ = DeviceLink.Invoke("deviceWizard.restart");
        _rtsp = "";
        _ip = "";
        _transport = DJ.Options(_view, "transports").FirstOrDefault()?.Value ?? DJ.Text(_view, "transport");
        (_refusal, _picked, _base, _signInShown, _editRtsp) = (null, null, "", false, false);
        GoTo(1);
    }

    private async void Credentials(string user, string pass)
    {
        (_user, _pass) = (user, pass);
        var from = _base;
        if (from.Length == 0) return;
        var url = DJ.Text(await DeviceLink.Invoke("deviceWizard.signIn", from, user, pass), "result");
        if (_user == user && _pass == pass && url.Length > 0)
        {
            _rtsp = url;
            Paint();
        }
    }

    private PortInput Input(JsonObject port) => _inputs.GetValueOrDefault(DJ.Text(port, "iface"), new PortInput(DJ.Flag(port, "open"), DJ.Text(port, "suggest")));

    private void SetInput(JsonObject port, PortInput input)
    {
        _inputs = _inputs.SetItem(DJ.Text(port, "iface"), input);
        Paint();
    }

    private async void Upload()
    {
        if (_ctx is not { } ctx) return;
        if (CameraName.Length == 0) { ctx.Say("Name the camera first."); return; }
        if (_file is not { } file) { ctx.Say("Choose a video file."); return; }
        _uploading = true;
        Paint();
        var sent = await DeviceLink.Upload(DeviceLink.Put, $"/api/streams/{DeviceLink.Escaped(CameraName)}/file", new JsonArray(DeviceLink.FilePart("file", file)));
        _uploading = false;
        Paint();
        if (sent.Refusal is null)
        {
            ctx.Say("Video source added.");
            _done(CameraName);
        }
        else ctx.Say(sent.Refusal == DeviceLink.NoAnswer ? "Upload failed — check the file size and try again."
            : DJ.Text(sent.Result, "error") is { Length: > 0 } error ? error : "Upload failed.");
    }

    private async void ChooseFile()
    {
        if (await FilePicks.Open(XamlRoot, [".mp4", ".mkv", ".mov", ".ts", FilePicks.Any]) is not { } picked) return;
        _file = picked;
        Paint();
    }

    private void Paint()
    {
        if (_ctx is null) return;
        DeviceTree.Sync(_root, new Node?[] { Steps() }.Concat(_step switch
        {
            1 => Camera(),
            2 => Port(),
            _ => Picture(),
        }).OfType<Node>());
    }

    private Node Steps()
    {
        var skips = _step == 1 && _form.SkipsPort;
        return DeviceTree.Leaf("steps", DJ.Sig(_step, skips), () => Kit.Flow(StepNames.Select((label, at) =>
        {
            var skipped = at == 1 && skips;
            var bar = new Border
            {
                Width = 24,
                Height = 6,
                CornerRadius = new CornerRadius(3),
                Opacity = skipped ? 0.4 : 1,
                Background = FormLook.Brush(at + 1 <= _step ? "AccentBrush" : "ControlStrokeBrush"),
                VerticalAlignment = VerticalAlignment.Center,
            };
            var text = FormLook.Caption(skipped ? $"{label} (skipped)" : label, at + 1 == _step ? "TextPrimaryBrush" : "TextSecondaryBrush");
            return (UIElement?)FormLook.Line(8, bar, text);
        }).ToArray()));
    }

    private Node Field(string key, string label, string value, Action<string> changed, string placeholder = "", string hint = "", bool error = false) =>
        DeviceTree.Kept($"field:{key}", () =>
        {
            var box = FormLook.Field("", label, placeholder);
            box.IsSpellCheckEnabled = false;
            AutomationProperties.SetName(box, label);
            box.TextChanged += (_, _) =>
            {
                if (box.Tag is Bound bound && box.Text != bound.Value) bound.Changed(box.Text);
            };
            return box;
        }, box =>
        {
            box.Tag = new Bound(value, changed);
            if (box.Text != value) box.Text = value;
            box.Header = label;
            box.PlaceholderText = placeholder;
            box.Description = hint.Length == 0 ? null : FormLook.Caption(hint, error ? "CriticalBrush" : "TextTertiaryBrush");
            box.BorderBrush = error ? FormLook.Brush("CriticalBrush") : null;
        });

    private Node Secret(string key, string label, string value, Action<string> changed) =>
        DeviceTree.Kept($"secret:{key}", () =>
        {
            var box = new PasswordBox { Header = label };
            AutomationProperties.SetName(box, label);
            box.PasswordChanged += (_, _) =>
            {
                if (box.Tag is Bound bound && box.Password != bound.Value) bound.Changed(box.Password);
            };
            return box;
        }, box =>
        {
            box.Tag = new Bound(value, changed);
            if (box.Password != value) box.Password = value;
        });

    private Node Fold(string key, string title, bool open, Action<bool> toggle, Func<IEnumerable<Node?>> content, string summary = "") =>
        DeviceTree.Kept($"fold:{key}", () => new FoldBox(), box => box.Show(title, summary, open, toggle, content));

    private Node Lead(string key, bool leads, string text, Action press, bool enabled = true) =>
        Kit.Button(key, text, press, leads ? ButtonLook.Accent : ButtonLook.Standard, enabled);

    private static Node Boxed(string key, string signature, Func<UIElement> content) =>
        DeviceTree.Leaf(key, signature, () => new Border
        {
            BorderBrush = FormLook.Brush("ControlStrokeBrush"),
            BorderThickness = new Thickness(1),
            CornerRadius = FormLook.Radius("RadiusOverlay"),
            Padding = new Thickness(16, 12, 16, 12),
            Child = content(),
        });

    private IEnumerable<Node?> Camera()
    {
        var pi = DJ.Obj(_view, "piCamera");
        var templates = DJ.Objs(_wizard, "templates");
        var busy = DJ.Flag(_wizard, "templateBusy");
        var checklist = DJ.Text(_wizard, "checklist");
        var scanning = DJ.Flag(_wizard, "scanning");
        var cams = DJ.Objs(_wizard, "cams");
        var details = DJ.Text(_wizard, "details");
        var lookingFor = DJ.Text(_view, "lookingFor");
        return new Node?[]
        {
            pi is null ? null : DeviceTree.Leaf("pi", DJ.Sig(pi), () => new Border
            {
                Background = FormLook.Brush("ControlBrush"),
                CornerRadius = FormLook.Radius("RadiusOverlay"),
                Padding = new Thickness(16, 12, 16, 12),
                Child = FormLook.Row(FormLook.Text(DJ.Text(pi, "note")), FormLook.Button(DJ.Text(pi, "action"), async () => await Advance(PiCamera, _name), ButtonLook.Accent)),
            }),
        }
        .Concat(templates.Select(template => (Node?)Make(template, busy)))
        .Concat(new Node?[]
        {
            DJ.Text(_wizard, "templateNote") is { Length: > 0 } templateNote ? Kit.Hint(templateNote) : null,
            lookingFor.Length > 0 ? Kit.StatusLine(lookingFor, DeviceLevel.Warn) : null,
            checklist.Length == 0 ? Kit.Hint("Plug the camera in and look for it.") : null,
            checklist.Length == 0 ? Kit.Buttons("scan", [Lead("scan", _rtsp.Trim().Length == 0 && _picked is null, scanning ? "Looking…" : "Scan for cameras",
                () => _ = DeviceLink.Invoke("deviceWizard.scan"), !scanning)]) : null,
            Kit.Hint(DJ.Text(_wizard, "note")),
            checklist.Length == 0 ? null : Kit.Group("checklist", [
                Kit.Sub(checklist),
                .. CableChecks.Select(check => Kit.Text($"☐ {check}")),
                Kit.Buttons("again", [Kit.Button("again", scanning ? "Looking…" : "Scan again", () => _ = DeviceLink.Invoke("deviceWizard.scan"), ButtonLook.Accent, !scanning)]),
            ]),
            cams.Count == 0 ? null : Kit.Sub(DJ.Text(_wizard, "camHeading")),
        })
        .Concat(cams.Select(cam =>
        {
            var selected = _picked is not null && DJ.Sig(_picked) == DJ.Sig(cam);
            return (Node?)DeviceTree.Leaf($"cam:{DJ.Text(cam, "host")} {DJ.Text(cam, "url")}", DJ.Sig(cam, selected), () =>
            {
                var words = FormLook.Stack(2, Kit.MonoText(DJ.Text(cam, "host")), FormLook.Caption(DJ.Text(cam, "note")));
                var button = FormLook.Stretched(new Grid
                {
                    Padding = new Thickness(16, 12, 16, 12),
                    ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto } },
                    Children = { words, Right(FormLook.Text(selected ? "Selected" : DJ.Text(cam, "action"), "FontSizeBody", "AccentTextBrush", true)) },
                }, DJ.Text(cam, "host"), async () => await Pick(cam));
                button.BorderBrush = FormLook.Brush(selected ? "AccentBrush" : "ControlStrokeBrush");
                button.BorderThickness = new Thickness(1);
                button.CornerRadius = FormLook.Radius("RadiusOverlay");
                return button;
            });
        }))
        .Concat(new Node?[]
        {
            Kit.Hint(DJ.Text(_wizard, "camsHint")),
            details.Length == 0 ? null : Fold("tech", details, _techOpen, open => { _techOpen = open; Paint(); }, ScanDetails),
            !_signInShown ? null : Kit.Group("sign-in", [
                Kit.Hint("This camera asked for a sign-in. Enter the username and password set in the camera's own app."),
                Field("user", "Camera username", _user, user => Credentials(user, _pass)),
                Secret("pass", "Camera password", _pass, pass => Credentials(_user, pass)),
            ]),
            Locked
                ? DeviceTree.Leaf("locked", _base, () => FormLook.Row(
                    FormLook.Lead(8, FormLook.Caption("Stream"), Kit.MonoText(_base)),
                    FormLook.Button("Change", () => { (_editRtsp, _addressOpen) = (true, true); Paint(); }, ButtonLook.Subtle)))
                : Fold("address", "I know my camera's address", _addressOpen, open => { _addressOpen = open; Paint(); }, () => [
                    Field("rtsp", "Camera RTSP URL", _rtsp, text => { _rtsp = text; RefreshForm(); }, DJ.Text(_view, "rtspHint"),
                        _picked is null ? "Find it in the camera's app or manual — for example rtsp://192.168.8.10:554/stream" : "",
                        _refusal?.Field == "rtsp" || ApplyField == "rtsp"),
                ]),
            !IsNew ? null : Fold("name", $"Named {(_name.Trim().ToLowerInvariant() is { Length: > 0 } n ? n : "…")} — change", _namingOpen, open => { _namingOpen = open; Paint(); }, () => [
                Field("name", "Camera name", _name, text => { _name = text; Paint(); }, "cam",
                    $"Used in the stream URLs (rtsp://…/{DJ.Text(_view, "suggestedName")}) — lowercase letters, numbers, dashes.",
                    _refusal?.Field == "name" || ApplyField == "name"),
            ]),
            _refusal is null ? null : DeviceTree.Leaf("refusal", DJ.Sig(_refusal.Text, _refusal.Warn), () => FormLook.Lead(8,
                FormLook.Glyph("", 16, _refusal.Warn ? "CautionBrush" : "CriticalBrush"),
                FormLook.Text(_refusal.Text, "FontSizeBody", _refusal.Warn ? "CautionBrush" : "CriticalBrush"))),
            Kit.Buttons("continue", [
                Lead("next", _rtsp.Trim().Length > 0, _form.Next, () => _ = Advance(_rtsp, _name)),
                _cancel is null ? null : Kit.Button("cancel", "Cancel", _cancel, ButtonLook.Subtle),
            ]),
            Fold("file", "No camera? Use a video file", _fileOpen, open => { _fileOpen = open; Paint(); }, () => [
                Kit.Hint("Upload a video and the device loops it as a stream — a stand-in camera for trying out detection and the overlays."),
                Kit.Buttons("file", [
                    Kit.Button("choose", "Choose a video file", ChooseFile),
                    DeviceTree.Leaf("chosen", _file ?? "", () => FormLook.Caption(_file is null ? "H.264 .mp4 works best" : System.IO.Path.GetFileName(_file))),
                ]),
                Kit.Buttons("upload", [Kit.Button("upload", _uploading ? "Uploading…" : "Upload & start", Upload, ButtonLook.Standard, !_uploading)]),
            ]),
        });
    }

    private Node Make(JsonObject template, bool busy) =>
        DeviceTree.Leaf($"make:{DJ.Text(template, "id")}", DJ.Sig(template, busy), () =>
        {
            var button = FormLook.Stretched(new Grid
            {
                Padding = new Thickness(16, 12, 16, 12),
                ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto } },
                Children =
                {
                    FormLook.Stack(2, FormLook.Text(DJ.Text(template, "label")), FormLook.Caption(DJ.Text(template, "sub"))),
                    Right(FormLook.Text(DJ.Text(template, "action"), "FontSizeBody", "AccentTextBrush", true)),
                },
            }, DJ.Text(template, "label"), () =>
            {
                _refusal = null;
                _ = DeviceLink.Invoke("deviceWizard.template", DJ.Text(template, "id"));
            });
            button.IsEnabled = !busy;
            button.Opacity = busy ? 0.6 : 1;
            button.BorderBrush = FormLook.Brush("ControlStrokeBrush");
            button.BorderThickness = new Thickness(1);
            button.CornerRadius = FormLook.Radius("RadiusOverlay");
            return button;
        });

    private static FrameworkElement Right(FrameworkElement element)
    {
        element.VerticalAlignment = VerticalAlignment.Center;
        Grid.SetColumn(element, 1);
        return element;
    }

    private IEnumerable<Node?> ScanDetails()
    {
        var todo = DJ.Objs(_wizard, "todo");
        var rest = DJ.Objs(_wizard, "rest");
        var cams = DJ.Objs(_wizard, "cams");
        var coverage = DJ.Text(_view, "coverage");
        IEnumerable<Node?> Rest() => rest.Select(port => (Node?)Boxed($"rest:{DJ.Text(port, "iface")}", DJ.Sig(port), () =>
            FormLook.Stack(2, FormLook.Text(DJ.Text(port, "label")), FormLook.Caption(DJ.Text(port, "detail")))));
        return new Node?[] { todo.Count > 0 ? Kit.Sub(DJ.Text(_wizard, "todoHeading")) : null }
            .Concat(todo.Select(port => (Node?)Kit.Group($"fix:{DJ.Text(port, "iface")}", PortFix(port))))
            .Append(rest.Count > 0 && todo.Count > 0 ? Fold("rest", DJ.Text(_wizard, "restTitle"), _restOpen, open => { _restOpen = open; Paint(); }, Rest) : null)
            .Concat(rest.Count > 0 && todo.Count == 0 && cams.Count > 0 ? Rest().Append(Kit.Hint(coverage)) : [])
            .Concat(rest.Count > 0 && todo.Count == 0 && cams.Count == 0 ? Rest().Prepend(Kit.Sub("Ports checked")) : [])
            .Append(cams.Count == 0 ? Fold("checked", "What was checked?", _checkedOpen, open => { _checkedOpen = open; Paint(); }, () => [Kit.Hint(coverage)]) : null);
    }

    private IEnumerable<Node?> PortFix(JsonObject port)
    {
        var input = Input(port);
        var iface = DJ.Text(port, "iface");
        return new Node?[]
        {
            DeviceTree.Leaf("port", DJ.Sig(DJ.Text(port, "label"), DJ.Text(port, "detail")), () =>
                FormLook.Stack(2, FormLook.Text(DJ.Text(port, "label"), "FontSizeBody", "AccentTextBrush"), FormLook.Caption(DJ.Text(port, "detail")))),
        }.Concat(input.Open
            ? new Node?[]
            {
                Field($"port:{iface}", "This port's address", input.Text, text => SetInput(port, input with { Text = text }), "192.168.144.2/24"),
                Kit.Buttons("chips", DJ.Objs(port, "options").Select(option => (Node?)Kit.Button($"cidr:{DJ.Text(option, "cidr")}", DJ.Text(option, "text"),
                    () => SetInput(port, Input(port) with { Text = DJ.Text(option, "cidr") }), ButtonLook.Subtle))),
                Kit.Hint(DJ.Text(port, "hint")),
                Kit.Buttons("go", [Lead("go", DJ.Text(port, "suggest").Length > 0, DJ.Text(port, "go"),
                    () => _ = DeviceLink.Invoke("deviceWizard.fixPort", iface, Input(port).Text.Trim()), !DJ.Flag(port, "busy") && input.Text.Trim().Length > 0)]),
                Kit.Hint(DJ.Text(port, "status")),
            }
            : [Kit.Buttons("change", [Kit.Button("change", "Change this port's address", () => SetInput(port, input with { Open = true }))])]);
    }

    private IEnumerable<Node?> Port()
    {
        var ports = DJ.Objs(_view, "interfaces").Select(port => new Option(DJ.Text(port, "value"), DJ.Text(port, "label"))).Prepend(new Option("", "Select a port")).ToList();
        return
        [
            DeviceTree.Leaf("port-select", DJ.Sig(_iface, ports.Count), () => Kit.Select("Which port is the camera plugged into?", _iface, ports, value =>
            {
                (_iface, _ifaceTouched) = (value, true);
                RefreshForm();
            })),
            Kit.Hint("Usually the Ethernet port your camera's cable connects to. Each port shows its own address — the camera has to be on that same subnet to be reachable."),
            Fold("port", "Port address & transport", _portOpen, open => { _portOpen = open; Paint(); }, () => [
                Field("ip", "This device's IP on the camera subnet", _ip, text => { _ip = text; RefreshForm(); }, "192.168.8.2/24",
                    "Suggested from the camera's address — edit if your subnet differs.", ApplyField == "address"),
                DeviceTree.Leaf("transport", _transport, () => Kit.Select("RTSP transport", _transport, DJ.Options(_view, "transports"), value => _transport = value)),
                Kit.Hint("Leave on Automatic; switch to TCP if the video stays black or choppy."),
            ], "This device's address must be on the same subnet as the camera's URL above — change it only if yours differs."),
            Kit.Buttons("apply", [
                Kit.Button("back", "← Back", () => GoTo(1)),
                Kit.Button("verify", "Set up & verify", () => ApplySetup(_rtsp, CameraName, _iface, _ip, _transport), ButtonLook.Accent),
            ]),
            Kit.Hint(_form.ApplyNote),
        ];
    }

    private IEnumerable<Node?> Picture()
    {
        var failed = DJ.Text(ApplyState, "state") == "failed";
        var text = DJ.Text(ApplyState, "text") is { Length: > 0 } said ? said : "Applying…";
        return
        [
            DeviceTree.Leaf("applying", DJ.Sig(failed, text), () => FormLook.Lead(12,
                failed ? FormLook.Glyph("", 20, "CriticalBrush") : new ProgressRing { IsActive = true, Width = 20, Height = 20 },
                FormLook.Text(text, "FontSizeBodyLarge", failed ? "CriticalBrush" : "TextPrimaryBrush"))),
            Kit.Buttons("after", [
                Kit.Button("over", "Start over", Restart, ButtonLook.Subtle),
                failed ? Kit.Button("change", "Change settings", () => GoTo(2)) : null,
                failed ? Kit.Button("again", "Try again", () => ApplySetup(_rtsp, CameraName, _iface, _ip, _transport), ButtonLook.Accent) : null,
            ]),
        ];
    }
}

internal sealed class FoldBox : UserControl
{
    private readonly TextBlock _title = FormLook.Text("", "FontSizeBody", "TextSecondaryBrush", true);
    private readonly TextBlock _summary = FormLook.Caption("", "TextTertiaryBrush");
    private readonly FontIcon _chevron = FormLook.Glyph("", 12);
    private readonly StackPanel _body = new() { Spacing = 12, Padding = new Thickness(0, 0, 0, 8) };
    private readonly Button _head;
    private Action<bool> _toggle = _ => { };
    private bool _open;

    public FoldBox()
    {
        _title.Visibility = Visibility.Visible;
        var words = FormLook.Stack(2, _title, _summary);
        words.VerticalAlignment = VerticalAlignment.Center;
        _chevron.VerticalAlignment = VerticalAlignment.Center;
        Grid.SetColumn(_chevron, 1);
        _head = FormLook.Stretched(new Grid
        {
            MinHeight = 44,
            Padding = new Thickness(0, 8, 8, 8),
            ColumnSpacing = 12,
            ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto } },
            Children = { words, _chevron },
        }, "", () => _toggle(!_open));
        Content = FormLook.Stack(0, _head, _body);
    }

    public void Show(string title, string summary, bool open, Action<bool> toggle, Func<IEnumerable<Node?>> content)
    {
        (_open, _toggle) = (open, toggle);
        _title.Text = title;
        FormLook.Say(_summary, summary);
        _chevron.Glyph = open ? "" : "";
        AutomationProperties.SetName(_head, $"{(open ? "Collapse" : "Expand")} {title}");
        _body.Visibility = FormLook.Shown(open);
        if (open) DeviceTree.Sync(_body, content().OfType<Node>());
    }
}
