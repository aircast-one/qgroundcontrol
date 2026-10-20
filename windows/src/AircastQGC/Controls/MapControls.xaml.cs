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
    private const string MapGlyph = "\uE800";
    private const string ShowThreeD = "Show the 3D view";
    private const string ShowMap = "Show the map";

    public event EventHandler? ZoomInRequested;
    public event EventHandler? ZoomOutRequested;
    public event EventHandler<MapPointEventArgs>? CenterOnVehicleRequested;
    public event EventHandler<string>? MapTypeChanged;
    public event EventHandler? ThreeDRequested;

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
        LayersMenu.Items.Clear();
        items.Concat(new MenuFlyoutItemBase[] { new MenuFlyoutSeparator(), obstacles }).ToList().ForEach(LayersMenu.Items.Add);
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
