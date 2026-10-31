using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Forms;
using AircastQGC.Pages.Analyze;
using AircastQGC.Pages.Settings;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using WinBattery = Windows.Devices.Power.Battery;

namespace AircastQGC.Controls;

public sealed class StatusStrip : UserControl
{
    private const string FlyStatePath = "view.flyState";
    private const string BatteryPath = "view.battery";
    private const string GpsPath = "view.gps";
    private const string LinksPath = "view.vehicleLinks";
    private const string EscPath = "view.escs";
    private const string RemoteIdPath = "view.remoteIdStatus";
    private const string ResiliencePath = "view.gpsResilience";
    private const string AircastLinkPath = "view.aircastLink";
    private const string JoystickPath = "view.joystick";
    private const string RtkPath = "view.gpsRtk";
    private const string OfflinePath = "view.offlineStatus";
    private const string FlightTimePath = "vehicle.flightTime";
    private const string ClearOverrides = "vehicle.clearRcChannelOverrides";
    private const string RemoteIdEmergency = "vehicle.remoteIDManager.setEmergency";
    private const string AdvancedUiPath = "view.advancedUi";
    private const string SetupPath = "view.setup";
    private const string FlyViewSettings = "Fly View";
    private const string BatteryIndicatorGroup = "batteryIndicatorSettings";
    private const string BatterySettingsPage = "Battery Settings";
    private const string PowerSetupPage = "Power";
    private const string RemoteIdSettingsPage = "Remote ID";
    private const double StaleOpacity = 0.45;

    private const string SupportLinksPath = "view.links";

    private static readonly string[] Watched = [FlyStatePath, BatteryPath, GpsPath, LinksPath, EscPath, RemoteIdPath, ResiliencePath, AircastLinkPath, JoystickPath];
    private static readonly string[] ListedWatched = [.. Watched, GimbalCalls.View, UsbRadio.View, SupportLinksPath];
    private static readonly string[] BatteryDisplayFacts = ["valueDisplay", "threshold1", "threshold2"];

    private readonly string _client = $"statusStrip.{Guid.NewGuid():N}";
    private readonly DispatcherTimer _tick = new() { Interval = TimeSpan.FromSeconds(1) };
    private readonly StatusStrip? _owner;
    private readonly IReadOnlyList<StatusCell> _cells;
    private readonly StatusRow? _row;
    private readonly IReadOnlyDictionary<string, Action<FrameworkElement>> _opens;
    private readonly GimbalIndicator? _gimbal;
    private readonly SupportForwardingCell? _support;
    private readonly StatusPill _gimbalPill = StatusLook.Pill("Gimbal", "");
    private readonly StatusPill _usbPill = StatusLook.Pill("USB radio", "");
    private readonly StatusPill _supportPill = StatusLook.Pill("Support", "");
    private ImmutableDictionary<FrameworkElement, StatusTone> _tones = ImmutableDictionary<FrameworkElement, StatusTone>.Empty;
    private readonly StatusPill _battery = StatusLook.Pill("Battery", "");
    private readonly StatusPill _flightTime = StatusLook.Pill("Flight time", "");
    private readonly StatusPill _gps = StatusLook.Pill("GPS", "");
    private readonly StatusPill _rc = StatusLook.Pill("RC signal", "");
    private readonly StatusPill _override = StatusLook.Pill("RC override", "");
    private readonly StatusPill _telemetry = StatusLook.Pill("Telemetry RSSI", "");
    private readonly StatusPill _aircastLink = StatusLook.Pill("Cellular link", "");
    private readonly StatusPill _esc = StatusLook.Pill("ESC status", "");
    private readonly StatusPill _joystick = StatusLook.Pill("Joystick", "");
    private readonly StatusPill _remoteId = StatusLook.Pill("Remote ID", "");
    private readonly StatusPill _resilience = StatusLook.Pill("GPS resilience", "");
    private readonly StatusPill _rtk = StatusLook.Pill("RTK GPS", "");
    private readonly StatusPill _gcsBattery = StatusLook.Pill("Ground station battery", "");
    private readonly IReadOnlyList<Sheet> _sheets;
    private readonly Sheet _remoteIdSheet;
    private readonly Sheet _batterySheet;
    private readonly Sheet _gpsSheet;
    private readonly Sheet _linksSheet;
    private readonly Sheet _offlineSheet;
    private readonly Sheet _rtkSettingsSheet;

