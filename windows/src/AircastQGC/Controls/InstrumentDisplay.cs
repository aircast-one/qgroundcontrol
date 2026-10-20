using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.WindowsRuntime;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using System.Xml;
using System.Xml.Linq;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Windows.Storage.Streams;
using Windows.UI;

namespace AircastQGC.Controls;

public enum RangeType { None, Color, Opacity, Icon }

public sealed record ValueDisplay
{
    public string Text { get; init; } = "";
    public bool ShowUnits { get; init; } = true;
    public bool ShowIcon { get; init; }
    public string Icon { get; init; } = "";
    public RangeType RangeType { get; init; } = RangeType.None;
    public IReadOnlyList<double> Values { get; init; } = [];
    public IReadOnlyList<long> Colours { get; init; } = [];
    public IReadOnlyList<double> Opacities { get; init; } = [];
    public IReadOnlyList<string> Icons { get; init; } = [];
}

internal static class InstrumentDisplays
{
    public const long NoColour = 0;
    public const long Green = 0xFF008000;
    public const double RangeLow = 0;
    public const double RangeHigh = 100;

    public static readonly IReadOnlyList<(long Colour, string Name)> RangeColours =
    [
        (Green, "Green"), (0xFFFFFF00, "Yellow"), (0xFFFFA500, "Orange"), (0xFFFF0000, "Red"), (0xFF0000FF, "Blue"), (0xFFFFFFFF, "White"), (NoColour, "No colour"),
    ];

    public static ValueDisplay WithRangeType(ValueDisplay display, RangeType type, string firstIcon)
    {
        IReadOnlyList<double> values = type == RangeType.None ? [] : [RangeLow, RangeHigh];
        var slots = type == RangeType.None ? 0 : values.Count + 1;
        return display with
        {
            RangeType = type,
            Values = values,
            Colours = type == RangeType.Color ? Enumerable.Repeat(Green, slots).ToList() : [],
            Opacities = type == RangeType.Opacity ? Enumerable.Repeat(1.0, slots).ToList() : [],
            Icons = type == RangeType.Icon ? Enumerable.Repeat(firstIcon, slots).ToList() : [],
        };
    }

    public static ValueDisplay WithRow(ValueDisplay display, string firstIcon) => display with
    {
        Values = display.Values.Append((display.Values.Count > 0 ? display.Values[^1] : RangeLow) + 1).ToList(),
        Colours = display.RangeType == RangeType.Color ? display.Colours.Append(Green).ToList() : display.Colours,
        Opacities = display.RangeType == RangeType.Opacity ? display.Opacities.Append(1.0).ToList() : display.Opacities,
        Icons = display.RangeType == RangeType.Icon ? display.Icons.Append(firstIcon).ToList() : display.Icons,
    };

    public static ValueDisplay WithoutRow(ValueDisplay display, int index) => display with
    {
        Values = display.Values.Where((_, i) => i != index).ToList(),
        Colours = display.Colours.Where((_, i) => i != index + 1).ToList(),
        Opacities = display.Opacities.Where((_, i) => i != index + 1).ToList(),
        Icons = display.Icons.Where((_, i) => i != index + 1).ToList(),
    };

    public static IReadOnlyList<T> Replaced<T>(IReadOnlyList<T> list, int index, T value) =>
        list.Select((old, i) => i == index ? value : old).ToList();

    public static int RangeIndex(double? raw, IReadOnlyList<double> values) =>
        raw is { } value && !double.IsNaN(value)
            ? values.Select((limit, i) => (limit, i)).Where(entry => value <= entry.limit).Select(entry => (int?)entry.i).FirstOrDefault() ?? values.Count
            : 0;

    public static long? Colour(ValueDisplay display, double? raw) =>
        display.RangeType == RangeType.Color && RangeIndex(raw, display.Values) is var at && at < display.Colours.Count && display.Colours[at] != NoColour
            ? display.Colours[at]
            : null;

