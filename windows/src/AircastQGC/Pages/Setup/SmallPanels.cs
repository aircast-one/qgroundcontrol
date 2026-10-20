using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices.WindowsRuntime;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media.Imaging;

namespace AircastQGC.Pages.Setup;

internal sealed class RemoteSupportPanel : UserControl
{
    private const string HostFact = "settings.mavlinkSettings.forwardMavlinkAPMSupportHostName";
    private const string CreateSupport = "links.createMavlinkForwardingSupportLink";
    private const string UntilRestart = "Forwarding traffic: Mavlink traffic will keep being forwarded until application restarts";
    private static readonly TimeSpan Debounce = TimeSpan.FromMilliseconds(250);

    private readonly DispatcherTimer _debounce = new() { Interval = Debounce };
    private readonly FormView _host = SetupUi.Form(() => Bridge.Get($"view.control({HostFact})"),
        raw => SetupUi.Control(raw) is { } control && MapUiJson.Text(raw["kind"]) == "object" ? [SetupUi.Section("", "", SetupUi.Block("", control))] : [],
        loading: "Reading the support address.");
    private readonly ContentControl _state = new() { HorizontalContentAlignment = HorizontalAlignment.Stretch, IsTabStop = false };
    private readonly TextBlock _hint = FormLook.Caption("");
    private readonly TextBlock _until = FormLook.Text("");
    private readonly Button _connect = FormLook.Button("Connect", () => SetupUi.Detached(() => FormWrites.Ok(CreateSupport)), ButtonLook.Accent);
    private bool _forwarding;
    private string _typed = "";
    private (bool Valid, string Error)? _verdict;
    private int _asks;

    public RemoteSupportPanel()
    {
        _connect.HorizontalAlignment = HorizontalAlignment.Right;
        Content = SetupUi.Page(_state, _host, _hint, _connect, _until);
        _debounce.Tick += (_, _) =>
        {
            _debounce.Stop();
            Ask();
        };
        Live.Watch(this, "view.links", view =>
        {
            var forwarding = MapUiJson.Flag(view["supportForwarding"]);
            if (forwarding == _forwarding && _state.Content is not null) return;
            _forwarding = forwarding;
            Render();
        });
        Live.Watch(this, $"view.control({HostFact})", view =>
        {
            var typed = FormJson.Shown(view["valueString"]);
            if (typed == _typed && _verdict is not null) return;
            _typed = typed;
            if (typed.Length > 0 && typed != typed.Trim())
            {
                _verdict = (false, "Remove the space before or after the address.");
                Render();
                return;
            }
            _debounce.Stop();
            _debounce.Start();
        });
    }

    private async void Ask()
    {
        var ask = ++_asks;
        var host = _typed;
        var answer = await FormWrites.Off(() => Bridge.Get($"view.supportHost({host})"));
        if (ask != _asks) return;
        _verdict = MapUiJson.Text(answer["class"]) == "SupportHost" ? (MapUiJson.Flag(answer["valid"]), MapUiJson.Text(answer["error"])) : null;
        Render();
    }

    private void Render()
    {
        var forwarding = _forwarding;
        var valid = _verdict?.Valid == true;
        _state.Content = FormLook.Empty("", forwarding ? "Forwarding to support" : "Not forwarding", "Sends live telemetry, including position, to an ArduPilot support engineer.");
        _connect.IsEnabled = !forwarding && valid;
        FormLook.Say(_hint, forwarding || valid ? null : _verdict?.Error is { Length: > 0 } error ? error : "Enter the address your support engineer gave you first.");
        FormLook.Say(_until, forwarding ? UntilRestart : null);
    }
}

internal sealed class OpticalFlowPanel : UserControl
{
    private const string View = "view.opticalFlow";
    private static readonly TimeSpan Poll = TimeSpan.FromMilliseconds(500);

    private readonly Image _image = new() { Stretch = Microsoft.UI.Xaml.Media.Stretch.Uniform, MaxWidth = 360 };
    private int _index = -1;

    public OpticalFlowPanel()
    {
        var title = FormLook.Text("Optical flow camera", "FontSizeBodyLarge", "TextPrimaryBrush", true);
        title.HorizontalAlignment = HorizontalAlignment.Center;
        _image.HorizontalAlignment = HorizontalAlignment.Center;
        Microsoft.UI.Xaml.Automation.AutomationProperties.SetName(_image, "Optical flow camera image");
        Content = SetupUi.Page(title, _image);
        _ = Live.Poll(this, Poll, () => Bridge.Get(View), Apply);
    }

