using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Shapes;

namespace AircastQGC.Pages.Setup;

internal sealed record RadioPrompt(string Title, string Body, string Action, IReadOnlyList<string> Choices);

internal sealed record MeterLine(string Label, string Reading, double Fraction, bool Shown, bool Warn);

internal static class StickDiagram
{
    private const double Box = 72;

    public static UIElement Of(IReadOnlyList<double> positions, bool single, IReadOnlyList<double>? live = null) =>
        FormLook.Line(24, new[] { 0, 2 }.Take(single ? 1 : 2).Select(at => (UIElement?)Stick(positions, live, at)).ToArray());

    private static Canvas Stick(IReadOnlyList<double> positions, IReadOnlyList<double>? live, int at)
    {
        var canvas = new Canvas { Width = Box, Height = Box };
        canvas.Children.Add(new Rectangle { Width = Box, Height = Box, Stroke = FormLook.Brush("ControlStrokeBrush"), StrokeThickness = 1 });
        canvas.Children.Add(Knob(positions, at, live is null ? Box * 0.18 : Box * 0.24, live is not null));
        if (live is not null) canvas.Children.Add(Knob(live, at, Box * 0.14, false));
        return canvas;
    }

    private static Ellipse Knob(IReadOnlyList<double> values, int at, double radius, bool ring)
    {
        var half = Box / 2;
        var x = half + Value(values, at) * half * 0.75;
        var y = half - Value(values, at + 1) * half * 0.75;
        var knob = new Ellipse
        {
            Width = radius * 2,
            Height = radius * 2,
            Fill = ring ? null : FormLook.Brush("AccentBrush"),
            Stroke = ring ? FormLook.Brush("AccentBrush") : null,
            StrokeThickness = ring ? 2 : 0,
            StrokeDashArray = ring ? new DoubleCollection { 2, 1.5 } : null,
        };
        Canvas.SetLeft(knob, x - radius);
        Canvas.SetTop(knob, y - radius);
        return knob;
    }

    private static double Value(IReadOnlyList<double> values, int at) => at < values.Count ? values[at] : 0;
}

internal sealed class Meters : UserControl
{
    private readonly bool _mono;
    private IReadOnlyList<(TextBlock Label, TextBlock Reading, ProgressBar Bar)> _rows = [];
    private string _shape = "";

    public Meters(bool mono) => _mono = mono;

    public void Show(IReadOnlyList<MeterLine> lines)
    {
        var shape = string.Join("|", lines.Select(l => l.Label));
        if (shape != _shape)
        {
            _shape = shape;
            _rows = lines.Select(_ => (Label: Text(), Reading: Text(), Bar: SetupUi.Bar(0))).ToList();
            Content = FormLook.Card(_rows.Select(row =>
            {
                var grid = new Grid { Padding = new Thickness(16, 8, 16, 8), ColumnSpacing = 12 };
                new[] { _mono ? new GridLength(32) : new GridLength(120), new GridLength(1, GridUnitType.Star), _mono ? new GridLength(56) : GridLength.Auto }
                    .ToList()
                    .ForEach(width => grid.ColumnDefinitions.Add(new ColumnDefinition { Width = width }));
                row.Label.VerticalAlignment = row.Reading.VerticalAlignment = VerticalAlignment.Center;
                Grid.SetColumn(row.Bar, 1);
                Grid.SetColumn(row.Reading, 2);
                grid.Children.Add(row.Label);
                grid.Children.Add(row.Bar);
                grid.Children.Add(row.Reading);
                return (UIElement)grid;
            }));
            Visibility = FormLook.Shown(lines.Count > 0);
        }
        lines.Zip(_rows).ToList().ForEach(pair =>
        {
            pair.Second.Label.Text = pair.First.Label;
            pair.Second.Reading.Text = pair.First.Reading;
            pair.Second.Reading.Foreground = FormLook.Brush(pair.First.Warn ? "CriticalBrush" : "TextSecondaryBrush");
            pair.Second.Bar.Value = Math.Clamp(pair.First.Fraction, 0, 1);
            pair.Second.Bar.Visibility = FormLook.Shown(pair.First.Shown);
        });
    }

    private TextBlock Text()
    {
        var block = FormLook.Text("", _mono ? "FontSizeCaption" : "FontSizeBody", "TextPrimaryBrush");
        block.Visibility = Visibility.Visible;
        if (_mono) block.FontFamily = SetupUi.Mono;
        return block;
    }
}

internal sealed class RadioPanel : UserControl
{
    public const string View = "view.radio";
    private const string Cal = "radioCal";
    private const string SwitchesPage = "Radio Switches";
    private const string ReversedTitle = "Throttle channel reversed";
    private const string ReversedText = "Calibration failed. The throttle channel on your transmitter is reversed. You must correct this on your transmitter in order to complete calibration.";
    private const string Footnote = "Move each stick and switch — every channel you use should move here. Calibration asks you to hold each stick at its extremes in turn and rewrites the mapping, so do it with the propellers off.";
    private static readonly string[] EnterActions = ["start", "open"];
    private static readonly string[] LeaveActions = ["cancelButtonClicked", "close"];

