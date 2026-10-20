using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading;
using System.Threading.Tasks;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Pages.Settings;

internal sealed class LinksPanel : UserControl
{
    private const int SettleTries = 27;
    private static readonly TimeSpan SettlePause = TimeSpan.FromMilliseconds(150);

    private readonly SettingsNav _nav;
    private readonly StackPanel _body = new();
    private readonly TextBlock _notice = FormLook.Caption("", "CriticalBrush");
    private JsonObject _view = new();

    public LinksPanel(SettingsNav nav, FormView footer)
    {
        _nav = nav;
        var advanced = new Expander
        {
            Header = FormLook.Text("Advanced", "FontSizeBody", "TextPrimaryBrush", true),
            Content = footer,
            HorizontalAlignment = HorizontalAlignment.Stretch,
            HorizontalContentAlignment = HorizontalAlignment.Stretch,
            Margin = new Thickness(0, 24, 0, 0),
        };
        Content = SettingsBodies.Scrolled(FormLook.Stack(0,
            _notice,
            _body,
            FormLook.Margined(FormLook.Button("Add link…", () => _nav.Push("Add link", new AddLinkPage(_nav)), ButtonLook.Accent, ""),
                new Thickness(0, 20, 0, 0)),
            advanced));
        Live.Watch(this, LinkModel.View, view =>
        {
            _view = view;
            Render();
        });
    }

    private void Render()
    {
        var rows = LinkModel.Rows(_view);
        var auto = LinkModel.AutoLinks(_view);
        var connected = rows.Where(r => r.Connected).ToList();
        var saved = rows.Where(r => !r.Connected).ToList();
        FormLook.Fill(_body, rows.Count == 0 && auto.Count == 0
            ?
            [
                FormLook.Footnote("No links saved. A USB cable or telemetry radio connects by itself; add a link for Wi-Fi or a network connection."),
            ]
            :
            [
                connected.Count + auto.Count > 0 ? FormLook.Heading("Connected") : null,
                connected.Count + auto.Count > 0 ? FormLook.Card(connected.Select(RowView).Concat(auto.Select(AutoView))) : null,
                saved.Count > 0 ? FormLook.Heading("Saved") : null,
                saved.Count > 0 ? FormLook.Card(saved.Select(RowView)) : null,
            ]);
    }

    private UIElement RowView(LinkRow row)
    {
        var more = FormLook.IconButton("", $"More actions for {row.Name}", () => { });
        var edit = new MenuFlyoutItem { Text = "Edit link", IsEnabled = row.Editable };
        var delete = new MenuFlyoutItem { Text = "Delete link" };
        edit.Click += (_, _) => _nav.Push($"Edit {row.Name}", new LinkEditor(row, _nav));
        delete.Click += (_, _) => Delete(row);
        more.Flyout = new MenuFlyout { Items = { edit, delete } };
        return FormLook.Row(
            FormLook.Lead(16, FormLook.Badge(LinkModel.Glyph(row.Type), row.Connected), FormLook.Stack(2,
                FormLook.Text(row.Name),
                FormLook.Caption(row.Connected && row.StatusLine == LinkModel.ConnectedStatus ? "" : row.StatusLine, row.GoneQuiet ? "CriticalBrush" : "TextSecondaryBrush"),
                FormLook.Caption(row.LastError, "CriticalBrush"),
                FormLook.Caption(row.LastError.Length > 0 ? LinkModel.RemedyText(row.ErrorRemedy) ?? "" : ""))),
            FormLook.Line(8,
                row.Connected ? FormLook.Margined(FormLook.Text(LinkModel.ConnectedStatus, "FontSizeBody", "SuccessBrush", true), new Thickness(0, 6, 8, 0)) : null,
                FormLook.Button(LinkModel.PrimaryAction(row), () => Primary(row)),
                more));
    }

    private UIElement AutoView(AutoLink link) => FormLook.Row(
        FormLook.Lead(16, FormLook.Badge(LinkModel.Glyph(link.Type), link.Heard), FormLook.Stack(2,
            FormLook.Text(link.Name),
            FormLook.Caption(LinkModel.AutoLinkSubtitle(link)))),
        FormLook.Line(8,
            FormLook.Margined(FormLook.Text(LinkModel.AutoLinkStatus(link), "FontSizeBody", link.Heard ? "SuccessBrush" : "TextSecondaryBrush"), new Thickness(0, 6, 8, 0)),
            link.Type == LinkType.Mock
                ? FormLook.Button("Stop", () => Attempt("stop",
                    () => LinkModel.AutoLinks(Bridge.Get(LinkModel.View)).All(l => l.Name != link.Name),
                    () => LinkCommands.Disconnect(link.Index)))
                : null));

