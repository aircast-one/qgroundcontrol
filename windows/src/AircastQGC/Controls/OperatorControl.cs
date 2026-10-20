using System;
using System.Collections.Immutable;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;

namespace AircastQGC.Controls;

internal sealed record ControlStation(bool Known, bool? InControl, int? HolderSystemId, bool? TakeoverAllowed, bool RequestAllowed, string Reason);

internal sealed record IncomingControlRequest(int SystemId, long TimeoutMs, long RemainingMs);

internal sealed record ControlAsk(string? Refusal, int TimeoutSeconds);

internal static class OperatorControl
{
    public const string ViewPath = "view.operatorControl";
    public const string RequestPath = "vehicle.requestOperatorControl";
    public const string RevertPath = "vehicle.startTimerRevertAllowTakeover";
    public const string AllowTakeoverPath = "settings.flyViewSettings.requestControlAllowTakeover";
    public const string RequestTimeoutPath = "settings.flyViewSettings.requestControlTimeout";
    public const string GcsSystemIdPath = "settings.mavlinkSettings.gcsMavlinkSystemID";
    public const string VehiclesPath = "view.vehicles";

    public static string Control(string path) => $"view.control({path})";

    public static ControlStation? Station(JsonObject view) =>
        MapUiJson.Flag(view["available"])
            ? new ControlStation(
                MapUiJson.Flag(view["known"]),
                Maybe(view["inControl"]),
                Whole(view["holderSystemId"]),
                Maybe(view["takeoverAllowed"]),
                MapUiJson.Flag(view["requestAllowed"]),
                MapUiJson.Text(view["reason"]))
            : null;

    public static IncomingControlRequest? Incoming(JsonObject view) =>
        view["incomingRequest"] is JsonObject incoming
            ? new IncomingControlRequest(
                Whole(incoming["systemId"]) ?? 0,
                (long)(MapUiJson.Number(incoming["timeoutMs"]) ?? 0),
                (long)(MapUiJson.Number(incoming["remainingMs"]) ?? 0))
            : null;

    public static long? RevertMs(JsonObject view) =>
        MapUiJson.Flag(view["inControl"]) && MapUiJson.Number(view["takeoverRevertMs"]) is double ms ? (long)ms : null;

    public static int SecondsLeft(long ms) => (int)Math.Ceiling(ms / 1000.0);

    public static bool Elsewhere(ControlStation? station) => station?.InControl == false;

    public static string? ControlLine(ControlStation? station) =>
        station is not { Known: true } || station.InControl == true || string.IsNullOrWhiteSpace(station.Reason) ? null
        : station.HolderSystemId is int holder ? $"{station.Reason} (system {holder})"
        : station.Reason;

    public static string? HolderLine(ControlStation? station) =>
        station is { Known: true, InControl: false }
            ? station.HolderSystemId is int holder ? $"System in control: {holder}"
            : string.IsNullOrWhiteSpace(station.Reason) ? null : station.Reason
            : null;

    public static string? AcquireLabel(ControlStation? station) =>
        station is not { Known: true, InControl: false } ? null
        : station.TakeoverAllowed == true ? "Acquire control"
        : "Send request";

    public static bool AcquireEnabled(ControlStation station, bool counting) => station.RequestAllowed && !counting;

    public static string? WaitLine(ControlStation? station) =>
        station is { Known: true, InControl: false, RequestAllowed: false } ? "Waiting for the other station to answer" : null;

    public static string? InControlLine(ControlStation? station) =>
        station is { Known: true, InControl: true }
            ? station.HolderSystemId is int holder ? $"System in control: This GCS ({holder})" : "System in control: This GCS"
            : null;

    public static string? TakeoverLine(ControlStation? station) =>
        station is { Known: true, TakeoverAllowed: bool allowed } ? allowed ? "Takeover allowed" : "Takeover NOT allowed" : null;

    public static bool TakeoverChangeable(ControlStation? station, bool? allowTakeover) =>
        station?.InControl == true && allowTakeover != null && station.TakeoverAllowed != allowTakeover;

    public static string RequestSentLabel(long remainingMs) =>
        "Request sent: " + (Math.Max(0, remainingMs) / 1000.0).ToString("F1", CultureInfo.InvariantCulture);

    public static bool AllowTakeoverEditable(ControlStation? station) =>
        station?.InControl == true || station?.TakeoverAllowed == true;

