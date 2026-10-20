using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.WindowsRuntime;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Media.Imaging;
using Windows.Storage.Streams;

namespace AircastQGC.Pages.Setup;

internal static class AirframeImages
{
    public static string Folder => Path.Combine(AppContext.BaseDirectory, "Assets", "Airframe");

    public static Image Of(string file, double height)
    {
        var image = new Image { Height = height, Stretch = Microsoft.UI.Xaml.Media.Stretch.Uniform, Visibility = Visibility.Collapsed };
        Load(image, file);
        return image;
    }

    private static async void Load(Image image, string file)
    {
        var path = Path.Combine(Folder, Path.GetFileName(file.EndsWith(".svg", StringComparison.OrdinalIgnoreCase) ? file : $"{file}.svg"));
        if (file.Trim().Length == 0 || !File.Exists(path)) return;
        try
        {
            var bytes = await File.ReadAllBytesAsync(path);
            using var stream = new InMemoryRandomAccessStream();
            await stream.WriteAsync(bytes.AsBuffer());
            stream.Seek(0);
            var svg = new SvgImageSource();
            if (await svg.SetSourceAsync(stream) != SvgImageSourceLoadStatus.Success) return;
            image.Source = svg;
            image.Visibility = Visibility.Visible;
        }
        catch (Exception error) when (error is IOException or COMException or UnauthorizedAccessException) { }
    }
}

internal sealed class ApmAirframePanel : UserControl
{
    private const string View = "view.apmAirframe";
    private const string PickClass = "apmAirframe.pickClass";
    private const string PickType = "apmAirframe.pickType";
    private static readonly TimeSpan Poll = TimeSpan.FromSeconds(1);

    private readonly Poller _poller;
    private string? _refusal;
    private JsonObject _view = new();

    public ApmAirframePanel()
    {
        _poller = Live.Changes(this, Poll, () => Bridge.Get(View), view =>
        {
            _view = view;
            Render();
        });
    }

    private void Render()
    {
        if (!MapUiJson.Flag(_view["available"]))
        {
            Content = SetupUi.Notice("This page is for an ArduPilot vehicle with a FRAME_CLASS parameter.");
            return;
        }
        var classes = MapUiJson.Objects(_view["classes"]);
        var frameType = MapUiJson.Number(_view["frameType"]) is { } type ? (int)type : (int?)null;
        var chosen = classes.FirstOrDefault(c => MapUiJson.Flag(c["chosen"]) && MapUiJson.Flag(c["valid"], true) && MapUiJson.Objects(c["types"]).Count > 0);
        var invalid = MapUiJson.Text(_view["invalidText"]);
        Content = SetupUi.Page(
            new Border
            {
                Background = FormLook.Brush("CardBrush"),
                CornerRadius = FormLook.Radius("RadiusOverlay"),
                Padding = new Thickness(16),
                Margin = new Thickness(0, 4, 0, 0),
                Child = FormLook.Text(MapUiJson.Text(_view["help"])),
            },
            _refusal is null ? null : FormLook.Text(_refusal, "FontSizeBody", "CriticalBrush"),
            chosen is null ? null : FormLook.Stack(4,
                FormLook.Caption("Frame type"),
                FormLook.Line(8, MapUiJson.Objects(chosen["types"]).Select(t =>
                {
                    var value = (int)(MapUiJson.Number(t["value"]) ?? 0);
                    var chip = new ToggleButton { Content = MapUiJson.Text(t["name"]), IsChecked = value == frameType };
                    chip.Click += (_, _) =>
                    {
                        if (value != frameType) Act(PickType, value);
                        else chip.IsChecked = true;
                    };
                    return (UIElement?)chip;
                }).ToArray())),
            FormLook.Fill(new VariableSizedWrapGrid { Orientation = Orientation.Horizontal, ItemWidth = 132, ItemHeight = 148 },
                classes.Select(card => Tile(card, invalid))));
    }

    private UIElement Tile(JsonObject card, string invalidText)
    {
        var chosen = MapUiJson.Flag(card["chosen"]);
        var valid = MapUiJson.Flag(card["valid"], true);
        var name = MapUiJson.Text(card["name"]);
        var value = (int)(MapUiJson.Number(card["value"]) ?? 0);
        var image = AirframeImages.Of(MapUiJson.Text(card["image"]), 88);
        var label = FormLook.Text(name, "FontSizeBody", "TextPrimaryBrush", true);
        label.HorizontalAlignment = HorizontalAlignment.Center;
        var body = FormLook.Stack(4, image, label);
        body.Opacity = valid ? 1 : 0.5;
        var overlay = valid ? null : FormLook.Text(invalidText, "FontSizeCaption", "CriticalBrush");
        if (overlay is not null)
        {
            overlay.HorizontalAlignment = HorizontalAlignment.Center;
            overlay.VerticalAlignment = VerticalAlignment.Center;
            overlay.TextAlignment = TextAlignment.Center;
        }
        var button = new Button
        {
            Width = 124,
            Height = 140,
            Margin = new Thickness(0, 0, 8, 8),
            Padding = new Thickness(8),
            CornerRadius = FormLook.Radius("RadiusOverlay"),
            BorderThickness = new Thickness(chosen ? 2 : 1),
            IsEnabled = !chosen || !valid,
            Content = new Grid { Children = { body, (UIElement?)overlay ?? new Border() } },
        };
        FormLook.Paint(button, ButtonLook.Standard);
        if (chosen) button.BorderBrush = FormLook.Brush("AccentBrush");
        AutomationProperties.SetName(button, name);
        button.Click += (_, _) => Act(PickClass, value);
        return button;
    }

