using System;
using System.IO;
using System.Runtime.InteropServices;

namespace AircastQGC.Core;

internal static partial class VideoNative
{
    private const string Lib = "qgc_video";
    private const int AbiVersion = 4;

    internal const int Main = 0;
    internal const int Pip = 1;

    private static readonly Lazy<bool> Loaded = new(Load);

    internal static bool Ready => Loaded.Value;

    private static bool Load()
    {
        var root = Path.Combine(AppContext.BaseDirectory, "gstreamer");
        var registry = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "AircastQGC");
        Directory.CreateDirectory(registry);
        Environment.SetEnvironmentVariable("PATH", Path.Combine(root, "bin") + Path.PathSeparator + Environment.GetEnvironmentVariable("PATH"));
        Environment.SetEnvironmentVariable("GST_PLUGIN_SYSTEM_PATH_1_0", Path.Combine(root, "lib", "gstreamer-1.0"));
        Environment.SetEnvironmentVariable("GST_PLUGIN_SCANNER_1_0", Path.Combine(root, "libexec", "gstreamer-1.0", "gst-plugin-scanner.exe"));
        Environment.SetEnvironmentVariable("GST_REGISTRY_1_0", Path.Combine(registry, "gst-registry-arm64.bin"));
        try { return qgc_video_abi_version() == AbiVersion && qgc_video_available(); }
        catch (DllNotFoundException) { return false; }
        catch (EntryPointNotFoundException) { return false; }
    }

    internal static string Text(IntPtr raw) => raw == IntPtr.Zero ? "" : Marshal.PtrToStringUTF8(raw) ?? "";

    [LibraryImport(Lib)]
    internal static partial int qgc_video_abi_version();

    [LibraryImport(Lib)]
    [return: MarshalAs(UnmanagedType.U1)]
    internal static partial bool qgc_video_available();

    [LibraryImport(Lib, StringMarshalling = StringMarshalling.Utf8)]
    [return: MarshalAs(UnmanagedType.U1)]
    internal static partial bool qgc_video_start(int channel, string pipeline);

    [LibraryImport(Lib)]
    internal static partial void qgc_video_stop(int channel);

    [LibraryImport(Lib)]
    [return: MarshalAs(UnmanagedType.U1)]
    internal static partial bool qgc_video_running(int channel);

    [LibraryImport(Lib)]
    internal static partial int qgc_video_width(int channel);

    [LibraryImport(Lib)]
    internal static partial int qgc_video_height(int channel);

    [LibraryImport(Lib)]
    internal static partial long qgc_video_frames(int channel);

    [LibraryImport(Lib)]
    internal static partial long qgc_video_source_buffers(int channel);

    [LibraryImport(Lib)]
    internal static partial IntPtr qgc_video_last_error(int channel);

    [LibraryImport(Lib)]
    internal static partial IntPtr qgc_video_stream_error(int channel);

    [LibraryImport(Lib)]
    [return: MarshalAs(UnmanagedType.U1)]
    internal static unsafe partial bool qgc_video_copy_frame(int channel, byte* destination, int capacity, out int width, out int height, out int stride);

    [LibraryImport(Lib, StringMarshalling = StringMarshalling.Utf8)]
    [return: MarshalAs(UnmanagedType.U1)]
    internal static partial bool qgc_video_start_recording(int channel, string file, int format);

    [LibraryImport(Lib)]
    internal static partial void qgc_video_stop_recording(int channel);

    [LibraryImport(Lib)]
    [return: MarshalAs(UnmanagedType.U1)]
    internal static partial bool qgc_video_recording(int channel);

    [LibraryImport("groundstation")]
    [return: MarshalAs(UnmanagedType.U1)]
    internal static partial bool qgc_core_video_take_restart(int channel);
}
