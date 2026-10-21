using System;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

public sealed partial class NoVehicleState : UserControl
{
    public const string FlyStatePath = "view.flyState";
    private const string Client = "noVehicleState.flyState";

    private bool? _present;

    public event EventHandler? OpenConnectionsRequested;
    public event EventHandler? OpenFlasherRequested;
    public event EventHandler<bool>? VehiclePresenceChanged;

    public bool HasVehicle => _present ?? false;

    public NoVehicleState()
    {
        InitializeComponent();
        PreflightLook.Accent(OpenConnectionsButton);
        Loaded += (_, _) =>
        {
            BridgeWatch.Watch(Client, new[] { FlyStatePath }, Apply);
            Apply(Bridge.Group(FlyStatePath));
        };
        Unloaded += (_, _) => BridgeWatch.Stop(Client);
    }

    public static bool IsNoVehicle(JsonObject flyState) => !PreflightJson.Flag(flyState["connected"]);

    public static bool NoVehicleNow() => IsNoVehicle(Bridge.Group(FlyStatePath));

    private void Apply(JsonObject flyState)
    {
        var present = !IsNoVehicle(flyState);
        Visibility = present ? Visibility.Collapsed : Visibility.Visible;
        if (_present == present) return;
        _present = present;
        VehiclePresenceChanged?.Invoke(this, present);
    }

    private void OnOpenConnections(object sender, RoutedEventArgs e) =>
        OpenConnectionsRequested?.Invoke(this, EventArgs.Empty);

    private void OnOpenFlasher(object sender, RoutedEventArgs e) =>
        OpenFlasherRequested?.Invoke(this, EventArgs.Empty);
}