    private void Act(string path, int value) => SetupUi.Run(() => SetupUi.Refusal(path, value), refused =>
    {
        _refusal = refused;
        _poller.Now();
        Render();
    });
}

internal sealed class Px4AirframePanel : UserControl
{
    private const string View = "view.px4Airframe";
    private const string Apply = "px4Airframe.apply";
    private const string Reset = "px4Airframe.reset";

    private JsonObject _view = new();
    private (string Group, int Index)? _selection;
    private long? _autostart;
    private string? _refusal;

    public Px4AirframePanel() => Live.Watch(this, View, view =>
    {
        _view = view;
        var autostart = (long?)MapUiJson.Number(view["autostartId"]);
        if (autostart != _autostart)
        {
            _autostart = autostart;
            _selection = MapUiJson.Text(view["currentType"]) is { Length: > 0 } type ? (type, (int)(MapUiJson.Number(view["currentIndex"]) ?? 0)) : null;
        }
        Render();
    });

    private void Render()
    {
        if (!MapUiJson.Flag(_view["available"]))
        {
            Content = SetupUi.Notice("This vehicle has no SYS_AUTOSTART and SYS_AUTOCONFIG.");
            return;
        }
        var refusal = _refusal is null ? null : FormLook.Text(_refusal, "FontSizeBody", "CriticalBrush");
        if (MapUiJson.Flag(_view["custom"]))
        {
            var reset = FormLook.Button("Reset", () => Act(Reset), ButtonLook.Accent);
            reset.HorizontalAlignment = HorizontalAlignment.Center;
            Content = SetupUi.Page(FormLook.Text(MapUiJson.Text(_view["customText"])), reset, refusal);
            return;
        }
        var groups = MapUiJson.Objects(_view["types"]);
        var chosen = _selection is { } picked
            ? MapUiJson.Objects(groups.FirstOrDefault(g => MapUiJson.Text(g["name"]) == picked.Group)?["airframes"]).ElementAtOrDefault(picked.Index)
            : null;
        var applyTitle = MapUiJson.Text(_view["applyTitle"]);
        var apply = FormLook.Button(applyTitle, () => Confirm(chosen), ButtonLook.Accent);
        apply.IsEnabled = chosen is not null;
        var heading = new Grid { ColumnSpacing = 12, Margin = new Thickness(0, 4, 0, 0), ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto } } };
        heading.Children.Add(FormLook.Text(MapUiJson.Text(_view["heading"])));
        Grid.SetColumn(apply, 1);
        heading.Children.Add(apply);
        Content = SetupUi.Page(new UIElement?[] { heading, refusal }.Concat(groups.Select(Group)).ToArray());
    }

    private UIElement Group(JsonObject group)
    {
        var name = MapUiJson.Text(group["name"]);
        var frames = MapUiJson.Objects(group["airframes"]).Select(f => MapUiJson.Text(f["name"])).ToList();
        var picked = _selection?.Group == name;
        var index = picked ? _selection?.Index ?? 0 : 0;
        var radio = new RadioButton { IsChecked = picked, GroupName = "px4Airframe", MinWidth = 0 };
        AutomationProperties.SetName(radio, name);
        radio.Checked += (_, _) => Pick(name, index);
        var combo = FormLook.Combo(frames, Math.Min(index, frames.Count - 1), 260);
        AutomationProperties.SetName(combo, $"{name} airframe");
        combo.SelectionChanged += (_, _) =>
        {
            if (combo.SelectedIndex >= 0) Pick(name, combo.SelectedIndex);
        };
        var grid = new Grid
        {
            ColumnSpacing = 12,
            Padding = new Thickness(12),
            ColumnDefinitions = { new ColumnDefinition { Width = GridLength.Auto }, new ColumnDefinition() },
        };
        grid.Children.Add(radio);
        var right = FormLook.Stack(4,
            FormLook.Text(name, "FontSizeBody", "TextPrimaryBrush", true),
            MapUiJson.Text(group["image"]) is { Length: > 0 } image ? AirframeImages.Of(image, 96) : null,
            combo);
        Grid.SetColumn(right, 1);
        grid.Children.Add(right);
        return new Border
        {
            Child = grid,
            CornerRadius = FormLook.Radius("RadiusControl"),
            BorderThickness = new Thickness(1),
            BorderBrush = FormLook.Brush(picked ? "AccentBrush" : "CardStrokeBrush"),
            Background = FormLook.Brush(picked ? "SubtleSelectedBrush" : "CardBrush"),
        };
    }

    private void Pick(string group, int index)
    {
        if (_selection == (group, index)) return;
        _selection = (group, index);
        Render();
    }

    private async void Confirm(JsonObject? chosen)
    {
        if (chosen is null) return;
        var id = (long)(MapUiJson.Number(chosen["autostartId"]) ?? 0);
        if (!await FormLook.Confirm(XamlRoot, MapUiJson.Text(_view["applyTitle"]), MapUiJson.Text(_view["applyText"]).Replace("<br>", "\n"), "Apply")) return;
        Act(Apply, id);
    }

    private void Act(string path, params object?[] args) => SetupUi.Run(() => SetupUi.Refusal(path, args), refused =>
    {
        _refusal = refused;
        Render();
    });
}

