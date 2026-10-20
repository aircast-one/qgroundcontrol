using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Controls;
using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.Windows.Storage.Pickers;

namespace AircastQGC.Pages.Plan;

public sealed partial class PlanPage
{
    private const string PlanRoot = "plan";
    private const string MissionRoot = "plan.missionController";
    private const string CreateFromTemplate = "plan.createFromTemplate";
    private const string LoadVehiclePlan = "core.plan.loadVehiclePlan";
    private const string KeepCurrentPlan = "core.plan.keepCurrentPlan";
    private const string ApplyDefaultAltitude = "core.plan.applyDefaultAltitude";
    private const string DismissAltitudePrompt = "core.plan.dismissAltitudePrompt";
    private const string OffsetMission = "plan.missionController.offsetMission";
    private const string RotateMission = "plan.missionController.rotateMission";
    private const string TransformView = "view.planTransform";
    private const string VehiclesView = "view.vehicles";
    private const string MissionItemsView = "view.missionItems";
    private const string BlankTemplate = "Blank mission";
    private const string DefaultName = "mission";
    private const string Untitled = "Untitled Plan";
    private const string PlanExtension = ".plan";
    private const string KmlExtension = ".kml";
    private const string FlightSpeedUsed = "The mission flight speed is used instead";
    private static readonly string[] OpenTypes = [".plan", ".mission", ".waypoints", ".txt"];
    private static readonly string[] BoundaryTypes = [".kml", ".shp", ".shx", ".dbf", ".prj"];
    private static readonly string[] DefaultSpeeds = ["cruise", "hover", "ascent", "descent"];
    private static readonly HashSet<string> SpeedsSetByFlightSpeed = ["cruise", "hover"];
    private static readonly TimeSpan SyncGrace = TimeSpan.FromSeconds(3);

    private sealed record Choice(string? Name);

    private sealed record DistanceUnit(string Name, double MetresPerUnit);

    private (DateTime Since, bool Synced)? _fitWait;

    private void OnPill(object sender, RoutedEventArgs e)
    {
        if (_status.Offline) Save();
        else if (PlanRules.UploadLabel(_status) == PlanRules.Uploaded) FlyRequested?.Invoke(this, EventArgs.Empty);
        else Upload();
    }

    private async void Upload()
    {
        var aircraft = PlanCalls.VehiclePosition();
        if (aircraft is { } at && PlanRules.FarFromAircraft(at, _items) is double metres)
        {
            var move = await _dialogs.Choose(
                PlanRules.StartsFromAircraft(metres, MapUiJson.Flag(_summary["imperial"])),
                PlanRules.FarFromAircraftText, "Move it to the aircraft", "Upload anyway");
            if (move == true)
                Bridged("Moving the route to the aircraft", () => PlanCalls.Reposition(at), () => Recentre(at), "Route moved to the aircraft");
            if (move != false) return;
        }
        await SendUpload();
    }

    private async Task SendUpload()
    {
        if (PlanRules.SyncRefusal(_status, "upload to") is { } refusal)
        {
            Say(refusal);
            return;
        }
        var fresh = PlanStatus.From(PlanCalls.Read(PlanCalls.PlanView));
        switch (PlanRules.Step(fresh))
        {
            case RefuseStep refuse:
                FocusNotReady(fresh);
                Say(refuse.Reason);
                break;
            case ConfirmStep { Gate: var gate }:
                if (await _dialogs.Confirm(UploadHeading(gate), gate.Refusal, gate.ProceedTitle.Length > 0 ? gate.ProceedTitle : "Upload"))
                    Send(gate.PausesFirst);
                break;
            default:
                Send(false);
                break;
        }
    }

    private static string UploadHeading(UploadGate gate)
    {
        var asked = PlanRules.UploadHeading(gate);
        var vehicles = PlanCalls.Read(VehiclesView);
        var target = MapUiJson.Flag(vehicles["ambiguous"])
            ? MapUiJson.Objects(vehicles["vehicles"])
                .Where(vehicle => MapUiJson.Flag(vehicle["active"]))
                .Select(vehicle => MapUiJson.Text(vehicle["name"]) is { Length: > 0 } name ? name : PlanRules.Invariant($"Vehicle {MapUiJson.Number(vehicle["id"]) ?? 0}"))
                .FirstOrDefault()
            : null;
        return target == null ? asked : asked.EndsWith('?') ? $"{asked[..^1]} to {target}?" : $"{asked} to {target}";
    }