    public static string? SectionTitle(ControlStation station) => station.InControl switch
    {
        true => "Change takeover condition",
        false => "Send control request",
        null => null,
    };

    public static bool RequestTimeoutEditable(ControlStation station) =>
        station.InControl == false && station.TakeoverAllowed != true;

    public static int RequestTimeoutSeconds(ControlStation station, int setting) =>
        station.TakeoverAllowed == true ? 0 : setting;

    public static bool? AllowTakeoverSetting() => Maybe(Bridge.Get(Control(AllowTakeoverPath))["value"]);

    public static string? SaveAllowTakeover(bool allow) =>
        MapUiJson.Flag(Bridge.Set($"{AllowTakeoverPath}.rawValue", allow)["ok"])
            ? null
            : "This station could not save whether it allows a takeover.";

    public static string? ChangeTakeover(bool allow) =>
        MapUiJson.Flag(Bridge.Invoke(RequestPath, new JsonArray(allow, 0))["ok"]) ? null : "The vehicle did not take the change.";

    public static string? AllowTakeover() =>
        MapUiJson.Flag(Bridge.Invoke(RequestPath, new JsonArray(true, 0))["ok"])
            ? MapUiJson.RefusalOf(Bridge.Invoke(RevertPath))
            : "The vehicle did not take the change.";

    public static ControlAsk Ask(ControlStation station)
    {
        if (AllowTakeoverSetting() is not bool allow)
            return new ControlAsk("This station cannot tell whether it would allow a takeover, so it did not ask.", 0);
        if (Whole(Bridge.Get(Control(RequestTimeoutPath))["value"]) is not int timeout)
            return new ControlAsk("This station cannot tell how long it would wait, so it did not ask.", 0);
        var seconds = RequestTimeoutSeconds(station, timeout);
        var answer = Bridge.Invoke(RequestPath, new JsonArray(allow, seconds));
        return MapUiJson.Flag(answer["ok"])
            ? new ControlAsk(null, seconds)
            : new ControlAsk(MapUiJson.Text(answer["reason"]) is { Length: > 0 } reason ? reason : "The vehicle did not take the request.", 0);
    }

    public static int? ActiveVehicle(JsonObject vehicles) => Whole(vehicles["activeId"]) is int id && id > 0 ? id : null;

    internal static bool? Maybe(JsonNode? node) => node is JsonValue value && value.TryGetValue<bool>(out var flag) ? flag : null;

    internal static int? Whole(JsonNode? node) => MapUiJson.Number(node) is double number ? (int)number : null;
}

public sealed class ControlRequestPrompt : UserControl
{
    private static readonly TimeSpan Tick = TimeSpan.FromMilliseconds(100);

    private readonly DispatcherTimer _timer = new() { Interval = Tick };
    private readonly TextBlock _title = FormLook.Text("", "FontSizeBodyLarge", strong: true);
    private readonly TextBlock _line = FormLook.Text("", "FontSizeBody", "TextSecondaryBrush");
    private readonly ProgressBar _progress = new() { Minimum = 0, Maximum = 1 };
    private readonly TextBlock _refusal = FormLook.Caption("", "CriticalBrush");
    private readonly Button _allow;
    private IncomingControlRequest? _incoming;
    private long? _revertMs;
    private long _seenAt;
    private int? _ignored;
    private bool _revertIgnored;
    private int? _allowedFrom;

    public ControlRequestPrompt()
    {
        _allow = FormLook.Button("Allow takeover", Allow, ButtonLook.Accent);
        var ignore = FormLook.Button("Ignore", Ignore);
        var card = new Border
        {
            Width = 360,
            Padding = new Thickness(16),
            Background = FormLook.Brush("CardBrush"),
            BorderBrush = FormLook.Brush("CardStrokeBrush"),
            BorderThickness = new Thickness(1),
            CornerRadius = FormLook.Radius("RadiusOverlay"),
            Child = FormLook.Stack(12, _title, _line, _progress, _refusal, FormLook.Line(8, ignore, _allow)),
        };
        AutomationProperties.SetName(card, "Control request");
        Content = card;
        Visibility = Visibility.Collapsed;
        _timer.Tick += (_, _) => Render();
        Live.Watch(this, OperatorControl.ViewPath, view =>
        {
            _incoming = OperatorControl.Incoming(view);
            _revertMs = OperatorControl.RevertMs(view);
            _seenAt = Environment.TickCount64;
            Render();
        });
        Unloaded += (_, _) => _timer.Stop();
    }

