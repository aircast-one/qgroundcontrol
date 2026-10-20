using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Windows.System;

namespace AircastQGC.Pages.Plan;

internal static class PlanFacts
{
    private const double ValueWidth = 150;

    internal static TextBlock Text(string text, string brush = "TextPrimaryBrush", string size = "FontSizeBody", bool strong = false) => new()
    {
        Text = text,
        TextWrapping = TextWrapping.Wrap,
        FontFamily = (FontFamily)Application.Current.Resources["AppFontFamily"],
        FontSize = (double)Application.Current.Resources[size],
        FontWeight = strong ? FontWeights.SemiBold : FontWeights.Normal,
        Foreground = (Brush)Application.Current.Resources[brush],
    };

    internal static TextBlock Note(string text) => Text(text, "TextSecondaryBrush", "FontSizeCaption");

    internal static TextBlock Warning(string text) => Text(text, "CautionBrush", "FontSizeCaption");

    internal static TextBlock Error(string text) => Text(text, "CriticalBrush", "FontSizeCaption");

    internal static TextBlock Heading(string text) => Text(text, "TextPrimaryBrush", "FontSizeBody", true);

    internal static Grid Pair(FrameworkElement label, FrameworkElement value)
    {
        var grid = new Grid
        {
            ColumnSpacing = 12,
            ColumnDefinitions = { new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) }, new ColumnDefinition { Width = GridLength.Auto } },
        };
        label.VerticalAlignment = VerticalAlignment.Center;
        value.VerticalAlignment = VerticalAlignment.Center;
        if (value is Control control && AutomationProperties.GetName(control).Length == 0 && Caption(label) is { Length: > 0 } caption)
            AutomationProperties.SetName(control, caption);
        Grid.SetColumn(value, 1);
        grid.Children.Add(label);
        grid.Children.Add(value);
        return grid;
    }

    private static string? Caption(FrameworkElement label) => label switch
    {
        TextBlock text => text.Text,
        Panel panel when panel.Children.FirstOrDefault() is TextBlock first => first.Text,
        _ => null,
    };

    internal static Button Action(string label, Action click, bool critical = false)
    {
        var button = new Button { Content = label };
        if (critical) button.Foreground = (Brush)Application.Current.Resources["CriticalBrush"];
        button.Click += (_, _) => click();
        return button;
    }

    internal static StackPanel Row(params UIElement[] children)
    {
        var row = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8 };
        children.ToList().ForEach(row.Children.Add);
        return row;
    }

    internal static TextBox Entry(string text, Action<string> commit, double width = ValueWidth)
    {
        var box = new TextBox { Text = text, Width = width };
        var shown = text;
        void Commit()
        {
            if (box.Text == shown) return;
            shown = box.Text;
            commit(box.Text);
        }
        box.KeyDown += (_, e) =>
        {
            if (e.Key != VirtualKey.Enter) return;
            e.Handled = true;
            Commit();
        };
        box.LostFocus += (_, _) => Commit();
        return box;
    }

    internal static FrameworkElement Row(JsonObject control, Func<string, JsonNode?, string?> write, Action written)
    {
        var path = MapUiJson.Text(control["path"]);
        var kind = MapUiJson.Text(control["control"]);
        var units = MapUiJson.Text(control["units"]);
        var label = MapUiJson.SentenceCase(Title(control));
        var enabled = MapUiJson.Flag(control["enabled"], true) && !MapUiJson.Flag(control["readOnly"]);
        var optional = MapUiJson.Flag(control["optional"]);
        var set = control["value"] != null && MapUiJson.Text(control["valueString"]).Length > 0;
        var refusal = Error(string.Empty);
        refusal.Visibility = Visibility.Collapsed;

        void Write(string target, JsonNode? value)
        {
            var refused = write(target, value);
            refusal.Text = refused ?? string.Empty;
            refusal.Visibility = refused == null ? Visibility.Collapsed : Visibility.Visible;
            if (refused == null) written();
        }

        var editor = Editor(control, kind, path, units, enabled && (!optional || set), Write);
        var title = new StackPanel { Spacing = 2, Children = { Text(label) } };
        if (MapUiJson.Text(control["disabledReason"]) is { Length: > 0 } reason && !enabled) title.Children.Add(Note(reason));
        ToolTipService.SetToolTip(title, MapUiJson.Text(control["longDescription"]) is { Length: > 0 } help ? help : null);
        var row = kind == "bitmask" ? (FrameworkElement)new StackPanel { Spacing = 4, Children = { title, editor } } : Pair(title, editor);
        var stacked = new StackPanel { Spacing = 2, Children = { row, refusal } };
        if (!optional) return stacked;
        var toggle = new ToggleSwitch { IsOn = set, OnContent = string.Empty, OffContent = string.Empty, MinWidth = 0 };
        toggle.Toggled += (_, _) => Write(path, toggle.IsOn ? 0.0 : null);
        return Pair(stacked, toggle);
    }

    internal static string Title(JsonObject control) =>
        MapUiJson.Text(control["label"]) is { Length: > 0 } label ? label : MapUiJson.Text(control["name"]);

    private static FrameworkElement Editor(JsonObject control, string kind, string path, string units, bool enabled, Action<string, JsonNode?> write)
    {
        if (MapUiJson.Flag(control["readOnly"]))
            return Text(string.Join(' ', new[] { MapUiJson.Text(control["display"]), kind == "choice" ? string.Empty : units }.Where(t => t.Length > 0)), "TextSecondaryBrush");
        return kind switch
        {
            "toggle" => Toggle(control, path, enabled, write),
            "choice" => Choice(control, path, enabled, write),
            "bitmask" => Bits(control, path, enabled, write),
            _ => Typed(control, kind, path, units, enabled, write),
        };
    }

    private static FrameworkElement Toggle(JsonObject control, string path, bool enabled, Action<string, JsonNode?> write)
    {
        var toggle = new ToggleSwitch
        {
            IsOn = MapUiJson.Flag(control["value"]) || MapUiJson.Number(control["value"]) is > 0,
            IsEnabled = enabled,
            OnContent = string.Empty,
            OffContent = string.Empty,
            MinWidth = 0,
        };
        toggle.Toggled += (_, _) => write(path, toggle.IsOn);
        return toggle;
    }

    private static FrameworkElement Choice(JsonObject control, string path, bool enabled, Action<string, JsonNode?> write)
    {
        var options = MapUiJson.Objects(control["options"]);
        var labels = options.Select(option => MapUiJson.Text(option["label"])).ToList();
        var box = new ComboBox { Width = ValueWidth + 40, IsEnabled = enabled };
        labels.ForEach(text => box.Items.Add(MapUiJson.SentenceCase(text)));
        box.SelectedIndex = labels.IndexOf(MapUiJson.Text(control["display"]));
        var raw = MapUiJson.Flag(control["rawChoice"]);
        box.SelectionChanged += (_, _) =>
        {
            var at = box.SelectedIndex;
            if (at < 0) return;
            if (raw && long.TryParse(MapUiJson.Text(options[at]["raw"]), NumberStyles.Integer, CultureInfo.InvariantCulture, out var value)) write(path, value);
            else write($"{path}.enumIndex", at);
        };
        return box;
    }

    private static FrameworkElement Bits(JsonObject control, string path, bool enabled, Action<string, JsonNode?> write)
    {
        var bits = MapUiJson.Objects(control["bits"])
            .Select(bit => (Label: MapUiJson.Text(bit["label"]), Raw: long.TryParse(MapUiJson.Text(bit["raw"]), NumberStyles.Integer, CultureInfo.InvariantCulture, out var raw) ? raw : 0, Set: MapUiJson.Flag(bit["set"])))
            .Where(bit => bit.Raw != 0)
            .ToList();
        var boxes = bits.Select(bit => new CheckBox { Content = bit.Label, IsChecked = bit.Set, IsEnabled = enabled }).ToList();
        var others = (long)(MapUiJson.Number(control["value"]) ?? 0) & ~bits.Aggregate(0L, (mask, bit) => mask | bit.Raw);
        long Mask() => boxes.Zip(bits).Where(pair => pair.First.IsChecked == true).Aggregate(others, (mask, pair) => mask | pair.Second.Raw);
        boxes.ForEach(box => box.Click += (_, _) => write(path, Mask()));
        var panel = new StackPanel();
        boxes.ForEach(panel.Children.Add);
        return panel;
    }

    private static FrameworkElement Typed(JsonObject control, string kind, string path, string units, bool enabled, Action<string, JsonNode?> write)
    {
        var box = Entry(MapUiJson.Text(control["valueString"]), typed =>
        {
            var trimmed = typed.Trim();
            JsonNode? value = kind == "number" && PlanRules.TypedNumber(trimmed) is double number ? number : trimmed;
            write(path, value);
        }, kind == "text" ? ValueWidth + 60 : ValueWidth - 40);
        box.IsEnabled = enabled;
        return units.Length == 0 || kind == "text" ? box : Row(box, Note(units));
    }

    internal static FrameworkElement Stepper(string label, double? value, string units, double step, string? note, Action<double> set, Button? trailing = null)
    {
        var shown = value is double current ? current.ToString("0.##", CultureInfo.InvariantCulture) : string.Empty;
        var box = Entry(shown, typed =>
        {
            if (PlanRules.TypedNumber(typed) is double wanted) set(wanted);
        }, 80);
        var minus = new Button { Content = "−", MinWidth = 32, IsEnabled = value != null };
        var plus = new Button { Content = "+", MinWidth = 32, IsEnabled = value != null };
        minus.Click += (_, _) => set(Math.Max(0, (value ?? 0) - step));
        plus.Click += (_, _) => set((value ?? 0) + step);
        var values = Row(minus, box, Note(units), plus);
        if (trailing != null) values.Children.Add(trailing);
        var title = new StackPanel { Spacing = 2, Children = { Text(label) } };
        if (!string.IsNullOrEmpty(note)) title.Children.Add(Note(note));
        return new StackPanel { Spacing = 4, Children = { title, values } };
    }

    internal static IReadOnlyList<JsonObject> Fields(JsonObject view) => MapUiJson.Objects(view["fields"]);
}