    private void Send(bool pauseFirst)
    {
        if (pauseFirst) PlanCalls.Invoke("vehicle.pauseVehicle");
        Say(PlanCalls.Invoke($"{PlanRoot}.sendToVehicle").Ok ? "Upload sent to vehicle" : "Upload did not start");
    }

    private async void OnEmptyDownload(object sender, RoutedEventArgs e)
    {
        if (PlanRules.SyncRefusal(_status, "download from") is { } refusal)
        {
            Say(refusal);
            return;
        }
        if (_status.Dirty && !await _dialogs.Confirm("Load plan from vehicle?", PlanRules.ReplaceWarning(ItemCount), "Replace", "Keep mine")) return;
        Select(null, false);
        Discard("loadFromVehicle", "Download requested from vehicle", "Download did not start", fit: true);
    }

    private async void OnDownload(object sender, RoutedEventArgs e)
    {
        if (_status.Dirty && !await Confirmed(PlanConfirm.Download)) return;
        Select(null, false);
        Discard("loadFromVehicle", "Loading the plan from the vehicle.", "The plan could not be loaded from the vehicle.", fit: true);
    }

    private async void OnClearMission(object sender, RoutedEventArgs e)
    {
        if (!await Confirmed(PlanConfirm.ClearMission)) return;
        Select(null, false);
        Discard("removeAllFromVehicle", "Clear sent to the vehicle.", "The clear could not be sent to the vehicle.");
    }

    private Task<bool> Confirmed(PlanConfirm kind)
    {
        var copy = PlanRules.Confirm(kind);
        return _dialogs.Confirm(copy.Title, copy.Body, copy.Confirm, "Keep editing", copy.Destructive);
    }

    private void Discard(string method, string success, string failure, bool fit = false)
    {
        var done = PlanCalls.Invoke($"{PlanRoot}.{method}").Ok;
        if (done && fit) _fitWait = (DateTime.UtcNow, false);
        Say(done ? success : failure);
        Changed();
    }

    private void FitIfSettled()
    {
        if (_fitWait is not { } wait) return;
        if (_status.Syncing)
        {
            _fitWait = (wait.Since, true);
            return;
        }
        if (!wait.Synced && DateTime.UtcNow - wait.Since < SyncGrace) return;
        _fitWait = null;
        Fit(AllPoints());
    }

    private void OnNewPlan(object sender, RoutedEventArgs e) => NewPlan();

    private void OnTemplates(object sender, RoutedEventArgs e) => NewPlan();

    private async void NewPlan()
    {
        var replacing = _status.ContainsItems;
        var choices = _status.Templates
            .Where(name => name != _status.BlankTemplate)
            .Select(name => (Value: new Choice(name), Label: MapUiJson.SentenceCase(name), Enabled: _status.TemplatesEnabled))
            .Prepend((Value: new Choice(null), Label: BlankTemplate, Enabled: true))
            .ToList();
        var picked = await _dialogs.Pick("New plan",
            replacing ? "Replaces the plan you are editing. Templates start at the map centre." : "Templates start at the map centre.",
            choices);
        if (picked == null || (replacing && !await Confirmed(PlanConfirm.NewPlan))) return;
        StartPlan(picked.Name);
    }

    private void StartPlan(string? template)
    {
        if (template == null)
        {
            Select(null, false);
            Discard("removeAll", "New plan.", "The plan could not be cleared.");
            return;
        }
        if (MapCentre() is not { } centre)
        {
            Say("The map has not reported its centre yet.");
            return;
        }
        var outcome = PlanCalls.Invoke(CreateFromTemplate, template, centre.Latitude, centre.Longitude);
        if (!outcome.Ok)
        {
            Say(outcome.Reason);
            return;
        }
        Select(null, false);
        _newPattern = NewestPattern;
        Changed();
    }

    private void OnOpen(object sender, RoutedEventArgs e) => Open();

    private async void Open()
    {
        if (!_status.CanOpen || (_status.Dirty && !await Confirmed(PlanConfirm.Open))) return;
        if (await PickOpen(OpenTypes) is not { } path) return;
        var outcome = PlanCalls.Invoke($"{PlanRoot}.loadFromFile", path);
        if (!outcome.Ok)
        {
            Say(outcome.Reason);
            return;
        }
        Select(null, false);
        await Changed();
        Fit(AllPoints());
        Say("Plan opened.");
    }

    private void OnSave(object sender, RoutedEventArgs e) => Save();