    private async void Primary(LinkRow row)
    {
        if (!row.Connected)
        {
            Attempt("connect", () => LinkModel.Connected(LinkCommands.Rows(), row.Index), () => LinkCommands.Connect(row.Configuration));
            return;
        }
        if (!NoVehicleState.NoVehicleNow() && !await FormLook.Confirm(XamlRoot, $"Disconnect {row.Name}?",
                "A vehicle is connected on this link. Disconnecting stops telemetry and you will not be able to command it until it reconnects.",
                "Disconnect", "Keep connected")) return;
        Attempt("disconnect", () => !LinkModel.Connected(LinkCommands.Rows(), row.Index), () => LinkCommands.Disconnect(row.Index));
    }

    private async void Delete(LinkRow row)
    {
        if (!await FormLook.Confirm(XamlRoot, $"Remove {row.Name}?",
                row.Connected
                    ? "This link is connected. Removing it disconnects it first and forgets the settings."
                    : "Aircast will forget this link and its settings.",
                "Remove")) return;
        Attempt("remove", () => LinkCommands.Rows().All(r => r.Name != row.Name), () => LinkCommands.Remove(row));
    }

    private async void Attempt(string action, Func<bool> settled, Func<string?> call)
    {
        FormLook.Say(_notice, null);
        var refused = await FormWrites.Off(call);
        var done = refused is null && await FormWrites.Off(() => Enumerable.Range(0, SettleTries).Any(_ => settled() || Paused()));
        FormLook.Say(_notice, refused ?? (done ? null : $"Could not {action} that link."));
    }

    private static bool Paused()
    {
        Thread.Sleep(SettlePause);
        return false;
    }
}

internal sealed class AddLinkPage : UserControl
{
    public AddLinkPage(SettingsNav nav)
    {
        var view = Bridge.Group(LinkModel.View);
        var offered = LinkModel.Addable(view, AircastCloud.Offered());
        var pilot = LinkModel.PilotKinds(offered, LinkModel.Ports(view).Count > 0);
        var tools = LinkModel.ToolKinds(offered);
        UIElement KindRow(LinkType type) => FormLook.Stretched(
            FormLook.Row(FormLook.Lead(16, FormLook.Badge(LinkModel.Glyph(type), false), FormLook.Stack(2,
                    FormLook.Text(LinkModel.Kind(type).Title),
                    FormLook.Caption(LinkModel.Kind(type).Detail))),
                FormLook.Glyph("", 12)),
            LinkModel.Kind(type).Title,
            () => nav.Push(LinkModel.Kind(type).Title, new LinkDetails(type, nav)));
        Content = SettingsBodies.Scrolled(FormLook.Stack(0,
            FormLook.Heading("Connect over"),
            FormLook.Card(pilot.Select(KindRow)),
            tools.Count > 0 ? FormLook.Heading("Other") : null,
            tools.Count > 0 ? FormLook.Card(tools.Select(KindRow)) : null));
    }
}

internal sealed class LinkDetails : UserControl
{
    private readonly LinkType _type;
    private readonly SettingsNav _nav;
    private readonly string _udpDefault;
    private readonly TextBlock _error = FormLook.Caption("", "CriticalBrush");
    private readonly TextBlock _hostError = FormLook.Caption("", "CriticalBrush");
    private readonly TextBlock _portError = FormLook.Caption("", "CriticalBrush");
    private readonly TextBox _name = FormLook.Field("", "Name");
    private readonly Button _connect;
    private readonly AircastCloudFields? _cloud;
    private LinkDraft _draft;
    private JsonObject _view;

