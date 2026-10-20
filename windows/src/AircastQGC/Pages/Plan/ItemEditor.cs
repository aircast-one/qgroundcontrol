using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Controls;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Pages.Plan;

internal sealed class ItemEditor : UserControl
{
    private const string Absent = "—";
    private const string RawEditNote = "Provides advanced access to all commands/parameters. Be very careful!";
    private const string RawEditStuck = "You have made changes to the mission item which cannot be shown in Simple Mode";
    private const string ItemContext = "item";
    private const string MissionContext = "mission";
    private const int MixedFrame = 0;
    private const int RelativeFrame = 1;
    private const double AltitudeStep = 1;
    private const double SpeedStep = 0.5;
    private const string SavePreset = "core.plan.savePreset";
    private const string ApplyPreset = "core.plan.applyPreset";
    private const string DeletePreset = "core.plan.deletePreset";
    private const string PresetsFirstSetting = "settings.planViewSettings.displayPresetsTabFirst.rawValue";
    private static readonly (string Name, string Label)[] OfflineClasses =
        [("offlineEditingFirmwareClass", "Firmware"), ("offlineEditingVehicleClass", "Vehicle")];

    private readonly int _index;
    private readonly Func<PlanItem?> _item;
    private readonly Func<PlanStatus> _status;
    private readonly Func<Geo?> _mapCentre;
    private readonly PlanDialogs _dialogs;
    private readonly Func<Task> _changed;
    private readonly Action? _remove;
    private readonly StackPanel _root = new() { Spacing = 10 };
    private bool _advanced;
    private string? _refusal;
    private string? _preset;

    public ItemEditor(int index, Func<PlanItem?> item, Func<PlanStatus> status, Func<Geo?> mapCentre, PlanDialogs dialogs, Func<Task> changed, Action? remove)
    {
        _index = index;
        _item = item;
        _status = status;
        _mapCentre = mapCentre;
        _dialogs = dialogs;
        _changed = changed;
        _remove = remove;
        Content = _root;
        Render();
    }

    public void Render()
    {
        var view = PlanCalls.Read($"view.itemFacts({_index})");
        _root.Children.Clear();
        Sections(view).ToList().ForEach(_root.Children.Add);
    }

    private string Item(string property) => $"{PlanCalls.ItemRoot}.{_index}.{property}";

    private async void Wrote(string? refusal)
    {
        _refusal = refusal;
        await _changed();
        Render();
    }

    private static string? Write(string path, JsonNode? value) => PlanCalls.Refusal(PlanCalls.Set(path, value));

    private IEnumerable<UIElement> Sections(JsonObject view)
    {
        var item = _item();
        var simple = MapUiJson.Flag(view["simple"]);
        var top = Top(view, item);
        var toggle = simple ? new UIElement[] { AdvancedToggle() } : [];
        var rest = !simple || _advanced ? Advanced(view, item) : [];
        return top.Concat(toggle).Concat(rest);
    }

    private IEnumerable<UIElement> Top(JsonObject view, PlanItem? item)
    {
        if (item == null || _index == PlanRules.HomeItem) return [];
        var globalFrame = _status().GlobalFrame;
        var altitude = item.Altitude is double shown
            ? PlanFacts.Stepper("Altitude", shown, item.AltitudeUnits.Length > 0 ? item.AltitudeUnits : "m", AltitudeStep,
                item.FrameText.Length > 0 ? item.FrameText : null,
                wanted => Wrote(PlanCalls.Refusal(PlanCalls.SetAltitude(_index, wanted))))
            : null;
        var speed = view["speedSection"] is JsonObject section && MapUiJson.Flag(section["available"]) ? Speed(section) : null;
        var mode = item.Altitude != null && globalFrame != RelativeFrame ? AltitudeMode(item, globalFrame) : null;
        var camera = MapUiJson.Flag(view["simple"]) ? ItemCamera.Section(_index, Wrote) : [];
        return new[] { altitude, speed, mode }.OfType<UIElement>().Concat(camera);
    }