    public static double Opacity(ValueDisplay display, double? raw) =>
        display.RangeType == RangeType.Opacity && RangeIndex(raw, display.Values) is var at && at < display.Opacities.Count
            ? Math.Clamp(display.Opacities[at], 0, 1)
            : 1;

    public static string? IconOf(ValueDisplay display, double? raw) =>
        (display.RangeType == RangeType.Icon ? display.Icons.ElementAtOrDefault(RangeIndex(raw, display.Values))
            : display.ShowIcon ? display.Icon
            : null) is { Length: > 0 } icon ? icon : null;

    public static bool IconShown(ValueDisplay display) => display.RangeType == RangeType.Icon || display.ShowIcon;

    public static string RangeHelp(RangeType type) => type switch
    {
        RangeType.Color => "Specify the color you want to apply based on value ranges. The color will be applied to the icon if available, otherwise to the value itself.",
        RangeType.Opacity => "Specify the icon opacity you want based on value ranges.",
        RangeType.Icon => "Specify the icon you want to display based on value ranges.",
        _ => "",
    };

    public static Color ColorOf(long argb) =>
        Color.FromArgb((byte)(argb >> 24), (byte)(argb >> 16), (byte)(argb >> 8), (byte)argb);

    public static string Hex(long argb) => $"#{(argb >> 16) & 0xFF:X2}{(argb >> 8) & 0xFF:X2}{argb & 0xFF:X2}";

    public static JsonObject ToJson(ValueDisplay display) => new()
    {
        ["text"] = display.Text,
        ["showUnits"] = display.ShowUnits,
        ["showIcon"] = display.ShowIcon,
        ["icon"] = display.Icon,
        ["rangeType"] = display.RangeType.ToString(),
        ["values"] = new JsonArray(display.Values.Select(v => (JsonNode?)JsonValue.Create(v)).ToArray()),
        ["colours"] = new JsonArray(display.Colours.Select(v => (JsonNode?)JsonValue.Create(v)).ToArray()),
        ["opacities"] = new JsonArray(display.Opacities.Select(v => (JsonNode?)JsonValue.Create(v)).ToArray()),
        ["icons"] = new JsonArray(display.Icons.Select(v => (JsonNode?)JsonValue.Create(v)).ToArray()),
    };

    private static IReadOnlyList<T> List<T>(JsonNode? node, Func<JsonNode?, T?> read) where T : struct =>
        (node as JsonArray ?? []).Select(read).Where(value => value.HasValue).Select(value => value!.Value).ToList();

    public static ValueDisplay FromJson(JsonObject? json) => json is null ? new ValueDisplay() : new ValueDisplay
    {
        Text = InstrumentJson.Text(json, "text"),
        ShowUnits = json["showUnits"] is JsonValue units && units.TryGetValue<bool>(out var shown) ? shown : true,
        ShowIcon = InstrumentJson.Flag(json, "showIcon"),
        Icon = InstrumentJson.Text(json, "icon"),
        RangeType = Enum.TryParse<RangeType>(InstrumentJson.Text(json, "rangeType"), out var type) ? type : RangeType.None,
        Values = List(json["values"], n => n is JsonValue v && v.TryGetValue<double>(out var d) ? d : (double?)null),
        Colours = List(json["colours"], n => n is JsonValue v && v.TryGetValue<long>(out var l) ? l : (long?)null),
        Opacities = List(json["opacities"], n => n is JsonValue v && v.TryGetValue<double>(out var d) ? d : (double?)null),
        Icons = (json["icons"] as JsonArray ?? []).Select(n => n is JsonValue v && v.TryGetValue<string>(out var s) ? s : "").ToList(),
    };
}

