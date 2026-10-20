using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Controls;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Pages.Plan;

internal sealed record CameraChoices(IReadOnlyList<string> Labels, int Chosen);

internal sealed record AngleRange(double From, double To)
{
    public double Clamp(double value) => Math.Clamp(value, From, To);
}

internal sealed record CameraExtras(
    double? IntervalTime, double? IntervalDistance, string DistanceUnits, bool ModeSupported, bool CommandsMode, int Mode,
    bool CommandsGimbal, double Pitch, double Yaw, AngleRange? PitchRange, AngleRange? YawRange);

internal static class ItemCamera
{
    public static readonly IReadOnlyList<string> Modes = ["Photo", "Video", "Survey"];

    public static string ViewPath(int index) => $"view.itemCamera({index})";

    public static string MemberPath(int index, string member) => $"{PlanCalls.ItemRoot}.{index}.cameraSection.{member}";

    private static bool Available(JsonObject? view) => view is not null && MapUiJson.Flag(view["available"]);

    private static double? Measured(JsonObject view, string key) => MapUiJson.Number((view[key] as JsonObject)?["value"]);

    private static AngleRange? UserRange(JsonObject view, string key) =>
        (view[key] as JsonObject)?["slider"] is JsonObject slider
        && MapUiJson.Number(slider["from"]) is { } from && MapUiJson.Number(slider["to"]) is { } to && from != to
            ? new AngleRange(Math.Min(from, to), Math.Max(from, to))
            : null;

    public static CameraExtras? Extras(JsonObject? view) =>
        !Available(view) ? null : new CameraExtras(
            Measured(view!, "intervalTime"),
            Measured(view!, "intervalDistance"),
            MapUiJson.Text((view!["intervalDistance"] as JsonObject)?["units"]) is { Length: > 0 } units ? units : "m",
            MapUiJson.Flag(view["cameraModeSupported"]),
            MapUiJson.Flag(view["commandsMode"]),
            (int)(MapUiJson.Number((view["cameraMode"] as JsonObject)?["choice"]) ?? 0),
            MapUiJson.Flag(view["commandsGimbal"]),
            Measured(view, "gimbalPitch") ?? 0,
            Measured(view, "gimbalYaw") ?? 0,
            UserRange(view, "gimbalPitch"),
            UserRange(view, "gimbalYaw"));

    public static CameraChoices? Choices(JsonObject? view) =>
        Available(view) && view!["cameraAction"] is JsonObject measure && measure["choices"] is JsonArray listed
        && listed.Select(MapUiJson.Text).ToList() is var labels && labels.Any(label => label.Trim().Length > 0)
            ? new CameraChoices(labels, (int)(MapUiJson.Number(measure["choice"]) ?? -1))
            : null;

    public static string MeasureText(JsonObject? view, string key) =>
        view?[key] is JsonObject measure && MapUiJson.Text(measure["text"]) is { Length: > 0 } text
            ? MapUiJson.Text(measure["units"]) is { Length: > 0 } units ? $"{text} {units}" : text
            : "";

    public static string NamedAction(string text) =>
        text.Trim().Length == 0 || FormText.Parse(text) is not null ? "" : text;

    public static string ActionLabel(JsonObject? view) =>
        Choices(view) is { } choices && choices.Chosen >= 0 && choices.Chosen < choices.Labels.Count
            ? choices.Labels[choices.Chosen]
            : NamedAction(MeasureText(view, "cameraAction"));

    public static string? Note(JsonObject? view) =>
        Available(view) && MapUiJson.Text(view!["note"]) is { Length: > 0 } note ? note : null;

    public static string? Text(JsonObject? view)
    {
        if (!Available(view)) return null;
        var angles = MapUiJson.Flag(view!["commandsGimbal"])
            ? string.Join(" / ", new[] { MeasureText(view, "gimbalPitch"), MeasureText(view, "gimbalYaw") }.Where(text => text.Length > 0))
            : "";
        var shown = string.Join(" · ", new[] { ActionLabel(view), angles.Length > 0 ? $"gimbal {angles}" : "" }.Where(text => text.Length > 0));
        return shown.Length > 0 ? shown : null;
    }

    public static string? TextBeside(JsonObject? view, string? pickerLabel) => Text(view) is { } text && text != pickerLabel ? text : null;