    private readonly WinBattery _aggregate = WinBattery.AggregateBattery;

    private ImmutableDictionary<string, JsonObject> _views = ImmutableDictionary<string, JsonObject>.Empty;
    private JsonObject _rtkView = new();
    private JsonObject _offlineView = new();
    private GcsBatteryReading? _gcs;
    private double? _flownSeconds;
    private DateTime? _lostSince;
    private bool _started;
    private HoldButton? _emergency;
    private BatteryParts? _batteryParts;
    private (UIElement Row, Button Save)? _base;

    public StatusStrip()
        : this(null)
    {
    }

    private StatusStrip(StatusStrip? owner)
    {
        _owner = owner;
        _batterySheet = new Sheet("Battery", () => null, BatteryParts, closed: () => _batteryParts = null, refreshed: BatteryRefresh);
        _gpsSheet = new Sheet("Vehicle GPS status", GpsBody, () => [StatusLook.Link("RTK GPS settings", () => Then(_gpsSheet, () => _rtkSettingsSheet.ShowAt(Anchor(StatusKeys.Gps)))), StatusLook.Footnote(StatusText.Footnote)]);
        _linksSheet = new Sheet("Links to this aircraft", LinksBody, () => [StatusLook.Footnote(StatusText.Footnote), StatusLook.Action("Connection settings", () => Then(_linksSheet, FlyRequests.OpenConnections))]);
        _offlineSheet = new Sheet("Connect a vehicle", OfflineBody, OfflineFooter, opened: () => _offlineView = Bridge.Get(OfflinePath), key: () => _offlineView.ToJsonString());
        var rcSheet = new Sheet("RC RSSI status", () => Detail(Fly.RcRows), Footnote);
        var telemetrySheet = new Sheet("Telemetry RSSI status", () => Detail(Fly.Telemetry?.Rows ?? []), Footnote);
        var escSheet = new Sheet("ESC Status Overview", EscBody);
        _remoteIdSheet = new Sheet("RemoteID Status", RemoteIdBody, RemoteIdFooter, closed: () => _emergency = null, key: () => View(RemoteIdPath).ToJsonString());
        var resilienceSheet = new Sheet("GPS resilience", ResilienceBody);
        var rtkSheet = new Sheet("RTK GPS Status", RtkBody, RtkSettings, closed: ForgetBase, refreshed: PaintBase);
        _rtkSettingsSheet = new Sheet("RTK GPS settings", () => null, RtkSettings, closed: ForgetBase, refreshed: PaintBase);
        var aircastSheet = new Sheet("Cellular link", AircastBody);
        var joystickSheet = new Sheet("Joystick", JoystickBody);
        var gcsSheet = new Sheet("Ground Station", GcsBody);
        _sheets = [_batterySheet, _gpsSheet, _linksSheet, _offlineSheet, _rtkSettingsSheet, rcSheet, telemetrySheet, escSheet, _remoteIdSheet, resilienceSheet, rtkSheet, aircastSheet, joystickSheet, gcsSheet];

        _gimbal = owner is null ? new GimbalIndicator() : null;
        _support = owner is null ? new SupportForwardingCell() : null;
        _opens = new Dictionary<string, Action<FrameworkElement>>
        {
            [StatusKeys.Battery] = _batterySheet.ShowAt,
            [StatusKeys.Gps] = _gpsSheet.ShowAt,
            [StatusKeys.Rc] = rcSheet.ShowAt,
            [StatusKeys.RcOverride] = _ => Bridge.Invoke(ClearOverrides),
            [StatusKeys.Telemetry] = telemetrySheet.ShowAt,
            [StatusKeys.AircastLink] = aircastSheet.ShowAt,
            [StatusKeys.Esc] = escSheet.ShowAt,
            [StatusKeys.Joystick] = joystickSheet.ShowAt,
            [StatusKeys.RemoteId] = _remoteIdSheet.ShowAt,
            [StatusKeys.GpsResilience] = resilienceSheet.ShowAt,
            [StatusKeys.Rtk] = rtkSheet.ShowAt,
            [StatusKeys.GcsBattery] = gcsSheet.ShowAt,
            [StatusKeys.Gimbal] = anchor => _gimbal?.ShowSheetAt(anchor),
            [StatusKeys.SupportForwarding] = anchor => _support?.ShowSheetAt(anchor),
        };
        ToolTipService.SetToolTip(_override.Button, "RC override active. Click to release every overridden channel.");
        new[] { _flightTime, _usbPill }.ToList().ForEach(pill =>
        {
            pill.Button.IsHitTestVisible = false;
            pill.Button.IsTabStop = false;
        });

        _cells =
        [
            new(StatusKeys.Battery, "Battery", _battery.Button, true),
            new(StatusKeys.FlightTime, "Flight time", _flightTime.Button, true),
            new(StatusKeys.Gps, "GPS", _gps.Button, true),
            new(StatusKeys.Rc, "RC signal", _rc.Button, true),
            new(StatusKeys.RcOverride, "RC override", _override.Button, true),
            new(StatusKeys.Telemetry, "Telemetry RSSI", _telemetry.Button, true),
            new(StatusKeys.AircastLink, "Cellular link", _aircastLink.Button, true),
            new(StatusKeys.Esc, "ESC status", _esc.Button, true),
            new(StatusKeys.Joystick, "Joystick", _joystick.Button, true),
            new(StatusKeys.UsbRadio, "USB radio", owner is null ? new UsbRadioCell() : _usbPill.Button, false),
            new(StatusKeys.RemoteId, "Remote ID", _remoteId.Button, true),
            new(StatusKeys.GpsResilience, "GPS resilience", _resilience.Button, true),
            new(StatusKeys.Rtk, "RTK GPS", _rtk.Button, false),
            new(StatusKeys.GcsBattery, "Ground station battery", _gcsBattery.Button, false),
            new(StatusKeys.Gimbal, "Gimbal", (FrameworkElement?)_gimbal ?? _gimbalPill.Button, true),
            new(StatusKeys.SupportForwarding, "Support forwarding", (FrameworkElement?)_support ?? _supportPill.Button, false),
        ];
        _cells.Where(cell => cell.Cell is Button && _opens.ContainsKey(cell.Key)).ToList()
            .ForEach(cell => ((Button)cell.Cell).Click += (_, _) => Open(cell.Key, cell.Cell));
        _row = owner is null ? new StatusRow(_cells, cell => cell is UsbRadioCell usb ? usb.Tone : _tones.GetValueOrDefault(cell, StatusTone.Normal), () => new StatusStrip(this)) : null;
        if (_row is not null) _row.NeedChanged += () => NeedChanged?.Invoke();
        Content = _row?.View ?? new StatusList(_cells).View;
        VerticalAlignment = VerticalAlignment.Center;

        _tick.Tick += (_, _) => Tick();
        Loaded += (_, _) => Start();
        Unloaded += (_, _) => Stop();
        Render();
    }