    public LinkDetails(LinkType type, SettingsNav nav)
    {
        _type = type;
        _nav = nav;
        _view = Bridge.Group(LinkModel.View);
        _udpDefault = LinkModel.UdpDefault(Bridge.Get(LinkModel.UdpListenPort)["value"]);
        _draft = new LinkDraft(type, Port: LinkModel.PortFor(type, _udpDefault)).WithPluggedPort(LinkModel.Ports(_view));
        _connect = FormLook.Button(type == LinkType.Mock ? "Start" : "Connect", Add, ButtonLook.Accent);
        _connect.HorizontalAlignment = HorizontalAlignment.Stretch;
        _connect.MinHeight = 40;
        _name.TextChanged += (_, _) => Change(_draft with { Name = _name.Text });
        _cloud = type == LinkType.AircastCloud
            ? new AircastCloudFields(_draft.ApiBase, _draft.DeviceId, api => Change(_draft with { ApiBase = api }), id => Change(_draft with { DeviceId = id }))
            : null;
        Content = SettingsBodies.Scrolled(FormLook.Stack(12,
            FormLook.Text(LinkModel.Kind(type).About, "FontSizeBody", "TextSecondaryBrush"),
            Fields(),
            type == LinkType.Mock ? null : LinkForms.Advanced(false, Advanced()),
            _error,
            _connect));
        Suggest();
    }

    private IReadOnlyList<SerialPortChoice> Ports => LinkModel.Ports(_view);

    private void Change(LinkDraft draft)
    {
        _draft = draft;
        FormLook.Say(_hostError, null);
        FormLook.Say(_portError, null);
        Suggest();
    }

    private void Suggest() => _name.PlaceholderText = LinkModel.SuggestedName(_draft, Ports, _udpDefault);

    private UIElement Fields() => _type switch
    {
        LinkType.Mock => LinkForms.Mock(_draft.Choices, choices => Change(_draft with { Mock = choices })),
        LinkType.LogReplay => LinkForms.LogPicker(this, "Choose a log file", path => Change(_draft with { ReplayLog = path })),
        LinkType.Bluetooth => new BluetoothPicker(device => Change(_draft with { Device = device })),
        LinkType.AircastCloud when _cloud is not null => _cloud,
        LinkType.Serial when Ports.Count == 0 => FormLook.Text("Nothing is plugged in. Connect a radio over USB and it will appear here.", "FontSizeBody", "TextSecondaryBrush"),
        LinkType.Serial => LinkForms.Serial(Ports, LinkModel.Bauds(_view), _draft.PortName, _draft.Baud, true,
            port => Change(_draft with { PortName = port }), baud => Change(_draft with { Baud = baud })),
        LinkType.Udp => FormLook.Stack(4, LinkForms.Port("Listening port", _draft.Port, true, port => Change(_draft with { Port = port })), _portError),
        _ => FormLook.Stack(4,
            LinkForms.Host(_draft.Host, true, host => Change(_draft with { Host = host })),
            _hostError,
            LinkForms.Port("Port", _draft.Port, true, port => Change(_draft with { Port = port })),
            _portError),
    };

    private UIElement Advanced() => FormLook.Stack(12,
        _name,
        _type == LinkType.Udp
            ? new ServerList(_draft.Servers,
                typed =>
                {
                    var servers = LinkModel.WithServer(_draft.Servers, typed, _draft.ShownPort(_udpDefault));
                    FormLook.Say(_error, LinkModel.UdpServer(typed, _draft.ShownPort(_udpDefault)) is null ? "Enter a server as an address, or address:port." : null);
                    _draft = _draft with { Servers = servers };
                    return Task.FromResult(servers);
                },
                gone =>
                {
                    _draft = _draft with { Servers = _draft.Servers.Where(s => s != gone).ToList() };
                    return Task.FromResult(_draft.Servers);
                })
            : null,
        _type == LinkType.Serial && Ports.Count > 0 ? LinkForms.Framing(_draft.Framing, framing => _draft = _draft with { Framing = framing }) : null,
        LinkForms.Flags(_draft.AutoConnect, _draft.HighLatency,
            on => _draft = _draft with { AutoConnect = on },
            on => _draft = _draft with { HighLatency = on }));