    private long Left(long ms) => ms - (Environment.TickCount64 - _seenAt);

    private void Render()
    {
        var live = _incoming is { } request && Left(request.RemainingMs) > 0 ? request : null;
        if (live == null) _ignored = null;
        var revert = _revertMs is long ms && Left(ms) > 0 ? Left(ms) : (long?)null;
        if (revert == null) _revertIgnored = false;
        var incoming = live?.SystemId == _ignored ? null : live;
        var shownRevert = incoming == null && !_revertIgnored ? revert : null;
        Visibility = FormLook.Shown(incoming != null || shownRevert != null);
        if (incoming != null || shownRevert != null) _timer.Start();
        else _timer.Stop();
        if (incoming != null)
        {
            var left = Left(incoming.RemainingMs);
            FormLook.Say(_title, $"GCS {incoming.SystemId} is requesting control");
            FormLook.Say(_line, $"Ignoring automatically in {OperatorControl.SecondsLeft(left)} seconds");
            _progress.Value = incoming.TimeoutMs > 0 ? (double)left / incoming.TimeoutMs : 0;
            _progress.Visibility = Visibility.Visible;
            _allow.Visibility = Visibility.Visible;
        }
        else if (shownRevert is long revertLeft)
        {
            FormLook.Say(_title, null);
            FormLook.Say(_line, $"Reverting back to takeover not allowed if GCS {_allowedFrom?.ToString(CultureInfo.InvariantCulture) ?? ""} doesn't take control in {OperatorControl.SecondsLeft(revertLeft)} seconds ...");
            _progress.Visibility = Visibility.Collapsed;
            _allow.Visibility = Visibility.Collapsed;
        }
    }

    private void Ignore()
    {
        if (_incoming is { } request && _ignored != request.SystemId && Left(request.RemainingMs) > 0) _ignored = request.SystemId;
        else _revertIgnored = true;
        FormLook.Say(_refusal, null);
        Render();
    }

    private async void Allow()
    {
        if (_incoming is not { } request) return;
        _allowedFrom = request.SystemId;
        _revertIgnored = false;
        var refusal = await FormWrites.Off(OperatorControl.AllowTakeover);
        if (refusal == null) _ignored = request.SystemId;
        FormLook.Say(_refusal, refusal);
        Render();
    }
}

public sealed class OperatorControlButton : UserControl
{
    private const double PillHeight = 28;
    private const double SheetWidth = 380;
    private const string InControlGlyph = "\uE8D7";
    private const string ElsewhereGlyph = "\uE7BA";
    private static readonly TimeSpan Tick = TimeSpan.FromMilliseconds(100);

    private readonly Button _pill;
    private readonly FontIcon _icon = FormLook.Glyph(InControlGlyph, 12);
    private readonly TextBlock _label = FormLook.Caption("", "TextPrimaryBrush");
    private readonly Flyout _sheet;
    private readonly StackPanel _live = new() { Spacing = 8 };
    private readonly TextBlock _refusal = FormLook.Caption("", "CriticalBrush");
    private readonly FrameworkElement _timeoutRow;
    private readonly FrameworkElement _systemIdRow;
    private readonly DispatcherTimer _timer = new() { Interval = Tick };
    private ControlStation? _station;
    private bool? _allowServed;
    private bool? _allowTyped;
    private int? _activeVehicle;
    private ImmutableDictionary<int, long> _deadlines = ImmutableDictionary<int, long>.Empty;
    private bool _open;

