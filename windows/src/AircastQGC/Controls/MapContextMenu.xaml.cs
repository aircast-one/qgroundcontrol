using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Windows.Foundation;

namespace AircastQGC.Controls;

public sealed partial class MapContextMenu : UserControl
{
    private const string Client = "mapui.contextMenu";
    private const string MapClickPath = "view.mapClick";
    private const string GuidedAltitudePath = "view.guidedAltitude";
    private const string ReturnHomePath = "vehicle.guidedModeRTL";
    private const string ReturnHomeTitle = "Return home instead";
    private const string GoToKind = "GoTo";
    private const string OrbitKind = "Orbit";
    private const string OrbitUnchosen = "Choose a radius and a height for the orbit.";
    private const double ClickGap = 18;
    private const double EdgeMargin = 12;

    private JsonObject _view = new();
    private IReadOnlyList<MapClickAction> _actions = [];
    private Point _click;

    public event EventHandler<GuidedAsk>? ConfirmRequested;
    public event EventHandler? OpenChanged;

    public MapContextMenu()
    {
        InitializeComponent();
        SizeChanged += (_, _) => Place();
        Unloaded += (_, _) => Hide();
    }

    public (double Latitude, double Longitude)? Target { get; private set; }

    public bool Show(double latitude, double longitude, Point at)
    {
        Read();
        if (_actions.Count == 0)
        {
            Hide();
            return false;
        }
        var opening = Target == null;
        Target = (latitude, longitude);
        _click = at;
        MenuCoordinate.Text = CoordinateText(latitude, longitude);
        ShowMenu();
        Visibility = Visibility.Visible;
        if (opening) BridgeWatch.Watch(Client, new[] { MapClickPath }, _ => OnViewChanged());
        OpenChanged?.Invoke(this, EventArgs.Empty);
        DispatcherQueue.TryEnqueue(() => ActionList.Focus(FocusState.Programmatic));
        return true;
    }

    public void Hide()
    {
        if (Target == null) return;
        BridgeWatch.Stop(Client);
        Target = null;
        Visibility = Visibility.Collapsed;
        OpenChanged?.Invoke(this, EventArgs.Empty);
    }

    public static string? Execute(string path, double latitude, double longitude) =>
        MapUiJson.RefusalOf(Bridge.Invoke(path, new JsonArray(new JsonObject { ["latitude"] = latitude, ["longitude"] = longitude })));

    private void Read()
    {
        _view = Bridge.Get(MapClickPath);
        _actions = Offered(_view);
    }

    private static IReadOnlyList<MapClickAction> Offered(JsonObject view) =>
        MapUiJson.Objects(view["actions"])
            .Select(action => new MapClickAction(
                MapUiJson.Text(action["id"]),
                MapUiJson.Text(action["path"]),
                MapUiJson.SentenceCase(MapUiJson.Text(action["label"])),
                MapUiJson.SentenceCase(MapUiJson.Text(action["title"])),
                MapUiJson.Text(action["message"]),
                MapUiJson.Flag(action["confirm"], true)))
            .Where(action => action.Path.Length > 0)
            .ToList();

    private void OnViewChanged()
    {
        if (Target == null) return;
        var shown = _actions;
        Read();
        if (_actions.Count == 0) Hide();
        else if (!shown.SequenceEqual(_actions)) ShowMenu();
    }

    private void ShowMenu()
    {
        ActionList.ItemsSource = _actions;
        Refusal.Visibility = Visibility.Collapsed;
        Place();
    }

    private void OnActionClick(object sender, ItemClickEventArgs e)
    {
        if (e.ClickedItem is not MapClickAction action || Target is not { } at) return;
        if (!action.Confirm)
        {
            Report(Execute(action.Path, at.Latitude, at.Longitude));
            return;
        }
        var ask = Ask(action, at.Latitude, at.Longitude);
        Hide();
        ConfirmRequested?.Invoke(this, ask);
    }

    private GuidedAsk Ask(MapClickAction action, double latitude, double longitude)
    {
        var preview = action.Kind == GoToKind
            ? Bridge.Get(FormattableString.Invariant($"view.gotoPreview({latitude:F7},{longitude:F7})"))
            : new JsonObject();
        var previewed = MapUiJson.Flag(preview["available"]);
        var pastReturnPoint = previewed && MapUiJson.Flag(preview["pastReturnPoint"]);
        var verdict = MapUiJson.Text(preview["verdict"]);
        var trip = Joined(" · ", MapUiJson.Text(preview["distanceText"]), MapUiJson.Text(preview["heightText"]));
        var detail = Joined("\n", trip, pastReturnPoint ? string.Empty : verdict);
        var orbit = action.Kind == OrbitKind ? new OrbitForm(_view, Bridge.Get(GuidedAltitudePath)) : null;

        return new GuidedAsk(
            action.Title,
            previewed && detail.Length > 0 ? detail : action.Message,
            () => orbit != null ? orbit.Send(action.Path, latitude, longitude) : Execute(action.Path, latitude, longitude))
        {
            Detail = CoordinateText(latitude, longitude),
            Warning = pastReturnPoint ? verdict : string.Empty,
            Fields = orbit?.Root,
            Alternative = pastReturnPoint ? (ReturnHomeTitle, ReturnHome) : null,
            WatchPath = MapClickPath,
            StillOffered = view => Offered(view).Any(offered => offered.Kind == action.Kind),
        };
    }