    private async void Add()
    {
        var taken = LinkModel.Rows(_view).Select(r => r.Name).ToList();
        var draft = _draft;
        _cloud?.ShowErrors();
        var invalid = LinkModel.DraftError(draft, _udpDefault, taken, Ports.Count > 0);
        var field = _type is LinkType.Udp or LinkType.Tcp ? LinkModel.ErrorField(_type, draft.Host, draft.ShownPort(_udpDefault), _udpDefault) : null;
        FormLook.Say(_hostError, field == "host" ? invalid : null);
        FormLook.Say(_portError, field == "port" ? invalid : null);
        FormLook.Say(_error, field is null ? invalid : null);
        if (invalid is not null) return;
        var name = draft.Name.Trim().Length > 0 ? draft.Name.Trim() : LinkModel.UniqueName(LinkModel.SuggestedName(draft, Ports, _udpDefault), taken);
        _connect.IsEnabled = false;
        FormLook.Say(_error, null);
        var failed = await FormWrites.Off(() => LinkCommands.Add(draft, name, _udpDefault));
        _connect.IsEnabled = true;
        if (failed is not null)
        {
            FormLook.Say(_error, failed);
            return;
        }
        _nav.Pop();
        _nav.Pop();
    }
}

internal sealed class LinkEditor : UserControl
{
    private readonly LinkRow _row;
    private readonly SettingsNav _nav;
    private readonly string _udpDefault;
    private readonly TextBlock _error = FormLook.Caption("", "CriticalBrush");
    private readonly TextBlock _locked = FormLook.Caption("", "CriticalBrush");
    private readonly Button _save;
    private readonly StackPanel _fields = new() { Spacing = 12 };
    private LinkEdit _edit;
    private BluetoothDevice? _device;

    public LinkEditor(LinkRow row, SettingsNav nav)
    {
        _row = row;
        _nav = nav;
        _edit = LinkModel.Edit(row);
        _udpDefault = LinkModel.UdpDefault(Bridge.Get(LinkModel.UdpListenPort)["value"]);
        var view = Bridge.Group(LinkModel.View);
        _save = FormLook.Button("Save and connect", Save, ButtonLook.Accent);
        _save.HorizontalAlignment = HorizontalAlignment.Stretch;
        _save.MinHeight = 40;
        var name = FormLook.Field(_edit.Name, "Name");
        name.TextChanged += (_, _) => _edit = _edit with { Name = name.Text };
        FormLook.Fill(_fields, [Fields(LinkModel.Ports(view), LinkModel.Bauds(view))]);
        Content = SettingsBodies.Scrolled(FormLook.Stack(12,
            FormLook.Text(LinkModel.Kind(row.Type).About, "FontSizeBody", "TextSecondaryBrush"),
            _locked,
            _fields,
            LinkForms.Advanced(LinkModel.EditShowsAdvanced(row), FormLook.Stack(12,
                name,
                row.Editing == LinkEditing.PortOnly
                    ? new ServerList(row.Servers,
                        host => FormWrites.Off(() => LinkCommands.ChangeServer(row.Index, "addHost", host)),
                        host => FormWrites.Off(() => LinkCommands.ChangeServer(row.Index, "removeHost", host)))
                    : null,
                row.Editing == LinkEditing.Serial ? LinkForms.Framing(_edit.Framing, framing => _edit = _edit with { Framing = framing }) : null,
                LinkForms.Flags(_edit.AutoConnect, _edit.HighLatency,
                    on => _edit = _edit with { AutoConnect = on },
                    on => _edit = _edit with { HighLatency = on }))),
            _error,
            _save));
        Live.Watch(this, LinkModel.View, live =>
        {
            var editable = !LinkModel.Connected(LinkModel.Rows(live), row.Index);
            FormLook.Say(_locked, editable ? null : LinkModel.EditWhileConnected);
            _fields.IsHitTestVisible = editable;
            _fields.Opacity = editable ? 1 : 0.5;
            _save.IsEnabled = editable;
        });
    }

    private UIElement Fields(IReadOnlyList<SerialPortChoice> ports, IReadOnlyList<int> bauds) => _row.Editing switch
    {
        LinkEditing.Device => new BluetoothPicker(device => _device = device),
        LinkEditing.LogFile => LinkForms.LogPicker(this, Path.GetFileName(_edit.LogFile) is { Length: > 0 } file ? file : "Choose another log file",
            path => _edit = _edit with { LogFile = path }),
        LinkEditing.Serial => LinkForms.Serial(ports, bauds, _edit.PortName, _edit.Baud, true,
            port => _edit = _edit with { PortName = port }, baud => _edit = _edit with { Baud = baud }),
        LinkEditing.HostAndPort => FormLook.Stack(12,
            LinkForms.Host(_edit.Host, true, host => _edit = _edit with { Host = host }),
            LinkForms.Port("Port", _edit.Port, true, port => _edit = _edit with { Port = port })),
        _ => LinkForms.Port(_row.Editing == LinkEditing.PortOnly ? "Listening port" : "Port", _edit.Port, true, port => _edit = _edit with { Port = port }),
    };