    private FrameworkElement Speed(JsonObject section)
    {
        var specified = MapUiJson.Flag(section["specified"]);
        var path = MapUiJson.Text(section["path"]);
        var specify = MapUiJson.Text(section["specifyPath"]);
        var auto = specified ? PlanFacts.Action("Auto", () => Wrote(Write(specify, false))) : null;
        return PlanFacts.Stepper("Speed", MapUiJson.Number(section["value"]), MapUiJson.Text(section["units"]), SpeedStep,
            specified ? null : "Mission speed",
            wanted => Wrote(Write(specify, true) ?? Write(path, wanted)),
            auto);
    }

    private FrameworkElement? AltitudeMode(PlanItem item, int? globalFrame)
    {
        var view = PlanCalls.Read($"view.altitudeModes({ItemContext},{item.AltitudeMode ?? -1})");
        if (MapUiJson.Text(view["class"]) != "AltitudeModes") return null;
        var offers = MapUiJson.Objects(view["modes"]).Where(offer => (int)(MapUiJson.Number(offer["raw"]) ?? -1) != MixedFrame).ToList();
        var live = (globalFrame == null || globalFrame == MixedFrame) && offers.Count(offer => MapUiJson.Flag(offer["enabled"])) > 1;
        if (offers.Count == 0) return PlanFacts.Pair(PlanFacts.Text("Altitude mode"), PlanFacts.Note(item.FrameText.Length > 0 ? item.FrameText : "Frame"));
        var picker = OfferBox(offers, live, raw => Wrote(PlanCalls.Refusal(PlanCalls.SetAltitudeMode(_index, raw))));
        var notes = live
            ? offers.Where(offer => !MapUiJson.Flag(offer["current"]) && !MapUiJson.Flag(offer["enabled"]) && MapUiJson.Text(offer["reason"]).Length > 0)
                .Select(offer => (UIElement)PlanFacts.Note($"{MapUiJson.Text(offer["title"])}: {MapUiJson.Text(offer["reason"])}"))
            : [];
        var panel = new StackPanel { Spacing = 4, Children = { PlanFacts.Pair(PlanFacts.Text("Altitude mode"), picker) } };
        notes.ToList().ForEach(panel.Children.Add);
        return panel;
    }

    private static ComboBox OfferBox(IReadOnlyList<JsonObject> offers, bool live, Action<int> pick)
    {
        var box = new ComboBox { MinWidth = 180, IsEnabled = live };
        offers.Select(offer => new ComboBoxItem
        {
            Content = MapUiJson.SentenceCase(MapUiJson.Text(offer["title"])),
            IsEnabled = MapUiJson.Flag(offer["enabled"]) || MapUiJson.Flag(offer["current"]),
            Tag = (int)(MapUiJson.Number(offer["raw"]) ?? -1),
        }).ToList().ForEach(box.Items.Add);
        var help = string.Join("\n", offers.Select(offer => MapUiJson.Text(offer["help"])).Where(text => text.Length > 0));
        ToolTipService.SetToolTip(box, help.Length > 0 ? help : null);
        var current = offers.ToList().FindIndex(offer => MapUiJson.Flag(offer["current"]));
        box.SelectedIndex = current;
        box.SelectionChanged += (_, _) =>
        {
            if (box.SelectedIndex != current && box.SelectedItem is ComboBoxItem { Tag: int raw }) pick(raw);
        };
        return box;
    }

    private UIElement AdvancedToggle()
    {
        var button = new Button
        {
            Content = _advanced ? "Hide advanced settings" : "Advanced",
            HorizontalAlignment = HorizontalAlignment.Stretch,
            HorizontalContentAlignment = HorizontalAlignment.Left,
        };
        button.Click += (_, _) =>
        {
            _advanced = !_advanced;
            Render();
        };
        return button;
    }

