using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.WindowsRuntime;
using System.Text;
using System.Text.RegularExpressions;
using System.Xml;
using System.Xml.Linq;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Windows.Storage.Streams;

namespace AircastQGC.Forms;

internal static partial class AssetImages
{
    public const string Airframe = "Airframe";
    public const string SetupSections = "SetupSections";
    public const string Calibration = "Calibration";

    private static readonly Dictionary<string, BitmapImage> Pngs = [];

    [GeneratedRegex(@"(?<=(?:fill|stroke)\s*[:=]\s*""?)(#[0-9a-fA-F]{3,8}|black|white)\b")]
    private static partial Regex Paint();

    [GeneratedRegex(@"([^{}]+)\{([^}]*)\}")]
    private static partial Regex Rule();

    public static Image Svg(string folder, string file, double height, string? tint = null, double width = double.NaN)
    {
        var image = new Image { Height = height, Width = width, Stretch = Stretch.Uniform, Visibility = Visibility.Collapsed };
        Load(image, PathOf(folder, file.EndsWith(".svg", StringComparison.OrdinalIgnoreCase) ? file : $"{file}.svg"), tint);
        return image;
    }

    public static Image Png(string folder, string file, double height)
    {
        var path = PathOf(folder, file);
        var image = new Image { Height = height, Stretch = Stretch.Uniform };
        if (!File.Exists(path)) return image;
        image.Source = Pngs.TryGetValue(path, out var held) ? held : Pngs[path] = new BitmapImage { DecodePixelHeight = (int)(height * 2), UriSource = new Uri(path) };
        return image;
    }

    public static string Hex(string brushKey) =>
        FormLook.Brush(brushKey) is SolidColorBrush { Color: var c } ? $"#{c.R:X2}{c.G:X2}{c.B:X2}" : "#FFFFFF";

    private static string PathOf(string folder, string file) => Path.Combine(AppContext.BaseDirectory, "Assets", folder, Path.GetFileName(file));

    private static string Tinted(string svg, string tint) => Paint().Replace(Inlined(svg), tint);

    private static string Inlined(string svg)
    {
        try
        {
            var doc = XDocument.Parse(svg);
            var styles = doc.Descendants().Where(e => e.Name.LocalName == "style").ToList();
            var rules = styles
                .SelectMany(style => Rule().Matches(style.Value).Select(m => (Selectors: m.Groups[1].Value.Split(','), Body: m.Groups[2].Value.Trim().TrimEnd(';'))))
                .SelectMany(rule => rule.Selectors.Select(selector => (Class: selector.Trim().TrimStart('.'), rule.Body)))
                .Where(rule => rule.Class.Length > 0 && rule.Body.Length > 0)
                .ToList();
            styles.ForEach(style => style.Remove());
            doc.Descendants()
                .Select(e => (Element: e, Classes: ((string?)e.Attribute("class") ?? "").Split(' ', StringSplitOptions.RemoveEmptyEntries)))
                .Where(e => e.Classes.Length > 0)
                .ToList()
                .ForEach(e =>
                {
                    var declared = string.Join(";", rules.Where(rule => e.Classes.Contains(rule.Class)).Select(rule => rule.Body)
                        .Append((string?)e.Element.Attribute("style") ?? "").Where(part => part.Length > 0));
                    if (declared.Length > 0) e.Element.SetAttributeValue("style", declared);
                });
            return doc.ToString(SaveOptions.DisableFormatting);
        }
        catch (XmlException) { return svg; }
    }

    private static async void Load(Image image, string path, string? tint)
    {
        if (!File.Exists(path)) return;
        try
        {
            var text = await File.ReadAllTextAsync(path);
            var shown = tint is null ? Inlined(text) : Tinted(text, tint);
            using var stream = new InMemoryRandomAccessStream();
            await stream.WriteAsync(Encoding.UTF8.GetBytes(shown).AsBuffer());
            stream.Seek(0);
            var svg = new SvgImageSource();
            if (await svg.SetSourceAsync(stream) != SvgImageSourceLoadStatus.Success) return;
            image.Source = svg;
            image.Visibility = Visibility.Visible;
        }
        catch (Exception error) when (error is IOException or COMException or UnauthorizedAccessException) { }
    }
}
