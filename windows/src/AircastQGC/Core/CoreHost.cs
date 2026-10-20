using System;
using System.Linq;
using System.Runtime.InteropServices;

namespace AircastQGC.Core;

public static class CoreHost
{
    private static bool _started;

    public static bool Started => _started;

    public static int Start(params string[] args)
    {
        if (_started) return 0;
        BluetoothHost.Start();
        var argv = args.Select(Marshal.StringToCoTaskMemUTF8).ToArray();
        var table = Marshal.AllocCoTaskMem(IntPtr.Size * argv.Length);
        try
        {
            for (var i = 0; i < argv.Length; i++) Marshal.WriteIntPtr(table, i * IntPtr.Size, argv[i]);
            var code = NativeMethods.qgc_start(argv.Length, table);
            _started = code == 0;
            return code;
        }
        finally
        {
            Marshal.FreeCoTaskMem(table);
            foreach (var p in argv) Marshal.FreeCoTaskMem(p);
        }
    }

    public static void Shutdown()
    {
        if (!_started) return;
        BluetoothHost.Stop();
        NativeMethods.qgc_request_quit();
        NativeMethods.qgc_shutdown();
        _started = false;
    }
}