    private IEnumerable<UIElement> Advanced(JsonObject view, PlanItem? item)
    {
        var wizard = MapUiJson.Flag(view["wizardMode"]) ? MapUiJson.Texts(view["wizardText"]) : [];
        var fields = PlanFacts.Fields(view).Where(field => view["hold"] == null || MapUiJson.Text(field["label"]) != "Hold").ToList();
        var help = MapUiJson.Text(view["areaHelp"]);
        var leg = item == null ? null : PlanRules.LegDetail(item);
        var raw = MapUiJson.Flag(view["simple"]) ? (On: MapUiJson.Flag(view["rawEdit"]), Friendly: MapUiJson.Flag(view["friendlyEditAllowed"])) : ((bool On, bool Friendly)?)null;
        var note = raw is { On: true } ? RawEditNote : MapUiJson.Text(view["commandDescription"]);
        var connected = PlanCalls.HasVehicle();
        var presetKind = MapUiJson.Text(view["presetKind"]);
        var presets = help.Length == 0 && wizard.Count == 0 && presetKind.Length > 0 ? Presets(presetKind) : [];
        var presetsFirst = presets.Count > 0 && MapUiJson.Flag(PlanCalls.Read(PresetsFirstSetting)["value"]);
        var relative = view["altitudesAreRelative"] is JsonValue flag && flag.TryGetValue<bool>(out var on) ? on : (bool?)null;
        var parts = new IEnumerable<UIElement>[]
        {
            leg != null ? [PlanFacts.Note(leg)] : [],
            [Commands(view, item)],
            wizard.Count > 0 ? Wizard(wizard, fields) : [],
            _index == PlanRules.HomeItem ? Launch(view) : [],
            wizard.Count == 0 && relative is bool relativeOn ? [Relative(relativeOn)] : [],
            MapUiJson.Text(view["altitudeHint"]) is { Length: > 0 } hint ? [PlanFacts.Note(hint)] : [],
            raw is { } rawEdit && AdvancedUi() ? [RawToggle(rawEdit.On, rawEdit.Friendly)] : [],
            note.Length > 0 ? [PlanFacts.Note(note)] : [],
            _refusal != null ? [PlanFacts.Error(_refusal)] : [],
            MapUiJson.Flag(view["landing"]) && connected && wizard.Count == 0 ? [LandingButtons()] : [],
            wizard.Count == 0 ? MapUiJson.Texts(view["landingNotes"]).Select(text => (UIElement)PlanFacts.Warning(text)) : [],
            help.Length > 0 ? [PlanFacts.Text(help, "TextSecondaryBrush")] : [],
            presetsFirst ? presets : [],
            help.Length == 0 && wizard.Count == 0 ? Fields(fields, relative) : [],
            help.Length == 0 && wizard.Count == 0 && !MapUiJson.Flag(view["simple"]) ? ItemCamera.Section(_index, Wrote) : [],
            help.Length == 0 && wizard.Count == 0 ? Camera(view["camera"] as JsonObject) : [],
            MapUiJson.Text(view["gridNote"]) is { Length: > 0 } grid ? [PlanFacts.Note(grid)] : [],
            view["entryPoint"] is JsonObject entry && MapUiJson.Text(entry["path"]).Length > 0 ? [EntryPoint(entry)] : [],
            item is { Complex: true } ? Statistics() : [],
            presetsFirst ? [] : presets,
            _remove != null && _index > PlanRules.HomeItem ? [PlanFacts.Action("Delete from the plan", _remove, critical: true)] : [],
        };
        return parts.SelectMany(part => part);
    }

    private IReadOnlyList<UIElement> Presets(string kind)
    {
        var names = MapUiJson.Texts(PlanCalls.Read($"view.patternPresets({kind})")["names"]);
        _preset = _preset != null && names.Contains(_preset) ? _preset : names.FirstOrDefault();
        var picker = new ComboBox { MinWidth = 180, IsEnabled = names.Count > 0, PlaceholderText = "None" };
        names.ToList().ForEach(picker.Items.Add);
        picker.SelectedItem = _preset;
        picker.SelectionChanged += (_, _) => _preset = picker.SelectedItem as string;
        var apply = PlanFacts.Action("Apply preset", () =>
        {
            if (_preset is { } name) Wrote(PlanCalls.Refusal(PlanCalls.Invoke(ApplyPreset, _index, name)));
        });
        var delete = PlanFacts.Action("Delete preset", async () =>
        {
            if (_preset is { } name && await _dialogs.Confirm("Delete preset", $"Are you sure you want to delete '{name}' preset?", "OK"))
                Wrote(PlanCalls.Refusal(PlanCalls.Invoke(DeletePreset, kind, name)));
        });
        apply.IsEnabled = delete.IsEnabled = names.Count > 0;
        var save = PlanFacts.Action("Save settings as new preset", async () =>
        {
            if (await _dialogs.Ask("Save preset", "Save the current settings as a named preset.", "Preset name", "Enter preset name", "Save", PresetNameProblem) is not { } name) return;
            var refusal = PlanCalls.Refusal(PlanCalls.Invoke(SavePreset, _index, name));
            if (refusal == null) _preset = name;
            Wrote(refusal);
        });
        return [PlanFacts.Heading("Presets"), PlanFacts.Pair(PlanFacts.Text("Preset"), picker), PlanFacts.Row(apply, delete), save];
    }