internal static class InstrumentDisplayStore
{
    private static readonly string StorePath = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "AircastQGC", "instrument-displays.json");

    private static bool Recoverable(Exception e) =>
        e is IOException or UnauthorizedAccessException or JsonException or NotSupportedException;

    private static JsonObject All()
    {
        try { return File.Exists(StorePath) ? JsonNode.Parse(File.ReadAllText(StorePath)) as JsonObject ?? new JsonObject() : new JsonObject(); }
        catch (Exception e) when (Recoverable(e)) { return new JsonObject(); }
    }

    public static IReadOnlyDictionary<string, ValueDisplay> Load(string vehicleClass) =>
        All()
            .Where(pair => pair.Key.StartsWith($"{vehicleClass}/", StringComparison.Ordinal))
            .ToDictionary(pair => pair.Key[(vehicleClass.Length + 1)..], pair => InstrumentDisplays.FromJson(pair.Value as JsonObject));

    public static void Save(string vehicleClass, string id, ValueDisplay display)
    {
        var all = All();
        all[$"{vehicleClass}/{id}"] = InstrumentDisplays.ToJson(display);
        try
        {
            Directory.CreateDirectory(Path.GetDirectoryName(StorePath)!);
            File.WriteAllText(StorePath, all.ToJsonString());
        }
        catch (Exception e) when (Recoverable(e))
        {
        }
    }
}

internal static class InstrumentIcons
{
    private const string Folder = "InstrumentValueIcons";
    private static readonly Dictionary<string, SvgImageSource> Loaded = [];

    private static string Root => Path.Combine(AppContext.BaseDirectory, "Assets", Folder);

    public static IReadOnlyList<string> Names() =>
        Directory.Exists(Root)
            ? Directory.GetFiles(Root, "*.svg").Select(Path.GetFileName).OfType<string>().Order(StringComparer.Ordinal).ToList()
            : [];

    public static Image Image(string name, string tint, double size)
    {
        var image = new Image { Width = size, Height = size, Stretch = Stretch.Uniform };
        AutomationProperties.SetName(image, Path.GetFileNameWithoutExtension(name));
        Load(image, name, tint);
        return image;
    }

    public static async void Load(Image image, string name, string tint)
    {
        var key = $"{name}|{tint}";
        if (Loaded.TryGetValue(key, out var held))
        {
            image.Source = held;
            return;
        }
        var path = Path.Combine(Root, Path.GetFileName(name));
        if (!File.Exists(path)) return;
        try
        {
            var doc = XDocument.Parse(await File.ReadAllTextAsync(path));
            doc.Root?.SetAttributeValue("fill", tint);
            using var stream = new InMemoryRandomAccessStream();
            await stream.WriteAsync(Encoding.UTF8.GetBytes(doc.ToString(SaveOptions.DisableFormatting)).AsBuffer());
            stream.Seek(0);
            var svg = new SvgImageSource();
            if (await svg.SetSourceAsync(stream) != SvgImageSourceLoadStatus.Success) return;
            Loaded[key] = svg;
            image.Source = svg;
        }
        catch (Exception error) when (error is IOException or COMException or UnauthorizedAccessException or XmlException) { }
    }
}

internal sealed class DisplayEditor
{
    private const string XamlNs = "http://schemas.microsoft.com/winfx/2006/xaml/presentation";

    private readonly IReadOnlyList<string> _icons = InstrumentIcons.Names();
    private readonly StackPanel _body = new() { Spacing = 8, Width = 460 };
    private readonly IReadOnlyList<(string Label, Action Then)> _extra;
    private readonly string _label;
    private ContentDialog? _dialog;
    private Action? _then;
    private ValueDisplay _display;

    private DisplayEditor(string label, ValueDisplay initial, IReadOnlyList<(string Label, Action Then)> extra)
    {
        _label = label;
        _display = initial;
        _extra = extra;
        Render();
    }

    private string FirstIcon => _icons.FirstOrDefault() ?? "";

    public static async Task<ValueDisplay?> Ask(XamlRoot root, string label, ValueDisplay initial, IReadOnlyList<(string Label, Action Then)> extra)
    {
        var editor = new DisplayEditor(label, initial, extra);
        editor._dialog = FormLook.Dialog(root, "Telemetry display", new ScrollViewer { Content = editor._body, MaxHeight = 520 }, "Done");
        var done = await FormLook.Show(editor._dialog) == ContentDialogResult.Primary;
        editor._then?.Invoke();
        return done ? editor._display : null;
    }