    public void ShowLinks(FrameworkElement anchor) => (Fly.Connected ? _linksSheet : _offlineSheet).ShowAt(anchor);

    public double Needed => _row?.Needed ?? 0;

    public event Action? NeedChanged;

    private IReadOnlyList<string> Paths => _owner is null ? Watched : ListedWatched;

    private void Open(string key, FrameworkElement from)
    {
        if (_owner is { } owner) owner.OpenFromList(key);
        else if (_opens.TryGetValue(key, out var open)) open(from);
    }

    private void OpenFromList(string key)
    {
        _row?.HideAll();
        DispatcherQueue.TryEnqueue(() =>
        {
            if (_opens.TryGetValue(key, out var open)) open(Anchor(key));
        });
    }

    private FrameworkElement Anchor(string key)
    {
        var cell = _cells.First(c => c.Key == key).Cell;
        return _row is { } row && !row.Shows(cell) ? row.Anchor : cell;
    }

    private FlyStatus Fly => FlyStatus.Parse(View(FlyStatePath));

    private RemoteIdStatus? RemoteId => RemoteIdStatus.Parse(View(RemoteIdPath));

    private JsonObject View(string path) => _views.TryGetValue(path, out var view) ? view : new JsonObject();

    private string Client(string path) => $"{_client}.{path}";

    private bool Stale => Fly.Stale;

    private void Start()
    {
        if (!CoreHost.Started || _started) return;
        _started = true;
        _views = Paths.ToImmutableDictionary(path => path, Bridge.Get);
        Paths.ToList().ForEach(path => BridgeWatch.Watch(Client(path), [path], view =>
        {
            _views = _views.SetItem(path, view);
            Render();
        }));
        _aggregate.ReportUpdated += OnGcsBattery;
        ReadGcsBattery();
        Tick();
        _tick.Start();
    }