    private static string? PresetNameProblem(string typed) =>
        typed.Trim().Length == 0 ? "Preset name cannot be blank."
        : typed.Contains('/') ? "Preset name cannot include the \"/\" character."
        : null;

    private static bool AdvancedUi() => MapUiJson.Flag(PlanCalls.Read("view.advancedUi")["shown"], true);

    private UIElement Commands(JsonObject view, PlanItem? item)
    {
        var row = PlanFacts.Row();
        row.HorizontalAlignment = HorizontalAlignment.Right;
        if (item?.At != null || MapUiJson.Flag(view["specifiesCoordinate"])) row.Children.Add(PositionMenu(view, item));
        if (MapUiJson.Flag(view["simple"]) && !MapUiJson.Flag(view["takeoff"]))
            row.Children.Add(PlanFacts.Action("Change command", ChangeCommand(MapUiJson.Text(view["category"]))));
        return row;
    }

    private Action ChangeCommand(string category) => async () =>
    {
        if (await _dialogs.PickCommand(category.Length > 0 ? category : null) is not int command) return;
        if (_mapCentre() is { } centre) PlanCalls.Invoke(Item("setMapCenterHintForCommandChange"), PlanCalls.Coordinate(centre));
        Wrote(Write(Item("command"), command));
    };

    private UIElement PositionMenu(JsonObject view, PlanItem? item)
    {
        var previous = MapOverlayJson.Plottable(view["previousCoordinate"]);
        var connected = PlanCalls.HasVehicle();
        var start = item?.At ?? _mapCentre() ?? previous;
        var menu = new MenuFlyout();
        MenuFlyoutItem Entry(string text, bool enabled, Action click)
        {
            var entry = new MenuFlyoutItem { Text = text, IsEnabled = enabled };
            entry.Click += (_, _) => click();
            return entry;
        }
        void MoveTo(Geo? target) =>
            Wrote(target is { } at && PlanCalls.Move(_index, at) ? null : "The item could not be moved there.");
        async void Edit()
        {
            if (start is not { } from) return;
            if (await _dialogs.EditPosition(from, connected) is { } moved) MoveTo(moved);
        }
        menu.Items.Add(Entry("Move to Vehicle Position", connected, () => MoveTo(PlanCalls.VehiclePosition())));
        menu.Items.Add(Entry("Move to Previous Item", previous != null, () => MoveTo(previous)));
        menu.Items.Add(Entry("Edit position…", start != null, Edit));
        return new DropDownButton { Content = "Position", Flyout = menu };
    }

    private IEnumerable<UIElement> Wizard(IReadOnlyList<string> lines, IReadOnlyList<JsonObject> fields)
    {
        var done = new Button { Content = "Done", HorizontalAlignment = HorizontalAlignment.Stretch, Style = (Style)Application.Current.Resources["AccentButtonStyle"] };
        done.Click += (_, _) => Wrote(Write(Item("wizardMode"), false));
        return new UIElement[] { PlanFacts.Text(lines[0]) }
            .Concat(lines.Skip(1).Select(line => (UIElement)PlanFacts.Note(line)))
            .Concat(fields.Where(field => MapUiJson.Text(field["path"]).EndsWith(".loiterClockwise", StringComparison.Ordinal)).Select(FactRow))
            .Append(done);
    }

