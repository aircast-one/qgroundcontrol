using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;

namespace AircastQGC.Pages.Settings;

internal enum LinkType { Udp, Tcp, Serial, Bluetooth, LogReplay, Mock, AircastCloud, Other }

internal enum LinkEditing { HostAndPort, PortOnly, Serial, LogFile, Device, None }

internal sealed record SerialFraming(int DataBits = 8, int StopBits = 1, int Parity = 0, int FlowControl = 0);

internal sealed record LinkRow(
    int Index,
    string Name,
    string StatusLine,
    bool GoneQuiet,
    bool Connected,
    bool Heard,
    string LastError,
    string ErrorRemedy,
    LinkEditing Editing,
    string Host,
    int Port,
    string PortName,
    int Baud,
    SerialFraming Framing,
    IReadOnlyList<string> Servers,
    bool AutoConnect,
    bool HighLatency,
    LinkType Type,
    string Filename)
{
    public bool Editable => !Connected && Editing != LinkEditing.None;

    public bool FixByEditing => ErrorRemedy == LinkModel.RemedyEditAddress && Editable;

    public string Configuration => $"@{LinkModel.Configurations}.{Index}";
}

internal sealed record AutoLink(string Name, string Summary, bool Heard, LinkType Type, int Index);

internal sealed record SerialPortChoice(string Port, string Label);

internal sealed record BluetoothDevice(string Name, string Address);

internal sealed record BluetoothState(bool Available, bool Scanning, IReadOnlyList<BluetoothDevice> Devices);

internal sealed record LinkKind(LinkType Type, string Title, string Detail, string About);

internal sealed record MockChoices(bool StatusText = false, bool Camera = false, bool Gimbal = false, bool Proximity = false, bool FreshParams = false, int Vehicle = 0, int VideoStream = 0)
{
    public bool Apm => LinkModel.MockVehicles[Vehicle].Id.StartsWith("apm", StringComparison.Ordinal);

    public MockChoices WithVehicle(int vehicle) => Fresh(this with { Vehicle = vehicle });

    private static MockChoices Fresh(MockChoices picked) => picked.Apm ? picked : picked with { FreshParams = false };

    public object[] Arguments => [LinkModel.MockVehicles[Vehicle].Id, StatusText, Camera, Gimbal, Proximity, FreshParams && Apm, VideoStream];
}

internal sealed record LinkDraft(
    LinkType Type,
    string Name = "",
    string Host = "",
    string Port = "",
    string PortName = "",
    int Baud = LinkModel.DefaultBaud,
    bool AutoConnect = false,
    bool HighLatency = false,
    BluetoothDevice? Device = null,
    string ReplayLog = "",
    MockChoices? Mock = null,
    string ApiBase = "",
    string DeviceId = "")
{
    public IReadOnlyList<string> Servers { get; init; } = [];
    public SerialFraming Framing { get; init; } = new();
    public MockChoices Choices => Mock ?? new MockChoices();

    public string ShownPort(string udpDefault) => Port.Trim().Length > 0 ? Port.Trim() : LinkModel.PortFor(Type, udpDefault);

    public LinkDraft WithPluggedPort(IReadOnlyList<SerialPortChoice> ports) =>
        PortName.Length > 0 || ports.Count != 1 ? this : this with { PortName = ports[0].Port };
}

internal sealed record LinkEdit(string Name, string Host, string Port, string PortName, int Baud, bool AutoConnect, bool HighLatency, SerialFraming Framing, string LogFile);

internal static class LinkModel
{
    public const string View = "view.links";
    public const string Configurations = "links.linkConfigurations";
    public const string Commit = "links.commitLinkConfigurations";
    public const string BluetoothScan = "links.bluetoothScan";
    public const string Connect = "links.createConnectedLink";
    public const string Remove = "links.removeConfiguration";
    public const string CreateAndConnect = "links.createAndConnectLink";
    public const string CreateReplay = "links.createLogReplayConfiguration";
    public const string CreateBluetooth = "links.createBluetoothLink";
    public const string CreateSerial = "links.createSerialConfiguration";
    public const string StartMock = "links.startMockLink";
    public const string UdpListenPort = "settings.autoConnectSettings.udpListenPort";
    public const string Subject = "The link";
    public const string RemedyEditAddress = "editAddress";
    public const string DefaultPort = "14550";
    public const string DefaultTcpPort = "5760";
    public const string ReplayName = "Log Replay";
    public const string ConnectedStatus = "Connected";
    public const string EditWhileConnected = "Disconnect this link to change it.";
    public const int DefaultBaud = 57600;