internal sealed class SubFramePanel : UserControl
{
    private const string View = "view.apmSubFrame";
    private const string Set = "apmSubFrame.set";
    private const string LoadDefaults = "apmSubFrame.loadDefaults";
    private static readonly TimeSpan Poll = TimeSpan.FromSeconds(1);

    private readonly Dictionary<int, BitmapImage?> _images = [];
    private string? _refusal;
    private JsonObject _view = new();

    public SubFramePanel() => _ = Live.Changes(this, Poll, () => Bridge.Get(View), view =>
    {
        _view = view;
        Render();
    });

    private void Render()
    {
        if (!MapUiJson.Flag(_view["available"]))
        {
            Content = SetupUi.Notice("This page is for an ArduSub vehicle.");
            return;
        }
        var selected = MapUiJson.Number(_view["selected"]) is { } value ? (int)value : (int?)null;
        var problem = _refusal ?? (MapUiJson.Text(_view["loadError"]) is { Length: > 0 } error ? error : null);
        Content = SetupUi.Page(
            MapUiJson.Flag(_view["loadingDefaults"]) ? FormLook.Caption("Loading the frame's default parameters…") : null,
            problem is null ? null : FormLook.Text(problem, "FontSizeBody", "CriticalBrush"),
            FormLook.Fill(new VariableSizedWrapGrid { Orientation = Orientation.Horizontal, ItemWidth = 176, ItemHeight = 176 },
                MapUiJson.Objects(_view["frames"]).Select(frame => Tile(frame, selected))));
    }

    private UIElement Tile(JsonObject frame, int? selected)
    {
        var value = (int)(MapUiJson.Number(frame["value"]) ?? 0);
        var name = MapUiJson.Text(frame["name"]);
        var image = new Image { Height = 120, Stretch = Microsoft.UI.Xaml.Media.Stretch.Uniform };
        Picture(image, value);
        var button = new Button
        {
            Width = 168,
            Height = 168,
            Margin = new Thickness(0, 0, 8, 8),
            Padding = new Thickness(8),
            VerticalContentAlignment = VerticalAlignment.Top,
            HorizontalContentAlignment = HorizontalAlignment.Stretch,
            Content = FormLook.Stack(4, FormLook.Text(name, "FontSizeBody", "TextPrimaryBrush", true), image),
        };
        FormLook.Paint(button, value == selected ? ButtonLook.Accent : ButtonLook.Standard);
        AutomationProperties.SetName(button, name);
        button.Click += (_, _) => Pick(frame);
        return button;
    }

    private async void Picture(Image image, int value)
    {
        if (!_images.TryGetValue(value, out var cached))
        {
            var png = await Task.Run(() => MapUiJson.Text(Bridge.Get($"view.apmSubFrameImage({value})")["png"]));
            cached = await Images.Png(png);
            _images[value] = cached;
        }
        image.Source = cached;
    }

    private async void Pick(JsonObject frame)
    {
        var value = (int)(MapUiJson.Number(frame["value"]) ?? 0);
        var name = MapUiJson.Text(frame["name"]);
        if (!MapUiJson.Flag(_view["confirmFirst"]))
        {
            Act(Set, value);
            return;
        }
        var defaults = MapUiJson.Flag(frame["hasDefaults"]);
        var dialog = FormLook.Dialog(XamlRoot, "Frame selection",
            FormLook.Text(defaults ? "Would you like to load the default parameters for the frame?" : "Would you like to set the desired frame?"),
            defaults ? $"Yes, Load default parameter set for {name}" : $"Confirm frame {name}", "Close");
        dialog.SecondaryButtonText = defaults ? "No, set frame only" : "";
        var answer = await FormLook.Show(dialog);
        if (answer == ContentDialogResult.Primary) Act(defaults ? LoadDefaults : Set, value);
        if (answer == ContentDialogResult.Secondary) Act(Set, value);
    }

    private void Act(string path, int value) => SetupUi.Run(() => SetupUi.Refusal(path, value), refused =>
    {
        _refusal = refused;
        Render();
    });
}
