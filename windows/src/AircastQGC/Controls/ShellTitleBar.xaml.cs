using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;

namespace AircastQGC.Controls;

public sealed partial class ShellTitleBar : UserControl
{
    private const string FlyStatePath = "view.flyState";
    private const string VehicleLinksPath = "view.vehicleLinks";
    private const string FlightModesPath = "view.flightModes";
    private const string BatteryPath = "view.battery";
    private const string GpsPath = "view.gps";
    private const string ArmedGlyph = "";
    private const string DisarmedGlyph = "";
    private const double StaleOpacity = 0.45;

    private static readonly string[] Watched = { FlyStatePath, VehicleLinksPath, FlightModesPath, BatteryPath, GpsPath };
    private static readonly string[] KnownStates = { "notConnected", "contactLost", "flying", "landing", "armed", "disarmed" };

    private readonly Dictionary<string, JsonObject> _views = new();
    private string _pendingMode = "";

    private enum Tone { Good, Caution, Critical, Unknown }

    public ShellTitleBar()
    {
        InitializeComponent();
        Loaded += (_, _) => Start();
        Unloaded += (_, _) => Array.ForEach(Watched, path => BridgeWatch.Stop(Client(path)));
        Render();
    }

    public event EventHandler<FrameworkElement>? FlightModeRequested;

    public FrameworkElement DragRegion => DragArea;

    public FrameworkElement FlightModeAnchor => ModePill;

    public double CaptionInset
    {
        get => CaptionColumn.Width.Value;
        set => CaptionColumn.Width = new GridLength(Math.Max(0, value));
    }

    public string PendingMode
    {
        get => _pendingMode;
        set
        {
            _pendingMode = value ?? "";
            Render();
        }
    }

    private static string Client(string path) => $"shellTitleBar.{path}";

    private void Start()
    {
        if (!CoreHost.Started) return;
        Array.ForEach(Watched, path =>
        {
            _views[path] = Bridge.Get(path);
            BridgeWatch.Watch(Client(path), new[] { path }, view =>
            {
                _views[path] = view;
                Render();
            });
        });
        Render();
    }

    private void OnModeClick(object sender, RoutedEventArgs e) => FlightModeRequested?.Invoke(this, ModePill);

    private JsonObject View(string path) => _views.TryGetValue(path, out var view) ? view : new JsonObject();

    private void Render()
    {
        var state = View(FlyStatePath);
        var connected = Flag(state, "connected") == true;
        var token = Text(state, "state");
        var stateText = Text(state, "stateText");
        var alarming = connected && (token == "contactLost" || !KnownStates.Contains(token));

        Subtitle.Text = !connected ? "Connect a vehicle to fly" : stateText.Length == 0 ? "Vehicle state unknown" : stateText;
        Subtitle.Foreground = Token(alarming ? "CriticalBrush" : "TextTertiaryBrush");

        var (linkText, linkTone) = Link(connected, Flag(state, "contactLost"), View(VehicleLinksPath));
        LinkText.Text = linkText;
        Paint(LinkPill, LinkIcon, linkTone);

        Readings.Visibility = Shown(connected);
        Readings.Opacity = Text(state, "staleNotice").Length == 0 ? 1 : StaleOpacity;

        var mode = Text(state, "mode");
        var pending = _pendingMode.Length > 0;
        var offersModes = View(FlightModesPath)["modes"] is JsonArray { Count: > 0 };
        ModeText.Text = pending ? _pendingMode : mode.Length == 0 ? "Mode unknown" : mode;
        ModePill.IsEnabled = offersModes;
        ModeChevron.Visibility = Shown(offersModes && !pending);
        ModePending.Visibility = Shown(pending);
        ModePending.IsActive = pending;

        var armed = Flag(state, "armed") == true;
        ArmedText.Text = armed ? "Armed" : "Disarmed";
        ArmedIcon.Glyph = armed ? ArmedGlyph : DisarmedGlyph;
        Paint(ArmedPill, ArmedIcon, armed ? Tone.Critical : Tone.Unknown);

        var battery = View(BatteryPath);
        var (batteryText, batteryTone) = BatteryReading(battery);
        BatteryPill.Visibility = Shown(Flag(battery, "available") == true);
        BatteryText.Text = batteryText;
        Paint(BatteryPill, BatteryIcon, batteryTone);

        var gps = View(GpsPath);
        var (gpsText, gpsTone) = GpsReading(gps);
        GpsPill.Visibility = Shown(Flag(gps, "available") == true);
        GpsText.Text = gpsText;
        Paint(GpsPill, GpsIcon, gpsTone);
    }

