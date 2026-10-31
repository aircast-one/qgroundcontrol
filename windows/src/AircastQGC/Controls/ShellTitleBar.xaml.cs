using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;

namespace AircastQGC.Controls;

public sealed partial class ShellTitleBar : UserControl
{
    private const string FlyStatePath = "view.flyState";
    private const string VehicleLinksPath = "view.vehicleLinks";
    private const string FlightModesPath = "view.flightModes";
    private const string WarningsPath = "view.warnings";
    private const string ReadyGlyph = "\uE73E";
    private const string NotReadyGlyph = "\uE7BA";
    private const string ArmedGlyph = "";
    private const string DisarmedGlyph = "";
    private const double StaleOpacity = 0.45;
    private const double StripFloor = 200;
    private const double DragFloor = 64;
    private const double VerdictFloor = 80;

    private static readonly string[] Watched = { FlyStatePath, VehicleLinksPath, FlightModesPath, WarningsPath };
    private static readonly string[] KnownStates = { "notConnected", "contactLost", "flying", "landing", "armed", "disarmed" };

    private readonly Dictionary<string, JsonObject> _views = new();
    private readonly VehicleStatusSheet _status = new();
    private string _pendingMode = "";
    private bool _onFly;

    public ShellTitleBar()
    {
        InitializeComponent();
        Loaded += (_, _) => Start();
        Bar.SizeChanged += (_, _) => Fit();
        Readout.SizeChanged += (_, _) => Fit();
        Lead.SizeChanged += (_, _) => Fit();
        MoreButton.SizeChanged += (_, _) => Fit();
        Strip.NeedChanged += Fit;
        PageMenu.Opening += (_, _) => PageMenu.Items.OfType<RadioMenuFlyoutItem>().ToList()
            .ForEach(item => item.IsChecked = item.Tag as string == CurrentPage);
        Unloaded += (_, _) => Array.ForEach(Watched, path => BridgeWatch.Stop(Client(path)));
        Render();
    }

    public event EventHandler<FrameworkElement>? FlightModeRequested;

    public event EventHandler<string>? PageRequested;

    public event EventHandler? BackRequested;

    public event EventHandler? PaneToggleRequested;

    public FrameworkElement DragRegion => DragArea;

    public FrameworkElement FlightModeAnchor => ModePill;

    public VehiclePickerButton VehiclePicker => VehicleSwitch;

    public double CaptionInset
    {
        get => CaptionColumn.Width.Value;
        set
        {
            CaptionColumn.Width = new GridLength(Math.Max(0, value));
            Fit();
        }
    }

    public string CurrentPage { get; set; } = NavPane.Fly;

    public bool CanGoBack
    {
        get => BackItem.Visibility == Visibility.Visible;
        set
        {
            BackItem.Visibility = Shown(value);
            BackSeparator.Visibility = Shown(value);
        }
    }

