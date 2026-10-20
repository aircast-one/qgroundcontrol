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

namespace AircastQGC.Pages.Setup;

internal static class SensorSettingsForm
{
    public const string View = "view.sensorSettings";
    public const string Dialog = "sensorSettings";
    private const string Priority = "sensorSettings.priority";
    private const string Board = "";
    private const string Declination = "Magnetic declination";
    public const string OrientationsHelp = "Adjust orientations as needed.\n\nROTATION_NONE indicates component points in direction of flight.";

    public static FormView Block(bool calibrating, bool showCompasses, Action<bool>? onSimpleAccel = null)
    {
        var opened = new Lazy<bool>(() => FormWrites.Ok(SetupModel.DialogOpened, Dialog, calibrating));
        JsonObject Read()
        {
            _ = opened.Value;
            return Bridge.Get(View);
        }
        var simple = new SimpleAccel();
        var last = new Snapshot();
        return SetupUi.Form(Read, raw =>
        {
            last.Raw = raw;
            return Sections(raw, calibrating, showCompasses);
        }, new FormHooks
        {
            AfterBlock = (form, _, block) => block.Title == Board ? SimpleAccelRow(last.Raw, onSimpleAccel, simple)
                : block.Title == Declination ? DeclinationToggle(form, last.Raw)
                : Compass(form, block, last.Raw),
        }, "Reading sensor settings.");
    }

    public static FormView Orientations()
    {
        var opened = new Lazy<bool>(() => FormWrites.Ok(SetupModel.DialogOpened, Dialog, false));
        return SetupUi.Form(() =>
        {
            _ = opened.Value;
            return Bridge.Get(View);
        }, raw =>
        {
            var oriented = Compasses(raw)
                .Select(c => SetupUi.Control(c["orientation"]) is { } orientation ? SetupUi.Titled(orientation, MapUiJson.Text(c["orientationTitle"])) : null)
                .OfType<FormControl>()
                .ToList();
            return oriented.Count == 0 ? [] : [new FormSection("", "", OrientationsHelp, "", [], [new FormBlock("", oriented)])];
        });
    }

    private static IReadOnlyList<JsonObject> Compasses(JsonObject raw) => MapUiJson.Objects(raw["compasses"]);

    private static IReadOnlyList<FormSection> Sections(JsonObject raw, bool calibrating, bool showCompasses)
    {
        if (!MapUiJson.Flag(raw["available"])) return [];
        var board = SetupUi.Control(raw["boardRotation"]) is { } rotation ? SetupUi.Titled(rotation, MapUiJson.Text(raw["boardTitle"])) : null;
        var help = MapUiJson.Text(raw[calibrating ? "helpCal" : "helpSet"]);
        var compasses = showCompasses && (!calibrating || MapUiJson.Flag(raw["compassesWhileCalibrating"]))
            ? Compasses(raw).Select(c => SetupUi.Block(
                MapUiJson.Text(c["label"]),
                SetupUi.Control(c["use"]) is { } use ? SetupUi.Titled(use, "Use compass") : null,
                SetupUi.Control(c["orientation"]) is { } orientation ? SetupUi.Titled(orientation, MapUiJson.Text(c["orientationTitle"])) : null))
            : [];
        var declination = showCompasses && (!calibrating || MapUiJson.Flag(raw["compassesWhileCalibrating"])) && raw["declination"] is JsonObject d
            ? [SetupUi.Block(Declination, SetupUi.Control(d["value"]) is { } value ? value with { Enabled = MapUiJson.Flag(d["manual"]) } : null)]
            : Array.Empty<FormBlock>();
        return [new FormSection("", "", help, "", [], new[] { SetupUi.Block(Board, board) }.Concat(compasses).Concat(declination).ToList())];
    }

    private static UIElement? SimpleAccelRow(JsonObject raw, Action<bool>? report, SimpleAccel simple)
    {
        if (report is null) return null;
        var help = FormLook.Caption(MapUiJson.Text(raw["simpleAccelHelp"]));
        var box = FormLook.Check("Simple accelerometer calibration", simple.On, on =>
        {
            simple.On = on;
            report(on);
        });
        return FormLook.Margined(FormLook.Stack(4, help, box), new Thickness(0, 8, 0, 0));
    }

    private static UIElement? DeclinationToggle(FormView form, JsonObject raw)
    {
        var declination = raw["declination"] as JsonObject;
        if (declination is null) return null;
        var path = MapUiJson.Text(declination["autoDecPath"]);
        return FormLook.Margined(FormLook.Check("Manual magnetic declination", MapUiJson.Flag(declination["manual"]), manual =>
            SetupUi.Run(() => FormWrites.Set($"{path}.rawValue", manual ? 0 : 1), _ => form.Reload())), new Thickness(0, 8, 0, 0));
    }

    private static UIElement? Compass(FormView form, FormBlock block, JsonObject raw)
    {
        if (Compasses(raw).FirstOrDefault(c => MapUiJson.Text(c["label"]) == block.Title) is not { } compass) return null;
        var priorities = MapUiJson.Texts(raw["priorities"]);
        var index = (int)(MapUiJson.Number(compass["index"]) ?? 0);
        var priority = MapUiJson.Number(compass["priority"]) is { } slot ? (int)slot : (int?)null;
        var picker = priority is { } current && priorities.Count > 0
            ? FormLook.Combo(priorities, current < priorities.Count ? current : -1)
            : null;
        if (picker is not null)
        {
            AutomationProperties.SetName(picker, $"{block.Title} priority");
            picker.SelectionChanged += (_, _) =>
            {
                if (picker.SelectedIndex < 0 || picker.SelectedIndex == priority) return;
                var picked = picker.SelectedIndex;
                SetupUi.Run(() => FormWrites.Ok(Priority, index, picked), _ => form.Reload());
            };
        }
        return FormLook.Stack(4,
            block.Controls.Count == 0 ? FormLook.Heading(block.Title) : null,
            FormLook.Margined(FormLook.Caption(MapUiJson.Text(compass["device"])), new Thickness(4, 4, 0, 0)),
            picker is null ? null : FormLook.Row(FormLook.Text("Priority"), picker));
    }

    private sealed class SimpleAccel
    {
        public bool On { get; set; }
    }

    private sealed class Snapshot
    {
        public JsonObject Raw { get; set; } = new();
    }
}