    public static readonly IReadOnlyList<(string Id, string Label)> MockVehicles =
    [
        ("px4", "PX4 Vehicle"),
        ("apmCopter", "APM ArduCopter Vehicle"),
        ("apmPlane", "APM ArduPlane Vehicle"),
        ("apmSub", "APM ArduSub Vehicle"),
        ("apmRover", "APM ArduRover Vehicle"),
        ("generic", "Generic Vehicle"),
    ];

    public static readonly IReadOnlyList<string> MockVideoStreams = ["Disabled", "RTP/UDP H.264", "RTP/UDP H.265", "RTSP (H.264)", "MPEG-TS (UDP)", "MPEG-TS (TCP)"];

    public static readonly IReadOnlyList<(string Label, int Value)> Parities = [("None", 0), ("Even", 2), ("Odd", 3)];
    public static readonly IReadOnlyList<int> DataBits = [5, 6, 7, 8];
    public static readonly IReadOnlyList<int> StopBits = [1, 2];

    public static readonly IReadOnlyList<(string Title, string Note)> FlagText =
    [
        ("Connect on start", "Open this link when the app launches."),
        ("High latency", "Tune the link for satellite or cellular round trips."),
    ];

    private static readonly IReadOnlyList<LinkType> Creatable = [LinkType.Udp, LinkType.Tcp, LinkType.Serial];
    private static readonly IReadOnlyList<LinkType> PilotOrder = [LinkType.Udp, LinkType.Serial, LinkType.Bluetooth, LinkType.AircastCloud, LinkType.Tcp];
    private static readonly IReadOnlyList<LinkType> ToolOrder = [LinkType.LogReplay, LinkType.Mock];

    private static readonly IReadOnlyDictionary<string, LinkType> TypeIds = new Dictionary<string, LinkType>
    {
        ["udp"] = LinkType.Udp,
        ["tcp"] = LinkType.Tcp,
        ["serial"] = LinkType.Serial,
        ["bluetooth"] = LinkType.Bluetooth,
        ["logReplay"] = LinkType.LogReplay,
        ["mock"] = LinkType.Mock,
        ["aircastCloud"] = LinkType.AircastCloud,
    };

    private static readonly IReadOnlyDictionary<string, LinkEditing> EditingIds = new Dictionary<string, LinkEditing>
    {
        ["hostAndPort"] = LinkEditing.HostAndPort,
        ["portOnly"] = LinkEditing.PortOnly,
        ["serial"] = LinkEditing.Serial,
        ["logFile"] = LinkEditing.LogFile,
        ["device"] = LinkEditing.Device,
    };

    public static LinkType Type(string id) => TypeIds.TryGetValue(id, out var type) ? type : LinkType.Other;

    public static string Id(LinkType type) => TypeIds.FirstOrDefault(t => t.Value == type).Key ?? "";

    public static IReadOnlyList<LinkRow> Rows(JsonObject view) =>
        MapUiJson.Objects(view["configured"]).Select(link => new LinkRow(
            Int(link["index"]),
            MapUiJson.Text(link["name"]),
            MapUiJson.Text(link["statusLine"]),
            MapUiJson.Flag(link["goneQuiet"]),
            MapUiJson.Flag(link["connected"]),
            MapUiJson.Flag(link["heardVehicle"]),
            MapUiJson.Text(link["lastError"]),
            MapUiJson.Text(link["errorRemedy"]),
            EditingIds.TryGetValue(MapUiJson.Text(link["editing"]), out var editing) ? editing : LinkEditing.None,
            MapUiJson.Text(link["host"]),
            Int(link["port"]),
            MapUiJson.Text(link["portName"]),
            Int(link["baud"]),
            new SerialFraming(Int(link["dataBits"], 8), Int(link["stopBits"], 1), Int(link["parity"]), Int(link["flowControl"])),
            MapUiJson.Texts(link["hostList"]),
            MapUiJson.Flag(link["autoConnect"]),
            MapUiJson.Flag(link["highLatency"]),
            Type(MapUiJson.Text(link["type"])),
            MapUiJson.Text(link["filename"]))).ToList();

