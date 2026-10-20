using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Threading;
using System.Threading.Tasks;

namespace AircastQGC.Core;

internal static unsafe partial class BluetoothHost
{
    private const string Lib = "groundstation";
    private const int ReadBytes = 1024;
    private const string ClosedByDevice = "The Bluetooth device closed the connection.";
    private const string Lost = "The Bluetooth connection was lost.";
    private static readonly TimeSpan ScanWindow = TimeSpan.FromSeconds(15);
    private static readonly ConcurrentDictionary<uint, RadioLink> Links = new();
    private static readonly ConcurrentDictionary<string, (string Name, long Order)> Found = new();
    private static readonly object Gate = new();
    private static IReadOnlyList<RadioDevice> _paired = [];
    private static Action? _stopScan;
    private static Timer? _window;
    private static bool _installed;
    private static long _order;

    [StructLayout(LayoutKind.Sequential)]
    private struct Hooks
    {
        public delegate* unmanaged[Cdecl]<uint, byte*, int> Open;
        public delegate* unmanaged[Cdecl]<uint, byte*, nuint, int> Write;
        public delegate* unmanaged[Cdecl]<uint, void> Close;
        public delegate* unmanaged[Cdecl]<int, int> Scan;
    }

    public static void Start()
    {
        if (_installed) return;
        try
        {
            var hooks = new Hooks { Open = &Open, Write = &Write, Close = &Close, Scan = &Scan };
            qgc_bluetooth_install(&hooks);
            _installed = true;
        }
        catch (Exception)
        {
            return;
        }
        Task.Run(() =>
        {
            _paired = Paired();
            Publish();
        });
    }

    public static void Stop()
    {
        if (!_installed) return;
        EndScan();
        Links.Keys.ToList().ForEach(Drop);
    }

    public static string? Unusable()
    {
        try { return BluetoothRadio.Unusable(); }
        catch (Exception) { return BluetoothRadio.NoRadio; }
    }

    private static IReadOnlyList<RadioDevice> Paired()
    {
        try { return Unusable() is null ? BluetoothRadio.Paired() : []; }
        catch (Exception) { return []; }
    }

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    private static int Open(uint id, byte* address)
    {
        try { return Connect(id, Marshal.PtrToStringUTF8((IntPtr)address) ?? "") ? 1 : 0; }
        catch (Exception) { return 0; }
    }

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    private static int Write(uint id, byte* bytes, nuint length)
    {
        if (!Links.TryGetValue(id, out var link)) return 0;
        try
        {
            lock (link)
            {
                link.Output.Write(new ReadOnlySpan<byte>(bytes, checked((int)length)));
                link.Output.Flush();
            }
            return 1;
        }
        catch (Exception) { return 0; }
    }

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    private static void Close(uint id)
    {
        try { Drop(id); }
        catch (Exception) { }
    }

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    private static int Scan(int start)
    {
        try { return (start != 0 ? BeginScan() : EndScan()) ? 1 : 0; }
        catch (Exception) { return 0; }
    }

    private static bool Connect(uint id, string address)
    {
        EndScan();
        try
        {
            var link = BluetoothRadio.Connect(address);
            Links[id] = link;
            new Thread(() => Read(id, link)) { IsBackground = true, Name = $"bluetooth-link-{id}" }.Start();
            return true;
        }
        catch (Exception failure)
        {
            qgc_bluetooth_refused(id, Reason(address, failure));
            return false;
        }
    }

    private static string Reason(string address, Exception failure) => failure is BluetoothRefusal
        ? failure.Message
        : $"Bluetooth device {address} could not be connected: {failure.Message.Split('\n')[0].Trim()}";

    private static void Read(uint id, RadioLink link)
    {
        var reason = Pump(id, link);
        if (!Links.TryRemove(id, out _)) return;
        link.Connection.Dispose();
        qgc_bluetooth_closed(id, reason);
    }

    private static string Pump(uint id, RadioLink link)
    {
        var buffer = new byte[ReadBytes];
        try
        {
            while (true)
            {
                var count = link.Input.Read(buffer, 0, buffer.Length);
                if (count <= 0) return ClosedByDevice;
                fixed (byte* bytes = buffer) qgc_bluetooth_received(id, bytes, (nuint)count);
            }
        }
        catch (Exception failure)
        {
            return failure.Message.Length > 0 ? $"{Lost} {failure.Message.Split('\n')[0].Trim()}" : Lost;
        }
    }

    private static void Drop(uint id)
    {
        if (Links.TryRemove(id, out var link)) link.Connection.Dispose();
    }

    private static bool BeginScan()
    {
        if (Unusable() is not null) return false;
        lock (Gate)
        {
            Halt();
            Found.Clear();
            _paired = Paired();
            _stopScan = BluetoothRadio.Discover(Seen);
            _window = new Timer(_ => EndScan(), null, ScanWindow, Timeout.InfiniteTimeSpan);
            qgc_bluetooth_set_scanning(1);
        }
        Publish();
        return true;
    }

    private static bool EndScan()
    {
        lock (Gate) Halt();
        qgc_bluetooth_set_scanning(0);
        return true;
    }

    private static void Halt()
    {
        _window?.Dispose();
        _window = null;
        var stop = _stopScan;
        _stopScan = null;
        try { stop?.Invoke(); }
        catch (Exception) { }
    }

    private static void Seen(RadioDevice device)
    {
        var order = Interlocked.Increment(ref _order);
        Found.AddOrUpdate(device.Address, (device.Name, order), (_, known) => (device.Name.Length > 0 ? device.Name : known.Name, known.Order));
        Publish();
    }

    private static void Publish() =>
        qgc_bluetooth_set_devices(Lines(_paired, Found.OrderBy(f => f.Value.Order).Select(f => new RadioDevice(f.Value.Name, f.Key)).ToList()));

    internal static string Lines(IReadOnlyList<RadioDevice> paired, IReadOnlyList<RadioDevice> found) => string.Join('\n',
        paired.Concat(found)
            .GroupBy(d => d.Address)
            .Select(g => $"{Clean(g.Select(d => d.Name).LastOrDefault(n => n.Length > 0) ?? "")}\t{g.Key}"));

    private static string Clean(string name) => name.Replace('\t', ' ').Replace('\n', ' ').Replace('\r', ' ').Trim();

    [LibraryImport(Lib)]
    private static partial void qgc_bluetooth_install(Hooks* hooks);

    [LibraryImport(Lib, StringMarshalling = StringMarshalling.Utf8)]
    private static partial void qgc_bluetooth_set_devices(string lines);

    [LibraryImport(Lib)]
    private static partial void qgc_bluetooth_set_scanning(int scanning);

    [LibraryImport(Lib)]
    private static partial void qgc_bluetooth_received(uint id, byte* bytes, nuint length);

    [LibraryImport(Lib, StringMarshalling = StringMarshalling.Utf8)]
    private static partial void qgc_bluetooth_closed(uint id, string reason);

    [LibraryImport(Lib, StringMarshalling = StringMarshalling.Utf8)]
    private static partial void qgc_bluetooth_refused(uint id, string reason);
}