    private static string? ReturnHome() =>
        MapUiJson.RefusalOf(Bridge.Invoke(ReturnHomePath, new JsonArray(false)));

    private void OnScrimTapped(object sender, TappedRoutedEventArgs e) => Hide();

    private void OnScrimRightTapped(object sender, RightTappedRoutedEventArgs e) => Hide();

    private void OnEscape(KeyboardAccelerator sender, KeyboardAcceleratorInvokedEventArgs args)
    {
        args.Handled = true;
        Hide();
    }

    private void OnCardSizeChanged(object sender, SizeChangedEventArgs e) => Place();

    private void Report(string? refusal)
    {
        if (refusal == null)
        {
            Hide();
            return;
        }
        Refusal.Text = refusal;
        Refusal.Visibility = Visibility.Visible;
    }

    private void Place()
    {
        Canvas.SetLeft(Card, Placed(_click.X, Card.Width, ActualWidth));
        Canvas.SetTop(Card, Placed(_click.Y, Card.ActualHeight, ActualHeight));
    }

    private static double Placed(double click, double extent, double container)
    {
        var high = container - EdgeMargin - extent;
        var after = click + ClickGap;
        return high <= EdgeMargin ? EdgeMargin
            : after <= high ? Math.Max(EdgeMargin, after)
            : Math.Max(EdgeMargin, Math.Min(click - ClickGap - extent, high));
    }

    private static string CoordinateText(double latitude, double longitude) =>
        FormattableString.Invariant($"{latitude:F6}, {longitude:F6}");

    private static string Labelled(string label, string unit) =>
        unit.Length > 0 ? $"{label} ({unit})" : label;

    private static string Joined(string separator, params string[] parts) =>
        string.Join(separator, parts.Where(part => part.Trim().Length > 0));

    private sealed class OrbitForm
    {
        private readonly NumberBox _radius;
        private readonly ToggleSwitch _clockwise;
        private readonly NumberBox _height;
        private readonly bool _available;
        private readonly double _metresPerUnit;

        public OrbitForm(JsonObject view, JsonObject height)
        {
            _available = MapUiJson.Flag(height["available"]);
            _metresPerUnit = MapUiJson.Number(view["orbitMetresPerUnit"]) is double perUnit && perUnit > 0 ? perUnit : 1.0;
            var label = MapUiJson.Text(height["label"]);
            _radius = new NumberBox
            {
                Header = Labelled("Radius", MapUiJson.Text(view["orbitRadiusUnit"])),
                Minimum = 0,
                Value = MapUiJson.Number(view["orbitDefaultRadius"]) ?? double.NaN,
                SpinButtonPlacementMode = NumberBoxSpinButtonPlacementMode.Compact,
            };
            _clockwise = new ToggleSwitch
            {
                Header = "Direction",
                OnContent = "Clockwise",
                OffContent = "Counter-clockwise",
                IsOn = MapUiJson.Flag(view["orbitClockwise"], true),
            };
            _height = new NumberBox
            {
                Header = Labelled(label.Length > 0 ? label : "Height above launch", MapUiJson.Text(height["unit"])),
                Maximum = MapUiJson.Number(height["maximum"]) ?? double.MaxValue,
                Minimum = MapUiJson.Number(height["minimum"]) ?? double.MinValue,
                Value = MapUiJson.Number(height["current"]) ?? double.NaN,
                IsEnabled = _available,
                SpinButtonPlacementMode = NumberBoxSpinButtonPlacementMode.Compact,
            };
            AutomationProperties.SetName(_radius, "Orbit radius");
            AutomationProperties.SetName(_height, "Orbit height above launch");
            Root = new StackPanel { Spacing = (double)Application.Current.Resources["SpaceSm"] };
            new UIElement[] { _radius, _clockwise, _height }.ToList().ForEach(Root.Children.Add);
        }

        public StackPanel Root { get; }

        public string? Send(string path, double latitude, double longitude)
        {
            var radius = _radius.Value * _metresPerUnit;
            var aboveHome = _available && double.IsFinite(_height.Value)
                ? MapUiJson.Number(Bridge.Get(FormattableString.Invariant($"{GuidedAltitudePath}({_height.Value:F2})"))["targetMeters"])
                : null;
            return double.IsFinite(radius) && radius > 0 && aboveHome is double metres
                ? MapUiJson.RefusalOf(Bridge.Invoke(path, new JsonArray(latitude, longitude, radius, _clockwise.IsOn, metres)))
                : OrbitUnchosen;
        }
    }
}