    private void Leave(Action then)
    {
        _then = then;
        _dialog?.Hide();
    }

    private void Change(ValueDisplay display)
    {
        _display = display;
        Render();
    }

    private void Render()
    {
        var d = _display;
        FormLook.Fill(_body, new UIElement?[]
            {
                FormLook.Caption(_label),
                _extra.Count > 0 ? FormLook.Fill(new VariableSizedWrapGrid { Orientation = Orientation.Horizontal }, _extra.Select(entry => (UIElement)FormLook.Button(entry.Label, () => Leave(entry.Then), ButtonLook.Subtle))) : null,
                Choice("Label", ["Icon", "Text"], d.ShowIcon ? 0 : 1, at => Change(at == 0
                    ? _display with { ShowIcon = true, Icon = _display.Icon.Length > 0 ? _display.Icon : FirstIcon }
                    : _display with { ShowIcon = false })),
                d.ShowIcon ? IconPicker("Icon", d.Icon, name => Change(_display with { Icon = name })) : TextEntry(d.Text, text => _display = _display with { Text = text }),
                FormLook.Row(FormLook.Text("Show units"), Toggle("Show units", d.ShowUnits, on => Change(_display with { ShowUnits = on }))),
                FormLook.Text("Value range", "FontSizeBody", "TextPrimaryBrush", true),
                FormLook.Caption("Change the color, opacity or icon when the value crosses a threshold"),
                Choice("Value range", Enum.GetNames<RangeType>(), (int)d.RangeType, at =>
                {
                    if ((RangeType)at != _display.RangeType) Change(InstrumentDisplays.WithRangeType(_display, (RangeType)at, FirstIcon));
                }),
            }
            .Concat(d.RangeType == RangeType.None ? [] : Ranges(d)));
    }

    private IEnumerable<UIElement?> Ranges(ValueDisplay d) =>
        new UIElement?[] { FormLook.Caption(InstrumentDisplays.RangeHelp(d.RangeType)) }
            .Concat(Enumerable.Range(0, d.Values.Count + 1).Select(row => RangeRow(d, row)))
            .Append(FormLook.Button("Add row", () => Change(InstrumentDisplays.WithRow(_display, FirstIcon)), ButtonLook.Subtle));

    private UIElement RangeRow(ValueDisplay d, int row)
    {
        UIElement limit = row < d.Values.Count
            ? Number($"Up to, row {row + 1}", d.Values[row], typed => _display = _display with { Values = InstrumentDisplays.Replaced(_display.Values, row, typed) })
            : FormLook.Caption("above");
        if (limit is FrameworkElement sized) sized.Width = 110;
        var remove = row < d.Values.Count && d.Values.Count > 1
            ? FormLook.IconButton("", $"Remove row {row + 1}", () => Change(InstrumentDisplays.WithoutRow(_display, row)))
            : null;
        return FormLook.Line(6, limit, Cell(d, row), remove);
    }

    private UIElement? Cell(ValueDisplay d, int row) => d.RangeType switch
    {
        RangeType.Color => FormLook.Line(4, InstrumentDisplays.RangeColours.Select(entry => Swatch(entry.Colour, entry.Name, d.Colours.ElementAtOrDefault(row) == entry.Colour,
            () => Change(_display with { Colours = InstrumentDisplays.Replaced(_display.Colours, row, entry.Colour) }))).ToArray()),
        RangeType.Opacity => Number($"Opacity, row {row + 1}", d.Opacities.ElementAtOrDefault(row), typed =>
            _display = _display with { Opacities = InstrumentDisplays.Replaced(_display.Opacities, row, Math.Clamp(typed, 0, 1)) }),
        RangeType.Icon => d.Icons.ElementAtOrDefault(row) is { } current
            ? IconPicker($"Icon, row {row + 1}", current, name => Change(_display with { Icons = InstrumentDisplays.Replaced(_display.Icons, row, name) }))
            : null,
        _ => null,
    };

