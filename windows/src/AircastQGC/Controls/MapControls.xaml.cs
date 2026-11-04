using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

public sealed partial class MapControls : UserControl
{
    private const string Client = "mapui.controls";
    private const string CoordinatePath = "vehicle.coordinate";
    private const string MapTypesPath = "view.mapTypes";
    private const string ObstacleSetting = "settings.flyViewSettings.showObstacleDistanceOverlay";
    private const string ThreeDGlyph = "\uF158";
    private const string MapGlyph = "\uE707";
    private const string ShowThreeD = "Show the 3D view";
    private const string ShowMap = "Show the map";
    private const string Map3dTitle = "3D view";
    private const string Map3dDetail = "Terrain and buildings around the aircraft, and its height above the ground";

    public event EventHandler? ZoomInRequested;
    public event EventHandler? ZoomOutRequested;
    public event EventHandler<MapPointEventArgs>? CenterOnVehicleRequested;
    public event EventHandler<string>? MapTypeChanged;
    public event EventHandler? ThreeDRequested;
    public event EventHandler? Map3dRequested;

    private (bool Offered, bool Shown) _map3d;

    public MapControls()
    {
        InitializeComponent();
        Loaded += (_, _) => StartWatching();
        Unloaded += (_, _) => BridgeWatch.Stop(Client);
    }

    public string CurrentMapType => MapUiJson.Text(Bridge.Get(MapTypesPath)["current"]);

    private void StartWatching()
    {
        BridgeWatch.Watch(Client, new[] { CoordinatePath }, _ => RefreshCenter());
        RefreshCenter();
    }

    private void RefreshCenter() => CenterButton.IsEnabled = MapUiJson.VehiclePosition() != null;

    private void OnZoomIn(object sender, RoutedEventArgs e) => ZoomInRequested?.Invoke(this, EventArgs.Empty);

    private void OnZoomOut(object sender, RoutedEventArgs e) => ZoomOutRequested?.Invoke(this, EventArgs.Empty);

    private void OnCenter(object sender, RoutedEventArgs e)
    {
        if (MapUiJson.VehiclePosition() is not { } at) return;
        CenterOnVehicleRequested?.Invoke(this, new MapPointEventArgs(at.Latitude, at.Longitude));
    }

    public void ThreeD(bool offered, bool shown)
    {
        ThreeDButton.Visibility = offered ? Visibility.Visible : Visibility.Collapsed;
        ThreeDButton.Content = shown ? MapGlyph : ThreeDGlyph;
        ToolTipService.SetToolTip(ThreeDButton, shown ? ShowMap : ShowThreeD);
        Microsoft.UI.Xaml.Automation.AutomationProperties.SetName(ThreeDButton, shown ? ShowMap : ShowThreeD);
    }

    private void OnThreeD(object sender, RoutedEventArgs e) => ThreeDRequested?.Invoke(this, EventArgs.Empty);

    public void Map3d(bool offered, bool shown)
    {
        _map3d = (offered, shown);
        Map3dButton.Visibility = offered ? Visibility.Visible : Visibility.Collapsed;
        Map3dButton.Content = Map3dLook.ToggleText(shown);
        ToolTipService.SetToolTip(Map3dButton, Map3dLook.ToggleLabel(shown));
        Microsoft.UI.Xaml.Automation.AutomationProperties.SetName(Map3dButton, Map3dLook.ToggleLabel(shown));
    }

    private void OnMap3d(object sender, RoutedEventArgs e) => Map3dRequested?.Invoke(this, EventArgs.Empty);

    private void OnLayersOpening(object sender, object e)
    {
        var view = Bridge.Get(MapTypesPath);
        var path = MapUiJson.Text(view["path"]);
        var current = MapUiJson.Text(view["current"]);
        var types = MapUiJson.Texts(view["types"]);
        var items = types.Count == 0 || path.Length == 0
            ? new List<MenuFlyoutItemBase> { new MenuFlyoutItem { Text = "No map types for this provider", IsEnabled = false, FontFamily = FontFamily } }
            : types.Select(type => TypeItem(type, type == current, path)).ToList();
        var obstacles = new ToggleMenuFlyoutItem
        {
            Text = "Obstacle distance",
            IsChecked = MapUiJson.Flag(Bridge.Get(ObstacleSetting)["value"]),
            FontFamily = FontFamily,
        };
        ToolTipService.SetToolTip(obstacles, "Proximity sensor readings around the vehicle");
        obstacles.Click += (_, _) => Bridge.Set(ObstacleSetting, JsonValue.Create(obstacles.IsChecked));
        var map3d = new ToggleMenuFlyoutItem { Text = Map3dTitle, IsChecked = _map3d.Shown, FontFamily = FontFamily };
        ToolTipService.SetToolTip(map3d, Map3dDetail);
        map3d.Click += (_, _) => Map3dRequested?.Invoke(this, EventArgs.Empty);
        var toggles = _map3d.Offered ? new MenuFlyoutItemBase[] { map3d, obstacles } : new MenuFlyoutItemBase[] { obstacles };
        LayersMenu.Items.Clear();
        items.Append(new MenuFlyoutSeparator()).Concat(toggles).ToList().ForEach(LayersMenu.Items.Add);
    }

    private MenuFlyoutItemBase TypeItem(string type, bool current, string path)
    {
        var item = new RadioMenuFlyoutItem { Text = type, IsChecked = current, GroupName = Client, FontFamily = FontFamily };
        item.Click += (_, _) => Choose(path, type);
        return item;
    }

    private void Choose(string path, string type)
    {
        if (!MapUiJson.Flag(Bridge.Set(path, JsonValue.Create(type))["ok"])) return;
        MapTypeChanged?.Invoke(this, type);
    }
}