    private async void Save()
    {
        var edit = _edit;
        var device = _device;
        var invalid = LinkModel.EditError(_row.Editing, edit, _udpDefault);
        FormLook.Say(_error, invalid);
        if (invalid is not null) return;
        var rows = await FormWrites.Off(LinkCommands.Rows);
        if (LinkModel.Connected(rows, _row.Index) || rows.All(r => r.Index != _row.Index))
        {
            FormLook.Say(_error, LinkModel.EditWhileConnected);
            return;
        }
        var name = edit.Name.Trim().Length > 0
            ? edit.Name.Trim()
            : LinkModel.UniqueName(LinkModel.EditSuggestion(_row, edit, device, LinkModel.Ports(Bridge.Group(LinkModel.View)), _udpDefault),
                rows.Where(r => r.Index != _row.Index).Select(r => r.Name).ToList());
        var writes = LinkModel.EditWrites(_row.Editing, edit, name, LinkModel.EditedPort(_row.Editing, edit.Port, _udpDefault));
        _save.IsEnabled = false;
        await FormWrites.Off(() => LinkCommands.Apply(_row.Index, writes, device));
        _nav.Pop();
    }
}

internal static class LinkForms
{
    public static Expander Advanced(bool open, UIElement content) => new()
    {
        Header = FormLook.Text("Advanced", "FontSizeBody", "TextPrimaryBrush", true),
        Content = content,
        IsExpanded = open,
        HorizontalAlignment = HorizontalAlignment.Stretch,
        HorizontalContentAlignment = HorizontalAlignment.Stretch,
    };

    public static TextBox Port(string label, string port, bool enabled, Action<string> changed)
    {
        var box = FormLook.Field(port, label);
        box.IsEnabled = enabled;
        box.InputScope = new Microsoft.UI.Xaml.Input.InputScope { Names = { new Microsoft.UI.Xaml.Input.InputScopeName(Microsoft.UI.Xaml.Input.InputScopeNameValue.Number) } };
        box.TextChanged += (_, _) => changed(box.Text);
        return box;
    }

    public static TextBox Host(string host, bool enabled, Action<string> changed)
    {
        var box = FormLook.Field(host, "Host address", "Example: 192.168.1.10");
        box.IsEnabled = enabled;
        box.IsSpellCheckEnabled = false;
        box.TextChanged += (_, _) => changed(box.Text);
        return box;
    }

    public static UIElement Serial(IReadOnlyList<SerialPortChoice> ports, IReadOnlyList<int> bauds, string portName, int baud, bool enabled,
        Action<string> port, Action<int> rate)
    {
        var portBox = FormLook.Combo(ports.Select(p => p.Label), ports.Select(p => p.Port).ToList().IndexOf(portName), FormLook.ChoiceWidth,
            portName.Length > 0 ? portName : "Choose a port");
        var baudBox = FormLook.Combo(bauds.Select(b => b.ToString(CultureInfo.InvariantCulture)), bauds.ToList().IndexOf(baud));
        portBox.IsEnabled = enabled;
        baudBox.IsEnabled = enabled;
        portBox.SelectionChanged += (_, _) =>
        {
            if (portBox.SelectedIndex >= 0) port(ports[portBox.SelectedIndex].Port);
        };
        baudBox.SelectionChanged += (_, _) =>
        {
            if (baudBox.SelectedIndex >= 0) rate(bauds[baudBox.SelectedIndex]);
        };
        AutomationProperties.SetName(portBox, "USB port");
        AutomationProperties.SetName(baudBox, "Baud rate");
        return FormLook.Card(FormLook.Row(FormLook.Text("USB port"), portBox), FormLook.Row(FormLook.Text("Baud rate"), baudBox));
    }