    public static IEnumerable<UIElement> Section(int index, Action<Task<string?>> wrote)
    {
        var view = PlanCalls.Read(ViewPath(index));
        if (Choices(view) is not { } choices) return [];
        void Write(string member, JsonNode? value) => wrote(PlanCalls.Refused(() => PlanCalls.Set(MemberPath(index, member), value)));
        var picked = choices.Chosen >= 0 && choices.Chosen < choices.Labels.Count ? choices.Labels[choices.Chosen] : null;
        var action = new ComboBox { MinWidth = 200, ItemsSource = choices.Labels.ToList(), SelectedIndex = choices.Chosen, PlaceholderText = "…" };
        AutomationProperties.SetName(action, "Camera action");
        action.SelectionChanged += (_, _) =>
        {
            if (action.SelectedIndex >= 0 && action.SelectedIndex != choices.Chosen) Write("cameraAction.enumIndex", action.SelectedIndex);
        };
        return new UIElement?[]
            {
                PlanFacts.Heading("Camera"),
                TextBeside(view, picked) is { } beside ? PlanFacts.Note(beside) : null,
                PlanFacts.Pair(PlanFacts.Text("Camera"), action),
            }
            .Concat(Extras(view) is { } extras ? ExtrasRows(extras, Write) : [])
            .Append(Note(view) is { } note ? PlanFacts.Note(note) : null)
            .OfType<UIElement>();
    }

    private static IEnumerable<UIElement?> ExtrasRows(CameraExtras extras, Action<string, JsonNode?> write) =>
    [
        extras.IntervalTime is { } time ? NumberRow("Time (s)", time, true, typed => write("cameraPhotoIntervalTime", typed)) : null,
        extras.IntervalDistance is { } distance ? NumberRow($"Distance ({extras.DistanceUnits})", distance, true, typed => write("cameraPhotoIntervalDistance", typed)) : null,
        extras.ModeSupported ? ModeRow(extras, write) : null,
        PlanFacts.Pair(PlanFacts.Text("Gimbal"), Switch("Gimbal", extras.CommandsGimbal, on => write("specifyGimbal", on))),
        NumberRow("Pitch", extras.Pitch, extras.CommandsGimbal, typed => write("gimbalPitch", extras.PitchRange?.Clamp(typed) ?? typed)),
        extras.PitchRange is { } pitch ? AngleSlider("Pitch slider", extras.Pitch, pitch, extras.CommandsGimbal, chosen => write("gimbalPitch", chosen)) : null,
        NumberRow("Yaw", extras.Yaw, extras.CommandsGimbal, typed => write("gimbalYaw", extras.YawRange?.Clamp(typed) ?? typed)),
        extras.YawRange is { } yaw ? AngleSlider("Yaw slider", extras.Yaw, yaw, extras.CommandsGimbal, chosen => write("gimbalYaw", chosen)) : null,
    ];

    private static UIElement NumberRow(string label, double value, bool enabled, Action<double> done)
    {
        var box = PlanFacts.Entry(FormText.Spell(value), typed =>
        {
            if (PlanRules.TypedNumber(typed) is double number) done(number);
        }, 110);
        box.IsEnabled = enabled;
        return PlanFacts.Pair(PlanFacts.Text(label), box);
    }

    private static ToggleSwitch Switch(string name, bool on, Action<bool> changed)
    {
        var toggle = new ToggleSwitch { IsOn = on, OnContent = string.Empty, OffContent = string.Empty, MinWidth = 0 };
        AutomationProperties.SetName(toggle, name);
        toggle.Toggled += (_, _) =>
        {
            if (toggle.IsOn != on) changed(toggle.IsOn);
        };
        return toggle;
    }

    private static UIElement ModeRow(CameraExtras extras, Action<string, JsonNode?> write)
    {
        var modes = new ComboBox { MinWidth = 120, ItemsSource = Modes.ToList(), SelectedIndex = extras.Mode, IsEnabled = extras.CommandsMode };
        AutomationProperties.SetName(modes, "Camera mode choice");
        modes.SelectionChanged += (_, _) =>
        {
            if (modes.SelectedIndex >= 0 && modes.SelectedIndex != extras.Mode) write("cameraMode", modes.SelectedIndex);
        };
        return PlanFacts.Pair(PlanFacts.Text("Mode"), PlanFacts.Row(Switch("Camera mode", extras.CommandsMode, on => write("specifyCameraMode", on)), modes));
    }

    private static UIElement AngleSlider(string name, double value, AngleRange range, bool enabled, Action<double> done)
    {
        var slider = new Slider
        {
            Minimum = range.From,
            Maximum = range.To,
            StepFrequency = 1,
            SmallChange = 1,
            Value = range.Clamp(value),
            IsEnabled = enabled,
            IsThumbToolTipEnabled = true,
        };
        AutomationProperties.SetName(slider, name);
        _ = new SliderCommit(slider, Math.Round, (chosen, _) => done(chosen));
        return slider;
    }
}

internal sealed record CameraCalcBlock(
    string Brand, string Model, IReadOnlyList<string> Brands, IReadOnlyList<string> Models, bool Manual, bool Custom,
    bool ValueSetIsDistance, string ValueSetIsDistancePath, string BrandPath, string ModelPath,
    IReadOnlyList<(string Suffix, JsonObject Control)> Facts, int? DistanceMode, IReadOnlyList<(int Raw, string Title)> DistanceModes, string DistanceModePath);

internal static class CameraCalc
{
    private const string DistanceSuffix = "cameraCalc.distanceToSurface";
    private const string DensitySuffix = "cameraCalc.imageDensity";
    private static readonly IReadOnlySet<string> SpacingSuffixes = new HashSet<string> { "cameraCalc.adjustedFootprintFrontal", "cameraCalc.adjustedFootprintSide" };

