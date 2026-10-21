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