    public static UIElement Framing(SerialFraming framing, Action<SerialFraming> changed)
    {
        var held = new Holder<SerialFraming>(framing);
        void Set(SerialFraming next)
        {
            held.Value = next;
            changed(next);
        }
        return FormLook.Card(
            FormLook.Row(FormLook.Text("Enable flow control"),
                Toggle(framing.FlowControl != 0, on => Set(held.Value with { FlowControl = on ? 1 : 0 }), "Enable flow control")),
            Pick("Parity", LinkModel.Parities.Select(p => p.Label), LinkModel.Parities.Select(p => p.Value).ToList().IndexOf(framing.Parity),
                at => Set(held.Value with { Parity = LinkModel.Parities[at].Value })),
            Pick("Data bits", LinkModel.DataBits.Select(b => b.ToString(CultureInfo.InvariantCulture)), LinkModel.DataBits.ToList().IndexOf(framing.DataBits),
                at => Set(held.Value with { DataBits = LinkModel.DataBits[at] })),
            Pick("Stop bits", LinkModel.StopBits.Select(b => b.ToString(CultureInfo.InvariantCulture)), LinkModel.StopBits.ToList().IndexOf(framing.StopBits),
                at => Set(held.Value with { StopBits = LinkModel.StopBits[at] })));
    }

    public static UIElement Flags(bool autoConnect, bool highLatency, Action<bool> onAutoConnect, Action<bool> onHighLatency) => FormLook.Card(
        LinkModel.FlagText.Zip(new[] { (autoConnect, onAutoConnect), (highLatency, onHighLatency) })
            .Select(pair => (UIElement)FormLook.Row(
                FormLook.Stack(2, FormLook.Text(pair.First.Title), FormLook.Caption(pair.First.Note)),
                Toggle(pair.Second.Item1, pair.Second.Item2, pair.First.Title))));

    public static UIElement Mock(MockChoices initial, Action<MockChoices> changed)
    {
        var held = new Holder<MockChoices>(initial);
        var fresh = new CheckBox { Content = "Start with fresh firmware parameters (setup required)", IsChecked = initial.FreshParams, FontFamily = PreflightLook.Font };
        var stream = FormLook.Combo(LinkModel.MockVideoStreams, initial.VideoStream);
        var streamRow = FormLook.Card(FormLook.Row(FormLook.Text("Served video stream"), stream));
        var vehicle = FormLook.Combo(LinkModel.MockVehicles.Select(v => v.Label), initial.Vehicle, 280);
        void Show(MockChoices next)
        {
            fresh.Visibility = FormLook.Shown(next.Apm);
            fresh.IsChecked = next.FreshParams;
            streamRow.Visibility = FormLook.Shown(next.Camera);
        }
        void Update(MockChoices next)
        {
            held.Value = next;
            Show(next);
            changed(next);
        }
        FormLook.Toggled(fresh, on =>
        {
            if (on != held.Value.FreshParams) Update(held.Value with { FreshParams = on });
        });
        vehicle.SelectionChanged += (_, _) => Update(held.Value.WithVehicle(Math.Max(0, vehicle.SelectedIndex)));
        stream.SelectionChanged += (_, _) => Update(held.Value with { VideoStream = Math.Max(0, stream.SelectedIndex) });
        Show(initial);
        AutomationProperties.SetName(vehicle, "Vehicle type");
        AutomationProperties.SetName(stream, "Served video stream");
        return FormLook.Stack(8,
            FormLook.Card(FormLook.Row(FormLook.Text("Vehicle type"), vehicle)),
            FormLook.Check("Send status text + voice", initial.StatusText, on => Update(held.Value with { StatusText = on })),
            FormLook.Check("Enable camera", initial.Camera, on => Update(held.Value with { Camera = on })),
            FormLook.Check("Enable gimbal", initial.Gimbal, on => Update(held.Value with { Gimbal = on })),
            FormLook.Check("Enable proximity sensors", initial.Proximity, on => Update(held.Value with { Proximity = on })),
            fresh,
            streamRow);
    }

    public static UIElement LogPicker(FrameworkElement owner, string label, Action<string> chosen)
    {
        var shown = FormLook.Text("", "FontSizeBody", "TextSecondaryBrush");
        var button = FormLook.Button(label, async () =>
        {
            if (await FilePicks.Open(owner.XamlRoot, [FilePicks.Any]) is not { } path) return;
            FormLook.Say(shown, Path.GetFileName(path));
            chosen(path);
        });
        return FormLook.Stack(8, button, shown);
    }

    private static ToggleSwitch Toggle(bool on, Action<bool> changed, string name)
    {
        var toggle = new ToggleSwitch { IsOn = on, OnContent = "", OffContent = "", MinWidth = 0 };
        AutomationProperties.SetName(toggle, name);
        toggle.Toggled += (_, _) => changed(toggle.IsOn);
        return toggle;
    }

