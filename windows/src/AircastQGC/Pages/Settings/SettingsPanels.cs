using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Pages.Settings;

internal static class Panels
{
    public static readonly TimeSpan Second = TimeSpan.FromSeconds(1);

    public static long Whole(JsonNode? node) => (long)(MapUiJson.Number(node) ?? 0);

    public static string Fixed(double value, int decimals) => value.ToString($"F{decimals}", CultureInfo.InvariantCulture);

    public static string DataSize(long bytes) =>
        bytes < 1024 ? $"{bytes} B"
        : bytes < 1048576 ? $"{Fixed(bytes / 1024.0, 1)} KB"
        : $"{Fixed(bytes / 1048576.0, 1)} MB";

    public static string DataRate(double bytesPerSec) =>
        bytesPerSec < 1024 ? $"{Fixed(bytesPerSec, 0)} B/s" : $"{Fixed(bytesPerSec / 1024, 1)} KB/s";

    public static VariableSizedWrapGrid Wrapped(IEnumerable<UIElement> items) =>
        FormLook.Fill(new VariableSizedWrapGrid { Orientation = Orientation.Horizontal }, items);
}

internal sealed class NtripStatusPanel : UserControl
{
    public const string View = "view.ntrip";
    private const string ConnectSetting = "settings.ntripSettings.ntripServerConnectEnabled";
    private const string NoData = "Connected but no data received recently";

    private readonly Button _button;
    private readonly TextBlock _label = new() { FontFamily = PreflightLook.Font, FontSize = FormLook.Size("FontSizeBody") };
    private readonly Border _dot = FormLook.Dot("TextTertiaryBrush", 10);
    private readonly TextBlock _message = FormLook.Text("Disconnected");
    private readonly StackPanel _details = new();
    private bool _active;

    public NtripStatusPanel(Action written)
    {
        _button = FormLook.Button("", async () =>
        {
            var connect = !_active;
            await FormWrites.Off(() => FormWrites.Set(ConnectSetting, connect));
            written();
        });
        _button.Content = _label;
        Visibility = Visibility.Collapsed;
        Content = FormLook.Stack(0,
            FormLook.Heading("Connection"),
            FormLook.Card(FormLook.Row(FormLook.Lead(10, _dot, _message), _button), _details));
        Live.Changes(this, Panels.Second, () => Bridge.Get(View), Render);
    }

    private void Render(JsonObject view)
    {
        var status = MapUiJson.Text(view["status"]);
        Visibility = FormLook.Shown(status.Length > 0);
        _active = MapUiJson.Flag(view["active"]);
        _label.Text = MapUiJson.Text(view["button"]);
        AutomationProperties.SetName(_button, _label.Text);
        _button.IsEnabled = MapUiJson.Flag(view["buttonEnabled"]);
        _dot.Background = FormLook.Brush(DotBrush(status));
        _message.Text = MapUiJson.Text(view["statusMessage"]) is { Length: > 0 } message ? message : "Disconnected";
        FormLook.Fill(_details, (status == "disconnected" ? [] : Details(view, status)).Select(row => (UIElement)new Border
        {
            BorderBrush = FormLook.Brush("DividerBrush"),
            BorderThickness = new Thickness(0, 1, 0, 0),
            Child = row,
        }));
    }

