using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using Windows.Devices.Bluetooth;
using Windows.Devices.Bluetooth.Rfcomm;
using Windows.Devices.Enumeration;
using Windows.Devices.Radios;
using Windows.Networking.Sockets;

namespace AircastQGC.Core;

internal sealed record RadioDevice(string Name, string Address);

internal sealed record RadioLink(Stream Input, Stream Output, IDisposable Connection);

internal sealed class BluetoothRefusal(string reason) : Exception(reason);

internal static class BluetoothRadio
{
    public const string NoRadio = "This PC has no Bluetooth radio.";
    public const string RadioOff = "Bluetooth is turned off. Turn it on in Windows Settings > Bluetooth & devices.";
    private const string AddressKey = "System.Devices.Aep.DeviceAddress";
    private const string NameKey = "System.ItemNameDisplay";
    private static readonly string[] Keys = [AddressKey];

    public static string? Unusable()
    {
        var adapter = BluetoothAdapter.GetDefaultAsync().AsTask().GetAwaiter().GetResult();
        var radio = adapter is { IsClassicSupported: true } ? adapter.GetRadioAsync().AsTask().GetAwaiter().GetResult() : null;
        return adapter is not { IsClassicSupported: true } ? NoRadio
            : radio is not null && radio.State != RadioState.On ? RadioOff
            : null;
    }

    public static IReadOnlyList<RadioDevice> Paired() =>
        DeviceInformation.FindAllAsync(BluetoothDevice.GetDeviceSelectorFromPairingState(true), Keys, DeviceInformationKind.AssociationEndpoint)
            .AsTask().GetAwaiter().GetResult()
            .Select(Device)
            .OfType<RadioDevice>()
            .ToList();

    public static Action Discover(Action<RadioDevice> found)
    {
        var addresses = new ConcurrentDictionary<string, string>();
        var watcher = DeviceInformation.CreateWatcher(BluetoothDevice.GetDeviceSelectorFromPairingState(false), Keys, DeviceInformationKind.AssociationEndpoint);
        watcher.Added += (_, info) =>
        {
            if (Device(info) is not { } device) return;
            addresses[info.Id] = device.Address;
            found(device);
        };
        watcher.Updated += (_, update) =>
        {
            if (addresses.TryGetValue(update.Id, out var address) && update.Properties.TryGetValue(NameKey, out var name) && name is string { Length: > 0 } named)
                found(new RadioDevice(named, address));
        };
        watcher.Start();
        return () =>
        {
            if (watcher.Status is DeviceWatcherStatus.Started or DeviceWatcherStatus.EnumerationCompleted) watcher.Stop();
        };
    }

    public static RadioLink Connect(string address)
    {
        if (Unusable() is { } unusable) throw new BluetoothRefusal(unusable);
        var device = BluetoothDevice.FromBluetoothAddressAsync(Number(address)).AsTask().GetAwaiter().GetResult()
            ?? throw new BluetoothRefusal($"Bluetooth device {address} is out of range or switched off.");
        var name = device.Name.Length > 0 ? device.Name : address;
        Pair(device, name);
        var services = device.GetRfcommServicesForIdAsync(RfcommServiceId.SerialPort, BluetoothCacheMode.Uncached).AsTask().GetAwaiter().GetResult();
        var service = services.Error == BluetoothError.Success ? services.Services.FirstOrDefault() : null;
        if (service is null)
            throw new BluetoothRefusal(services.Error == BluetoothError.Success
                ? $"{name} has no serial port (SPP) service. Aircast connects to Bluetooth telemetry radios."
                : $"{name} did not answer ({services.Error}). Check it is on and in range.");
        var socket = new StreamSocket();
        try
        {
            socket.ConnectAsync(service.ConnectionHostName, service.ConnectionServiceName, SocketProtectionLevel.BluetoothEncryptionAllowNullAuthentication)
                .AsTask().GetAwaiter().GetResult();
        }
        catch (Exception)
        {
            socket.Dispose();
            throw;
        }
        return new RadioLink(socket.InputStream.AsStreamForRead(0), socket.OutputStream.AsStreamForWrite(0), socket);
    }

    private static void Pair(BluetoothDevice device, string name)
    {
        var pairing = device.DeviceInformation.Pairing;
        if (pairing.IsPaired || !pairing.CanPair) return;
        var status = pairing.PairAsync().AsTask().GetAwaiter().GetResult().Status;
        if (status is not (DevicePairingResultStatus.Paired or DevicePairingResultStatus.AlreadyPaired))
            throw new BluetoothRefusal($"{name} is not paired ({status}). Pair it in Windows Settings > Bluetooth & devices, then connect again.");
    }

    private static RadioDevice? Device(DeviceInformation info) =>
        info.Properties.TryGetValue(AddressKey, out var raw) && raw is string { Length: > 0 } address
            ? new RadioDevice(info.Name, address.ToUpperInvariant())
            : null;

    private static ulong Number(string address) =>
        ulong.TryParse(address.Replace(":", "", StringComparison.Ordinal), NumberStyles.HexNumber, CultureInfo.InvariantCulture, out var number)
            ? number
            : throw new BluetoothRefusal($"{address} is not a Bluetooth address.");
}