    public static IReadOnlyList<AutoLink> AutoLinks(JsonObject view) =>
        MapUiJson.Objects(view["links"])
            .Where(link => MapUiJson.Flag(link["dynamic"]) && MapUiJson.Flag(link["connected"]))
            .Select(link => new AutoLink(MapUiJson.Text(link["name"]), MapUiJson.Text(link["displaySummary"]),
                MapUiJson.Flag(link["heardVehicle"]), Type(MapUiJson.Text(link["type"])), Int(link["index"])))
            .ToList();

    public static IReadOnlyList<SerialPortChoice> Ports(JsonObject view) =>
        MapUiJson.Objects(view["serialPorts"])
            .Select(entry => (Port: MapUiJson.Text(entry["port"]), Label: MapUiJson.Text(entry["label"])))
            .Where(entry => entry.Port.Length > 0)
            .Select(entry => new SerialPortChoice(entry.Port, entry.Label.Length > 0 ? entry.Label : entry.Port))
            .ToList();

    public static IReadOnlyList<int> Bauds(JsonObject view) =>
        (view["baudRates"] as JsonArray ?? [])
            .Select(rate => (int)(MapUiJson.Number(rate) ?? 0))
            .Where(rate => rate > 0)
            .DefaultIfEmpty(DefaultBaud)
            .ToList();

    public static BluetoothState Bluetooth(JsonObject view) => view["bluetooth"] is JsonObject bluetooth
        ? new BluetoothState(MapUiJson.Flag(bluetooth["available"]), MapUiJson.Flag(bluetooth["scanning"]),
            MapUiJson.Objects(bluetooth["devices"]).Select(d => new BluetoothDevice(MapUiJson.Text(d["name"]), MapUiJson.Text(d["address"]))).ToList())
        : new BluetoothState(false, false, []);

    public static IReadOnlySet<LinkType> Served(JsonObject view) =>
        (view["linkTypeIds"] as JsonArray ?? []).Select(id => Type(MapUiJson.Text(id))).ToHashSet();

    public static IReadOnlyList<LinkType> Addable(JsonObject view, bool cloud) =>
        view["linkTypeIds"] is not JsonArray ? Creatable : Offered(Served(view), Bluetooth(view).Available, cloud);

    private static IReadOnlyList<LinkType> Offered(IReadOnlySet<LinkType> served, bool bluetooth, bool cloud) =>
        OrAll(Creatable.Where(served.Contains).ToList())
            .Concat(new[] { (Type: LinkType.Bluetooth, Usable: bluetooth), (Type: LinkType.AircastCloud, Usable: cloud) }
                .Where(t => t.Usable && served.Contains(t.Type))
                .Select(t => t.Type))
            .Concat(ToolOrder.Where(served.Contains))
            .ToList();

    private static IReadOnlyList<LinkType> OrAll(IReadOnlyList<LinkType> own) => own.Count > 0 ? own : Creatable;

    public static LinkKind Kind(LinkType type) => type switch
    {
        LinkType.Udp => new(type, "Wi-Fi or network", "Wi-Fi or Ethernet (UDP)", "Listens on a port for telemetry from the aircraft or its radio."),
        LinkType.Serial => new(type, "USB telemetry radio", "MAVLink radio on USB", "A telemetry radio or flight controller plugged into this device over USB."),
        LinkType.Bluetooth => new(type, "Bluetooth radio", "Paired telemetry radio", "A radio paired with or near this device over Bluetooth."),
        LinkType.AircastCloud => new(type, "Aircast Cloud", "Backup link via your account", "A backup link to the aircraft through your Aircast account."),
        LinkType.Tcp => new(type, "Network server (TCP)", "A listening ground station", "Calls out to a device that is listening, such as a ground station or bridge."),
        LinkType.LogReplay => new(type, "Replay a flight log", "Play back a saved log", "Plays back a saved telemetry log as if the vehicle were connected."),
        LinkType.Mock => new(type, "Simulated vehicle", "Try the app without hardware", "A simulated vehicle for trying the app without hardware. It is not saved, so it ends when the app restarts."),
        _ => new(type, TypeLabel(type), "", ""),
    };

    public static string TypeLabel(LinkType type) => type switch
    {
        LinkType.Serial => "Serial",
        LinkType.Bluetooth => "Bluetooth",
        LinkType.AircastCloud => "Aircast Cloud",
        LinkType.LogReplay => "Log replay",
        LinkType.Mock => "Simulated",
        _ => Id(type).ToUpperInvariant(),
    };

    public static string Glyph(LinkType type) => type switch
    {
        LinkType.Serial => "",
        LinkType.Udp => "",
        LinkType.Bluetooth => "",
        LinkType.Tcp => "",
        LinkType.AircastCloud => "",
        LinkType.LogReplay => "",
        LinkType.Mock => "",
        _ => "",
    };