    public OperatorControlButton()
    {
        _label.TextWrapping = TextWrapping.NoWrap;
        _timeoutRow = FactField(OperatorControl.RequestTimeoutPath, "Request timeout (sec)");
        _systemIdRow = FactField(OperatorControl.GcsSystemIdPath, "This GCS MAVLink system ID");
        _sheet = new Flyout
        {
            FlyoutPresenterStyle = Presenter(),
            Content = new ScrollViewer { MaxHeight = 560, Content = FormLook.Stack(12, _live, _timeoutRow, _systemIdRow, _refusal) },
        };
        _sheet.Opening += (_, _) =>
        {
            _open = true;
            FormLook.Say(_refusal, null);
            Render();
        };
        _sheet.Closed += (_, _) =>
        {
            _open = false;
            _timer.Stop();
        };
        _pill = new Button
        {
            Height = PillHeight,
            Padding = new Thickness(10, 0, 10, 0),
            CornerRadius = new CornerRadius(PillHeight / 2),
            BorderThickness = new Thickness(1),
            Flyout = _sheet,
            Content = FormLook.Line(6, _icon, _label),
        };
        PreflightLook.Chip(_pill);
        AutomationProperties.SetName(_pill, "Vehicle control");
        Content = _pill;
        Visibility = Visibility.Collapsed;
        _timer.Tick += (_, _) => Render();
        Live.Watch(this, OperatorControl.ViewPath, view =>
        {
            _station = OperatorControl.Station(view);
            if (_station?.TakeoverAllowed == true && _activeVehicle is int id) _deadlines = _deadlines.Remove(id);
            Paint();
            if (_open) Render();
        });
        Live.Watch(this, OperatorControl.Control(OperatorControl.AllowTakeoverPath), view =>
        {
            _allowServed = OperatorControl.Maybe(view["value"]);
            _allowTyped = null;
            if (_open) Render();
        });
        Live.Watch(this, OperatorControl.VehiclesPath, view => _activeVehicle = OperatorControl.ActiveVehicle(view));
        Unloaded += (_, _) => _timer.Stop();
    }

    private int DeadlineKey => _activeVehicle ?? 0;

    private bool? Allow => _allowTyped ?? _allowServed;

    private void Paint()
    {
        var station = _station;
        Visibility = FormLook.Shown(station?.Known == true);
        if (station is not { Known: true }) return;
        var elsewhere = OperatorControl.Elsewhere(station);
        _icon.Glyph = elsewhere ? ElsewhereGlyph : InControlGlyph;
        _icon.Foreground = FormLook.Brush(elsewhere ? "CautionBrush" : station.InControl == true ? "SuccessBrush" : "TextSecondaryBrush");
        FormLook.Say(_label, station.InControl switch
        {
            true => "In control",
            false => station.HolderSystemId is int holder ? $"GCS {holder} in control" : "Another station in control",
            null => "Control unknown",
        });
        var line = OperatorControl.ControlLine(station) ?? OperatorControl.InControlLine(station) ?? station.Reason;
        ToolTipService.SetToolTip(_pill, line.Length > 0 ? line : "Vehicle control");
    }

    private void Render()
    {
        var station = _station;
        var deadline = _deadlines.TryGetValue(DeadlineKey, out var endsAt) ? endsAt : (long?)null;
        var remaining = deadline is long ends ? ends - Environment.TickCount64 : (long?)null;
        if (remaining <= 0)
        {
            _deadlines = _deadlines.Remove(DeadlineKey);
            remaining = null;
        }
        if (remaining != null) _timer.Start();
        else _timer.Stop();
        FormLook.Fill(_live, station == null ? [FormLook.Text("No vehicle is connected.", "FontSizeBody", "TextSecondaryBrush")]
            : station.InControl == true ? InControlRows(station)
            : HolderRows(station, remaining));
        _timeoutRow.Visibility = FormLook.Shown(station != null && OperatorControl.RequestTimeoutEditable(station));
        _systemIdRow.Visibility = FormLook.Shown(station != null && (station.InControl == true || OperatorControl.HolderLine(station) != null));
    }

    private UIElement?[] HolderRows(ControlStation station, long? remaining)
    {
        if (OperatorControl.HolderLine(station) is not string line)
            return [FormLook.Text(station.Reason.Length > 0 ? station.Reason : "This vehicle has not said who is flying it.", "FontSizeBody", "TextSecondaryBrush")];
        var label = OperatorControl.AcquireLabel(station);
        var acquire = label == null ? null : FormLook.Button(label, () => Acquire(station), ButtonLook.Accent);
        if (acquire != null) acquire.IsEnabled = OperatorControl.AcquireEnabled(station, remaining != null);
        return
        [
            FormLook.Text(line),
            Small(OperatorControl.TakeoverLine(station)),
            Title(OperatorControl.SectionTitle(station)),
            remaining is long left ? Small(OperatorControl.RequestSentLabel(left)) : null,
            AllowTakeoverBox(station),
            remaining == null ? Small(OperatorControl.WaitLine(station)) : null,
            acquire,
        ];
    }