    private static (string Label, Tone Level) Link(bool connected, bool? contactLost, JsonObject view)
    {
        if (!connected) return ("No vehicle", Tone.Unknown);
        if (contactLost == true) return ("Contact lost", Tone.Critical);
        var links = (view["links"] as JsonArray)?.OfType<JsonObject>().ToList() ?? new List<JsonObject>();
        var silent = links.Count(link => Flag(link, "commLost") == true);
        var heard = contactLost == false ? Tone.Good : Tone.Unknown;
        var single = links.Count == 1 ? Text(links[0], "name") : "";
        return links.Count switch
        {
            < 2 => (single.Length == 0 ? "Connected" : single, heard),
            _ when silent == 0 => ($"{links.Count} links", heard),
            _ when silent == links.Count => ("No link heard", Tone.Critical),
            _ => (silent == 1 ? "1 link lost" : $"{silent} links lost", Tone.Caution),
        };
    }

    private static (string Label, Tone Level) BatteryReading(JsonObject view)
    {
        var first = view["packs"] is JsonArray { Count: > 0 } packs ? packs[0] as JsonObject : null;
        var main = first is null ? "" : Text(first, "text");
        var secondary = first is null ? "" : Text(first, "secondaryText");
        var text = main.Length == 0 ? "—" : secondary.Length == 0 || secondary == main ? main : $"{main} · {secondary}";
        var tone = Text(view, "level") switch
        {
            "normal" => Tone.Good,
            "caution" or "warning" => Tone.Caution,
            "critical" => Tone.Critical,
            _ => Tone.Unknown,
        };
        return (text, tone);
    }

    private static (string Label, Tone Level) GpsReading(JsonObject view)
    {
        var fixType = Whole(view, "lock");
        var satellites = Whole(view, "satellites");
        var fix = Text(view, "lockText");
        var text = fixType is null ? "—"
            : satellites is null ? (fix.Length == 0 ? "—" : fix)
            : fix.Length == 0 ? $"{satellites} sats" : $"{fix} · {satellites} sats";
        var tone = fixType switch
        {
            null => Tone.Unknown,
            >= 3 => Tone.Good,
            2 => Tone.Caution,
            _ => Tone.Critical,
        };
        return (text, tone);
    }

    private static void Paint(Border pill, FontIcon icon, Tone tone)
    {
        icon.Foreground = Token(tone switch
        {
            Tone.Good => "SuccessBrush",
            Tone.Caution => "CautionBrush",
            Tone.Critical => "CriticalBrush",
            _ => "TextTertiaryBrush",
        });
        pill.Background = Token(tone switch
        {
            Tone.Caution => "CautionBgBrush",
            Tone.Critical => "CriticalBgBrush",
            _ => "ControlBrush",
        });
    }

    private static Brush Token(string key) =>
        Application.Current.Resources.TryGetValue(key, out var value) && value is Brush brush
            ? brush
            : new SolidColorBrush(Colors.Transparent);

    private static bool? Flag(JsonObject view, string key) =>
        view[key] is JsonValue value && value.TryGetValue<bool>(out var flag) ? flag : null;

    private static string Text(JsonObject view, string key) =>
        view[key] is JsonValue value && value.TryGetValue<string>(out var text) ? text ?? "" : "";

    private static int? Whole(JsonObject view, string key) =>
        view[key] is JsonValue value && value.TryGetValue<double>(out var number) ? (int)number : null;

    private static Visibility Shown(bool on) => on ? Visibility.Visible : Visibility.Collapsed;
}