    private void OnSaveAs(object sender, RoutedEventArgs e) => Guarded(SaveAs);

    private void Save()
    {
        if (!_status.CanSave) return;
        Guarded(async () =>
        {
            if (KnownPlanFile() is { } path) WriteTo(path);
            else await SaveAs();
        });
    }

    private string? KnownPlanFile() =>
        _status.FilePath is { } path && path.EndsWith(PlanExtension, StringComparison.OrdinalIgnoreCase) ? path : null;

    private async Task SaveAs()
    {
        var title = PlanRules.PlanTitle(_status.File);
        if (await PickSave("Plan", PlanExtension, title == Untitled ? DefaultName : title) is { } path) WriteTo(path);
    }

    private void WriteTo(string path) =>
        Say(PlanCalls.Invoke($"{PlanRoot}.saveToFile", path).Ok ? "Plan saved." : "The plan could not be saved.");

    private void OnExportKml(object sender, RoutedEventArgs e) => Guarded(async () =>
    {
        if (await PickSave("KML", KmlExtension, DefaultName) is not { } path) return;
        var before = File.Exists(path) ? File.GetLastWriteTimeUtc(path) : DateTime.MinValue;
        PlanCalls.Invoke($"{PlanRoot}.saveToKml", path);
        var written = new FileInfo(path);
        Say(written.Exists && written.Length > 0 && written.LastWriteTimeUtc > before ? "KML exported." : "The plan could not be exported.");
    });

    private async void Guarded(Func<Task> action)
    {
        var fresh = PlanStatus.From(PlanCalls.Read(PlanCalls.PlanView));
        if (fresh.NotReady is { } blocked)
        {
            FocusNotReady(fresh);
            Say(blocked);
            return;
        }
        await action();
    }

    private async void OnImportBoundary(object sender, RoutedEventArgs e)
    {
        var paths = await PickMany(BoundaryTypes);
        if (paths.Count == 0) return;
        var patterns = _status.Patterns;
        if (patterns.Count == 0)
        {
            Say("This vehicle offers no pattern to import into.");
            return;
        }
        var pattern = patterns.Count == 1
            ? patterns[0]
            : (await _dialogs.Pick("Import as which pattern?", string.Empty, patterns.Select(name => (Value: new Choice(name), Label: name, Enabled: true)).ToList()))?.Name;
        if (pattern != null) ImportFrom(paths, pattern);
    }

    private void ImportFrom(IReadOnlyList<string> paths, string pattern)
    {
        var main = paths.FirstOrDefault(path => path.EndsWith(".shp", StringComparison.OrdinalIgnoreCase))
            ?? paths.FirstOrDefault(path => path.EndsWith(KmlExtension, StringComparison.OrdinalIgnoreCase))
            ?? paths[0];
        var label = Path.GetFileName(main);
        var before = VisualItems().Count;
        PlanCalls.Invoke($"{MissionRoot}.insertComplexMissionItemFromKMLOrSHP", pattern, main, before, true);
        var after = VisualItems();
        var empty = after.Count > before && MapUiJson.Number(after[^1]["patternDistance"]) is not > 0;
        if (empty) PlanCalls.Invoke($"{MissionRoot}.removeVisualItem", after.Count - 1);
        Say(after.Count <= before ? $"{label} added nothing to the plan." : empty ? $"{label} holds no area for a {pattern}." : "Boundary imported.");
        Changed();
    }

    private static IReadOnlyList<JsonObject> VisualItems() => MapUiJson.Objects(PlanCalls.Read(MissionItemsView)["items"]);

    private WindowId? AppWindow => XamlRoot?.ContentIslandEnvironment?.AppWindowId;

    private async Task<T?> Picked<T>(Func<WindowId, Task<T?>> pick) where T : class
    {
        if (AppWindow is not { } window) return null;
        try { return await pick(window); }
        catch (Exception)
        {
            Say("The file picker could not open.");
            return null;
        }
    }

    private Task<string?> PickOpen(IEnumerable<string> types) => Picked(async window =>
    {
        var picker = new FileOpenPicker(window) { SuggestedStartLocation = PickerLocationId.DocumentsLibrary };
        types.ToList().ForEach(picker.FileTypeFilter.Add);
        return (await picker.PickSingleFileAsync())?.Path;
    });