    private static IEnumerable<UIElement> Details(JsonObject view, string status)
    {
        var connected = status == "connected";
        var received = Panels.Whole(view["bytesReceived"]);
        var sent = Panels.Whole(view["bytesSent"]);
        var types = (view["messageTypes"] as JsonArray ?? [])
            .OfType<JsonArray>()
            .Select(pair => (Id: Panels.Whole(pair.ElementAtOrDefault(0)), Count: Panels.Whole(pair.ElementAtOrDefault(1))))
            .ToList();
        UIElement Note(string text, string brush) => FormLook.Margined(FormLook.Caption(text, brush), new Thickness(16, 8, 16, 8));
        return new UIElement?[]
        {
            connected && MapUiJson.Flag(view["dataStale"]) ? Note(NoData, "CautionBrush") : null,
            connected && MapUiJson.Text(view["mountpoint"]) is { Length: > 0 } mountpoint ? FormLook.Pair("Mountpoint", mountpoint) : null,
            connected && Panels.Whole(view["messages"]) is > 0 and var messages ? FormLook.Pair("Messages", messages.ToString(CultureInfo.InvariantCulture)) : null,
            connected && types.Count > 0
                ? FormLook.Row(FormLook.Text("Message types"), null,
                    Panels.Wrapped(types.Select(t => (UIElement)FormLook.Chip($"{(t.Id == 0 ? "unknown" : t.Id.ToString(CultureInfo.InvariantCulture))} × {t.Count}"))))
                : null,
            connected && received > 0
                ? FormLook.Pair("Data received", $"{Panels.DataSize(received)} ({Panels.DataRate(MapUiJson.Number(view["dataRateBytesPerSec"]) ?? 0)})")
                : null,
            connected && MapUiJson.Flag(view["dataWarning"])
                ? Note($"Warning: Data usage: {Panels.DataSize(received)} — consider connection costs", "CautionBrush")
                : null,
            connected && sent > 0
                ? FormLook.Pair("To vehicle", $"{Panels.DataSize(sent)} ({Panels.Fixed(MapUiJson.Number(view["sentKBps"]) ?? 0, 1)} KB/s)")
                : null,
            connected && MapUiJson.Text(view["ggaSource"]) is { Length: > 0 } gga ? FormLook.Pair("GGA source", gga) : null,
            MapUiJson.Text(view["securityWarning"]) is { Length: > 0 } warning ? Note(warning, "CautionBrush") : null,
            status == "error" ? Note(MapUiJson.Text(view["statusMessage"]), "CriticalBrush") : null,
        }.OfType<UIElement>();
    }

    private static string DotBrush(string status) => status switch
    {
        "connected" => "SuccessBrush",
        "connecting" or "reconnecting" => "CautionBrush",
        "error" => "CriticalBrush",
        _ => "TextTertiaryBrush",
    };
}

internal sealed class MountpointBrowser : UserControl
{
    private const string Fetch = "ntrip.fetchMountpoints";
    private const string Select = "ntrip.selectMountpoint";

    private readonly Action _written;
    private readonly Poller _poll;

    public MountpointBrowser(Action written)
    {
        _written = written;
        Margin = new Thickness(0, 8, 0, 0);
        _poll = Live.Changes(this, Panels.Second, () => Bridge.Get(NtripStatusPanel.View)["browser"] as JsonObject ?? new JsonObject(), Render);
    }

    private void Render(JsonObject browser)
    {
        var status = MapUiJson.Text(browser["status"]);
        var browse = FormLook.Button("Browse", async () =>
        {
            await FormWrites.Off(() => FormWrites.Ok(Fetch));
            _poll.Now();
        });
        browse.IsEnabled = MapUiJson.Flag(browser["canBrowse"]);
        var rows = MapUiJson.Objects(browser["mountpoints"]).Select(Mountpoint).ToList();
        Content = FormLook.Stack(8,
            browse,
            FormLook.Caption(status == "inProgress" ? "Fetching mountpoints…" : "", "CautionBrush"),
            FormLook.Caption(status == "error" ? MapUiJson.Text(browser["error"]) : "", "CriticalBrush"),
            rows.Count > 0 ? FormLook.Card(rows) : null);
    }

    private UIElement Mountpoint(JsonObject row)
    {
        var name = MapUiJson.Text(row["mountpoint"]);
        var selected = MapUiJson.Flag(row["selected"]);
        var pick = FormLook.Button(selected ? "Selected" : "Select", async () =>
        {
            await FormWrites.Off(() => FormWrites.Ok(Select, name));
            _poll.Now();
            _written();
        }, ButtonLook.Subtle);
        pick.IsEnabled = !selected;
        AutomationProperties.SetName(pick, $"Select {name}");
        return FormLook.Row(
            FormLook.Stack(2,
                FormLook.Line(6,
                    FormLook.Text(name, "FontSizeBody", selected ? "AccentTextBrush" : "TextPrimaryBrush"),
                    selected ? FormLook.Caption("(selected)", "AccentTextBrush") : null),
                FormLook.Caption(MapUiJson.Text(row["detail"]))),
            pick);
    }
}

