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

    public event EventHandler? ZoomInRequested;
    public event EventHandler? ZoomOutRequested;
    public event EventHandler<MapPointEventArgs>? CenterOnVehicleRequested;
    public event EventHandler<string>? MapTypeChanged;

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

    private void OnLayersOpening(object sender, object e)
    {
        var view = Bridge.Get(MapTypesPath);
        var path = MapUiJson.Text(view["path"]);
        var current = MapUiJson.Text(view["current"]);
        var types = MapUiJson.Texts(view["types"]);
        var items = types.Count == 0 || path.Length == 0
            ? new List<MenuFlyoutItemBase> { new MenuFlyoutItem { Text = "No map types for this provider", IsEnabled = false, FontFamily = FontFamily } }
            : types.Select(type => TypeItem(type, type == current, path)).ToList();
        LayersMenu.Items.Clear();
        items.ForEach(LayersMenu.Items.Add);
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