    private void Apply(JsonObject view)
    {
        var index = (int)(MapUiJson.Number(view["index"]) ?? 0);
        if (index == _index) return;
        _index = index;
        if (view["image"] is not JsonObject image) return;
        var width = (int)(MapUiJson.Number(image["width"]) ?? 0);
        var height = (int)(MapUiJson.Number(image["height"]) ?? 0);
        var rgba = Decode(MapUiJson.Text(image["data"]));
        if (width <= 0 || height <= 0 || rgba.Length < width * height * 4) return;
        var bgra = Enumerable.Range(0, width * height).SelectMany(pixel => new[] { rgba[pixel * 4 + 2], rgba[pixel * 4 + 1], rgba[pixel * 4], rgba[pixel * 4 + 3] }).ToArray();
        var bitmap = new WriteableBitmap(width, height);
        using (var stream = bitmap.PixelBuffer.AsStream()) stream.Write(bgra, 0, bgra.Length);
        bitmap.Invalidate();
        _image.Source = bitmap;
    }

    private static byte[] Decode(string base64)
    {
        try { return Convert.FromBase64String(base64); }
        catch (FormatException) { return []; }
    }
}

internal sealed partial class SyslinkPanel : UserControl
{
    private const string View = "view.syslink";
    private static readonly TimeSpan Poll = TimeSpan.FromSeconds(1);

    private readonly Poller _poller;
    private string? _refusal;
    private JsonObject _view = new();

    [GeneratedRegex("^[0-9A-Fa-f]{0,10}$")]
    private static partial Regex Hex();

    public SyslinkPanel()
    {
        _poller = Live.Changes(this, Poll, () => Bridge.Get(View), view =>
        {
            _view = view;
            Render();
        });
    }

    private void Render()
    {
        if (!MapUiJson.Flag(_view["available"]))
        {
            Content = SetupUi.Notice("This vehicle has no Syslink radio.");
            return;
        }
        var rates = MapUiJson.Texts(_view["rates"]);
        Content = SetupUi.Page(
            SetupUi.Heading("Radio Settings"),
            FormLook.Card(
                SetupUi.Entry("Channel", ((int)(MapUiJson.Number(_view["channel"]) ?? 0)).ToString(CultureInfo.InvariantCulture),
                    typed => { if (int.TryParse(typed, NumberStyles.Integer, CultureInfo.InvariantCulture, out var channel)) Act("syslink.setChannel", channel); },
                    hint: MapUiJson.Text(_view["channelHint"]), accept: typed => typed.All(char.IsDigit)),
                SetupUi.Entry("Address", MapUiJson.Text(_view["address"]), typed => Act("syslink.setAddress", typed),
                    hint: MapUiJson.Text(_view["addressHint"]), accept: typed => Hex().IsMatch(typed)),
                SetupUi.Choice("Data Rate", rates, (int)(MapUiJson.Number(_view["rate"]) ?? -1), at => Act("syslink.setRate", at))),
            _refusal is null ? null : FormLook.Text(_refusal, "FontSizeBody", "CriticalBrush"),
            FormLook.Button("Restore defaults", () => Act("syslink.resetDefaults")));
    }

    private void Act(string path, params object?[] args) => SetupUi.Run(() => SetupUi.Refusal(path, args), refused =>
    {
        _refusal = refused;
        _poller.Now();
        Render();
    });
}

internal sealed class EspBridgePanel : UserControl
{
    private const string View = "view.espBridge";
    private static readonly TimeSpan Poll = TimeSpan.FromSeconds(1);
    private static readonly IReadOnlyList<string> WifiModes = ["Access point mode", "Station mode"];
    private const int StationMode = 1;

    private readonly ContentControl _settings = new() { HorizontalContentAlignment = HorizontalAlignment.Stretch, IsTabStop = false };
    private readonly ContentControl _status = new() { HorizontalContentAlignment = HorizontalAlignment.Stretch, IsTabStop = false };
    private readonly TextBlock _refusal = SetupUi.Error();
    private readonly UIElement _layout;
    private string _settingsKey = "";
    private bool? _available;

    public EspBridgePanel()
    {
        _layout = SetupUi.Page(SetupUi.Heading("ESP WiFi bridge settings"), _settings, SetupUi.Heading("ESP WiFi bridge status"), _status, _refusal);
        Loaded += (_, _) => SetupUi.Detached(() => FormWrites.Ok("espBridge.open"));
        _ = Live.Changes(this, Poll, () => Bridge.Get(View), Apply);
    }

    private void Apply(JsonObject view)
    {
        var available = MapUiJson.Flag(view["available"]);
        if (available != _available)
        {
            _available = available;
            _settingsKey = "";
            Content = available ? _layout : SetupUi.Notice("No WiFi bridge is connected.");
        }
        if (!available) return;
        var key = new JsonObject(view.Where(kv => kv.Key is not ("status" or "busy")).Select(kv => KeyValuePair.Create(kv.Key, kv.Value?.DeepClone()))).ToJsonString();
        if (key != _settingsKey)
        {
            _settingsKey = key;
            _settings.Content = Settings(view);
        }
        _status.Content = Status(view);
    }

