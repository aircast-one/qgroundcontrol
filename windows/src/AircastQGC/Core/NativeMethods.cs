using System;
using System.Runtime.InteropServices;

namespace AircastQGC.Core;

[UnmanagedFunctionPointer(CallingConvention.Cdecl)]
public delegate void CoreEventHandler(IntPtr path, IntPtr json);

internal static partial class NativeMethods
{
    private const string Lib = "groundstation";

    [LibraryImport(Lib)]
    internal static partial int qgc_start(int argc, IntPtr argv);

    [LibraryImport(Lib)]
    internal static partial void qgc_shutdown();

    [LibraryImport(Lib)]
    internal static partial void qgc_request_quit();

    [LibraryImport(Lib, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial IntPtr qgc_bridge_get(string path);

    [LibraryImport(Lib, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial IntPtr qgc_bridge_get_fields(string path, string fieldsCsv);

    [LibraryImport(Lib, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial IntPtr qgc_bridge_set(string path, string valueJson);

    [LibraryImport(Lib, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial IntPtr qgc_bridge_invoke(string path, string argsJson);

    [LibraryImport(Lib, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial void qgc_bridge_watch_client(string client, string pathsCsv);

    [LibraryImport(Lib)]
    internal static partial void qgc_bridge_set_event_handler(IntPtr handler);

    [LibraryImport(Lib)]
    internal static partial void qgc_core_free(IntPtr text);

    internal static string Take(IntPtr raw)
    {
        if (raw == IntPtr.Zero) return string.Empty;
        try { return Marshal.PtrToStringUTF8(raw) ?? string.Empty; }
        finally { qgc_core_free(raw); }
    }
}
