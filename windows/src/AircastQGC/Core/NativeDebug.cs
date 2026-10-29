using System;
using System.Linq;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json.Nodes;
using Microsoft.UI.Xaml;

namespace AircastQGC.Core;

internal static unsafe partial class NativeDebug
{
    private const string Lib = "groundstation";
    private const string User = "user32.dll";
    private const uint MouseEvent = 0;
    private const uint KeyEvent = 1;
    private const uint KeyUp = 0x0002;
    private const uint Unicode = 0x0004;
    private const ushort Return = 0x0D;
    private const uint LeftDown = 0x0002;
    private const uint LeftUp = 0x0004;
    private const int ScreenHeight = 1;
    private const double BaseDpi = 96.0;
    private const int TitleMax = 256;

    private static IntPtr _window;

    [StructLayout(LayoutKind.Sequential)]
    private struct Hooks
    {
        public delegate* unmanaged[Cdecl]<byte*> Windows;
        public delegate* unmanaged[Cdecl]<byte*, double, double, byte*> Click;
        public delegate* unmanaged[Cdecl]<byte*, byte*, byte*> Type;
        public IntPtr Probe;
        public IntPtr Menu;
        public IntPtr MenuInvoke;
        public IntPtr BridgeStats;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct Rect
    {
        public int Left, Top, Right, Bottom;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct Spot
    {
        public int X, Y;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct Mouse
    {
        public int Dx, Dy;
        public uint Data, Flags, Time;
        public IntPtr Extra;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct Key
    {
        public ushort Vk, Scan;
        public uint Flags, Time;
        public IntPtr Extra;
    }

    [StructLayout(LayoutKind.Explicit)]
    private struct Input
    {
        [FieldOffset(0)] public uint Type;
        [FieldOffset(8)] public Mouse Mouse;
        [FieldOffset(8)] public Key Key;
    }

    [LibraryImport(Lib)]
    private static partial void qgc_native_debug_install(Hooks* hooks);

    [LibraryImport(User)]
    private static partial int GetWindowTextW(IntPtr window, char* text, int max);

    [LibraryImport(User)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool GetWindowRect(IntPtr window, Rect* rect);

    [LibraryImport(User)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool GetClientRect(IntPtr window, Rect* rect);

    [LibraryImport(User)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool ClientToScreen(IntPtr window, Spot* spot);

    [LibraryImport(User)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool IsWindowVisible(IntPtr window);

    [LibraryImport(User)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool SetForegroundWindow(IntPtr window);

    [LibraryImport(User)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool SetCursorPos(int x, int y);

    [LibraryImport(User)]
    private static partial IntPtr GetForegroundWindow();

    [LibraryImport(User)]
    private static partial uint GetDpiForWindow(IntPtr window);

    [LibraryImport(User)]
    private static partial uint SendInput(uint count, Input* inputs, int size);

    [LibraryImport(User)]
    private static partial int GetSystemMetrics(int index);

    public static void Install(Window window)
    {
        _window = WinRT.Interop.WindowNative.GetWindowHandle(window);
        var hooks = new Hooks { Windows = &ListWindows, Click = &ClickAt, Type = &TypeText };
        qgc_native_debug_install(&hooks);
    }

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    private static byte* ListWindows() => Guard(Describe);

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    private static byte* ClickAt(byte* title, double x, double y)
    {
        var named = Marshal.PtrToStringUTF8((IntPtr)title) ?? "";
        return Guard(() => Press(named, x, y));
    }

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    private static byte* TypeText(byte* title, byte* text)
    {
        var named = Marshal.PtrToStringUTF8((IntPtr)title) ?? "";
        var typed = Marshal.PtrToStringUTF8((IntPtr)text) ?? "";
        return Guard(() => Type(named, typed));
    }

    private static byte* Guard(Func<JsonObject> work)
    {
        try
        {
            return Encode(work());
        }
        catch (Exception error)
        {
            return Encode(Refuse(error.Message));
        }
    }

    private static byte* Encode(JsonObject body)
    {
        var bytes = Encoding.UTF8.GetBytes(body.ToJsonString());
        var text = (byte*)NativeMemory.Alloc((nuint)bytes.Length + 1);
        bytes.CopyTo(new Span<byte>(text, bytes.Length));
        text[bytes.Length] = 0;
        return text;
    }

    private static JsonObject Refuse(string reason) => new() { ["ok"] = false, ["error"] = reason };

    private static string Title()
    {
        var text = stackalloc char[TitleMax];
        return new string(text, 0, Math.Max(0, GetWindowTextW(_window, text, TitleMax)));
    }

    private static double Scale() => GetDpiForWindow(_window) / BaseDpi;

    private static Rect Client()
    {
        var area = default(Rect);
        GetClientRect(_window, &area);
        return area;
    }

    private static Spot OnScreen(int x, int y)
    {
        var spot = new Spot { X = x, Y = y };
        ClientToScreen(_window, &spot);
        return spot;
    }

    private static JsonObject Describe()
    {
        var frame = default(Rect);
        GetWindowRect(_window, &frame);
        var (area, scale, origin) = (Client(), Scale(), OnScreen(0, 0));
        var entry = new JsonObject
        {
            ["index"] = 0,
            ["title"] = Title(),
            ["number"] = _window.ToInt64(),
            ["visible"] = IsWindowVisible(_window),
            ["key"] = GetForegroundWindow() == _window,
            ["main"] = true,
            ["captureRect"] = new JsonObject { ["x"] = frame.Left, ["y"] = frame.Top, ["width"] = frame.Right - frame.Left, ["height"] = frame.Bottom - frame.Top },
            ["contentRect"] = new JsonObject { ["x"] = origin.X, ["y"] = origin.Y, ["width"] = area.Right, ["height"] = area.Bottom },
            ["contentSize"] = new JsonObject { ["width"] = Math.Round(area.Right / scale), ["height"] = Math.Round(area.Bottom / scale) },
            ["scale"] = scale,
        };
        return new JsonObject { ["windows"] = new JsonArray(entry), ["screenHeight"] = GetSystemMetrics(ScreenHeight) };
    }

    private static JsonObject Type(string title, string text)
    {
        if (title != Title()) return Refuse($"no visible window titled {title}");
        SetForegroundWindow(_window);
        var keys = text.SelectMany(Strokes).ToArray();
        fixed (Input* inputs = keys)
        {
            var sent = SendInput((uint)keys.Length, inputs, sizeof(Input));
            return sent == keys.Length
                ? new JsonObject { ["ok"] = true, ["typed"] = text }
                : Refuse($"SendInput delivered {sent} of {keys.Length} events");
        }
    }

    private static Input[] Strokes(char character) => character is '\r' or '\n'
        ? [Stroke(Return, 0, 0), Stroke(Return, 0, KeyUp)]
        : [Stroke(0, character, Unicode), Stroke(0, character, Unicode | KeyUp)];

    private static Input Stroke(ushort vk, ushort scan, uint flags) =>
        new() { Type = KeyEvent, Key = new Key { Vk = vk, Scan = scan, Flags = flags } };

    private static JsonObject Press(string title, double x, double y)
    {
        var (area, scale) = (Client(), Scale());
        var (px, py) = ((int)Math.Round(x * scale), (int)Math.Round(y * scale));
        if (title != Title()) return Refuse($"no visible window titled {title}");
        if (px < 0 || py < 0 || px >= area.Right || py >= area.Bottom)
            return Refuse($"point ({x}, {y}) is outside the {Math.Round(area.Right / scale)}x{Math.Round(area.Bottom / scale)} content view");
        var at = OnScreen(px, py);
        SetForegroundWindow(_window);
        SetCursorPos(at.X, at.Y);
        var inputs = stackalloc Input[]
        {
            new Input { Type = MouseEvent, Mouse = new Mouse { Flags = LeftDown } },
            new Input { Type = MouseEvent, Mouse = new Mouse { Flags = LeftUp } },
        };
        var sent = SendInput(2, inputs, sizeof(Input));
        return sent == 2
            ? new JsonObject { ["ok"] = true, ["windowPoint"] = new JsonObject { ["x"] = x, ["y"] = y }, ["screenPoint"] = new JsonObject { ["x"] = at.X, ["y"] = at.Y } }
            : Refuse($"SendInput delivered {sent} of 2 events");
    }
}