    private async Task<IReadOnlyList<string>> PickMany(IEnumerable<string> types) => await Picked<IReadOnlyList<string>>(async window =>
    {
        var picker = new FileOpenPicker(window) { SuggestedStartLocation = PickerLocationId.DocumentsLibrary };
        types.ToList().ForEach(picker.FileTypeFilter.Add);
        return (await picker.PickMultipleFilesAsync()).Select(result => result.Path).ToList();
    }) ?? [];

    private Task<string?> PickSave(string label, string extension, string name) => Picked(async window =>
    {
        var picker = new FileSavePicker(window)
        {
            SuggestedStartLocation = PickerLocationId.DocumentsLibrary,
            SuggestedFileName = name,
            DefaultFileExtension = extension,
        };
        picker.FileTypeChoices.Add(label, new List<string> { extension });
        return (await picker.PickSaveFileAsync())?.Path;
    });

    private async void Prompts()
    {
        if (_dialogs.Showing || XamlRoot == null) return;
        if (_status.VehiclePrompt is { } prompt)
        {
            if (await _dialogs.Choose(prompt.Title, prompt.Text, prompt.Primary, prompt.Secondary, dismissable: false) is not bool primary) return;
            Answered(primary == prompt.PrimaryKeeps ? KeepCurrentPlan : LoadVehiclePlan);
            return;
        }
        if (_status.AltitudePrompt is not { } asked) return;
        Answered(await _dialogs.Confirm(MapUiJson.SentenceCase(asked.Title), asked.Text, "Yes", "No") ? ApplyDefaultAltitude : DismissAltitudePrompt);
    }

    private void Answered(string invoke)
    {
        PlanCalls.Invoke(invoke);
        _status = PlanStatus.From(PlanCalls.Read(PlanCalls.PlanView));
        Changed();
    }

    private void RenderUndrawn()
    {
        UndrawnText.Text = PlanRules.UndrawnWarning(PlanRules.Undrawn(_items)) ?? string.Empty;
        UndrawnCard.Visibility = Shown(UndrawnText.Text.Length > 0);
    }

    private void OnMenuOpening(object sender, object e)
    {
        RedoItem.IsEnabled = _status.CanRedo;
        OpenItem.IsEnabled = _status.CanOpen;
        SaveItem.IsEnabled = _status.CanSave;
        SaveAsItem.IsEnabled = _status.CanSave;
        ExportItem.IsEnabled = _status.CanExportKml;
        ImportItem.IsEnabled = _status.CanOpen;
        TransformItem.IsEnabled = _status.ContainsItems;
        NewItem.IsEnabled = _status.CanNewPlan;
        DownloadItem.IsEnabled = _status.CanDownload;
        ClearItem.IsEnabled = _status.CanClear;
    }

    private void OnUndo(object sender, RoutedEventArgs e) => Undo();

    private void OnRedo(object sender, RoutedEventArgs e) => Redo();

    private void OnZoomIn(object sender, RoutedEventArgs e) => Map.ZoomIn();

    private void OnZoomOut(object sender, RoutedEventArgs e) => Map.ZoomOut();

    private void OnCentreOpening(object sender, object e)
    {
        var launch = _items.FirstOrDefault(item => item.Sequence == 0)?.At;
        var vehicle = PlanCalls.VehiclePosition();
        CentreMenu.Items.Clear();
        var follow = new ToggleMenuFlyoutItem { Text = "Follow vehicle", IsChecked = Map.Follow, IsEnabled = vehicle != null || Map.Follow };
        follow.Click += (_, _) => Follow(follow.IsChecked);
        CentreMenu.Items.Add(follow);
        new (string Label, bool Enabled, Action Act)[]
        {
            ("Mission", MissionPoints().Any(), () => Fit(MissionPoints())),
            ("All items", AllPoints().Any(), () => Fit(AllPoints())),
            ("Launch", launch != null, () => CentreOn(launch)),
            ("Vehicle", vehicle != null, () => CentreOn(vehicle)),
            ("Coordinates…", true, AskCoordinates),
        }.Select(entry => MenuEntry(entry.Label, entry.Enabled, entry.Act)).ToList().ForEach(CentreMenu.Items.Add);
    }

    private static MenuFlyoutItem MenuEntry(string text, bool enabled, Action act)
    {
        var entry = new MenuFlyoutItem { Text = text, IsEnabled = enabled };
        entry.Click += (_, _) => act();
        return entry;
    }

    private void CentreOn(Geo? at)
    {
        if (at is { } point) Recentre(point, Math.Max(Map.Zoom, FieldZoom));
    }

    private async void AskCoordinates() =>
        CentreOn(await _dialogs.EditPosition(MapCentre() ?? new Geo(0, 0), false, "Center map on coordinate", "Center"));