    private static IReadOnlyList<string> Strings(JsonNode? node) => (node as JsonArray ?? []).Select(MapUiJson.Text).ToList();

    public static CameraCalcBlock? Parse(JsonObject? block) => block is null ? null : new CameraCalcBlock(
        MapUiJson.Text(block["brand"]),
        MapUiJson.Text(block["model"]),
        Strings(block["brands"]),
        Strings(block["models"]),
        MapUiJson.Text(block["brand"]) == MapUiJson.Text(block["manualName"]),
        MapUiJson.Flag(block["custom"]),
        MapUiJson.Flag(block["valueSetIsDistance"], true),
        MapUiJson.Text(block["valueSetIsDistancePath"]),
        MapUiJson.Text(block["brandPath"]),
        MapUiJson.Text(block["modelPath"]),
        MapUiJson.Objects(block["facts"]).Select(control => (MapUiJson.Text(control["pathSuffix"]), control)).ToList(),
        MapUiJson.Number(block["distanceMode"]) is { } mode ? (int)mode : null,
        MapUiJson.Objects(block["distanceModes"]).Select(entry => ((int)(MapUiJson.Number(entry["raw"]) ?? 0), MapUiJson.Text(entry["title"]))).ToList(),
        MapUiJson.Text(block["distanceModePath"]));

    public static string? DistanceModeTitle(CameraCalcBlock block) =>
        block.DistanceModes.Where(entry => entry.Raw == block.DistanceMode).Select(entry => entry.Title).FirstOrDefault();

    public static string DistanceLabel(CameraCalcBlock block) =>
        block.Facts.Where(fact => fact.Suffix == DistanceSuffix).Select(fact => MapUiJson.Text(fact.Control["shortLabel"])).FirstOrDefault(label => label.Length > 0) ?? "Altitude";

    public static bool Shown(CameraCalcBlock block, string suffix) =>
        block.Manual ? suffix == DistanceSuffix || SpacingSuffixes.Contains(suffix)
        : suffix == DistanceSuffix ? block.ValueSetIsDistance
        : suffix != DensitySuffix || !block.ValueSetIsDistance;

    public static IReadOnlyList<JsonObject> ShownFacts(CameraCalcBlock block) =>
        block.Facts.Where(fact => Shown(block, fact.Suffix)).Select(fact => fact.Control).ToList();

    public static IEnumerable<UIElement> Header(CameraCalcBlock block, Action<string, JsonNode?> write)
    {
        var brand = Choice("Camera brand", block.Brand, block.Brands, picked => write(block.BrandPath, picked));
        var model = !block.Manual && !block.Custom
            ? Choice("Camera model", block.Model, block.Models, picked => write(block.ModelPath, picked), "Model")
            : null;
        var mode = DistanceModeTitle(block) is { } current
            ? PlanFacts.Pair(PlanFacts.Text("Altitude"), Choice("Altitude mode", current, block.DistanceModes.Select(entry => entry.Title).ToList(), picked =>
            {
                if (block.DistanceModes.Where(entry => entry.Title == picked).Select(entry => (int?)entry.Raw).FirstOrDefault() is { } raw) write(block.DistanceModePath, raw);
            }))
            : null;
        return new UIElement?[]
        {
            PlanFacts.Heading("Camera"),
            model is null ? brand : PlanFacts.Row(brand, model),
            mode,
            block.Manual ? null : SetBy(block, write),
        }.OfType<UIElement>();
    }

    private static UIElement SetBy(CameraCalcBlock block, Action<string, JsonNode?> write)
    {
        var choice = new RadioButtons { ItemsSource = new[] { DistanceLabel(block), "Ground res" }.ToList(), SelectedIndex = block.ValueSetIsDistance ? 0 : 1, MaxColumns = 2 };
        AutomationProperties.SetName(choice, "Set by");
        choice.SelectionChanged += (_, _) =>
        {
            if (choice.SelectedIndex >= 0 && choice.SelectedIndex == 0 != block.ValueSetIsDistance) write(block.ValueSetIsDistancePath, choice.SelectedIndex == 0);
        };
        return PlanFacts.Pair(PlanFacts.Text("Set by"), choice);
    }

    private static ComboBox Choice(string name, string current, IReadOnlyList<string> options, Action<string> pick, string placeholder = "")
    {
        var box = new ComboBox
        {
            MinWidth = 160,
            ItemsSource = options.ToList(),
            SelectedIndex = options.ToList().IndexOf(current),
            PlaceholderText = current.Length > 0 ? current : placeholder,
            IsEnabled = options.Count > 0,
        };
        AutomationProperties.SetName(box, name);
        box.SelectionChanged += (_, _) =>
        {
            if (box.SelectedItem is string picked && picked != current) pick(picked);
        };
        return box;
    }
}
