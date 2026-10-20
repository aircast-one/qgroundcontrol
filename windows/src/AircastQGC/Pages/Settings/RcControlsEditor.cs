using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Pages.Settings;

internal sealed record RcEntry(int Index, RcControl Control);

internal static class RcControlsJson
{
    public static IReadOnlyList<JsonObject> Entries(string json) =>
        Parsed(json) is JsonArray array ? array.Select(item => item is JsonObject entry ? (JsonObject)entry.DeepClone() : new JsonObject()).ToList() : [];

    public static IReadOnlyList<RcEntry> Controls(string json) =>
        Entries(json)
            .Select((entry, at) => (At: at, Channel: Channel(entry), Type: RcControls.TypeOf(MapUiJson.Text(entry["type"])), Label: MapUiJson.Text(entry["label"]).Trim()))
            .Where(read => read.Channel > 0 && read.Type is not null)
            .Select(read => new RcEntry(read.At, new RcControl(read.Label.Length > 0 ? read.Label : $"CH{read.Channel}", read.Channel, read.Type!.Value)))
            .ToList();

    public static string Added(string json, string label, int channel, RcControlType type) =>
        Encode(Entries(json).Append(Patched(new JsonObject(), label, channel, type)));

    public static string Removed(string json, int index) =>
        Encode(Entries(json).Where((_, at) => at != index));

    public static string Patched(string json, int index, string label, int channel, RcControlType type) =>
        Encode(Entries(json).Select((entry, at) => at == index ? Patched(entry, label, channel, type) : entry));

    public static string? Owner(string json, int channel, int ignoring, IReadOnlyDictionary<int, string> reserved) =>
        reserved.TryGetValue(channel, out var owner)
            ? owner
            : Entries(json)
                .Select((entry, at) => (Entry: entry, At: at))
                .Where(read => read.At != ignoring && Channel(read.Entry) == channel)
                .Select(read => MapUiJson.Text(read.Entry["label"]).Trim() is { Length: > 0 } label ? label : $"control {read.At + 1}")
                .FirstOrDefault();

    public static int FirstFree(string json, IReadOnlyDictionary<int, string> reserved) =>
        Enumerable.Range(RcControls.ChannelMin, RcControls.ChannelMax - RcControls.ChannelMin + 1)
            .Where(channel => Owner(json, channel, -1, reserved) is null)
            .DefaultIfEmpty(RcControls.ChannelMin)
            .First();

    private static int Channel(JsonObject entry) => (int)(MapUiJson.Number(entry["channel"]) ?? 0);

    private static JsonObject Patched(JsonObject entry, string label, int channel, RcControlType type)
    {
        entry["label"] = label;
        entry["channel"] = channel;
        entry["type"] = RcControls.TypeName(type);
        return entry;
    }

    private static string Encode(IEnumerable<JsonObject> entries) => new JsonArray(entries.Cast<JsonNode?>().ToArray()).ToJsonString();

    private static JsonNode? Parsed(string json)
    {
        try { return JsonNode.Parse(json.Length > 0 ? json : "[]"); }
        catch (System.Text.Json.JsonException) { return null; }
    }
}

internal sealed class RcControlsEditor : UserControl
{
    private const string View = "view.control(" + RcControls.Setting + ")";
    private const string Refused = "The setting would not take that list.";
    private static readonly TimeSpan UndoWindow = TimeSpan.FromSeconds(6);

    private readonly StackPanel _body = new() { Spacing = 4 };
    private readonly TextBlock _notice = FormLook.Caption("", "CriticalBrush");
    private readonly DispatcherTimer _undoTimer = new() { Interval = UndoWindow };
    private string _json = "[]";
    private ImmutableDictionary<string, int> _cameraChannels = ImmutableDictionary<string, int>.Empty;
    private (string Name, string Previous)? _undo;

    public RcControlsEditor()
    {
        Content = FormLook.Stack(4, FormLook.Heading("On-screen RC controls"), _notice, _body);
        _undoTimer.Tick += (_, _) =>
        {
            _undoTimer.Stop();
            _undo = null;
            Render();
        };
        Live.Watch(this, View, Read);
        RcControls.CameraChannels.ToList().ForEach(channel => Live.Watch(this, RcControls.ControlView(channel.Name), view =>
        {
            _cameraChannels = _cameraChannels.SetItem(channel.Name, RcControls.Channel(view));
            Render();
        }));
        Unloaded += (_, _) => _undoTimer.Stop();
    }

    private IReadOnlyDictionary<int, string> Reserved =>
        RcControls.CameraChannels
            .Select(channel => (Channel: _cameraChannels.GetValueOrDefault(channel.Name), channel.Owner))
            .Where(pair => pair.Channel > 0)
            .GroupBy(pair => pair.Channel)
            .ToDictionary(group => group.Key, group => group.First().Owner);

    private void Read(JsonObject view)
    {
        _json = MapUiJson.Text(view["value"]);
        Render();
    }