    private void Stop()
    {
        _started = false;
        _tick.Stop();
        _aggregate.ReportUpdated -= OnGcsBattery;
        Paths.ToList().ForEach(path => BridgeWatch.Stop(Client(path)));
    }

    private void OnGcsBattery(WinBattery sender, object args) => DispatcherQueue.TryEnqueue(() =>
    {
        ReadGcsBattery();
        Render();
    });

    private void ReadGcsBattery()
    {
        var report = _aggregate.GetReport();
        var charge = report.RemainingCapacityInMilliwattHours is int left && report.FullChargeCapacityInMilliwattHours is int full && full > 0
            ? (int?)Math.Clamp((int)Math.Round(left * 100.0 / full), 0, 100)
            : null;
        var charging = report.Status is Windows.System.Power.BatteryStatus.Charging or Windows.System.Power.BatteryStatus.Idle;
        _gcs = report.Status != Windows.System.Power.BatteryStatus.NotPresent && charge is int percent
            ? GcsBatteryReading.Parse(Bridge.Get(GcsBatteryReading.PathFor(percent, charging)))
            : null;
    }

    private void Tick()
    {
        _rtkView = Bridge.Get(RtkPath);
        _flownSeconds = Fly.Connected ? StatusJson.Number(Bridge.Get(FlightTimePath)["value"]) : null;
        if (_offlineSheet.IsOpen) _offlineView = Bridge.Get(OfflinePath);
        _row?.Refit();
        Render();
    }

    private void Render()
    {
        var fly = Fly;
        var lost = VehicleLinks.Parse(View(LinksPath))?.ContactLost == true;
        _lostSince = lost ? _lostSince ?? DateTime.UtcNow : null;
        if (fly.Connected && _offlineSheet.IsOpen) _offlineSheet.Hide();

        _cells.Where(c => c.Vehicle).ToList().ForEach(c => c.Cell.Opacity = fly.Stale ? StaleOpacity : 1);
        if (_owner is null) OverlayLayout.LockWhileArmed(fly.Connected && fly.Armed);

        var packs = BatteryModel.Packs(View(BatteryPath));
        var headline = BatteryHeadline.Parse(View(BatteryPath));
        Show(_battery, fly.Connected && packs.Count > 0,
            string.Join("  ", packs.Select(p => FormText.Spaced(p.Text, BatteryModel.CountText(p.Packs)))),
            BatteryModel.Worst(packs),
            packs.Count == 0 ? "" : headline?.TimeLeft ?? "");

        Show(_flightTime, fly.Connected && (fly.Armed || _flownSeconds > 0), StatusText.FlightTime(_flownSeconds), StatusTone.Normal);

        var gps = GpsStatus.Parse(View(GpsPath));
        Show(_gps, fly.Connected && gps is not null, gps?.CellText ?? "", gps?.Tone ?? StatusTone.Normal);

        var rc = fly.RcCell;
        Show(_rc, fly.Connected && rc is not null, rc?.Text ?? "", rc?.Lost == true ? StatusTone.Critical : StatusTone.Normal);
        Show(_override, fly.Connected && fly.OverrideShown, "RC override", StatusTone.Caution);
        Show(_telemetry, fly.Connected && fly.Telemetry is not null, fly.Telemetry?.CellText ?? "", StatusTone.Normal);

        var cellular = AircastLink.Parse(View(AircastLinkPath));
        Show(_aircastLink, fly.Connected && cellular is not null, cellular?.CellText ?? "", StatusTone.Normal);

        var esc = EscSummary.Parse(View(EscPath));
        Show(_esc, fly.Connected && esc is not null, esc?.CellText ?? "", esc?.Healthy == true ? StatusTone.Good : StatusTone.Critical);

        var joystick = JoystickBadge.Parse(View(JoystickPath));
        Show(_joystick, fly.Connected && joystick is not null, "Joystick", joystick?.Warn == true ? StatusTone.Caution : StatusTone.Normal);

        var remoteId = RemoteId;
        Show(_remoteId, fly.Connected && remoteId is not null, "RID", remoteId?.Tone ?? StatusTone.Muted);
        if (_emergency is not null) _emergency.Label.Text = remoteId?.EmergencyButton ?? "";

        var resilience = GpsResilience.Parse(View(ResiliencePath));
        Show(_resilience, fly.Connected && resilience is not null && (resilience.Interference.Shown || resilience.Authentication.Shown),
            string.Join(" ", new[] { resilience?.Interference.Shown == true ? "RF" : "", resilience?.Authentication.Shown == true ? "AUTH" : "" }.Where(t => t.Length > 0)),
            new[] { resilience?.Interference, resilience?.Authentication }.Where(m => m?.Shown == true).Select(m => m!.Tone).DefaultIfEmpty(StatusTone.Muted).Max());

        Show(_rtk, RtkStatus.Parse(_rtkView) is not null, "RTK", StatusTone.Normal);
        Show(_gcsBattery, _gcs is not null, _gcs?.LevelText ?? "", _gcs?.Tone ?? StatusTone.Normal);
        AutomationProperties.SetName(_gcsBattery.Button, _gcs is null ? "Ground station battery" : $"{_gcs.Title} {_gcs.LevelText}");

        if (_owner is not null)
        {
            var gimbal = PayloadJson.Gimbal(View(GimbalCalls.View));
            Show(_gimbalPill, fly.Connected && gimbal is not null, gimbal is { CellText.Length: > 0 } shown ? shown.CellText : "Gimbal", StatusTone.Normal);
            var usb = UsbRadio.Badge(View(UsbRadio.View));
            Show(_usbPill, usb is not null, usb?.Text ?? "", usb is null ? StatusTone.Normal : UsbRadio.Tone(usb.State));
            Show(_supportPill, MapUiJson.Flag(View(SupportLinksPath)["supportForwarding"]), "Support", StatusTone.Good);
        }

        _sheets.ToList().ForEach(sheet => sheet.Refresh());
        _row?.Paint();
    }

