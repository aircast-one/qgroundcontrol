using System;
using System.Runtime.InteropServices;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Core;

internal static partial class NativeMethods
{
    [LibraryImport(Lib)]
    internal static partial int qgc_synthetic_attach(IntPtr surface, int width, int height);

    [LibraryImport(Lib)]
    internal static partial void qgc_synthetic_release(IntPtr surface);
}

internal static partial class CompositionSurface
{
    private const uint AllAccess = 0x0003;
    private const int SetSwapChainHandleSlot = 4;
    private static readonly Guid PanelNative2 = new("88fd8248-10da-4810-bb4c-010dd27faea9");

    [LibraryImport("dcomp.dll")]
    private static partial int DCompositionCreateSurfaceHandle(uint desiredAccess, IntPtr securityAttributes, out IntPtr surfaceHandle);

    [LibraryImport("kernel32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool CloseHandle(IntPtr handle);

    public static IntPtr Bind(SwapChainPanel panel)
    {
        if (DCompositionCreateSurfaceHandle(AllAccess, IntPtr.Zero, out var surface) < 0 || surface == IntPtr.Zero) return IntPtr.Zero;
        if (Show(panel, surface) >= 0) return surface;
        CloseHandle(surface);
        return IntPtr.Zero;
    }

    public static void Close(IntPtr surface)
    {
        if (surface != IntPtr.Zero) CloseHandle(surface);
    }

    private static unsafe int Show(SwapChainPanel panel, IntPtr surface)
    {
        var unknown = ((WinRT.IWinRTObject)panel).NativeObject.ThisPtr;
        var found = Marshal.QueryInterface(unknown, in PanelNative2, out var native);
        if (found < 0) return found;
        try
        {
            var slots = *(IntPtr**)native;
            var setSwapChainHandle = (delegate* unmanaged[Stdcall]<IntPtr, IntPtr, int>)slots[SetSwapChainHandleSlot];
            return setSwapChainHandle(native, surface);
        }
        finally
        {
            Marshal.Release(native);
        }
    }
}
