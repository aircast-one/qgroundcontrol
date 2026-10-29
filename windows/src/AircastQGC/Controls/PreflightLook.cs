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
            Token("AccentOnBrush"), Token("AccentBrush"),
            Token("AccentFillColorDisabledBrush"), Token("TextOnAccentFillColorDisabledBrush"), Clear);

    public static void Standard(Button button) =>
        Paint(button, Token("ControlBrush"), Token("ControlHoverBrush"), Token("SubtleSelectedBrush"),
            Token("TextPrimaryBrush"), Token("ControlStrokeBrush"),
            Token("ControlBrush"), Token("TextDisabledBrush"), Token("ControlStrokeBrush"));

    public static void Stop(Button button) =>
        Paint(button, Token("StopBrush"), Token("StopHoverBrush"), Token("StopHoverBrush"),
            Token("StopOnBrush"), Token("StopBrush"),
            Token("ControlBrush"), Token("TextDisabledBrush"), Token("ControlStrokeBrush"));

    public static void Chip(Button button) =>
        Paint(button, Token("ChipBrush"), Token("ChipHoverBrush"), Token("SubtleSelectedBrush"),
            Token("TextPrimaryBrush"), Token("ControlStrokeBrush"),
            Token("ChipBrush"), Token("TextDisabledBrush"), Token("ControlStrokeBrush"));

    public static void Subtle(Button button) =>
        Paint(button, Clear, Token("SubtleHoverBrush"), Token("SubtleSelectedBrush"),
            Token("TextPrimaryBrush"), Clear,
            Clear, Token("TextDisabledBrush"), Clear);

    private static Brush Clear => new SolidColorBrush(Colors.Transparent);

    private static void Paint(Button button, Brush rest, Brush hover, Brush pressed, Brush text, Brush stroke,
        Brush disabled, Brush disabledText, Brush disabledStroke) =>
        button.Resources = new ResourceDictionary
        {
            ["ButtonBackground"] = rest,
            ["ButtonBackgroundPointerOver"] = hover,
            ["ButtonBackgroundPressed"] = pressed,
            ["ButtonBackgroundDisabled"] = disabled,
            ["ButtonForeground"] = text,
            ["ButtonForegroundPointerOver"] = text,
            ["ButtonForegroundPressed"] = text,
            ["ButtonForegroundDisabled"] = disabledText,
            ["ButtonBorderBrush"] = stroke,
            ["ButtonBorderBrushPointerOver"] = stroke,
            ["ButtonBorderBrushPressed"] = stroke,
            ["ButtonBorderBrushDisabled"] = disabledStroke,
        };
}