    private UIElement Settings(JsonObject view)
    {
        var mode = MapUiJson.Number(view["modeIndex"]) is { } index ? (int)index : (int?)null;
        var modePath = MapUiJson.Text(view["modePath"]);
        var channelPath = MapUiJson.Text(view["channelPath"]);
        var hostPort = view["hostPort"] as JsonObject;
        var hostPortPath = MapUiJson.Text(hostPort?["path"]);
        var station = mode == StationMode;
        return FormLook.Card(new UIElement?[]
        {
            mode is { } chosen ? SetupUi.Choice("WiFi mode", WifiModes, chosen, at => Set($"{modePath}.rawValue", at)) : null,
            SetupUi.Choice("WiFi channel", Enumerable.Range(1, 11).Select(c => c.ToString(CultureInfo.InvariantCulture)).ToList(),
                (int)(MapUiJson.Number(view["channel"]) ?? 1) - 1, at => Set($"{channelPath}.rawValue", at + 1), (mode ?? 0) == 0),
            Text("WiFi AP SSID", MapUiJson.Text(view["ssid"]), "ssid", true),
            Text("WiFi AP password", MapUiJson.Text(view["password"]), "password", true),
            Text("WiFi STA SSID", MapUiJson.Text(view["ssidSta"]), "ssidSta", station),
            Text("WiFi STA password", MapUiJson.Text(view["passwordSta"]), "passwordSta", station),
            SetupUi.Choice("UART baud rate", (view["baudRates"] as JsonArray ?? []).Select(b => FormJson.Shown(b)).ToList(),
                (int)(MapUiJson.Number(view["baudIndex"]) ?? 4), at => Act("espBridge.baud", at)),
            SetupUi.Entry("QGC UDP port", FormJson.Shown(hostPort?["valueString"]), typed =>
            {
                if (int.TryParse(typed, NumberStyles.Integer, CultureInfo.InvariantCulture, out var port) && port is >= 1024 and <= 65535) Set(hostPortPath, port);
            }, hostPortPath.Length > 0, accept: typed => typed.Length <= 5 && typed.All(char.IsDigit)),
        }.OfType<UIElement>());
    }

    private UIElement Text(string label, string value, string field, bool enabled) =>
        SetupUi.Entry(label, value, typed => Act("espBridge.setText", field, typed), enabled, accept: typed => typed.Length <= 16);

    private UIElement Status(JsonObject view)
    {
        var status = view["status"] as JsonObject;
        string Count(JsonNode? node) => MapUiJson.Number(node) is { } n ? ((long)n).ToString("N0", CultureInfo.InvariantCulture) : "";
        var reboot = FormLook.Button("Restart WiFi bridge", () => Reboot(MapUiJson.Text(view["rebootPrompt"])));
        reboot.IsEnabled = !MapUiJson.Flag(view["busy"]);
        return FormLook.Stack(8, new[] { ("Bridge/vehicle link", "vehicle"), ("Bridge/QGC link", "bridge"), ("QGC/bridge link", "qgc") }
            .Select(link => (UIElement?)FormLook.Stack(4,
                FormLook.Text(link.Item1, "FontSizeBody", "TextPrimaryBrush", true),
                FormLook.Pairs(
                [
                    ("Messages received", Count(status?[link.Item2]?["received"])),
                    ("Messages lost", Count(status?[link.Item2]?["lost"])),
                    ("Messages sent", Count(status?[link.Item2]?["sent"])),
                ])))
            .Append(FormLook.Line(8,
                FormLook.Button("Restore defaults", () => Act("espBridge.restoreDefaults")),
                reboot,
                FormLook.Button("Reset counters", () => Act("espBridge.resetCounters")))));
    }

    private async void Reboot(string prompt)
    {
        if (await FormLook.Confirm(XamlRoot, "Reboot WiFi bridge", prompt, "Yes", "No")) Act("espBridge.reboot");
    }

    private void Set(string path, object value) => SetupUi.Run(() => SetupUi.Write(path, value), refused => FormLook.Say(_refusal, refused));

    private void Act(string path, params object?[] args) => SetupUi.Run(() => SetupUi.Refusal(path, args), refused => FormLook.Say(_refusal, refused));
}

