using System.Collections.Generic;
using System.Collections.Immutable;
using System.Linq;
using AircastQGC.Controls;
using System.Text.Json.Nodes;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Pages.Device;

internal sealed class TelemetryScreen : DeviceScreen
{
    public TelemetryScreen(DeviceNav nav) : base("telemetry", nav) { }

    protected override IEnumerable<Node?>? Build(DeviceReading reading, DeviceContext ctx) =>
        Viewed(reading, view => view["status"] is JsonObject, view => new Node?[]
        {
            Kit.Section("Ground station", DJ.Badge(view, "status"), DJ.Badges(view, "notices").Select(Kit.Notice)
                .Concat(Kit.Endpoints(ctx, DJ.Endpoints(view, "endpoints")))
                .Append(Kit.Notice(DJ.MaybeBadge(view, "endpointNotice")))
                .Append(DJ.Obj(view, "cloud") is { } cloud ? Cloud(cloud) : null)),
            Kit.Section("MAVLink settings", null, [MavlinkEditor(ctx, view)]),
            DJ.Obj(view, "gimbal") is { } gimbal ? Gimbal(ctx, gimbal) : null,
        });

    private static Node Cloud(JsonObject cloud) =>
        DeviceTree.Leaf("cloud", DJ.Sig(cloud), () =>
        {
            var title = FormLook.Text("Cloud path");
            var badge = Kit.BadgeView(DJ.Badge(cloud, "status"));
            Grid.SetColumn(badge, 1);
            return Kit.Padded(FormLook.Stack(2,
                new Grid
                {
                    ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto } },
                    Children = { title, badge },
                },
                FormLook.Text(DJ.Text(cloud, "text"), "FontSizeBody", "TextSecondaryBrush")));
        });

    private static Node MavlinkEditor(DeviceContext ctx, JsonObject view)
    {
        var settings = DJ.Section(view, "settings");
        var schemes = view["schemes"] as JsonArray ?? [];
        return DeviceTree.Leaf("mavlink-editor", DJ.Sig(settings, schemes),
            () => new EndpointEditorBox(ctx, schemes, DJ.Text(settings, "endpoints"), DJ.Text(settings, "baud")));
    }

    private static Node Gimbal(DeviceContext ctx, JsonObject gimbal) => Kit.Section(DJ.Text(gimbal, "title"), DJ.Badge(gimbal, "status"), new Node?[]
    {
        Kit.Text(DJ.Text(gimbal, "about")),
        Kit.Form(ctx, "gimbal", [
                new Field("vendor", "Make", DJ.Text(gimbal, "vendor"), Options: DJ.Options(gimbal, "vendors")),
                new Field("model", "Model", DJ.Text(gimbal, "model"), Options: DJ.Options(gimbal, "models")),
                new Field("cameraAddr", "Camera address", DJ.Text(gimbal, "cameraAddr"), "192.168.144.25:37260", Kind: FieldKind.Uri),
                new Field("mavlinkAddr", "MAVLink endpoint", DJ.Text(gimbal, "mavlinkAddr"), "127.0.0.1:14550", Kind: FieldKind.Uri),
            ],
            DeviceLink.Put, "/api/gimbal/config", "Save", "Gimbal settings applied.",
            note: "Takes effect on save — no reboot. Changing the make resets the model and address to that make's defaults if the old ones do not fit."),
    });
}

internal sealed record EndpointRow(string Scheme, string Host, string Port)
{
    public static EndpointRow Of(JsonObject? json) => new(DJ.Text(json, "scheme"), DJ.Text(json, "host"), DJ.Text(json, "port"));

    public JsonObject Json => new() { ["scheme"] = Scheme, ["host"] = Host, ["port"] = Port };
}

internal sealed class EndpointEditorBox : UserControl
{
    private const string EndpointsPath = "deviceTelemetry.endpoints";

    private readonly DeviceContext _ctx;
    private readonly JsonArray _schemes;
    private readonly string _stored;
    private readonly StackPanel _rows = new() { Spacing = 8 };
    private readonly TextBox _baud;
    private readonly TextBlock _baudSaid;
    private readonly TextBlock _refusal = Kit.Refusal();
    private readonly Button _save;
    private ImmutableList<EndpointRow>? _typed;
    private JsonObject? _editor;
    private int _asked;

    public EndpointEditorBox(DeviceContext ctx, JsonArray schemes, string stored, string baud)
    {
        (_ctx, _schemes, _stored) = (ctx, schemes, stored);
        _baud = FormLook.Field(baud, "Serial baud", "115200");
        AutomationProperties.SetName(_baud, "Serial baud");
        _baudSaid = FormLook.Caption(BaudHint, "TextTertiaryBrush");
        _save = FormLook.Button("Save", Save);
        _save.HorizontalAlignment = HorizontalAlignment.Left;
        var add = FormLook.Button("+ Add endpoint", () => Edit(Rows.Add(EndpointRow.Of(DJ.Obj(_editor, "blank")))), ButtonLook.Subtle);
        add.HorizontalAlignment = HorizontalAlignment.Left;
        Content = Kit.Padded(FormLook.Stack(12,
            FormLook.Text("Ground-station endpoints"),
            _rows,
            add,
            FormLook.Stack(4, _baud, _baudSaid),
            FormLook.Caption("Takes effect on save — no reboot.", "TextTertiaryBrush"),
            _refusal,
            _save));
        Refresh();
    }

    private const string BaudHint = "The baud only matters for a serial flight controller — USB autopilots ignore it, Betaflight ships at 115200.";