    private void Render()
    {
        var controls = RcControlsJson.Controls(_json);
        var reserved = Reserved;
        FormLook.Fill(_body,
        [
            controls.Count == 0
                ? FormLook.Footnote("No on-screen controls yet. Add one to drive a channel from the Fly view.")
                : FormLook.Card(controls.Select(entry => Row(entry, reserved))),
            _undo is { } undo
                ? FormLook.Card(FormLook.Row(FormLook.Text($"Removed {undo.Name}."), FormLook.Button("Undo", () => Restore(undo.Previous))))
                : null,
            FormLook.Margined(FormLook.Button("Add control", () => Edit(null, reserved), ButtonLook.Accent, ""), new Thickness(0, 8, 0, 0)),
        ]);
    }

    private UIElement Row(RcEntry entry, IReadOnlyDictionary<int, string> reserved)
    {
        var control = entry.Control;
        var clash = RcControlsJson.Owner(_json, control.Channel, entry.Index, reserved);
        var edit = FormLook.Button("Edit", () => Edit(entry, reserved), ButtonLook.Subtle);
        var remove = FormLook.Button("Remove", () => Remove(entry), ButtonLook.Subtle);
        AutomationProperties.SetName(edit, $"Edit {control.Label}");
        AutomationProperties.SetName(remove, $"Remove {control.Label}");
        return FormLook.Row(
            FormLook.Stack(2,
                FormLook.Text(control.Label),
                FormLook.Caption($"Channel {control.Channel} · {RcControls.TypeLabel(control.Type)}"),
                clash is null ? null : FormLook.Caption($"Channel {control.Channel} is already driving {clash}.", "CriticalBrush")),
            FormLook.Line(4, edit, remove));
    }

    private void Remove(RcEntry entry)
    {
        _undo = (entry.Control.Label, _json);
        _undoTimer.Stop();
        _undoTimer.Start();
        Save(RcControlsJson.Removed(_json, entry.Index));
    }

    private void Restore(string previous)
    {
        _undoTimer.Stop();
        _undo = null;
        Save(previous);
    }

    private async void Save(string next)
    {
        FormLook.Say(_notice, null);
        var refusal = await FormWrites.Off(() => FormWrites.Set(RcControls.Setting, next));
        FormLook.Say(_notice, refusal is null ? null : Refused);
        Read(await FormWrites.Off(() => Bridge.Get(View)));
    }

    private async void Edit(RcEntry? entry, IReadOnlyDictionary<int, string> reserved)
    {
        if (XamlRoot is null) return;
        var index = entry?.Index ?? -1;
        var name = FormLook.Field(entry?.Control.Label ?? "", "Name");
        var channel = FormLook.Field((entry?.Control.Channel ?? RcControlsJson.FirstFree(_json, reserved)).ToString(), "Channel");
        channel.MaxLength = 2;
        channel.InputScope = new Microsoft.UI.Xaml.Input.InputScope { Names = { new Microsoft.UI.Xaml.Input.InputScopeName(Microsoft.UI.Xaml.Input.InputScopeNameValue.Number) } };
        channel.BeforeTextChanging += (_, args) => args.Cancel = !args.NewText.All(char.IsAsciiDigit);
        var types = new RadioButtons
        {
            Header = "Type",
            ItemsSource = RcControls.Types.Select(RcControls.TypeLabel).ToList(),
            SelectedIndex = RcControls.Types.ToList().IndexOf(entry?.Control.Type ?? RcControlType.Slider),
            MaxColumns = 2,
        };
        var problem = FormLook.Caption("", "CriticalBrush");
        AutomationProperties.SetName(name, "Name");
        AutomationProperties.SetName(channel, "Channel");
        AutomationProperties.SetName(types, "Control type");
        var dialog = FormLook.Dialog(XamlRoot, entry is null ? "New control" : "Edit control", FormLook.Stack(12, name, channel, problem, types), "Save");
        int Chosen() => int.TryParse(channel.Text, out var typed) ? typed : 0;
        void Check()
        {
            var picked = Chosen();
            var owner = RcControlsJson.Owner(_json, picked, index, reserved);
            FormLook.Say(problem, !RcControls.Usable(picked)
                ? $"Channels run from {RcControls.ChannelMin} to {RcControls.ChannelMax}."
                : owner is null ? null : $"Channel {picked} is already driving {owner}.");
            dialog.IsPrimaryButtonEnabled = RcControls.Usable(picked) && owner is null;
        }
        channel.TextChanged += (_, _) => Check();
        Check();
        if (await FormLook.Show(dialog) != ContentDialogResult.Primary) return;
        var picked = Chosen();
        var label = name.Text.Trim() is { Length: > 0 } typed ? typed : $"CH{picked}";
        var type = RcControls.Types[Math.Max(0, types.SelectedIndex)];
        Save(entry is null
            ? RcControlsJson.Added(_json, label, picked, type)
            : RcControlsJson.Patched(_json, index, label, picked, type));
    }
}
