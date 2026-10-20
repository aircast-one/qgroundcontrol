using System.Collections.Generic;
using System.Numerics;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Hosting;
using Windows.UI;

namespace AircastQGC.Controls;

internal static class Osd
{
    private const float ShadowBlur = 3f;
    private const float ShadowDrop = 0.5f;

    private static readonly IReadOnlyDictionary<string, string> Labels = new Dictionary<string, string>
    {
        ["distance to home"] = "D",
        ["alt (rel)"] = "H",
        ["altitude"] = "H",
        ["ground speed"] = "H.S",
        ["climb rate"] = "V.S",
        ["air speed"] = "A.S",
        ["airspeed"] = "A.S",
        ["distance to operator"] = "D.OP",
        ["from you"] = "D.OP",
        ["heading"] = "HDG",
        ["distancetohome"] = "D",
        ["altituderelative"] = "H",
        ["groundspeed"] = "H.S",
        ["climbrate"] = "V.S",
    };

    private static readonly IReadOnlySet<string> Speeds = new HashSet<string> { "H.S", "V.S", "A.S" };

    public static string Label(string label, string name) =>
        Labels.GetValueOrDefault(label.Trim().ToLowerInvariant())
        ?? Labels.GetValueOrDefault(name.Trim().ToLowerInvariant())
        ?? label.ToUpperInvariant();

    public static bool IsSpeed(string label, string name) => Speeds.Contains(Label(label, name));

    public static Grid Shadowed(TextBlock text)
    {
        var host = new Border();
        var compositor = ElementCompositionPreview.GetElementVisual(host).Compositor;
        var shadow = compositor.CreateDropShadow();
        shadow.BlurRadius = ShadowBlur;
        shadow.Offset = new Vector3(0, ShadowDrop, 0);
        shadow.Color = (Color)Application.Current.Resources["OsdShadowColor"];
        shadow.Mask = text.GetAlphaMask();
        var sprite = compositor.CreateSpriteVisual();
        sprite.Shadow = shadow;
        sprite.RelativeSizeAdjustment = Vector2.One;
        ElementCompositionPreview.SetElementChildVisual(host, sprite);
        return new Grid { Children = { host, text } };
    }
}