    public bool OnFly
    {
        get => _onFly;
        set
        {
            _onFly = value;
            MenuButton.Flyout = value ? PageMenu : null;
            MoreButton.Visibility = Shown(value);
            Subtitle.Visibility = Shown(!value);
            VehicleSwitch.Named = value;
            Render();
            Fit();
        }
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

    private void Fit()
    {
        var probe = new TextBlock { Text = AppTitle.Text, FontFamily = AppTitle.FontFamily, FontSize = AppTitle.FontSize };
        probe.Measure(new Windows.Foundation.Size(double.PositiveInfinity, double.PositiveInfinity));
        var title = probe.DesiredSize.Width + DragArea.ColumnSpacing;
        var brand = Lead.Margin.Left + Lead.ActualWidth + MoreButton.ActualWidth + RightGroup.Spacing + DragArea.Padding.Left + DragArea.Padding.Right + DragFloor;
        var room = Bar.ActualWidth - CaptionColumn.Width.Value - RightGroup.Margin.Right - RightGroup.Spacing - (Readout.ActualWidth - Strip.ActualWidth) - brand;
        ArmedText.MaxWidth = Math.Max(VerdictFloor, room + ArmedText.ActualWidth - Strip.Needed);
        var titled = !_onFly && Readings.Visibility != Visibility.Visible && room - title >= StripFloor;
        AppTitle.Visibility = Shown(titled);
        Strip.MaxWidth = Math.Max(0, room - (titled ? title : 0));
    }

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

    private void OnLinkClick(object sender, RoutedEventArgs e) => Strip.ShowLinks(LinkPill);

    private void OnStatusClick(object sender, RoutedEventArgs e) => _status.ShowAt(ArmedPill);

    private void OnPage(object sender, RoutedEventArgs e)
    {
        if (sender is MenuFlyoutItem { Tag: string page }) PageRequested?.Invoke(this, page);
    }

    private void OnBack(object sender, RoutedEventArgs e) => BackRequested?.Invoke(this, EventArgs.Empty);

    private void OnMenu(object sender, RoutedEventArgs e)
    {
        if (!_onFly) PaneToggleRequested?.Invoke(this, EventArgs.Empty);
    }

    private void OnMore(object sender, RoutedEventArgs e) => PageRequested?.Invoke(this, NavPane.Settings);

    private JsonObject View(string path) => _views.TryGetValue(path, out var view) ? view : new JsonObject();

    private void Render()
    {
        var state = View(FlyStatePath);
        var connected = Flag(state, "connected") == true;
        var token = Text(state, "state");
        var stateText = Text(state, "stateText");
        var alarming = connected && (token == "contactLost" || !KnownStates.Contains(token));

        Subtitle.Text = !connected ? "Connect a vehicle to fly" : !alarming ? "" : stateText.Length == 0 ? "Vehicle state unknown" : stateText;
        Subtitle.Foreground = Token(alarming ? "CriticalBrush" : "TextTertiaryBrush");
        Logo.Visibility = Shown(!_onFly && !connected);

        var links = (View(VehicleLinksPath)["links"] as JsonArray)?.OfType<JsonObject>().ToList() ?? [];
        var (linkText, linkTone) = Link(connected, Flag(state, "contactLost"), links);
        var linkNames = links.Select(link => Text(link, "name")).Where(name => name.Length > 0).ToList();
        LinkText.Text = linkText;
        LinkIcon.Foreground = StatusLook.Tint(linkTone);
        LinkText.Foreground = StatusLook.CellInk(linkTone);
        StatusLook.Dress(LinkPill, linkTone);
        ToolTipService.SetToolTip(LinkPill, linkNames.Count == 0 ? "Connections" : string.Join("\n", linkNames));

        Readings.Visibility = Shown(connected);
        Readings.Opacity = Text(state, "staleNotice").Length == 0 ? 1 : StaleOpacity;

        var verdict = Readiness.Of(FlyStatus.Parse(state), View(WarningsPath));
        var mode = Text(state, "mode");
        var pending = _pendingMode.Length > 0;
        var offersModes = View(FlightModesPath)["modes"] is JsonArray { Count: > 0 };
        var named = mode.Length == 0 ? "Mode unknown" : mode;
        ModeText.Text = pending ? _pendingMode : _onFly && stateText.Length > 0 && verdict is null ? $"{named} · {stateText}" : named;
        ModeText.Foreground = Token(_onFly && alarming ? "CriticalBrush" : "TextPrimaryBrush");
        ModePill.IsEnabled = offersModes;
        ModeChevron.Visibility = Shown(offersModes && !pending);
        ModePending.Visibility = Shown(pending);
        ModePending.IsActive = pending;

        var armed = Flag(state, "armed") == true;
        ArmedText.Text = verdict?.Text ?? (armed ? "Armed" : "Disarmed");
        ArmedIcon.Glyph = verdict switch
        {
            null => armed ? ArmedGlyph : DisarmedGlyph,
            { Tone: StatusTone.Good } => ReadyGlyph,
            _ => NotReadyGlyph,
        };
        var verdictTone = verdict?.Tone ?? (armed ? StatusTone.Critical : StatusTone.Muted);
        ArmedIcon.Foreground = StatusLook.Tint(verdictTone);
        ArmedText.Foreground = StatusLook.Ink(verdictTone);
        ToolTipService.SetToolTip(ArmedPill, verdict is null ? "Vehicle status" : string.Join("\n", new[] { $"Disarmed · {verdict.Text}", verdict.Detail }.Where(line => line.Length > 0)));
        AutomationProperties.SetHelpText(ArmedPill, verdict is null ? "" : $"Disarmed. {verdict.Text}");
    }

    private static (string Label, StatusTone Level) Link(bool connected, bool? contactLost, IReadOnlyList<JsonObject> links)
    {
        if (!connected) return ("No vehicle", StatusTone.Muted);
        if (contactLost == true) return ("Contact lost", StatusTone.Critical);
        var silent = links.Count(link => Flag(link, "commLost") == true);
        var heard = contactLost == false ? StatusTone.Good : StatusTone.Muted;
        return links.Count switch
        {
            < 2 => ("Connected", heard),
            _ when silent == 0 => ($"{links.Count} links", heard),
            _ when silent == links.Count => ("No link heard", StatusTone.Critical),
            _ => (silent == 1 ? "1 link lost" : $"{silent} links lost", StatusTone.Caution),
        };
    }

    private static Brush Token(string key) =>
        Application.Current.Resources.TryGetValue(key, out var value) && value is Brush brush
            ? brush
            : new SolidColorBrush(Colors.Transparent);

    private static bool? Flag(JsonObject view, string key) =>
        view[key] is JsonValue value && value.TryGetValue<bool>(out var flag) ? flag : null;

    private static string Text(JsonObject view, string key) =>
        view[key] is JsonValue value && value.TryGetValue<string>(out var text) ? text ?? "" : "";

    private static Visibility Shown(bool on) => on ? Visibility.Visible : Visibility.Collapsed;
}