    private void Show(StatusPill pill, bool shown, string text, StatusTone tone, string extra = "")
    {
        pill.Button.Visibility = Ui.Shown(shown);
        _tones = _tones.SetItem(pill.Button, shown ? tone : StatusTone.Normal);
        if (shown) StatusLook.Paint(pill, text, tone, extra);
    }

    private void Then(Sheet? open, Action next)
    {
        open?.Hide();
        DispatcherQueue.TryEnqueue(() => next());
    }

    private string? Silence => _lostSince is DateTime since ? StatusText.Silence((long)(DateTime.UtcNow - since).TotalSeconds) : null;

    private UIElement? SilenceLine() => Silence is string text ? Ui.Text(text, semibold: true, brush: "CriticalBrush") : null;

    private UIElement Detail(IReadOnlyList<DetailRow> rows) => Ui.Column([SilenceLine(), StatusLook.Rows(rows, Silence is not null)], 8);

    private static IEnumerable<UIElement?> Footnote() => [StatusLook.Footnote(StatusText.Footnote)];

    private UIElement BatteryHeadlineView()
    {
        var headline = BatteryHeadline.Parse(View(BatteryPath));
        var stale = Silence is not null;
        return Ui.Column(
        [
            SilenceLine(),
            headline is null ? null : Ui.Column(
            [
                Ui.Text(headline.Text, "FontSizeTitle", semibold: true).With(t => t.Foreground = stale ? Ui.Token("TextTertiaryBrush") : StatusLook.Ink(headline.HeadlineTone)),
                headline.Detail.Trim().Length > 0 ? StatusLook.Line(headline.Detail, StatusTone.Muted) : null,
                headline.Margin.Trim().Length > 0 ? StatusLook.Line(headline.Margin, stale ? StatusTone.Muted : StatusTone.Normal) : null,
                headline.Percent is null ? null : StatusLook.ReturnBar(headline, !Fly.Stale),
            ], 4),
        ], 8);
    }

    private UIElement BatteryRowsView()
    {
        var view = View(BatteryPath);
        var rows = new[] { BatteryModel.TotalDraw(view) is string draw ? new DetailRow("Total draw", draw) : null }
            .OfType<DetailRow>()
            .Concat(BatteryModel.Rows(view))
            .ToList();
        return StatusLook.Rows(rows, Silence is not null);
    }