    public static readonly IReadOnlyList<RadioPrompt> Prompts =
    [
        new("Spektrum bind", "Click Ok to place your Spektrum receiver in the bind mode.\n\nSelect the specific receiver type below:", "spektrumBindMode", ["DSM2 Mode", "DSMX (7 channels or less)", "DSMX (8 channels or more)"]),
        new("CRSF bind", "Click Ok to place your CRSF receiver in the bind mode.", "crsfBindMode", []),
        new("Copy trims", "Center your sticks and move throttle all the way down, then press Ok to copy trims. After pressing Ok, reset the trims on your radio back to zero.", "copyTrims", []),
    ];

    private readonly ContentControl _calibration = new() { HorizontalContentAlignment = HorizontalAlignment.Stretch, IsTabStop = false };
    private readonly TextBlock _noChannels = SetupUi.Notice("No transmitter signal. Turn the transmitter on and check the receiver is bound.");
    private readonly Meters _sticks = new(false);
    private readonly Meters _channels = new(true);
    private readonly FormView _switches = SetupForms.Plain(SwitchesPage);
    private readonly UIElement _layout;
    private string _calibrationKey = "";
    private bool? _connected;
    private bool _reversedSeen;

    public RadioPanel()
    {
        _layout = SetupUi.Page(
            _noChannels,
            _calibration,
            SetupUi.Heading("Attitude controls"),
            _sticks,
            _switches,
            SetupUi.Heading("Raw channel monitor"),
            _channels,
            FormLook.Footnote(Footnote));
        Loaded += (_, _) => SetupUi.InOrder(() => EnterActions.ToList().ForEach(action => FormWrites.Ok($"{Cal}.{action}")));
        Unloaded += (_, _) => SetupUi.InOrder(() => LeaveActions.ToList().ForEach(action => FormWrites.Ok($"{Cal}.{action}")));
        Live.Watch(this, View, Apply);
    }

    public static IReadOnlyList<MeterLine> Channels(JsonObject view) =>
        MapUiJson.Objects(view["channels"]).Select(c => new MeterLine(
            MapUiJson.Text(c["label"]), MapUiJson.Text(c["valueText"]), MapUiJson.Number(c["fraction"]) ?? 0, true, false)).ToList();

    public static string Step(string status)
    {
        var lines = status.Trim().Split('\n').ToList();
        var kept = lines.Take(lines.Count - lines.AsEnumerable().Reverse().TakeWhile(line => line.Trim().Length == 0 || line.Trim().StartsWith("Click ", StringComparison.Ordinal)).Count());
        return string.Join("\n", kept).Trim();
    }

    private void Apply(JsonObject view)
    {
        var connected = MapUiJson.Text(view["class"]) == "Radio" && MapUiJson.Flag(view["connected"]);
        if (connected != _connected)
        {
            _connected = connected;
            _calibrationKey = "";
            Content = connected ? _layout : SetupUi.Notice("Connect a vehicle to check its radio.");
            if (connected) _switches.Reload();
        }
        if (!connected) return;
        _noChannels.Visibility = FormLook.Shown((MapUiJson.Number(view["channelCount"]) ?? 0) == 0);
        RenderCalibration(view);
        _sticks.Show(MapUiJson.Objects(view["sticks"]).Select(s => new MeterLine(
            MapUiJson.Text(s["title"]),
            Reading(s),
            MapUiJson.Number(s["fraction"]) ?? 0,
            MapUiJson.Flag(s["mapped"]),
            !MapUiJson.Flag(s["mapped"]))).ToList());
        _channels.Show(Channels(view));
        var reversed = MapUiJson.Flag(view["throttleReversed"]);
        if (!reversed) _reversedSeen = false;
        if (reversed && !_reversedSeen)
        {
            _reversedSeen = true;
            _ = FormLook.Tell(XamlRoot, ReversedTitle, ReversedText);
        }
    }

    private static string Reading(JsonObject stick) =>
        !MapUiJson.Flag(stick["mapped"]) ? "Not mapped"
        : string.Join(" · ", new[]
        {
            MapUiJson.Number(stick["channel"]) is { } channel && channel > 0 ? $"Ch {channel}" : "",
            MapUiJson.Text(stick["valueText"]) is { Length: > 0 } value ? $"{value} µs" : "",
            MapUiJson.Flag(stick["reversed"]) ? "reversed" : "",
        }.Where(part => part.Length > 0));

    private void RenderCalibration(JsonObject view)
    {
        var fields = new[] { "calibrating", "statusText", "nextText", "nextEnabled", "skipEnabled", "cancelEnabled", "transmitterMode", "centeredThrottle", "joystickMode", "summary", "startPrompt", "notReady", "stickPositions" };
        var key = string.Join("|", fields.Select(f => view[f]?.ToJsonString() ?? ""));
        if (key == _calibrationKey) return;
        _calibrationKey = key;
        _calibration.Content = MapUiJson.Flag(view["calibrating"]) ? Running(view) : Start(view);
    }