    private IEnumerable<UIElement> Launch(JsonObject view)
    {
        var launch = view["launchAltitude"] is JsonObject control
            ? new UIElement[]
            {
                PlanFacts.Heading("Launch position"),
                FactRow(control),
                PlanFacts.Note("Actual position is set by the vehicle at flight time."),
                PlanFacts.Pair(PlanFacts.Text("Position"), CentreButton()),
            }
            : [];
        return MissionFrame().Concat(VehicleRows()).Concat(launch);
    }

    private Button CentreButton()
    {
        var button = PlanFacts.Action("Set to map center", () =>
        {
            if (_mapCentre() is not { } centre) return;
            Wrote(PlanCalls.Move(PlanRules.HomeItem, centre) ? null : "The launch position could not be moved there.");
        });
        button.IsEnabled = _mapCentre() != null;
        return button;
    }

    private IEnumerable<UIElement> MissionFrame()
    {
        if (_status().GlobalFrame is not int current) return [];
        var offers = MapUiJson.Objects(PlanCalls.Read($"view.altitudeModes({MissionContext},{current})")["modes"]).ToList();
        var live = offers.Count(offer => MapUiJson.Flag(offer["enabled"])) > 1;
        return offers.Count == 0
            ? []
            : [PlanFacts.Pair(PlanFacts.Text("Altitude frame"), OfferBox(offers, live, raw => Wrote(Write(PlanCalls.GlobalAltitudeMode, raw))))];
    }

    private IEnumerable<UIElement> VehicleRows()
    {
        var status = _status();
        var choosing = !PlanCalls.HasVehicle() && !status.HasMissionItems;
        return choosing
            ? OfflineClasses
                .Select(entry => (Label: entry.Label, Control: PlanCalls.Read($"view.control(settings.appSettings.{entry.Name})")))
                .Where(entry => MapUiJson.Text(entry.Control["path"]).Length > 0)
                .Select(entry =>
                {
                    entry.Control["label"] = entry.Label;
                    return FactRow(entry.Control);
                })
            : status.PlanningFor is { } planning
                ? new[] { ("Firmware", planning.Firmware), ("Vehicle", planning.Type) }
                    .Where(row => row.Item2.Length > 0)
                    .Select(row => (UIElement)PlanFacts.Pair(PlanFacts.Text(row.Item1), PlanFacts.Text(row.Item2, "TextSecondaryBrush")))
                : [];
    }

    private UIElement Relative(bool on)
    {
        var toggle = new ToggleSwitch { IsOn = on, OnContent = string.Empty, OffContent = string.Empty, MinWidth = 0 };
        toggle.Toggled += (_, _) => Wrote(Write(Item("altitudesAreRelative"), toggle.IsOn));
        return PlanFacts.Pair(PlanFacts.Text("Altitudes relative to launch"), toggle);
    }

    private UIElement RawToggle(bool on, bool friendlyAllowed)
    {
        var toggle = new ToggleSwitch { IsOn = on, OnContent = string.Empty, OffContent = string.Empty, MinWidth = 0 };
        toggle.Toggled += (_, _) => Wrote(on && !friendlyAllowed ? RawEditStuck : Write(Item("rawEdit"), toggle.IsOn));
        return PlanFacts.Pair(PlanFacts.Text("Show all values"), toggle);
    }

    private UIElement LandingButtons() => PlanFacts.Row(
        PlanFacts.Action("Set to vehicle heading", () =>
            Wrote(MapUiJson.Number(PlanCalls.Read("vehicle.heading")["value"]) is double heading
                ? Write(Item("landingHeading"), heading)
                : "The vehicle has not reported its heading.")),
        PlanFacts.Action("Set to vehicle location", () =>
            Wrote(PlanCalls.VehiclePosition() is { } at
                ? Write(Item("landingCoordinate"), PlanCalls.Coordinate(at))
                : "The vehicle has no position yet.")));