    private void OnMapTypeOpening(object sender, object e)
    {
        var view = PlanCalls.Read(PlanCalls.MapTypesView);
        var current = MapUiJson.Text(view["current"]);
        var path = MapUiJson.Text(view["path"]);
        var types = MapUiJson.Texts(view["types"]);
        MapTypeMenu.Items.Clear();
        types.Select(type =>
            {
                var entry = new ToggleMenuFlyoutItem { Text = type, IsChecked = type == current, IsEnabled = path.Length > 0 };
                entry.Click += (_, _) =>
                {
                    if (PlanCalls.Refusal(PlanCalls.Set(path, type)) is { } refused) Say(refused);
                };
                return (MenuFlyoutItemBase)entry;
            })
            .DefaultIfEmpty(new MenuFlyoutItem { Text = "No map types", IsEnabled = false })
            .ToList()
            .ForEach(MapTypeMenu.Items.Add);
    }

    private async void OnDefaults(object sender, RoutedEventArgs e) => await _dialogs.Content("Plan defaults", _ =>
    {
        var panel = new StackPanel { Spacing = 8 };
        void Fill()
        {
            panel.Children.Clear();
            DefaultsRows(PlanStatus.From(PlanCalls.Read(PlanCalls.PlanView)).Defaults, Fill).ToList().ForEach(panel.Children.Add);
        }
        Fill();
        return panel;
    });

    private static IEnumerable<UIElement> DefaultsRows(JsonObject defaults, Action refill)
    {
        var speed = defaults["flightSpeed"] as JsonObject;
        var specified = MapUiJson.Flag(speed?["specified"]);
        var altitude = defaults["altitude"] as JsonObject;
        var speeds = DefaultSpeeds
            .Select(key => (Key: key, Control: defaults[key] as JsonObject))
            .Where(entry => entry.Control != null)
            .Select(entry => specified && SpeedsSetByFlightSpeed.Contains(entry.Key) ? Disabled(entry.Control!, FlightSpeedUsed) : entry.Control!)
            .ToList();
        var note = altitude == null && speeds.Count == 0 ? "This core does not report the plan's defaults." : MapUiJson.Text(defaults["speedNote"]);
        return new IEnumerable<UIElement>[]
        {
            note.Length > 0 ? [PlanFacts.Note(note)] : [],
            altitude != null ? [DefaultRow(altitude)] : [],
            speed != null && MapUiJson.Flag(speed["available"], true) ? [FlightSpeed(speed, refill)] : [],
            speeds.Select(DefaultRow),
        }.SelectMany(rows => rows);
    }

    private static UIElement DefaultRow(JsonObject control) =>
        PlanFacts.Row(control, (path, value) => PlanCalls.Refusal(PlanCalls.Set(path, value)), () => { });

    private static JsonObject Disabled(JsonObject control, string reason)
    {
        var copy = (JsonObject)control.DeepClone();
        copy["enabled"] = false;
        copy["disabledReason"] = reason;
        return copy;
    }

    private static UIElement FlightSpeed(JsonObject speed, Action refill)
    {
        var specified = MapUiJson.Flag(speed["specified"]);
        var refusal = PlanFacts.Error(string.Empty);
        void Show(string? refused)
        {
            refusal.Text = refused ?? string.Empty;
            refusal.Visibility = Shown(refused != null);
        }
        Show(null);
        var toggle = new ToggleSwitch { IsOn = specified, OnContent = string.Empty, OffContent = string.Empty, MinWidth = 0 };
        toggle.Toggled += (_, _) =>
        {
            var refused = PlanCalls.Refusal(PlanCalls.Set(MapUiJson.Text(speed["specifyPath"]), toggle.IsOn));
            if (refused == null) refill();
            else Show(refused);
        };
        var rows = new StackPanel { Spacing = 4, Children = { PlanFacts.Pair(PlanFacts.Text("Flight speed"), toggle) } };
        if (specified)
        {
            var shown = MapUiJson.Number(speed["value"]) is double value ? PlanRules.Plain(Math.Round(value, 2)) : string.Empty;
            var entry = PlanFacts.Entry(shown, typed =>
                Show(PlanRules.TypedNumber(typed) is double wanted ? PlanCalls.Refusal(PlanCalls.Set(MapUiJson.Text(speed["path"]), wanted)) : "Not a speed"), 110);
            rows.Children.Add(PlanFacts.Pair(PlanFacts.Text(string.Join(' ', new[] { "Speed", MapUiJson.Text(speed["units"]) }.Where(text => text.Length > 0))), entry));
        }
        rows.Children.Add(refusal);
        return rows;
    }

