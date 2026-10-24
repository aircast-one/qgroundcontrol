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

    public static bool LoadHoldHintSeen() => File.Exists(HoldHintPath);

    public static void SaveHoldHintSeen()
    {
        try
        {
            Directory.CreateDirectory(Path.GetDirectoryName(HoldHintPath)!);
            File.WriteAllText(HoldHintPath, "");
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