    private IEnumerable<UIElement?> BatteryParts()
    {
        var advanced = StatusJson.Flag(Bridge.Get(AdvancedUiPath)["shown"], true);
        var power = StatusJson.Objects(Bridge.Get(SetupPath)["components"]).Any(c => StatusJson.Text(c["name"]) == PowerSetupPage);
        var parts = new BatteryParts(
            new ContentControl { HorizontalContentAlignment = HorizontalAlignment.Stretch, IsTabStop = false },
            StatusLook.Action("Return", () => Then(_batterySheet, () => FlyRequests.AskGuided("rtl")), Look.Critical).Named("Return home"),
            new ContentControl { HorizontalContentAlignment = HorizontalAlignment.Stretch, IsTabStop = false });
        _batteryParts = parts;
        return
        [
            parts.Headline,
            parts.Return,
            parts.Rows,
            StatusLook.Link("Battery failsafes", () => Then(_batterySheet, () => BatteryFailsafes().ShowAt(Anchor(StatusKeys.Battery)))),
            StatusLook.Link("Battery display", () => Then(_batterySheet, () => BatteryDisplay().ShowAt(Anchor(StatusKeys.Battery)))),
            advanced && power ? StatusLook.Link("Power setup", () => Then(_batterySheet, () => FlyRequests.OpenSetup(PowerSetupPage))) : null,
            StatusLook.Footnote(StatusText.Footnote),
        ];
    }

    private void BatteryRefresh()
    {
        if (_batteryParts is not BatteryParts parts) return;
        var rtl = GuidedOffer.List(Bridge.Get(GuidedOffer.ViewPath)).FirstOrDefault(o => o.Id == "rtl");
        parts.Headline.Content = BatteryHeadlineView();
        parts.Rows.Content = BatteryRowsView();
        parts.Return.Visibility = Ui.Shown(BatteryModel.ReturnOffered(View(BatteryPath)) && rtl is { Shown: true });
        parts.Return.IsEnabled = rtl?.Ready == true;
    }

    private static string? ParameterWait()
    {
        var setup = Bridge.Get(SetupPath);
        return StatusJson.Flag(setup["connected"]) && StatusJson.Flag(setup["parametersReady"]) ? null
            : StatusJson.Text(setup["parametersReason"]) == "skipped" ? "Parameters not available"
            : "Waiting for parameters…";
    }

    private static Sheet BatteryFailsafes() => new("Battery failsafes", () => null,
        () => [ParameterWait() is string wait ? StatusLook.Line(wait, StatusTone.Muted) : FormView.SetupPage(BatterySettingsPage)]);

    private static Sheet BatteryDisplay() => new("Battery display", () => null, () =>
    [
        new FormView(() => Bridge.Get(SettingsCatalog.PagePath(FlyViewSettings)), raw => FormParse.SettingsSections(raw)
            .Where(section => section.Group == BatteryIndicatorGroup)
            .Select(section => section with
            {
                Title = "",
                Blocks = section.Blocks
                    .Select(block => block with { Controls = block.Controls.Where(c => BatteryDisplayFacts.Contains(c.Name)).ToList() })
                    .Where(block => block.Controls.Count > 0)
                    .ToList(),
            })
            .Where(section => section.Blocks.Count > 0)
            .ToList()),
    ]);

    private UIElement GpsBody() => Detail(GpsStatus.Parse(View(GpsPath))?.Detail ?? []);

    private UIElement LinksBody() => Detail(VehicleLinks.Parse(View(LinksPath))?.Detail ?? []);

    private UIElement? OfflineBody()
    {
        if (OfflineStatus.Parse(_offlineView) is not OfflineStatus shown) return StatusLook.Line("Reading the links.", StatusTone.Muted);
        var footnote = Ui.Line(
        [
            shown.Busy ? new ProgressRing { IsActive = true, Width = 16, Height = 16, MinWidth = 16, MinHeight = 16 } : null,
            Ui.Caption(shown.Footnote, "TextTertiaryBrush"),
        ], 8);
        return Ui.Column(
        [
            StatusLook.Line(shown.Title).With(t => t.FontWeight = Microsoft.UI.Text.FontWeights.SemiBold),
            StatusLook.Line(OfflineStatus.LookingHint),
            shown.EditAddress ? StatusLook.ListRow(footnote, shown.Footnote, () => Then(_offlineSheet, FlyRequests.OpenConnections)) : footnote,
            shown.NoLinks ? StatusLook.Action("Add link…", () => Then(_offlineSheet, FlyRequests.OpenConnections), Look.Accent) : null,
            .. shown.Links.Select(link => (UIElement?)StatusLook.ListRow(
                StatusLook.Chevron(
                    Ui.Column([StatusLook.Line(link.Text), StatusLook.Line(link.Description, link.Failed ? StatusTone.Critical : StatusTone.Muted, small: true)], 2),
                    link.Connected ? new ProgressRing { IsActive = true, Width = 16, Height = 16, MinWidth = 16, MinHeight = 16 }
                    : link.Retry ? Ui.Caption("Retry", "AccentTextBrush") : null),
                link.Text,
                () => Ui.Off(() => _ = link.Connected ? LinkCommands.Disconnect(link.Index) : LinkCommands.Connect($"@{LinkModel.Configurations}.{link.Index}")))),
        ], 8);
    }