    private IEnumerable<UIElement> Fields(IReadOnlyList<JsonObject> fields, bool? relative)
    {
        var frame = relative is bool on ? on ? "Rel" : "AMSL" : null;
        var sections = fields.Select(field => MapUiJson.Text(field["section"])).ToList();
        return fields.SelectMany((field, at) =>
        {
            var section = sections[at];
            var heading = section.Length > 0 && (at == 0 || sections[at - 1] != section) ? new UIElement[] { PlanFacts.Heading(section) } : [];
            var shown = frame != null && new[] { ".finalApproachAltitude", ".landingAltitude" }.Any(end => MapUiJson.Text(field["path"]).EndsWith(end, StringComparison.Ordinal))
                ? WithUnits(field, frame)
                : field;
            return heading.Append(shown["choice"] is JsonObject choice ? Radio(shown, choice) : FactRow(shown));
        });
    }

    private static JsonObject WithUnits(JsonObject field, string frame)
    {
        var copy = (JsonObject)field.DeepClone();
        copy["units"] = string.Join(' ', new[] { MapUiJson.Text(field["units"]), frame }.Where(text => text.Length > 0));
        return copy;
    }

    private UIElement Radio(JsonObject field, JsonObject choice)
    {
        var radio = new RadioButton { IsChecked = MapUiJson.Flag(choice["selected"]), MinWidth = 0, GroupName = $"item{_index}" };
        radio.Checked += (_, _) =>
        {
            if (!MapUiJson.Flag(choice["selected"])) Wrote(Write(MapUiJson.Text(choice["path"]), MapUiJson.Flag(choice["value"])));
        };
        var fact = (FrameworkElement)FactRow(field);
        Grid.SetColumn(fact, 1);
        return new Grid
        {
            ColumnDefinitions = { new ColumnDefinition { Width = GridLength.Auto }, new ColumnDefinition() },
            Children = { radio, fact },
        };
    }

    private UIElement FactRow(JsonObject control) =>
        PlanFacts.Row(control, Write, () =>
        {
            _refusal = null;
            _changed();
        });

    private IEnumerable<UIElement> Camera(JsonObject? camera) =>
        CameraCalc.Parse(camera) is { } block
            ? CameraCalc.Header(block, (path, value) => Wrote(Write(path, value))).Concat(CameraCalc.ShownFacts(block).Select(FactRow))
            : [];

    private UIElement EntryPoint(JsonObject entry)
    {
        var button = new Button
        {
            HorizontalAlignment = HorizontalAlignment.Stretch,
            HorizontalContentAlignment = HorizontalAlignment.Stretch,
            Content = PlanFacts.Pair(PlanFacts.Text(MapUiJson.Text(entry["label"])), PlanFacts.Text(MapUiJson.Text(entry["value"]), "TextSecondaryBrush")),
        };
        button.Click += (_, _) => Wrote(PlanCalls.Refusal(PlanCalls.Invoke(MapUiJson.Text(entry["path"]))));
        return button;
    }

    private IEnumerable<UIElement> Statistics()
    {
        var view = PlanCalls.Read($"view.surveyStats({_index})");
        if (!MapUiJson.Flag(view["available"])) return [];
        string Stated(string key) => MapUiJson.Text(view[key]) is var text && text != Absent ? text : string.Empty;
        var structure = view["structure"] as JsonObject;
        var rows = (structure != null
                ? new[] { ("Layers", "layers"), ("Layer height", "layerHeight"), ("Top layer altitude", "top"), ("Bottom layer altitude", "bottom") }
                    .Select(row => (row.Item1, MapUiJson.Text(structure[row.Item2])))
                : new[] { ("Area", Stated("areaText")), ("Distance", Stated("distanceText")) })
            .Concat([("Photos", MapUiJson.Text(view["shotsText"])), ("Photo interval", Stated("intervalText"))])
            .Select(row => (UIElement)PlanFacts.Pair(PlanFacts.Text(row.Item1), PlanFacts.Text(row.Item2.Length > 0 ? row.Item2 : Absent, "TextSecondaryBrush")));
        var warning = MapUiJson.Text(view["warning"]) is { Length: > 0 } text ? new UIElement[] { PlanFacts.Warning(text) } : [];
        return warning.Append(PlanFacts.Heading("Statistics")).Concat(rows);
    }
}
