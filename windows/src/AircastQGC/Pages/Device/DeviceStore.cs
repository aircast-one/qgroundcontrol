using System;
using System.IO;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace AircastQGC.Pages.Device;

internal static class DeviceStore
{
    private static readonly string StorePath = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Aircast", "aircast-device.json");

    private static readonly object Gate = new();

    public static string? Read(string key)
    {
        lock (Gate) return Load()[key] is JsonValue value && value.TryGetValue<string>(out var text) ? text : null;
    }

    public static void Write(string key, string value)
    {
        lock (Gate)
        {
            var held = Load();
            held[key] = value;
            try
            {
                Directory.CreateDirectory(Path.GetDirectoryName(StorePath)!);
                System.IO.File.WriteAllText(StorePath, held.ToJsonString());
            }
            catch (IOException) { }
            catch (UnauthorizedAccessException) { }
        }
    }

    private static JsonObject Load()
    {
        try { return JsonNode.Parse(System.IO.File.ReadAllText(StorePath)) as JsonObject ?? new JsonObject(); }
        catch (IOException) { return new JsonObject(); }
        catch (UnauthorizedAccessException) { return new JsonObject(); }
        catch (JsonException) { return new JsonObject(); }
    }
}
