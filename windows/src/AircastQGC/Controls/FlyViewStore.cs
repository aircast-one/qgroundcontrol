using System;
using System.IO;
using System.Text.Json.Nodes;

namespace AircastQGC.Controls;

internal static class FlyViewStore
{
    private const string ThreeD = "ThreeD";
    private const string Map = "Map";

    private static readonly string StorePath = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "AircastQGC", "fly-view.json");

    private static readonly string HoldHintPath = Path.Combine(Path.GetDirectoryName(StorePath)!, "video-hold-hint-seen");

    private static readonly string Map3dPath = Path.Combine(Path.GetDirectoryName(StorePath)!, "fly-map-3d");

    private static readonly string Map3dHintPath = Path.Combine(Path.GetDirectoryName(StorePath)!, "fly-map-3d-hinted");

    public static bool LoadHoldHintSeen() => File.Exists(HoldHintPath);

    public static void SaveHoldHintSeen() => Mark(HoldHintPath, true);

    public static bool LoadMap3d() => File.Exists(Map3dPath);

    public static void SaveMap3d(bool shown) => Mark(Map3dPath, shown);

    public static bool LoadMap3dHinted() => File.Exists(Map3dHintPath);

    public static void SaveMap3dHinted() => Mark(Map3dHintPath, true);

    private static void Mark(string path, bool on)
    {
        try
        {
            Directory.CreateDirectory(Path.GetDirectoryName(path)!);
            if (on) File.WriteAllText(path, "");
            else File.Delete(path);
        }
        catch (Exception e) when (InstrumentSelectionStore.Recoverable(e))
        {
        }
    }

    public static bool LoadThreeD()
    {
        try
        {
            return File.Exists(StorePath) && JsonNode.Parse(File.ReadAllText(StorePath)) is JsonValue v && v.TryGetValue<string>(out var view) && view == ThreeD;
        }
        catch (Exception e) when (InstrumentSelectionStore.Recoverable(e))
        {
            return false;
        }
    }

    public static void SaveThreeD(bool threeD)
    {
        try
        {
            Directory.CreateDirectory(Path.GetDirectoryName(StorePath)!);
            File.WriteAllText(StorePath, JsonValue.Create(threeD ? ThreeD : Map).ToJsonString());
        }
        catch (Exception e) when (InstrumentSelectionStore.Recoverable(e))
        {
        }
    }
}