internal sealed class PacketRadioPanel : UserControl
{
    private const string View = "view.packetRadio";
    private const string Refresh = "packetRadio.refreshAdapters";
    private const string Device = "settings.packetRadioSettings.deviceName.rawValue";
    private const string Automatic = "Automatic";
    private const string Waiting = "waiting";

    private readonly Action _written;
    private readonly ContentControl _status = new() { HorizontalContentAlignment = HorizontalAlignment.Stretch, IsTabStop = false };
    private readonly ContentControl _radio = new() { HorizontalContentAlignment = HorizontalAlignment.Stretch, IsTabStop = false };
    private string? _radioSignature;

    public PacketRadioPanel(Action written)
    {
        _written = written;
        Content = FormLook.Stack(0, _status, FormLook.Heading("Radio"), _radio);
        var refreshed = false;
        Loaded += (_, _) =>
        {
            if (refreshed) return;
            refreshed = true;
            FormWrites.Off(() => FormWrites.Ok(Refresh));
        };
        Live.Changes(this, Panels.Second,
            () => new JsonObject { ["radio"] = Bridge.Get(View), ["device"] = Bridge.Get(Device)["value"]?.DeepClone() },
            Render);
    }

    private void Render(JsonObject read)
    {
        var radio = read["radio"] is JsonObject view && MapUiJson.Text(view["class"]) == "PacketRadio" ? view : null;
        var device = MapUiJson.Text(read["device"]);
        var adapters = MapUiJson.Texts(radio?["adapters"]);
        _status.Content = Status(radio);
        var signature = string.Join("|", adapters.Prepend(device));
        if (signature == _radioSignature) return;
        _radioSignature = signature;
        _radio.Content = Adapter(device, adapters);
    }

    private static UIElement Status(JsonObject? radio)
    {
        var status = MapUiJson.Text(radio?["statusText"]) is { Length: > 0 } text ? text : "Unavailable";
        var signal = MapUiJson.Flag(radio?["haveSignal"]);
        return radio is null || !MapUiJson.Flag(radio["linkActive"])
            ? FormLook.Card(FormLook.Pair("Status", status))
            : FormLook.Stack(0,
                FormLook.Card(FormLook.Pair("Status", status)),
                FormLook.Heading("Link quality"),
                FormLook.Card(
                    FormLook.Pair("Signal per antenna (0-126)", signal ? Joined(radio["antennaRssiRaw"]) : Waiting),
                    FormLook.Pair("Noise margin per antenna", signal ? $"{Joined(radio["antennaSnr"])} dB" : Waiting),
                    FormLook.Pair("Link score (1000-2000)", signal && MapUiJson.Number(radio["linkScore"]) is { } score ? Panels.Fixed(score, 0) : Waiting),
                    FormLook.Pair("Packets lost (last second)", MapUiJson.Number(radio["packetLoss"]) is { } lost ? Panels.Fixed(lost, 0) : ""),
                    FormLook.Pair("Video packets", MapUiJson.Number(radio["videoPackets"]) is { } packets ? Panels.Fixed(packets, 0) : "")));
    }

    private UIElement Adapter(string device, IReadOnlyList<string> adapters)
    {
        var choices = adapters.Prepend(Automatic).ToList();
        var combo = FormLook.Combo(choices, device.Length == 0 ? 0 : choices.IndexOf(device), FormLook.ChoiceWidth, device);
        AutomationProperties.SetName(combo, "Wi-Fi adapter");
        combo.SelectionChanged += async (_, _) =>
        {
            var picked = combo.SelectedIndex;
            if (picked < 0) return;
            var name = picked == 0 ? "" : choices[picked];
            if (name == device) return;
            await FormWrites.Off(() => FormWrites.Set(Device, name));
            _written();
        };
        return FormLook.Card(FormLook.Row(FormLook.Text("Wi-Fi adapter"), combo));
    }

