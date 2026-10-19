using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text.Json.Nodes;
using Microsoft.UI.Dispatching;

namespace AircastQGC.Core;

public static class BridgeWatch
{
    private static readonly object Gate = new();
    private static readonly Dictionary<string, string[]> Clients = new();
    private static readonly Dictionary<string, Action<JsonObject>> Handlers = new();
    private static readonly Dictionary<string, int> Counts = new();

    private static CoreEventHandler? _handler;
    private static bool _installed;
    private static DispatcherQueue? _ui;

    public static void BindDispatcher(DispatcherQueue ui) => _ui = ui;

    public static IReadOnlyDictionary<string, int> Delivered
    {
        get { lock (Gate) return new Dictionary<string, int>(Counts); }
    }

    public static void Watch(string client, IReadOnlyList<string> paths, Action<JsonObject>? onChange = null)
    {
        bool installing;
        string csv;
        lock (Gate)
        {
            if (paths.Count == 0) { Clients.Remove(client); Handlers.Remove(client); }
            else { Clients[client] = paths.ToArray(); if (onChange != null) Handlers[client] = onChange; }

            installing = !_installed && paths.Count > 0;
            _installed |= installing;
            csv = string.Join(",", Clients.TryGetValue(client, out var p) ? p : Array.Empty<string>());
        }

        if (installing)
        {
            _handler = Deliver;
            NativeMethods.qgc_bridge_set_event_handler(Marshal.GetFunctionPointerForDelegate(_handler));
        }
        NativeMethods.qgc_bridge_watch_client(client, csv);
    }

    public static void Stop(string client) => Watch(client, Array.Empty<string>());

    private static void Deliver(IntPtr pathPtr, IntPtr jsonPtr)
    {
        var path = Marshal.PtrToStringUTF8(pathPtr) ?? string.Empty;
        var json = Marshal.PtrToStringUTF8(jsonPtr) ?? string.Empty;

        List<Action<JsonObject>> called;
        lock (Gate)
        {
            Counts[path] = Counts.GetValueOrDefault(path) + 1;
            called = Clients
                .Where(kv => kv.Value.Contains(path))
                .Select(kv => Handlers.GetValueOrDefault(kv.Key))
                .Where(h => h != null)
                .Select(h => h!)
                .ToList();
        }
        if (called.Count == 0 || string.IsNullOrEmpty(json)) return;
        if (JsonNode.Parse(json) is not JsonObject view) return;

        void Fire() => called.ForEach(h => h(view));
        var ui = _ui;
        if (ui != null && !ui.HasThreadAccess) ui.TryEnqueue(Fire);
        else Fire();
    }
}