    private static UIElement Pick(string label, IEnumerable<string> choices, int selected, Action<int> picked)
    {
        var combo = FormLook.Combo(choices, selected, 120);
        AutomationProperties.SetName(combo, label);
        combo.SelectionChanged += (_, _) =>
        {
            if (combo.SelectedIndex >= 0) picked(combo.SelectedIndex);
        };
        return FormLook.Row(FormLook.Text(label), combo);
    }

    private sealed class Holder<T>(T value)
    {
        public T Value { get; set; } = value;
    }
}

internal sealed class BluetoothPicker : UserControl
{
    private readonly Action<BluetoothDevice> _picked;
    private readonly StackPanel _devices = new() { Spacing = 4 };
    private readonly TextBlock _chosen = FormLook.Caption("");
    private readonly Button _scan;
    private readonly Button _stop;
    private BluetoothState _state = new(false, false, []);
    private BluetoothDevice? _device;

    public BluetoothPicker(Action<BluetoothDevice> picked)
    {
        _picked = picked;
        _scan = FormLook.Button("Scan", () => FormWrites.Off(() => LinkCommands.ScanBluetooth(true)));
        _stop = FormLook.Button("Stop", () => FormWrites.Off(() => LinkCommands.ScanBluetooth(false)));
        Content = FormLook.Stack(8, _chosen, FormLook.Text("Bluetooth devices", "FontSizeBody", "TextPrimaryBrush", true), _devices, FormLook.Line(8, _scan, _stop));
        Live.Poll(this, TimeSpan.FromSeconds(1), () => LinkModel.Bluetooth(Bridge.Get(LinkModel.View)), state =>
        {
            if (state == _state) return;
            _state = state;
            Render();
        });
        Render();
    }

    private void Render()
    {
        _scan.IsEnabled = !_state.Scanning;
        _stop.IsEnabled = _state.Scanning;
        FormLook.Say(_chosen, _device is null ? "No device chosen" : $"Device: {_device.Name} · Address: {_device.Address}");
        FormLook.Fill(_devices, _state.Devices.Count == 0
            ? [FormLook.Caption(_state.Scanning ? "Scanning…" : "No devices found. Scan to look for radios nearby.")]
            : _state.Devices.Select(device =>
            {
                var choice = new RadioButton { Content = device.Name, IsChecked = device == _device, GroupName = "bluetooth" };
                choice.Checked += (_, _) =>
                {
                    _device = device;
                    _picked(device);
                    FormLook.Say(_chosen, $"Device: {device.Name} · Address: {device.Address}");
                };
                return (UIElement)choice;
            }));
    }
}

internal sealed class ServerList : UserControl
{
    private readonly Func<string, Task<IReadOnlyList<string>>> _add;
    private readonly Func<string, Task<IReadOnlyList<string>>> _remove;
    private readonly StackPanel _rows = new() { Spacing = 4 };

    public ServerList(IReadOnlyList<string> servers, Func<string, Task<IReadOnlyList<string>>> add, Func<string, Task<IReadOnlyList<string>>> remove)
    {
        _add = add;
        _remove = remove;
        var typed = FormLook.Field("", "Also send to", "127.0.0.1:14550");
        typed.IsSpellCheckEnabled = false;
        async void Add()
        {
            var text = typed.Text.Trim();
            if (text.Length == 0) return;
            typed.Text = "";
            Render(await _add(text));
        }
        typed.KeyDown += (_, e) =>
        {
            if (e.Key == Windows.System.VirtualKey.Enter) Add();
        };
        Content = FormLook.Stack(4,
            _rows,
            FormLook.Line(8, FormLook.Margined(typed, new Thickness(0)), FormLook.Margined(FormLook.IconButton("", "Add address", Add), new Thickness(0, 26, 0, 0))),
            FormLook.Caption("Optional. One address per device that should get telemetry too."));
        typed.Width = 300;
        Render(servers);
    }

    private void Render(IReadOnlyList<string> servers) => FormLook.Fill(_rows, servers.Select(server =>
        (UIElement)FormLook.Line(8,
            FormLook.Margined(FormLook.Text(server), new Thickness(0, 6, 0, 0)),
            FormLook.IconButton("", $"Remove {server}", async () => Render(await _remove(server))))));
}
