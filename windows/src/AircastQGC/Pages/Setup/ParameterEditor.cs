using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Pages.Setup;

internal static class ParameterEditor
{
    public const string ReadOnlyNote = "This parameter is read-only and cannot be modified.";
    public const string ForceEditNote = "Warning: This parameter is read-only. Force edit is enabled.";
    public const string InFlightWarning = "Warning: Modifying values while vehicle is in flight can lead to vehicle instability and possible vehicle loss. Make sure you know what you are doing and double-check your values before Save!";

    public static async void Show(XamlRoot? root, string key, bool offersRc, Action done)
    {
        if (root is null) return;
        var path = ParameterModel.PathOf(key);
        var name = key.Contains(':') ? key[(key.IndexOf(':') + 1)..] : key;
        var forceAllowed = MapUiJson.Flag(Bridge.Get(SetupModel.AdvancedUiPath)["shown"], true);
        var json = await FormWrites.Off(() => Bridge.Get(ParameterModel.ControlPath(key)));
        var loaded = MapUiJson.Text(json["kind"]) == "object" ? FormControl.Parse(json) : null;
        var defaultValue = json["defaultValue"]?.DeepClone();
        var body = new StackPanel { Spacing = 8, MinWidth = 420 };
        var dialog = FormLook.Dialog(root, key, new ScrollViewer { Content = body, MaxHeight = 560 }, null, "Done");
        var rc = false;
        if (loaded is null)
        {
            body.Children.Add(FormLook.Caption($"{name} is not a parameter on this vehicle."));
            await FormLook.Show(dialog);
            done();
            return;
        }
        var state = new EditorState();
        var refusal = SetupUi.Error();
        var hasChoices = loaded.IsChoice || loaded.IsBitmask;
        var editor = new ContentControl { HorizontalContentAlignment = HorizontalAlignment.Stretch, IsTabStop = false };
        var forcedBox = FormLook.Field("", "Value");
        var forceArea = FormLook.Stack(4, forcedBox, FormLook.Button("Save", () =>
        {
            var typed = forcedBox.Text;
            if (typed.Trim().Length == 0) return;
            object entered = loaded.IsText ? typed : FormText.Parse(typed.Trim()) is { } number ? number : typed.Trim();
            SetupUi.Run(() => SetupUi.ForcedWrite(path, entered), refused =>
            {
                FormLook.Say(refusal, refused);
                if (refused is null) RebuildEditor();
            });
        }));
        var note = FormLook.Caption("", "TextPrimaryBrush");
        var inFlight = FormLook.Caption(InFlightWarning, "CautionBrush");
        var forceCheck = Check("Force edit read-only param");
        var manualCheck = Check("Manual entry");
        var forceSaveCheck = Check("Force save (dangerous!)");
        var advancedCheck = Check("Advanced settings");
        void RebuildEditor()
        {
            var form = Editor(key, state.Manual);
            form.Rejected += (_, _) =>
            {
                state.Rejected = true;
                state.Repaint();
            };
            editor.Content = form;
        }
        void Paint()
        {
            var editable = !loaded.ReadOnly || state.Forced;
            FormLook.Say(note, !loaded.ReadOnly ? null : state.Forced ? ForceEditNote : ReadOnlyNote);
            note.Foreground = FormLook.Brush(state.Forced ? "CautionBrush" : "TextPrimaryBrush");
            inFlight.Visibility = FormLook.Shown(editable);
            advancedCheck.Visibility = FormLook.Shown((loaded.ReadOnly && forceAllowed) || (editable && hasChoices));
            forceCheck.Visibility = FormLook.Shown(state.Advanced && loaded.ReadOnly && forceAllowed);
            manualCheck.Visibility = FormLook.Shown(state.Advanced && editable && hasChoices);
            var numericEntry = state.Forced || (editable && (state.Manual || !hasChoices) && !loaded.IsText && !loaded.IsToggle);
            forceSaveCheck.Visibility = FormLook.Shown(numericEntry && !state.Forced && state.Rejected && forceAllowed);
            forceArea.Visibility = FormLook.Shown(numericEntry && (state.ForceSave || state.Forced));
            dialog.PrimaryButtonText = editable && defaultValue is not null ? "Reset to default" : "";
        }
        state.Repaint = Paint;
        forceCheck.Click += (_, _) =>
        {
            state.Forced = forceCheck.IsChecked == true;
            Paint();
        };
        manualCheck.Click += (_, _) =>
        {
            state.Manual = manualCheck.IsChecked == true;
            RebuildEditor();
            Paint();
        };
        forceSaveCheck.Click += (_, _) =>
        {
            state.ForceSave = forceSaveCheck.IsChecked == true;
            Paint();
        };
        advancedCheck.Click += (_, _) =>
        {
            state.Advanced = advancedCheck.IsChecked == true;
            state.Forced &= state.Advanced;
            state.Manual &= state.Advanced;
            forceCheck.IsChecked = state.Forced;
            manualCheck.IsChecked = state.Manual;
            RebuildEditor();
            Paint();
        };
        RebuildEditor();
        FormLook.Fill(body, new UIElement?[]
        {
            FormLook.Caption(loaded.LongDescription.Length > 0 ? loaded.LongDescription : loaded.Label),
            note,
            editor,
            FormLook.Caption(ParameterModel.RangeLine(loaded)),
        }
        .Concat(ParameterModel.RebootNotes(loaded).Select(text => (UIElement?)FormLook.Caption(text, "TextPrimaryBrush")))
        .Concat([inFlight, advancedCheck, forceCheck, manualCheck, forceSaveCheck, forceArea, refusal]));
        dialog.SecondaryButtonText = offersRc ? "RC to param" : "";
        dialog.PrimaryButtonClick += (_, args) =>
        {
            args.Cancel = true;
            var forced = state.Forced;
            SetupUi.Run(() => forced ? SetupUi.ForcedWrite(path, defaultValue) : SetupUi.Write(path, defaultValue), refused =>
            {
                FormLook.Say(refusal, refused);
                if (refused is null) dialog.Hide();
            });
        };
        dialog.SecondaryButtonClick += (_, _) => rc = true;
        Paint();
        await FormLook.Show(dialog);
        done();
        if (rc) RcToParam.Show(root, key, loaded, done);
    }