    private UIElement?[] InControlRows(ControlStation station)
    {
        var change = FormLook.Button("Change", () => Change(), ButtonLook.Accent);
        change.IsEnabled = OperatorControl.TakeoverChangeable(station, Allow);
        return
        [
            FormLook.Text(OperatorControl.InControlLine(station) ?? "", "FontSizeBody", "AccentTextBrush"),
            Small(OperatorControl.TakeoverLine(station)),
            Title(OperatorControl.SectionTitle(station)),
            AllowTakeoverBox(station),
            change,
        ];
    }

    private CheckBox AllowTakeoverBox(ControlStation station)
    {
        var allow = Allow;
        var box = new CheckBox
        {
            Content = "Allow takeover",
            IsChecked = allow == true,
            IsEnabled = allow != null && OperatorControl.AllowTakeoverEditable(station),
            FontFamily = PreflightLook.Font,
        };
        FormLook.Toggled(box, wanted => SaveAllowTakeover(wanted));
        return box;
    }

    private async void SaveAllowTakeover(bool wanted)
    {
        _allowTyped = wanted;
        Render();
        var refusal = await FormWrites.Off(() => OperatorControl.SaveAllowTakeover(wanted));
        FormLook.Say(_refusal, refusal);
        if (refusal != null) _allowTyped = null;
        Render();
    }

    private async void Acquire(ControlStation station)
    {
        var key = DeadlineKey;
        var ask = await FormWrites.Off(() => OperatorControl.Ask(station));
        FormLook.Say(_refusal, ask.Refusal);
        if (ask.Refusal == null && ask.TimeoutSeconds > 0) _deadlines = _deadlines.SetItem(key, Environment.TickCount64 + ask.TimeoutSeconds * 1000L);
        Render();
    }

    private async void Change()
    {
        if (Allow is not bool wanted) return;
        FormLook.Say(_refusal, await FormWrites.Off(() => OperatorControl.ChangeTakeover(wanted)));
        Render();
    }

    private FrameworkElement FactField(string path, string title)
    {
        var refusal = FormLook.Caption("", "CriticalBrush");
        var box = new NumberBox
        {
            Header = title,
            Value = double.NaN,
            SpinButtonPlacementMode = NumberBoxSpinButtonPlacementMode.Compact,
            ValidationMode = NumberBoxValidationMode.InvalidInputOverwritten,
            HorizontalAlignment = HorizontalAlignment.Stretch,
        };
        AutomationProperties.SetName(box, title);
        FormControl? control = null;
        var applying = false;
        box.ValueChanged += async (_, args) =>
        {
            if (applying || control is not { } fact || double.IsNaN(args.NewValue) || fact.Number == args.NewValue) return;
            var typed = args.NewValue;
            var refused = await FormWrites.Off(() => FormWrites.Number(fact, typed));
            FormLook.Say(refusal, refused);
            if (refused == null) return;
            applying = true;
            box.Value = fact.Number ?? double.NaN;
            applying = false;
        };
        var row = FormLook.Stack(4, box, refusal);
        Live.Watch(row, OperatorControl.Control(path), view =>
        {
            control = FormControl.Parse(view);
            box.IsEnabled = control?.AcceptsWrite == true;
            if (box.FocusState != FocusState.Unfocused) return;
            applying = true;
            box.Value = control?.Number ?? double.NaN;
            applying = false;
        });
        return row;
    }

    private static TextBlock? Small(string? text) => text == null ? null : FormLook.Caption(text);

    private static TextBlock? Title(string? text) => text == null ? null : FormLook.Text(text, "FontSizeBody", "TextPrimaryBrush", true);

    private static Style Presenter() => new(typeof(FlyoutPresenter))
    {
        Setters =
        {
            new Setter(FrameworkElement.MinWidthProperty, SheetWidth),
            new Setter(FrameworkElement.MaxWidthProperty, SheetWidth),
            new Setter(Control.PaddingProperty, new Thickness(16)),
            new Setter(Control.BackgroundProperty, FormLook.Brush("LayerBrush")),
            new Setter(Control.BorderBrushProperty, FormLook.Brush("CardStrokeBrush")),
            new Setter(Control.CornerRadiusProperty, FormLook.Radius("RadiusOverlay")),
        },
    };
}