    private IEnumerable<UIElement?> OfflineFooter() =>
    [
        new Border { Height = 1, Background = Ui.Token("DividerBrush"), Margin = new Thickness(0, 4, 0, 4) },
        StatusLook.Action("Connection settings", () => Then(_offlineSheet, FlyRequests.OpenConnections)),
    ];

    private UIElement? EscBody() => EscSummary.Parse(View(EscPath)) is not EscSummary esc ? null : Ui.Column(
    [
        StatusLook.Row("Healthy Motors", esc.HealthyMotorsText),
        StatusLook.Row("Total Errors", esc.TotalErrors.ToString(System.Globalization.CultureInfo.InvariantCulture)),
        .. esc.Motors.Select(motor => (UIElement?)Ui.Column(
        [
            StatusLook.Heading(motor.Title).With(t => t.Foreground = StatusLook.Tint(motor.Healthy ? StatusTone.Good : StatusTone.Critical)),
            StatusLook.Pairs(motor.Rows),
        ], 4)),
    ], 8);

    private UIElement? RemoteIdBody() => RemoteId is not RemoteIdStatus status ? null : Ui.Column(
    [
        .. status.Rows.Select(row => (UIElement?)StatusLook.ListRow(
            Ui.Text(row.Label, semibold: true).With(t => t.Foreground = StatusLook.Tint(row.Good ? StatusTone.Good : StatusTone.Critical)),
            row.Label,
            () => Then(_remoteIdSheet, () => FlyRequests.OpenSettings(RemoteIdSettingsPage)))),
        status.ArmError.Trim().Length > 0 ? StatusLook.Row("Arm Status Error", status.ArmError) : null,
        status.Comms ? StatusLook.Line(status.EmergencyHint, small: true) : null,
    ], 6);

    private IEnumerable<UIElement?> RemoteIdFooter()
    {
        var advanced = StatusJson.Flag(Bridge.Get(AdvancedUiPath)["shown"], true);
        _emergency = new HoldButton("Remote ID emergency", () => RemoteId?.HoldMs ?? 800, () =>
        {
            if (RemoteId is { } status) Bridge.Invoke(RemoteIdEmergency, [JsonValue.Create(!status.Emergency)]);
        });
        _emergency.Label.Text = RemoteId?.EmergencyButton ?? "";
        return
        [
            RemoteId?.Comms == true ? _emergency.Button : null,
            StatusLook.Heading("Self ID"),
            new FormView(() => Bridge.Get(SettingsCatalog.PagePath(RemoteIdSettingsPage)), SelfIdSections),
            advanced ? StatusLook.Action("Configure Remote ID", () => Then(_remoteIdSheet, () => FlyRequests.OpenSettings(RemoteIdSettingsPage))) : null,
        ];
    }

    private static IReadOnlyList<FormSection> SelfIdSections(JsonObject raw)
    {
        var all = FormParse.SettingsSections(raw).SelectMany(section => section.Controls).ToList();
        var named = RemoteIdStatus.SelfIdFacts
            .Select(name => all.FirstOrDefault(c => c.Name == name || c.Path.EndsWith("." + name, StringComparison.Ordinal)))
            .OfType<FormControl>()
            .ToList();
        var broadcasting = named.FirstOrDefault(c => c.Name == RemoteIdStatus.SendSelfId || c.Path.EndsWith("." + RemoteIdStatus.SendSelfId, StringComparison.Ordinal)) is { } send
            && FormJson.Truthy(send.Value);
        var controls = named
            .Select(c => (Control: c, Name: RemoteIdStatus.SelfIdFacts.First(n => c.Name == n || c.Path.EndsWith("." + n, StringComparison.Ordinal))))
            .Select(p => RemoteIdStatus.SelfIdLabels.TryGetValue(p.Name, out var label) ? (Control: p.Control with { Label = label, ShortLabel = "" }, p.Name) : p)
            .Select(p => RemoteIdStatus.BroadcastGated.Contains(p.Name) && !broadcasting ? p.Control with { Enabled = false, DisabledReason = "Broadcast is off" } : p.Control)
            .ToList();
        return controls.Count == 0
            ? []
            : [new FormSection("", "", broadcasting ? "" : RemoteIdStatus.SelfIdNote, "", [], [new FormBlock("", controls)])];
    }