    private ImmutableList<EndpointRow> Rows => _typed ?? DJ.Objs(_editor, "rows").Select(EndpointRow.Of).ToImmutableList();

    private void Edit(ImmutableList<EndpointRow> rows)
    {
        _typed = rows;
        Refresh();
    }

    private async void Refresh()
    {
        var asked = ++_asked;
        var answer = await Ask(_typed);
        if (asked != _asked || answer is null) return;
        _editor = answer;
        var rows = Rows;
        var views = DJ.Objs(_editor, "rows");
        var blank = DJ.Obj(_editor, "blank");
        DeviceTree.Sync(_rows, rows.Select((row, at) => DeviceTree.Kept($"endpoint:{at}",
            () => new EndpointRowBox(),
            box => box.Show(row, views.ElementAtOrDefault(at) ?? blank, next => Edit(Rows.SetItem(at, next)), () => Edit(Rows.RemoveAt(at))))));
    }

    private async System.Threading.Tasks.Task<JsonObject?> Ask(ImmutableList<EndpointRow>? rows) =>
        await DeviceLink.Result(EndpointsPath, _schemes, rows is null ? JsonValue.Create(_stored) : new JsonArray(rows.Select(r => (JsonNode?)r.Json).ToArray())) as JsonObject;

    private async void Save()
    {
        _save.IsEnabled = false;
        FormLook.Say(_refusal, null);
        var saved = await Ask(Rows);
        var refused = saved is null
            ? DeviceLink.NoAnswer
            : await DeviceLink.Call(new DeviceRequest(DeviceLink.Put, "/api/telemetry/config",
                DeviceLink.Body(("endpoints", DJ.Text(saved, "endpoints")), ("baud", _baud.Text))));
        _save.IsEnabled = true;
        var baud = refused?.Field == "baud";
        _baudSaid.Text = baud ? refused!.Message : BaudHint;
        _baudSaid.Foreground = FormLook.Brush(baud ? "CriticalBrush" : "TextTertiaryBrush");
        FormLook.Say(_refusal, baud ? null : refused?.Message);
        if (refused is null) _ctx.Say("MAVLink settings applied.");
    }
}

internal sealed class EndpointRowBox : UserControl
{
    private readonly ComboBox _type = new() { Header = "Connection type", HorizontalAlignment = HorizontalAlignment.Stretch };
    private readonly TextBox _host = FormLook.Field("", "Address");
    private readonly TextBox _port = FormLook.Field("", "Port", "14550");
    private readonly TextBlock _note = FormLook.Caption("", "TextTertiaryBrush");
    private IReadOnlyList<(string Value, string Label, EndpointRow Row)> _options = [];
    private EndpointRow _row = new("", "", "");
    private System.Action<EndpointRow> _change = _ => { };
    private System.Action _remove = () => { };
    private bool _quiet;

    public EndpointRowBox()
    {
        _type.FontFamily = PreflightLook.Font;
        AutomationProperties.SetName(_type, "Connection type");
        AutomationProperties.SetName(_port, "Port");
        _port.Width = 112;
        _type.SelectionChanged += (_, _) =>
        {
            if (!_quiet && _type.SelectedIndex >= 0 && _type.SelectedIndex < _options.Count) _change(_options[_type.SelectedIndex].Row);
        };
        _host.TextChanged += (_, _) =>
        {
            if (_host.Text != _row.Host) _change(_row with { Host = _host.Text });
        };
        _port.TextChanged += (_, _) =>
        {
            if (_port.Text != _row.Port) _change(_row with { Port = _port.Text });
        };
        var remove = FormLook.IconButton("", "Remove endpoint", () => _remove());
        remove.VerticalAlignment = VerticalAlignment.Bottom;
        Grid.SetColumn(remove, 1);
        Grid.SetColumn(_port, 1);
        Content = new Border
        {
            Background = FormLook.Brush("ControlBrush"),
            CornerRadius = FormLook.Radius("RadiusOverlay"),
            Padding = new Thickness(12),
            Child = FormLook.Stack(8,
                new Grid { ColumnSpacing = 8, ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto } }, Children = { _type, remove } },
                new Grid { ColumnSpacing = 8, ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto } }, Children = { _host, _port } },
                _note),
        };
    }

    public void Show(EndpointRow row, JsonObject? shown, System.Action<EndpointRow> change, System.Action remove)
    {
        (_row, _change, _remove) = (row, change, remove);
        _options = DJ.Objs(shown, "options").Select(o => (DJ.Text(o, "value"), DJ.Text(o, "label"), EndpointRow.Of(DJ.Obj(o, "row")))).ToList();
        _quiet = true;
        var labels = _options.Select(o => o.Label).ToList();
        if (_type.ItemsSource is not List<string> held || !held.SequenceEqual(labels)) _type.ItemsSource = labels;
        _type.SelectedIndex = _options.ToList().FindIndex(o => o.Value == row.Scheme);
        _host.Header = DJ.Text(shown, "hostLabel") is { Length: > 0 } label ? label : "Address";
        AutomationProperties.SetName(_host, _host.Header as string ?? "Address");
        _host.PlaceholderText = DJ.Text(shown, "placeholder");
        if (_host.Text != row.Host) _host.Text = row.Host;
        if (_port.Text != row.Port) _port.Text = row.Port;
        _quiet = false;
        var problem = DJ.Text(shown, "problem");
        _note.Text = problem.Length > 0 ? problem : DJ.Text(shown, "note");
        _note.Foreground = FormLook.Brush(problem.Length > 0 ? "CriticalBrush" : "TextTertiaryBrush");
        _note.Visibility = FormLook.Shown(_note.Text.Length > 0);
    }
}