    private static StackPanel Choice(string name, IReadOnlyList<string> options, int selected, Action<int> picked)
    {
        var group = $"{name}#{Guid.NewGuid():N}";
        var row = FormLook.Line(16, options.Select((option, at) =>
        {
            var radio = new RadioButton { Content = option, GroupName = group, IsChecked = at == selected, MinWidth = 0 };
            radio.Checked += (_, _) =>
            {
                if (at != selected) picked(at);
            };
            return (UIElement)radio;
        }).ToArray());
        AutomationProperties.SetName(row, name);
        return row;
    }

    private static ToggleSwitch Toggle(string name, bool on, Action<bool> changed)
    {
        var toggle = new ToggleSwitch { IsOn = on, OnContent = "", OffContent = "", MinWidth = 0 };
        AutomationProperties.SetName(toggle, name);
        toggle.Toggled += (_, _) =>
        {
            if (toggle.IsOn != on) changed(toggle.IsOn);
        };
        return toggle;
    }

    private static TextBox TextEntry(string text, Action<string> changed)
    {
        var box = FormLook.Field(text, "Text");
        AutomationProperties.SetName(box, "Text");
        box.TextChanged += (_, _) => changed(box.Text);
        return box;
    }

    private static NumberBox Number(string name, double value, Action<double> changed)
    {
        var box = new NumberBox { Value = value, SpinButtonPlacementMode = NumberBoxSpinButtonPlacementMode.Hidden, ValidationMode = NumberBoxValidationMode.InvalidInputOverwritten };
        AutomationProperties.SetName(box, name);
        box.ValueChanged += (_, args) =>
        {
            if (!double.IsNaN(args.NewValue)) changed(args.NewValue);
        };
        return box;
    }

    private static Button Swatch(long colour, string name, bool chosen, Action pick)
    {
        var button = new Button
        {
            Width = 28,
            Height = 28,
            Padding = new Thickness(0),
            CornerRadius = new CornerRadius(14),
            BorderThickness = new Thickness(chosen ? 3 : 1),
            BorderBrush = FormLook.Brush(chosen ? "AccentBrush" : "ControlStrokeBrush"),
            Background = colour == InstrumentDisplays.NoColour ? FormLook.Brush("ControlBrush") : new SolidColorBrush(InstrumentDisplays.ColorOf(colour)),
        };
        AutomationProperties.SetName(button, chosen ? $"{name}, chosen" : name);
        button.Click += (_, _) => pick();
        return button;
    }

    private UIElement IconPicker(string name, string current, Action<string> picked)
    {
        var tint = AssetImages.Hex("TextPrimaryBrush");
        var grid = new GridView
        {
            ItemsSource = _icons.ToList(),
            SelectionMode = ListViewSelectionMode.Single,
            SelectedItem = _icons.Contains(current) ? current : null,
            Width = 300,
            Height = 200,
            ItemTemplate = (DataTemplate)Microsoft.UI.Xaml.Markup.XamlReader.Load($"<DataTemplate xmlns='{XamlNs}'><Image Width='24' Height='24' Margin='4'/></DataTemplate>"),
        };
        AutomationProperties.SetName(grid, $"{name} choices");
        grid.ContainerContentChanging += (_, args) =>
        {
            if (args.InRecycleQueue || args.Item is not string icon || args.ItemContainer.ContentTemplateRoot is not Image image) return;
            InstrumentIcons.Load(image, icon, tint);
            AutomationProperties.SetName(args.ItemContainer, Path.GetFileNameWithoutExtension(icon));
            args.Handled = true;
        };
        grid.SelectionChanged += (_, _) =>
        {
            if (grid.SelectedItem is string chosen && chosen != current) picked(chosen);
        };
        var shown = current.Length > 0 ? InstrumentIcons.Image(current, tint, 24) : null;
        var expander = new Expander
        {
            Header = FormLook.Line(8, shown, FormLook.Text("Choose icon")),
            Content = grid,
            HorizontalAlignment = HorizontalAlignment.Stretch,
            HorizontalContentAlignment = HorizontalAlignment.Stretch,
        };
        AutomationProperties.SetName(expander, name);
        return expander;
    }
}