    private UIElement? ResilienceBody() => GpsResilience.Parse(View(ResiliencePath)) is not GpsResilience resilience ? null : Ui.Column(
        resilience.Sections.Select(section => (UIElement?)Ui.Column([StatusLook.Heading(section.Title), StatusLook.Pairs(section.Rows)], 4)), 8);

    private UIElement? RtkBody() => RtkStatus.Parse(_rtkView) is not RtkStatus rtk ? null : Ui.Column(
    [
        StatusLook.Line(rtk.Headline),
        StatusLook.Pairs(rtk.Rows),
    ], 8);

    private IEnumerable<UIElement?> RtkSettings()
    {
        var auto = Bridge.Get(RtkStatus.AutoConnectPath);
        var toggle = new ToggleSwitch { Header = "AutoConnect", IsOn = FormJson.Truthy(auto["value"]) }.Named("RTK AutoConnect");
        toggle.Toggled += (_, _) => Bridge.Set(RtkStatus.AutoConnectPath, JsonValue.Create(toggle.IsOn));
        var save = Ui.Button("Save", SaveBase).Named("Save current base position");
        var row = StatusLook.Spread(StatusLook.Line("Current base position"), save);
        _base = (row, save);
        return
        [
            StatusLook.Heading("RTK GPS Settings"),
            toggle,
            new FormView(() => Bridge.Get(SettingsCatalog.PagePath(RtkStatus.SettingsPage)), raw => FormParse.SettingsSections(raw)),
            row,
        ];
    }

    private void ForgetBase() => _base = null;

    private void PaintBase()
    {
        if (_base is not { } parts) return;
        var (row, save) = parts;
        var writes = RtkStatus.Parse(_rtkView)?.BaseWrites;
        row.Visibility = Ui.Shown(FormJson.Truthy(Bridge.Get($"{RtkStatus.SettingsGroup}.useFixedBasePosition")["value"]));
        save.IsEnabled = writes is not null;
        save.Content = writes is null ? "Not Yet Valid" : "Save";
    }

    private void SaveBase()
    {
        var writes = RtkStatus.Parse(_rtkView)?.BaseWrites;
        Ui.Off(() => writes?.ToList().ForEach(w => Bridge.Set(w.Path, JsonValue.Create(w.Value))));
    }

    private UIElement? AircastBody() => AircastLink.Parse(View(AircastLinkPath)) is not AircastLink link ? null : Ui.Column(
    [
        StatusLook.Pairs([("Signal:", link.SignalText), ("Network:", link.Network), ("Modem:", link.Modem), ("Video bitrate:", link.BitrateText)]),
        Ui.Caption("Signal, up to the last hour"),
        StatusLook.Sparkline(link.QualityHistory, AircastLink.SignalMaximum),
        Ui.Caption("Video bitrate, up to the last hour"),
        StatusLook.Sparkline(link.BitrateHistory, 0),
    ], 6);

    private UIElement? JoystickBody() => JoystickBadge.Parse(View(JoystickPath)) is not JoystickBadge badge ? null : Ui.Column(
    [
        StatusLook.Line(badge.Heading).With(t => t.FontWeight = Microsoft.UI.Text.FontWeights.SemiBold),
        StatusLook.Row("Enabled:", badge.EnabledText, badge.Warn ? StatusTone.Caution : StatusTone.Normal),
        StatusLook.Row("Type:", badge.TypeText),
        StatusLook.Row("Inputs:", badge.InputsText),
        .. badge.Details.Select(d => StatusLook.Row(d.Label, d.Value, d.Tone)),
    ], 6);

    private UIElement? GcsBody() => _gcs is not GcsBatteryReading gcs ? null : Ui.Column(
    [
        StatusLook.Row("Charge", gcs.LevelText, gcs.Tone),
        StatusLook.Row("State", gcs.StateText),
    ], 6);
}

internal sealed record BatteryParts(ContentControl Headline, Button Return, ContentControl Rows);