    public static IReadOnlyList<LinkType> PilotKinds(IReadOnlyList<LinkType> offered, bool radioPlugged) =>
        PilotOrder.Where(offered.Contains).OrderBy(t => radioPlugged && t == LinkType.Serial ? 0 : 1).ToList();

    public static IReadOnlyList<LinkType> ToolKinds(IReadOnlyList<LinkType> offered) => ToolOrder.Where(offered.Contains).ToList();

    public static string PortFor(LinkType type, string udpDefault) => type == LinkType.Tcp ? DefaultTcpPort : udpDefault;

    public static string UdpDefault(JsonNode? listenPort) =>
        MapUiJson.Number(listenPort) is { } port && port is >= 1 and <= 65535 ? ((int)port).ToString(CultureInfo.InvariantCulture) : DefaultPort;

    public static string AutoLinkStatus(AutoLink link) => link.Heard ? "Vehicle" : "Listening";

    public static string AutoLinkSubtitle(AutoLink link) =>
        link.Type == LinkType.Mock ? "Simulated" : FormText.Joined(link.Summary, "automatic");

    public static string AutoLinkName(LinkType type, string host, string port) =>
        type == LinkType.Udp ? $"UDP {port}"
        : host.Trim().Length == 0 ? Id(type).ToUpperInvariant()
        : $"{Id(type).ToUpperInvariant()} {host.Trim()}:{port}";

    public static string UniqueName(string wanted, IReadOnlyList<string> taken) =>
        new[] { wanted }.Concat(Enumerable.Range(2, taken.Count + 1).Select(n => $"{wanted} ({n})")).First(name => !taken.Contains(name));

    public static string PortLabel(IReadOnlyList<SerialPortChoice> ports, string portName) =>
        ports.FirstOrDefault(p => p.Port == portName)?.Label ?? "";

    public static string SerialName(string portLabel) => portLabel.Length > 0 ? portLabel : "Serial";

    public static string SuggestedName(LinkDraft draft, IReadOnlyList<SerialPortChoice> ports, string udpDefault) => draft.Type switch
    {
        LinkType.Serial => SerialName(PortLabel(ports, draft.PortName)),
        LinkType.LogReplay => ReplayName,
        LinkType.AircastCloud => AircastCloud.LinkName,
        LinkType.Bluetooth => draft.Device?.Name ?? "",
        _ => AutoLinkName(draft.Type, draft.Host, draft.ShownPort(udpDefault)),
    };

    public static string? FormError(LinkType type, string host, string port, string udpDefault) =>
        ParsedPort(type, port, udpDefault) is not { } parsed
            ? type == LinkType.Udp ? $"Enter a port between 1 and 65535, or leave it blank for {udpDefault}" : "Port must be a number between 1 and 65535."
            : type == LinkType.Tcp && host.Trim().Length == 0 ? "A TCP link needs the address of the device to call."
            : null;

    public static string? ErrorField(LinkType type, string host, string port, string udpDefault) =>
        ParsedPort(type, port, udpDefault) is null ? "port"
        : type == LinkType.Tcp && host.Trim().Length == 0 ? "host"
        : null;

    public static string? SerialError(string portName, int baud, IReadOnlyList<string> taken, string name, bool anyPorts) =>
        !anyPorts ? "Nothing is plugged in. Connect a radio over USB and it will appear here."
        : portName.Length == 0 ? "Pick the port the radio is plugged into."
        : baud <= 0 ? "Pick a baud rate."
        : name.Trim().Length > 0 && taken.Contains(name.Trim()) ? "A link with that name already exists."
        : null;

    public static string? DraftError(LinkDraft draft, string udpDefault, IReadOnlyList<string> taken, bool anyPorts) => draft.Type switch
    {
        LinkType.Mock => null,
        LinkType.AircastCloud => AircastCloud.Valid(draft.ApiBase, draft.DeviceId) ? null : "",
        LinkType.Bluetooth => draft.Device is null ? "Pick a Bluetooth device." : null,
        LinkType.LogReplay => draft.ReplayLog.Length == 0 ? "Choose a log file to replay." : null,
        LinkType.Serial => SerialError(draft.PortName, draft.Baud, taken, draft.Name, anyPorts),
        _ => FormError(draft.Type, draft.Host, draft.ShownPort(udpDefault), udpDefault),
    };

