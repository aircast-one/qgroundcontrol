using System;
using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;

namespace AircastQGC.Core;

internal static class Firewall
{
    private const int AnyIpVersion = 2;
    private const int Udp = 17;

    public static bool BlocksUdp(int port)
    {
        try
        {
            dynamic manager = Activator.CreateInstance(Type.GetTypeFromProgID("HNetCfg.FwMgr", true)!)!;
            manager.IsPortAllowed(Environment.ProcessPath, AnyIpVersion, port, "", Udp, out object allowed, out object _);
            return allowed is false;
        }
        catch (Exception error) when (error is COMException or UnauthorizedAccessException or InvalidCastException or Microsoft.CSharp.RuntimeBinder.RuntimeBinderException)
        {
            return false;
        }
    }

    public static void OpenAllowedApps()
    {
        try { Process.Start(new ProcessStartInfo("control.exe", "/name Microsoft.WindowsFirewall /page pageConfigureApps") { UseShellExecute = true }); }
        catch (Win32Exception) { }
    }
}