    private static UIElement Running(JsonObject view)
    {
        var positions = (view["stickPositions"] as JsonArray ?? []).Select(n => MapUiJson.Number(n) ?? 0).ToList();
        var next = Action(MapUiJson.Text(view["nextText"]) is { Length: > 0 } text ? text : "Next", "nextButtonClicked", ButtonLook.Accent, MapUiJson.Flag(view["nextEnabled"]));
        var skip = MapUiJson.Flag(view["skipEnabled"]) ? Action("Skip", "skipButtonClicked", ButtonLook.Standard, true) : null;
        var cancel = MapUiJson.Flag(view["cancelEnabled"]) ? Action("Cancel", "cancelButtonClicked", ButtonLook.Subtle, true) : null;
        return new Border
        {
            Margin = new Thickness(0, 16, 0, 0),
            Padding = new Thickness(16),
            CornerRadius = FormLook.Radius("RadiusOverlay"),
            Background = FormLook.Brush("InfoBgBrush"),
            Child = FormLook.Stack(16,
                FormLook.Text(Step(MapUiJson.Text(view["statusText"])), "FontSizeBodyLarge", "TextPrimaryBrush", true),
                positions.Count == 4 ? StickDiagram.Of(positions, false) : null,
                FormLook.Line(8, next, skip, cancel)),
        };
    }

    private UIElement Start(JsonObject view)
    {
        var mode = (int)(MapUiJson.Number(view["transmitterMode"]) ?? 2);
        var modes = FormLook.Line(8, Enumerable.Range(1, 4).Select(choice =>
        {
            var toggle = new ToggleButton { Content = $"Mode {choice}", IsChecked = choice == mode };
            toggle.Click += (_, _) => SetupUi.Detached(() => FormWrites.Set($"{Cal}.transmitterMode", choice));
            return (UIElement?)toggle;
        }).ToArray());
        var centered = MapUiJson.Flag(view["joystickMode"]) ? null
            : FormLook.Check("Centered throttle", MapUiJson.Flag(view["centeredThrottle"]), on => SetupUi.Detached(() => FormWrites.Set($"{Cal}.centeredThrottle", on)));
        var calibrate = FormLook.Button(MapUiJson.Text(view["nextText"]) is { Length: > 0 } text ? text : "Calibrate", () => Calibrate(view), ButtonLook.Accent);
        calibrate.IsEnabled = MapUiJson.Flag(view["nextEnabled"]);
        var summary = FormLook.Caption(MapUiJson.Text(view["summary"]));
        summary.VerticalAlignment = VerticalAlignment.Center;
        return FormLook.Margined(FormLook.Stack(12,
            FormLook.Caption("Transmitter mode — which stick is the throttle"),
            modes,
            centered,
            FormLook.Line(12, calibrate, summary),
            FormLook.Line(8, Prompts.Select(prompt => (UIElement?)FormLook.Button(prompt.Title, () => Ask(prompt))).ToArray())), new Thickness(0, 16, 0, 0));
    }

    private async void Calibrate(JsonObject view)
    {
        if (view["notReady"] is JsonObject notReady)
        {
            await FormLook.Tell(XamlRoot, MapUiJson.Text(notReady["title"]), MapUiJson.Text(notReady["message"]));
            return;
        }
        if (view["startPrompt"] is JsonObject prompt
            && !await FormLook.Confirm(XamlRoot, MapUiJson.Text(prompt["title"]), MapUiJson.Text(prompt["message"]), "Ok")) return;
        SetupUi.Detached(() => FormWrites.Ok($"{Cal}.nextButtonClicked"));
    }

    private async void Ask(RadioPrompt prompt)
    {
        var choices = new RadioButtons { ItemsSource = prompt.Choices.ToList(), SelectedIndex = prompt.Choices.Count - 1 };
        AutomationProperties.SetName(choices, prompt.Title);
        var dialog = FormLook.Dialog(XamlRoot, prompt.Title, FormLook.Stack(12, FormLook.Text(prompt.Body), prompt.Choices.Count > 0 ? choices : null), "Ok");
        if (await FormLook.Show(dialog) != ContentDialogResult.Primary) return;
        var choice = prompt.Choices.Count > 0 ? Math.Max(0, choices.SelectedIndex) : (int?)null;
        SetupUi.Detached(() =>
        {
            if (choice is { } picked) FormWrites.Ok($"{Cal}.{prompt.Action}", picked);
            else FormWrites.Ok($"{Cal}.{prompt.Action}");
        });
    }

    private static Button Action(string text, string action, ButtonLook look, bool enabled)
    {
        var button = FormLook.Button(text, () => SetupUi.Detached(() => FormWrites.Ok($"{Cal}.{action}")), look);
        button.IsEnabled = enabled;
        return button;
    }
}