internal sealed class ScriptingPanel : UserControl
{
    private const string View = "view.scripting";
    private const string Open = "scripting.open";
    private const string Upload = "scripting.upload";
    private const string Download = "scripting.download";
    private const string Delete = "scripting.delete";
    private const string Cancel = "scripting.cancel";
    private static readonly TimeSpan BusyPoll = TimeSpan.FromMilliseconds(300);
    private static readonly TimeSpan IdlePoll = TimeSpan.FromSeconds(2);

    private readonly DispatcherTimer _timer = new() { Interval = IdlePoll };
    private JsonObject _view = new();
    private string _signature = "";
    private bool _reading;

    public ScriptingPanel()
    {
        _timer.Tick += (_, _) => Refresh();
        Loaded += async (_, _) =>
        {
            await FormWrites.Off(() => FormWrites.Ok(Open));
            Refresh();
            _timer.Start();
        };
        Unloaded += (_, _) => _timer.Stop();
    }

    private bool Busy => MapUiJson.Flag(_view["busy"]);

    private async void Refresh()
    {
        if (_reading) return;
        _reading = true;
        try
        {
            var view = await FormWrites.Off(() => Bridge.Get(View));
            _view = view;
            _timer.Interval = Busy ? BusyPoll : IdlePoll;
            var signature = view.ToJsonString();
            if (signature == _signature) return;
            _signature = signature;
            Render();
        }
        finally { _reading = false; }
    }

    private void Render()
    {
        if (MapUiJson.Text(_view["class"]) != "Scripting") return;
        if (!MapUiJson.Flag(_view["available"]))
        {
            Content = SetupUi.Notice(MapUiJson.Text(_view["unsupportedText"]));
            return;
        }
        var enabled = MapUiJson.Flag(_view["enabled"]);
        var progress = MapUiJson.Number(_view["progress"]) ?? 0;
        var upload = FormLook.Button("Upload", Pick, ButtonLook.Accent);
        upload.IsEnabled = enabled && !Busy;
        upload.HorizontalAlignment = HorizontalAlignment.Left;
        Content = SetupUi.Page(new UIElement?[]
        {
            SetupUi.Control(_view["enable"]) is { } enable ? SetupUi.Embedded([SetupUi.Titled(enable, "Enable scripting")], Refresh) : null,
            FormLook.Text(MapUiJson.Text(_view["status"])),
            upload,
            MapUiJson.Texts(_view["scripts"]) is { Count: > 0 } scripts ? FormLook.Card(scripts.Select(name => (UIElement)FormLook.Row(
                FormLook.Text(name),
                FormLook.Line(4, Action("Download", () => Save(name), enabled), Action("Delete", () => Remove(name), enabled))))) : null,
            Busy ? FormLook.Line(12, FormLook.Button("Cancel operation", () => SetupUi.Run(() => FormWrites.Ok(Cancel), _ => Refresh()), ButtonLook.Subtle),
                FormLook.Text($"Transferring... {Math.Round(progress * 100)}%")) : null,
            Busy ? SetupUi.Bar(progress) : null,
        });
    }

    private Button Action(string text, Action click, bool enabled)
    {
        var button = FormLook.Button(text, click, ButtonLook.Subtle);
        button.IsEnabled = enabled && !Busy;
        return button;
    }

    private async void Pick()
    {
        if (await FilePicks.Open(XamlRoot, [FilePicks.Any]) is not { } path) return;
        var name = Path.GetFileName(path);
        var refused = await FormWrites.Off(() => File.Exists(path) ? SetupUi.Refusal(Upload, path, name) : $"File {name} does not exist");
        Refresh();
        if (refused is not null) await FormLook.Tell(XamlRoot, "Lua Upload", refused.Trim().Length > 0 ? refused : "Upload failed");
    }

    private async void Save(string name)
    {
        if (await FilePicks.Save(XamlRoot, name, "Lua script", ".lua") is not { } target) return;
        var refused = await Task.Run(async () =>
        {
            var answer = SetupUi.Refusal(Download, name, target, Path.GetFileName(target));
            while (answer is null && MapUiJson.Flag(Bridge.Get(View)["busy"])) await Task.Delay(BusyPoll);
            return answer;
        });
        Refresh();
        if (refused is null) return;
        await FormLook.Tell(XamlRoot, "Lua Download", refused.Trim().Length > 0 ? refused : "Download failed");
    }

    private async void Remove(string name)
    {
        if (!await FormLook.Confirm(XamlRoot, "Delete Lua script", $"Are you sure you want to delete the script \"{name}\"? This action cannot be undone.", "OK")) return;
        var refused = await FormWrites.Off(() => SetupUi.Refusal(Delete, name));
        Refresh();
        if (refused is not null) await FormLook.Tell(XamlRoot, "Lua Delete", refused.Trim().Length > 0 ? refused : "Delete failed");
    }
}