    private async void OnTransform(object sender, RoutedEventArgs e)
    {
        var view = PlanCalls.Read(TransformView);
        var home = MapOverlayJson.Plottable(view["home"]);
        var repositioning = false;
        await _dialogs.Content("Transform", hide => TransformPanel(view, home, () =>
        {
            repositioning = true;
            hide();
        }), "Close");
        if (!repositioning || home is not { } from) return;
        if (await _dialogs.EditPosition(from, PlanCalls.HasVehicle(), "Reposition mission", "Move to Position", "Move to Vehicle Position") is not { } to) return;
        var outcome = PlanCalls.Invoke(PlanCalls.RepositionMission, PlanCalls.Coordinate(to));
        Say(outcome.Ok ? "Mission moved" : outcome.Reason);
        Changed();
    }

    private UIElement TransformPanel(JsonObject view, Geo? home, Action reposition)
    {
        var horizontal = Unit(view, "horizontal");
        var vertical = Unit(view, "vertical");
        var refusal = PlanFacts.Error(string.Empty);
        void Apply(string path, IEnumerable<JsonNode?> args)
        {
            var outcome = PlanCalls.Invoke(path, args.ToArray());
            refusal.Text = outcome.Ok ? string.Empty : outcome.Reason;
            Changed();
        }
        static TextBox Number(string header) => new() { Header = header, Text = "0" };
        static CheckBox Check(string label) => new() { Content = label };
        var east = Number($"East ({horizontal.Name})");
        var north = Number($"North ({horizontal.Name})");
        var up = Number($"Up ({vertical.Name})");
        var offsetTakeoff = Check("Also move takeoff items");
        var offsetLanding = Check("Also move landing items");
        var offset = PlanFacts.Action("Apply offset", () =>
        {
            var metres = new[] { (Box: east, Unit: horizontal), (Box: north, Unit: horizontal), (Box: up, Unit: vertical) }
                .Select(axis => PlanRules.TypedNumber(axis.Box.Text.Trim().Length == 0 ? "0" : axis.Box.Text) * axis.Unit.MetresPerUnit)
                .ToList();
            if (metres.Any(value => value == null)) refusal.Text = "Enter a number for each offset.";
            else Apply(OffsetMission, metres.Select(value => (JsonNode?)value!.Value).Append(offsetTakeoff.IsChecked == true).Append(offsetLanding.IsChecked == true));
        });
        var degrees = Number("Clockwise (deg)");
        var rotateTakeoff = Check("Also move takeoff items");
        var rotateLanding = Check("Also move landing items");
        var rotate = PlanFacts.Action("Apply rotation", () =>
        {
            if (PlanRules.TypedNumber(degrees.Text) is double turn) Apply(RotateMission, [turn, rotateTakeoff.IsChecked == true, rotateLanding.IsChecked == true]);
            else refusal.Text = "Enter the rotation in degrees.";
        });
        var move = PlanFacts.Action("Move to Position", reposition);
        move.IsEnabled = home != null;
        rotate.IsEnabled = home != null;
        UIElement? Homeless(string action) => home == null ? PlanFacts.Note($"Home position must be set to {action} the mission.") : null;
        var panel = new StackPanel { Spacing = 8 };
        new UIElement?[]
        {
            PlanFacts.Heading("Offset mission"), east, north, up, offsetTakeoff, offsetLanding,
            PlanFacts.Note("Note: Home altitude is not modified."), offset,
            PlanFacts.Heading("Reposition mission"), Homeless("reposition"), move,
            PlanFacts.Heading("Rotate mission"), Homeless("rotate"), degrees, rotateTakeoff, rotateLanding,
            PlanFacts.Note("Note: Complex items are rotated by moving their reference coordinate: their geometry and orientation are not changed."), rotate,
            refusal,
        }.OfType<UIElement>().ToList().ForEach(panel.Children.Add);
        return panel;
    }

    private static DistanceUnit Unit(JsonObject view, string axis) =>
        MapUiJson.Number(view[$"{axis}MetresPerUnit"]) is double metres && metres > 0
            ? new DistanceUnit(MapUiJson.Text(view[$"{axis}Unit"]) is { Length: > 0 } name ? name : "m", metres)
            : new DistanceUnit("m", 1);
}
