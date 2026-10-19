using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;

namespace AircastQGC.Controls;

internal static class PreflightLook
{
    public static Brush Token(string key) => (Brush)Application.Current.Resources[key];

    public static FontFamily Font => (FontFamily)Application.Current.Resources["AppFontFamily"];

    public static double Size(string key) => (double)Application.Current.Resources[key];

    public static void Accent(Button button) =>
        Paint(button, Token("AccentBrush"), Token("AccentHoverBrush"), Token("AccentBrush"),
            Token("AccentOnBrush"), Token("AccentBrush"));

    public static void Standard(Button button) =>
        Paint(button, Token("ControlBrush"), Token("ControlHoverBrush"), Token("SubtleSelectedBrush"),
            Token("TextPrimaryBrush"), Token("ControlStrokeBrush"));

    public static void Subtle(Button button) =>
        Paint(button, Clear, Token("SubtleHoverBrush"), Token("SubtleSelectedBrush"),
            Token("TextPrimaryBrush"), Clear);

    private static Brush Clear => new SolidColorBrush(Colors.Transparent);

    private static void Paint(Button button, Brush rest, Brush hover, Brush pressed, Brush text, Brush stroke) =>
        button.Resources = new ResourceDictionary
        {
            ["ButtonBackground"] = rest,
            ["ButtonBackgroundPointerOver"] = hover,
            ["ButtonBackgroundPressed"] = pressed,
            ["ButtonBackgroundDisabled"] = rest,
            ["ButtonForeground"] = text,
            ["ButtonForegroundPointerOver"] = text,
            ["ButtonForegroundPressed"] = text,
            ["ButtonForegroundDisabled"] = Token("TextDisabledBrush"),
            ["ButtonBorderBrush"] = stroke,
            ["ButtonBorderBrushPointerOver"] = stroke,
            ["ButtonBorderBrushPressed"] = stroke,
            ["ButtonBorderBrushDisabled"] = stroke,
        };
}