    public static string? UdpServer(string typed, string localPort)
    {
        var parts = typed.Trim().Split(':');
        var host = parts[0].Trim();
        return parts.Length > 2 || host.Length == 0 || !int.TryParse((parts.Length > 1 ? parts[1] : localPort).Trim(), out var port) || port is < 1 or > 65535
            ? null
            : $"{host}:{port}";
    }

    public static IReadOnlyList<string> WithServer(IReadOnlyList<string> servers, string typed, string localPort) =>
        UdpServer(typed, localPort) is { } server && !servers.Contains(server) ? servers.Append(server).ToList() : servers;

    public static LinkEdit Edit(LinkRow row) => new(row.Name, row.Host, row.Port.ToString(CultureInfo.InvariantCulture), row.PortName,
        row.Baud > 0 ? row.Baud : DefaultBaud, row.AutoConnect, row.HighLatency, row.Framing, row.Filename);

    public static bool EditShowsAdvanced(LinkRow row) =>
        row.AutoConnect || row.HighLatency || row.Servers.Count > 0 || row.Framing != new SerialFraming();

    public static int EditedPort(LinkEditing editing, string port, string udpDefault) =>
        int.TryParse(port.Trim().Length > 0 ? port.Trim() : editing == LinkEditing.PortOnly ? udpDefault : "", out var parsed) ? parsed : 0;

    public static string? EditError(LinkEditing editing, LinkEdit edit, string udpDefault) => editing switch
    {
        LinkEditing.LogFile => edit.LogFile.Length == 0 ? "Choose a log file." : null,
        LinkEditing.PortOnly => FormError(LinkType.Udp, "", edit.Port, udpDefault),
        LinkEditing.HostAndPort => EditedPort(editing, edit.Port, udpDefault) is < 1 or > 65535 ? "Port must be a number between 1 and 65535." : null,
        _ => null,
    };

    public static string EditSuggestion(LinkRow row, LinkEdit edit, BluetoothDevice? device, IReadOnlyList<SerialPortChoice> ports, string udpDefault) =>
        row.Editing switch
        {
            LinkEditing.Device => device?.Name ?? row.Name,
            LinkEditing.Serial => SerialName(PortLabel(ports, edit.PortName)),
            LinkEditing.HostAndPort => AutoLinkName(LinkType.Tcp, edit.Host, edit.Port),
            _ => AutoLinkName(LinkType.Udp, edit.Host, edit.Port.Trim().Length > 0 ? edit.Port : udpDefault),
        };

    public static IReadOnlyList<(string Field, object Value)> EditWrites(LinkEditing editing, LinkEdit edit, string name, int port) =>
        new (string, object)[] { ("name", name), ("autoConnect", edit.AutoConnect), ("highLatency", edit.HighLatency) }
            .Concat(editing switch
            {
                LinkEditing.HostAndPort => new (string, object)[] { ("host", edit.Host), ("port", port) },
                LinkEditing.PortOnly => [("localPort", port)],
                LinkEditing.LogFile => [("filename", edit.LogFile)],
                LinkEditing.Serial =>
                [
                    ("portName", edit.PortName),
                    ("baud", edit.Baud),
                    ("dataBits", edit.Framing.DataBits),
                    ("stopBits", edit.Framing.StopBits),
                    ("parity", edit.Framing.Parity),
                    ("flowControl", edit.Framing.FlowControl),
                ],
                _ => [],
            })
            .ToList();

    public static string PrimaryAction(LinkRow row) => row.Connected ? "Disconnect" : row.FixByEditing ? "Try again" : "Connect";

    public static string? RemedyText(string remedy) =>
        remedy == RemedyEditAddress ? "Nothing is listening at that address. Retrying will not help until it is changed." : null;

    public static string AddFailure(LinkType type) =>
        type == LinkType.Mock ? "Could not start the simulated vehicle." : "Could not add that link. The name may already be in use.";

    public static bool Connected(IReadOnlyList<LinkRow> rows, int index) => rows.FirstOrDefault(r => r.Index == index)?.Connected == true;

    private static int? ParsedPort(LinkType type, string port, string udpDefault) =>
        int.TryParse(port.Trim().Length > 0 ? port.Trim() : type == LinkType.Udp ? udpDefault : "", out var parsed) && parsed is >= 1 and <= 65535
            ? parsed
            : null;

    private static int Int(JsonNode? node, int fallback = 0) => MapUiJson.Number(node) is { } number ? (int)number : fallback;
}

