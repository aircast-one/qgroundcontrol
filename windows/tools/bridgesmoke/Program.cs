using System.Runtime.InteropServices;

var cli = new[] { "bridgesmoke", "--no-autoconnect" };
var ptrs = cli.Select(Marshal.StringToCoTaskMemUTF8).ToArray();
var table = Marshal.AllocCoTaskMem(IntPtr.Size * ptrs.Length);
for (var i = 0; i < ptrs.Length; i++) Marshal.WriteIntPtr(table, i * IntPtr.Size, ptrs[i]);

var code = N.qgc_start(ptrs.Length, table);
Console.WriteLine($"qgc_start => {code}");

foreach (var path in new[] { "host", "links" })
{
    var raw = N.qgc_bridge_get(path);
    var s = Marshal.PtrToStringUTF8(raw) ?? "";
    N.qgc_core_free(raw);
    Console.WriteLine($"{path} => {(s.Length > 400 ? s[..400] + "..." : s)}");
}

N.qgc_shutdown();
Console.WriteLine("OK");

static class N
{
    const string L = "groundstation";
    [DllImport(L)] public static extern int qgc_start(int argc, IntPtr argv);
    [DllImport(L)] public static extern void qgc_shutdown();
    [DllImport(L)] public static extern IntPtr qgc_bridge_get([MarshalAs(UnmanagedType.LPUTF8Str)] string path);
    [DllImport(L)] public static extern void qgc_core_free(IntPtr p);
}