    private static CheckBox Check(string text) => new() { Content = text, FontFamily = PreflightLook.Font };

    private static FormView Editor(string key, bool manual) =>
        SetupUi.Form(
            () => Bridge.Get(ParameterModel.ControlPath(key)),
            raw => SetupUi.Control(raw) is not { } control
                ? []
                : [SetupUi.Section("", "", SetupUi.Block("", Entry(control, manual)))],
            loading: "Reading the parameter.");

    private static FormControl Entry(FormControl control, bool manual) =>
        (manual && (control.IsChoice || control.IsBitmask) ? control with { Kind = "number", Options = [], Bits = [] } : control) with
        {
            Label = control.Heading.Length > 0 ? control.Heading : "Value",
            ShortLabel = control.Heading.Length > 0 ? control.Heading : "Value",
            VehicleReboot = false,
            AppRestart = false,
            LongDescription = "",
        };

    private sealed class EditorState
    {
        public bool Advanced { get; set; }
        public bool Forced { get; set; }
        public bool Manual { get; set; }
        public bool ForceSave { get; set; }
        public bool Rejected { get; set; }
        public Action Repaint { get; set; } = () => { };
    }
}

internal static class RcToParam
{
    private const string Set = "parameters.setRcToParam";
    private const string OpenPath = "parameters.openRcToParam";
    private const string Ready = "parameters.rcToParamReady";
    private static readonly TimeSpan Poll = TimeSpan.FromMilliseconds(250);
    private static readonly IReadOnlyList<int> TuningIds = [1, 2, 3];

    public static async void Show(XamlRoot root, string key, FormControl control, Action done)
    {
        var raw = await FormWrites.Off(() => Bridge.Get(ParameterModel.PathOf(key)));
        var scale = FormLook.Field("1.0", "Scale");
        var center = FormLook.Field(MapUiJson.Number(control.Value) is { } number ? FormText.Spell(number) : control.ValueString, "Center Value");
        var min = FormLook.Field(MapUiJson.Text(raw["minString"]), "Min Value");
        var max = FormLook.Field(MapUiJson.Text(raw["maxString"]), "Max Value");
        var tuning = FormLook.Combo(TuningIds.Select(id => $"Tuning ID {id}"), 0);
        var waiting = FormLook.Text("Waiting on parameter update from Vehicle.");
        var refusal = SetupUi.Error();
        var fields = new Control[] { scale, center, min, max, tuning };
        var dialog = FormLook.Dialog(root, "RC To Param", FormLook.Stack(8,
            new UIElement?[]
            {
                FormLook.Caption("Bind an RC Channel to a parameter value. Tuning IDs can be mapped to an RC Channel from Radio Setup page."),
                waiting,
                FormLook.Text($"Parameter  {control.Name}"),
            }.Concat(fields).Concat(
            [
                FormLook.Caption("Double check that all values are correct prior to confirming dialog."),
                refusal,
            ])), "OK");
        IReadOnlyList<double>? Entered() =>
            new[] { scale.Text, center.Text, min.Text, max.Text }.Select(FormText.Parse).ToList() is var numbers && numbers.All(n => n is not null)
                ? numbers.Select(n => n!.Value).ToList()
                : null;
        void Paint(bool ready)
        {
            waiting.Visibility = FormLook.Shown(!ready);
            fields.ToList().ForEach(field => field.IsEnabled = ready);
            dialog.IsPrimaryButtonEnabled = ready && Entered() is not null;
        }
        var ready = false;
        var timer = new DispatcherTimer { Interval = Poll };
        timer.Tick += (_, _) => SetupUi.Run(() => FormWrites.Result(Ready, control.Name) is JsonValue value && value.TryGetValue<bool>(out var answer) ? answer : true, now =>
        {
            ready = now;
            if (now) timer.Stop();
            Paint(now);
        });
        new[] { scale, center, min, max }.ToList().ForEach(box => box.TextChanged += (_, _) => Paint(ready));
        dialog.Opened += (_, _) =>
        {
            SetupUi.Detached(() => FormWrites.Ok(OpenPath, control.Name));
            timer.Start();
        };
        dialog.Closed += (_, _) => timer.Stop();
        dialog.PrimaryButtonClick += (_, args) =>
        {
            args.Cancel = true;
            if (Entered() is not { } values) return;
            var index = Math.Max(0, tuning.SelectedIndex);
            SetupUi.Run(() => SetupUi.Refusal(Set, control.Name, values[0], values[1], index, values[2], values[3]), refused =>
            {
                FormLook.Say(refusal, refused);
                if (refused is null) dialog.Hide();
            });
        };
        Paint(false);
        await FormLook.Show(dialog);
        done();
    }
}
