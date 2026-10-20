using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using AircastQGC.Controls;
using AircastQGC.Core;
using Microsoft.UI;
using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using Windows.UI;
using Windows.UI.ViewManagement;

namespace AircastQGC.Theme;

internal static class Appearance
{
    public const string PaletteSetting = "settings.appSettings.indoorPalette";
    public const string FontSetting = "settings.appSettings.appFontPointSize";
    public const int PaletteDark = 1;
    public const int PaletteLight = 0;

    private const string Client = "theme.appearance";
    private const string LightKey = "Light";
    private const string DefaultKey = "Default";
    private const string MarkerToken = "BgBaseBrush";
    private const double PlatformPointSize = 14;
    private const double PointSizeMin = 6;
    private const double PointSizeMax = 48;
    private static readonly string[] FontTokens = ["FontSizeCaption", "FontSizeBody", "FontSizeBodyLarge", "FontSizeSubtitle", "FontSizeTitle", "FontSizeTitleLarge"];
    private static readonly UISettings SystemColours = new();

    private static IReadOnlyDictionary<string, Color> _darkColors = new Dictionary<string, Color>();
    private static IReadOnlyDictionary<string, double> _baseSizes = new Dictionary<string, double>();
    private static FrameworkElement? _root;
    private static AppWindow? _window;
    private static bool _dark = true;
    private static double _scale = 1;

    public static bool Dark => _dark;

    public static double Scale => _scale;

    public static bool IsDark(int? palette, bool systemDark) => palette switch
    {
        PaletteDark => true,
        PaletteLight => false,
        null => true,
        _ => systemDark,
    };

    public static double FontScale(double? pointSize) =>
        pointSize is { } size && size >= PointSizeMin && size <= PointSizeMax ? size / PlatformPointSize : 1;

    private static string ControlPath(string setting) => $"view.control({setting})";

    private static ResourceDictionary? Tokens =>
        Application.Current.Resources.MergedDictionaries.FirstOrDefault(d =>
            d.ThemeDictionaries.TryGetValue(LightKey, out var light) && light is ResourceDictionary held && held.ContainsKey(MarkerToken));

    private static IEnumerable<(string Key, SolidColorBrush Brush)> Brushes(ResourceDictionary? theme) =>
        theme is null ? [] : theme.Where(pair => pair.Key is string && pair.Value is SolidColorBrush).Select(pair => ((string)pair.Key, (SolidColorBrush)pair.Value));

    private static ResourceDictionary? ThemeDictionary(string key) =>
        Tokens is { } tokens && tokens.ThemeDictionaries.TryGetValue(key, out var theme) ? theme as ResourceDictionary : null;

    private static bool SystemDark() => SystemColours.GetColorValue(UIColorType.Background) is { R: < 128, G: < 128, B: < 128 };

    private static (bool Dark, double Scale) Read()
    {
        var palette = MapUiJson.Number(Bridge.Get(ControlPath(PaletteSetting))["value"]);
        var size = MapUiJson.Number(Bridge.Get(ControlPath(FontSetting))["value"]);
        return (IsDark(palette is { } raw ? (int)raw : null, SystemDark()), FontScale(size));
    }

    public static void Prepare()
    {
        _darkColors = Brushes(ThemeDictionary(DefaultKey)).ToDictionary(pair => pair.Key, pair => pair.Brush.Color);
        _baseSizes = Tokens is { } tokens
            ? FontTokens.Where(tokens.ContainsKey).ToDictionary(key => key, key => (double)tokens[key])
            : new Dictionary<string, double>();
        var (dark, scale) = Read();
        Paint(dark);
        Size(scale);
    }

    public static void Attach(FrameworkElement root, AppWindow window)
    {
        _root = root;
        _window = window;
        ApplyTheme(root, _dark, _scale != 1);
        BridgeWatch.Watch(Client, [ControlPath(PaletteSetting), ControlPath(FontSetting)], _ => Refresh());
        SystemColours.ColorValuesChanged += (_, _) => root.DispatcherQueue.TryEnqueue(Refresh);
    }

    public static async void Refresh()
    {
        var (dark, scale) = await Task.Run(Read);
        if (dark == _dark && scale == _scale) return;
        var resized = scale != _scale;
        Paint(dark);
        Size(scale);
        if (_root is { } root) ApplyTheme(root, dark, resized);
    }

    private static void Paint(bool dark)
    {
        _dark = dark;
        var light = Brushes(ThemeDictionary(LightKey)).ToDictionary(pair => pair.Key, pair => pair.Brush.Color);
        Brushes(ThemeDictionary(DefaultKey))
            .Where(pair => (dark ? _darkColors : light).ContainsKey(pair.Key))
            .ToList()
            .ForEach(pair => pair.Brush.Color = (dark ? _darkColors : light)[pair.Key]);
    }

    private static void Size(double scale)
    {
        _scale = scale;
        _baseSizes.ToList().ForEach(pair => Application.Current.Resources[pair.Key] = Math.Round(pair.Value * scale));
    }

    private static void ApplyTheme(FrameworkElement root, bool dark, bool reresolve)
    {
        var wanted = dark ? ElementTheme.Dark : ElementTheme.Light;
        if (reresolve) root.RequestedTheme = dark ? ElementTheme.Light : ElementTheme.Dark;
        root.RequestedTheme = wanted;
        Caption(dark);
    }

    private static void Caption(bool dark)
    {
        if (_window?.TitleBar is not { } bar) return;
        Color? Token(string key) => dark ? null : ThemeDictionary(LightKey)?[key] is SolidColorBrush brush ? brush.Color : null;
        Color? clear = dark ? null : Colors.Transparent;
        bar.ButtonBackgroundColor = clear;
        bar.ButtonInactiveBackgroundColor = clear;
        bar.ButtonForegroundColor = Token("TextPrimaryBrush");
        bar.ButtonHoverForegroundColor = Token("TextPrimaryBrush");
        bar.ButtonPressedForegroundColor = Token("TextPrimaryBrush");
        bar.ButtonInactiveForegroundColor = Token("TextTertiaryBrush");
        bar.ButtonHoverBackgroundColor = Token("SubtleHoverBrush");
        bar.ButtonPressedBackgroundColor = Token("SubtleSelectedBrush");
    }
}