    private static string Joined(JsonNode? values) =>
        string.Join("  |  ", (values as JsonArray ?? []).Select(FormJson.Shown));
}

internal sealed class MavlinkActionsPanel : UserControl
{
    private const string View = "view.mavlinkActions";
    private const string NoFile = "<None>";

    private readonly Action _written;

    public MavlinkActionsPanel(Action written)
    {
        _written = written;
        Loaded += async (_, _) => Render(await FormWrites.Off(() => Bridge.Get(View)));
    }

    private void Render(JsonObject view)
    {
        if (view["files"] is not JsonArray) return;
        var options = MapUiJson.Texts(view["files"]).Prepend(NoFile).ToList();
        Content = FormLook.Stack(4,
            FormLook.Card(
                Choice("Fly view actions", MapUiJson.Text(view["flyViewFile"]), MapUiJson.Text(view["flyViewPath"]), options),
                Choice("Joystick actions", MapUiJson.Text(view["joystickFile"]), MapUiJson.Text(view["joystickPath"]), options)),
            FormLook.Footnote(MapUiJson.Text(view["folderNote"])));
    }

    private UIElement Choice(string label, string current, string path, IReadOnlyList<string> options)
    {
        var shown = current.Length > 0 ? current : NoFile;
        var combo = FormLook.Combo(options, options.ToList().IndexOf(shown), FormLook.ChoiceWidth, shown);
        combo.IsEnabled = options.Count > 1;
        AutomationProperties.SetName(combo, label);
        combo.SelectionChanged += async (_, _) =>
        {
            if (combo.SelectedIndex < 0 || options[combo.SelectedIndex] == shown) return;
            var chosen = options[combo.SelectedIndex] == NoFile ? "" : options[combo.SelectedIndex];
            Render(await FormWrites.Off(() =>
            {
                FormWrites.Set(path, chosen);
                return Bridge.Get(View);
            }));
            _written();
        };
        return FormLook.Row(FormLook.Text(label), combo);
    }
}

internal sealed class GcsPositionPanel : UserControl
{
    public const string Position = "positionManager.gcsPosition";
    private const string Accuracy = "positionManager.gcsPositionHorizontalAccuracy";

    public GcsPositionPanel() =>
        Live.Changes(this, Panels.Second,
            () => new JsonObject { ["position"] = Bridge.Get(Position), ["accuracy"] = Bridge.Get(Accuracy) },
            read => Content = Rows(read["position"] as JsonObject, read["accuracy"] as JsonObject) is { } rows
                ? FormLook.Stack(0, FormLook.Heading("GCS position"), FormLook.Pairs(rows))
                : null);

    internal static IReadOnlyList<(string, string)>? Rows(JsonObject? position, JsonObject? accuracy)
    {
        var coordinate = position?["value"] as JsonObject ?? position;
        if (!MapUiJson.Flag(coordinate?["valid"])) return null;
        var hdop = MapUiJson.Number(accuracy?["value"]) ?? 0;
        return
        [
            ("Latitude", Panels.Fixed(MapUiJson.Number(coordinate?["latitude"]) ?? 0, 7)),
            ("Longitude", Panels.Fixed(MapUiJson.Number(coordinate?["longitude"]) ?? 0, 7)),
            ("HDOP", hdop > 0 ? $"{Panels.Fixed(hdop, 1)} m" : "N/A"),
        ];
    }
}

internal sealed class LinkStatusPanel : UserControl
{
    private const string View = "view.linkStatus";

    public LinkStatusPanel() => Live.Watch(this, View, view =>
    {
        var rows = MapUiJson.Objects(view["rows"])
            .Select(row => (UIElement)FormLook.Pair(MapUiJson.Text(row["label"]), FormJson.Shown(row["value"])))
            .ToList();
        Content = FormLook.Stack(0,
            FormLook.Heading("Link status (current vehicle)"),
            rows.Count == 0 ? FormLook.Caption("Not connected") : FormLook.Card(rows));
    });
}