internal static class LinkCommands
{
    public static IReadOnlyList<LinkRow> Rows() => LinkModel.Rows(Bridge.Get(LinkModel.View));

    public static bool CommitConfigurations() => FormWrites.Ok(LinkModel.Commit);

    public static bool ScanBluetooth(bool scanning) => FormWrites.Ok(LinkModel.BluetoothScan, scanning);

    public static string? Connect(string configuration) => FormWrites.Call(LinkModel.Connect, LinkModel.Subject, configuration);

    public static string? Remove(LinkRow row) => FormWrites.Call(LinkModel.Remove, LinkModel.Subject, row.Configuration, row.Name);

    public static string? Disconnect(int index) => FormWrites.Call($"{LinkModel.Configurations}.{index}.link.disconnect", LinkModel.Subject);

    public static bool ConnectNamed(string name) =>
        Rows().FirstOrDefault(r => r.Name == name) is { } row && Connect(row.Configuration) is null;

    public static bool Create(LinkDraft draft, string name, int port) => draft.Type switch
    {
        LinkType.Mock => FormWrites.ResultTrue(LinkModel.StartMock, draft.Choices.Arguments),
        LinkType.AircastCloud => AircastCloud.Create(name, draft.ApiBase, draft.DeviceId),
        LinkType.LogReplay => FormWrites.ResultTrue(LinkModel.CreateReplay, name, draft.ReplayLog) && ConnectNamed(name),
        LinkType.Bluetooth => draft.Device is { } device && FormWrites.ResultTrue(LinkModel.CreateBluetooth, name, device.Name, device.Address),
        LinkType.Serial => FormWrites.ResultTrue(LinkModel.CreateSerial, name, draft.PortName, draft.Baud) && WriteFraming(name, draft.Framing) && ConnectNamed(name),
        _ => FormWrites.ResultTrue(LinkModel.CreateAndConnect, LinkModel.Id(draft.Type), name, draft.Type == LinkType.Udp ? "" : draft.Host.Trim(), port),
    };

    public static string? Add(LinkDraft draft, string name, string udpDefault)
    {
        var created = Create(draft, name, int.TryParse(draft.ShownPort(udpDefault), out var port) ? port : 0);
        var saved = draft.Type != LinkType.Mock && Rows().Any(r => r.Name == name);
        if (saved) Finish(name, draft);
        return created || saved ? null : LinkModel.AddFailure(draft.Type);
    }

    public static void Apply(int index, IReadOnlyList<(string Field, object Value)> writes, BluetoothDevice? device)
    {
        writes.ToList().ForEach(write => FormWrites.Set($"{LinkModel.Configurations}.{index}.{write.Field}", write.Value));
        if (device is not null) FormWrites.Ok($"{LinkModel.Configurations}.{index}.setDeviceByAddress", device.Address);
        CommitConfigurations();
        Connect($"@{LinkModel.Configurations}.{index}");
    }

    public static IReadOnlyList<string> ChangeServer(int index, string action, string host)
    {
        FormWrites.Ok($"{LinkModel.Configurations}.{index}.{action}", host);
        return Rows().FirstOrDefault(r => r.Index == index)?.Servers ?? [];
    }

    private static bool WriteFraming(string name, SerialFraming framing)
    {
        if (framing == new SerialFraming()) return true;
        if (Rows().FirstOrDefault(r => r.Name == name) is not { } row) return false;
        new (string, int)[] { ("dataBits", framing.DataBits), ("stopBits", framing.StopBits), ("parity", framing.Parity), ("flowControl", framing.FlowControl) }
            .ToList()
            .ForEach(field => FormWrites.Set($"{LinkModel.Configurations}.{row.Index}.{field.Item1}", field.Item2));
        return CommitConfigurations();
    }

    private static void Finish(string name, LinkDraft draft)
    {
        if (Rows().FirstOrDefault(r => r.Name == name) is not { } row) return;
        draft.Servers.ToList().ForEach(server => FormWrites.Ok($"{LinkModel.Configurations}.{row.Index}.addHost", server));
        if (draft.AutoConnect || draft.HighLatency)
        {
            FormWrites.Set($"{LinkModel.Configurations}.{row.Index}.autoConnect", draft.AutoConnect);
            FormWrites.Set($"{LinkModel.Configurations}.{row.Index}.highLatency", draft.HighLatency);
        }
        if (draft.Servers.Count > 0 || draft.AutoConnect || draft.HighLatency) CommitConfigurations();
    }
}
