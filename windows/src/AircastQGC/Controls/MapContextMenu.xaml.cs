using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Windows.Foundation;
using Windows.System;

namespace AircastQGC.Controls;

public sealed partial class MapContextMenu : UserControl
{
    private const string Client = "mapui.contextMenu";
    private const string MapClickPath = "view.mapClick";
    private const string GuidedAltitudePath = "view.guidedAltitude";
    private const string ReturnHomePath = "vehicle.guidedModeRTL";
    private const string GoToKind = "GoTo";
    private const string OrbitKind = "Orbit";
    private const string OrbitUnchosen = "Choose a radius and a height for the orbit.";
    private const double MenuWidth = 240;
    private const double ConfirmWidth = 320;
    private const double ClickGap = 18;
    private const double EdgeMargin = 12;

    private JsonObject _view = new();
    private IReadOnlyList<MapClickAction> _actions = [];
    private MapClickAction? _confirming;
    private Point _click;

    public event EventHandler<MapClickActionEventArgs>? ActionRequested;
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
        _confirming = null;
        Visibility = Visibility.Collapsed;
        OpenChanged?.Invoke(this, EventArgs.Empty);
    }

    public static string? Execute(string path, double latitude, double longitude) =>
        MapUiJson.RefusalOf(Bridge.Invoke(path, new JsonArray(new JsonObject { ["latitude"] = latitude, ["longitude"] = longitude })));

    private void Read()
    {
        _view = Bridge.Get(MapClickPath);
        _actions = MapUiJson.Objects(_view["actions"])
            .Select(action => new MapClickAction(
                MapUiJson.Text(action["id"]),
                MapUiJson.Text(action["path"]),
                MapUiJson.SentenceCase(MapUiJson.Text(action["label"])),
                MapUiJson.SentenceCase(MapUiJson.Text(action["title"])),
                MapUiJson.Text(action["message"]),
                MapUiJson.Flag(action["confirm"], true)))
            .Where(action => action.Path.Length > 0)
            .ToList();
    }

    private void OnViewChanged()
    {
        if (Target == null) return;
        var shown = _actions;
        Read();
        if (_actions.Count == 0)
        {
            Hide();
            return;
        }
        var pendingGone = _confirming is { } pending && _actions.All(action => action.Kind != pending.Kind);
        var listChanged = _confirming == null && !shown.SequenceEqual(_actions);
        if (pendingGone || listChanged) ShowMenu();
    }

    private void ShowMenu()
    {
        _confirming = null;
        ActionList.ItemsSource = _actions;
        MenuPanel.Visibility = Visibility.Visible;
        ConfirmPanel.Visibility = Visibility.Collapsed;
        Refusal.Visibility = Visibility.Collapsed;
        Card.Width = MenuWidth;
        Place();
    }

    private void OnActionClick(object sender, ItemClickEventArgs e)
    {
        if (e.ClickedItem is not MapClickAction action || Target is not { } at) return;
        var request = new MapClickActionEventArgs(action, at.Latitude, at.Longitude);
        ActionRequested?.Invoke(this, request);
        if (request.Handled) Hide();
        else if (action.Confirm) ShowConfirm(action, at.Latitude, at.Longitude);
        else Report(Execute(action.Path, at.Latitude, at.Longitude));
    }

    private void ShowConfirm(MapClickAction action, double latitude, double longitude)
    {
        _confirming = action;
        var preview = action.Kind == GoToKind
            ? Bridge.Get(FormattableString.Invariant($"view.gotoPreview({latitude:F7},{longitude:F7})"))
            : new JsonObject();
        var previewed = MapUiJson.Flag(preview["available"]);
        var pastReturnPoint = previewed && MapUiJson.Flag(preview["pastReturnPoint"]);
        var verdict = MapUiJson.Text(preview["verdict"]);
        var trip = Joined(" · ", MapUiJson.Text(preview["distanceText"]), MapUiJson.Text(preview["heightText"]));
        var detail = Joined("\n", trip, pastReturnPoint ? string.Empty : verdict);

        ConfirmTitle.Text = action.Title;
        ConfirmButton.Content = action.Title;
        ConfirmMessage.Text = previewed && detail.Length > 0 ? detail : action.Message;
        ConfirmWarning.Text = pastReturnPoint ? verdict : string.Empty;
        ConfirmWarning.Visibility = pastReturnPoint ? Visibility.Visible : Visibility.Collapsed;
        ReturnHomeButton.Visibility = ConfirmWarning.Visibility;
        ConfirmCoordinate.Text = CoordinateText(latitude, longitude);
        ConfirmButton.IsEnabled = action.Kind == OrbitKind ? PrepareOrbit() : true;
        OrbitFields.Visibility = action.Kind == OrbitKind ? Visibility.Visible : Visibility.Collapsed;
        MenuPanel.Visibility = Visibility.Collapsed;
        ConfirmPanel.Visibility = Visibility.Visible;
        Refusal.Visibility = Visibility.Collapsed;
        Card.Width = ConfirmWidth;
        Place();
        DispatcherQueue.TryEnqueue(() => CancelButton.Focus(FocusState.Programmatic));
    }

    private bool PrepareOrbit()
    {
        OrbitRadius.Header = Labelled("Radius", MapUiJson.Text(_view["orbitRadiusUnit"]));
        OrbitRadius.Value = MapUiJson.Number(_view["orbitDefaultRadius"]) ?? double.NaN;
        OrbitClockwise.IsOn = MapUiJson.Flag(_view["orbitClockwise"], true);

        var height = Bridge.Get(GuidedAltitudePath);
        var available = MapUiJson.Flag(height["available"]);
        var label = MapUiJson.Text(height["label"]);
        OrbitHeight.Header = Labelled(label.Length > 0 ? label : "Height above launch", MapUiJson.Text(height["unit"]));
        OrbitHeight.Minimum = double.MinValue;
        OrbitHeight.Maximum = MapUiJson.Number(height["maximum"]) ?? double.MaxValue;
        OrbitHeight.Minimum = MapUiJson.Number(height["minimum"]) ?? double.MinValue;
        OrbitHeight.Value = MapUiJson.Number(height["current"]) ?? double.NaN;
        OrbitHeight.IsEnabled = available;
        return available;
    }

    private string? SendOrbit(string path, double latitude, double longitude)
    {
        var metresPerUnit = MapUiJson.Number(_view["orbitMetresPerUnit"]) is double perUnit && perUnit > 0 ? perUnit : 1.0;
        var radius = OrbitRadius.Value * metresPerUnit;
        var aboveHome = double.IsFinite(OrbitHeight.Value)
            ? MapUiJson.Number(Bridge.Get(FormattableString.Invariant($"{GuidedAltitudePath}({OrbitHeight.Value:F2})"))["targetMeters"])
            : null;
        return double.IsFinite(radius) && radius > 0 && aboveHome is double metres
            ? MapUiJson.RefusalOf(Bridge.Invoke(path, new JsonArray(latitude, longitude, radius, OrbitClockwise.IsOn, metres)))
            : OrbitUnchosen;
    }

    private void OnConfirm(object sender, RoutedEventArgs e)
    {
        if (_confirming is not { } action || Target is not { } at) return;
        Report(action.Kind == OrbitKind
            ? SendOrbit(action.Path, at.Latitude, at.Longitude)
            : Execute(action.Path, at.Latitude, at.Longitude));
    }

    private void OnReturnHome(object sender, RoutedEventArgs e) =>
        Report(MapUiJson.RefusalOf(Bridge.Invoke(ReturnHomePath, new JsonArray(false))));

    private void OnCancel(object sender, RoutedEventArgs e) => Hide();

    private void OnScrimTapped(object sender, TappedRoutedEventArgs e) => Hide();

    private void OnScrimRightTapped(object sender, RightTappedRoutedEventArgs e) => Hide();

    private void OnMenuKeyDown(object sender, KeyRoutedEventArgs e)
    {
        if (e.Key != VirtualKey.Escape) return;
        e.Handled = true;
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
}
