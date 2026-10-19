using System;
using System.Collections.Concurrent;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Threading;
using System.Threading.Tasks;

namespace AircastQGC.Core;

internal static partial class MapNative
{
    private const string Lib = "groundstation";

    private static readonly ConcurrentDictionary<long, TaskCompletionSource<byte[]?>> Pending = new();
    private static long _next;

    [LibraryImport(Lib)]
    private static partial IntPtr qgc_map_current_type();

    [LibraryImport(Lib, StringMarshalling = StringMarshalling.Utf8)]
    private static partial ulong qgc_map_tile_fetch_cancellable(string mapType, int x, int y, int zoom, IntPtr handler, IntPtr context);

    [LibraryImport(Lib)]
    private static partial void qgc_map_tile_cancel(ulong ticket);

    internal static string CurrentType()
    {
        try { return NativeMethods.Take(qgc_map_current_type()).Trim(); }
        catch (Exception) { return string.Empty; }
    }

    internal static async Task<byte[]?> FetchAsync(string mapType, int x, int y, int zoom, CancellationToken token)
    {
        var id = Interlocked.Increment(ref _next);
        var reply = new TaskCompletionSource<byte[]?>(TaskCreationOptions.RunContinuationsAsynchronously);
        Pending[id] = reply;
        if (Start(mapType, x, y, zoom, id) is not { } ticket)
        {
            Pending.TryRemove(id, out _);
            return null;
        }
        using var registration = token.Register(() => Cancel(ticket));
        return await reply.Task;
    }

    private static void Cancel(ulong ticket)
    {
        try { qgc_map_tile_cancel(ticket); }
        catch (Exception) { }
    }

    private static unsafe ulong? Start(string mapType, int x, int y, int zoom, long id)
    {
        try
        {
            delegate* unmanaged[Cdecl]<byte*, int, IntPtr, void> handler = &Arrived;
            return qgc_map_tile_fetch_cancellable(mapType, x, y, zoom, (IntPtr)(void*)handler, (IntPtr)id);
        }
        catch (Exception) { return null; }
    }

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    private static unsafe void Arrived(byte* bytes, int length, IntPtr context)
    {
        if (!Pending.TryRemove((long)context, out var reply)) return;
        reply.TrySetResult(bytes == null || length <= 0 ? null : new ReadOnlySpan<byte>(bytes, length).ToArray());
    }
}
