using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Pages.Settings;

internal sealed record OfflineSet(
    long Id,
    string Name,
    string MapType,
    bool DefaultSet,
    string ZoomText,
    string TotalText,
    string UniqueText,
    long UniqueCount,
    string DownloadedText,
    string SizeText,
    string TileCountText,
    long ErrorCount,
    string ErrorCountText,
    bool Downloading,
    bool Complete,
    string Subtitle,
    string RowText,
    bool CanDelete)
{
    public static OfflineSet Parse(JsonObject set) => new(
        Panels.Whole(set["id"]),
        MapUiJson.Text(set["name"]),
        MapUiJson.Text(set["mapTypeStr"]),
        MapUiJson.Flag(set["defaultSet"]),
        MapUiJson.Text(set["zoomText"]),
        MapUiJson.Text(set["totalText"]),
        MapUiJson.Text(set["uniqueText"]),
        Panels.Whole(set["uniqueCount"]),
        MapUiJson.Text(set["downloadedText"]),
        MapUiJson.Text(set["sizeText"]),
        MapUiJson.Text(set["tileCountText"]),
        Panels.Whole(set["errorCount"]),
        MapUiJson.Text(set["errorCountText"]),
        MapUiJson.Flag(set["downloading"]),
        MapUiJson.Flag(set["complete"]),
        MapUiJson.Text(set["subtitle"]),
        MapUiJson.Text(set["rowText"]),
        MapUiJson.Flag(set["canDelete"]));
}

internal sealed record OfflineRegion(double West, double North, double East, double South)
{
    private const double KmPerDegree = 111.32;
    private const double MaxLatitude = 85;

    public static OfflineRegion? Around(double latitude, double longitude, double km) =>
        double.IsFinite(latitude) && double.IsFinite(longitude) && km > 0 && Math.Abs(latitude) <= MaxLatitude && Math.Abs(longitude) <= 180
            ? Square(latitude, longitude, km / 2 / KmPerDegree)
            : null;

    private static OfflineRegion Square(double latitude, double longitude, double halfLatitude) =>
        Spanned(latitude, longitude, halfLatitude, halfLatitude / Math.Cos(latitude * Math.PI / 180));

    private static OfflineRegion Spanned(double latitude, double longitude, double halfLatitude, double halfLongitude) => new(
        longitude - halfLongitude,
        Math.Min(latitude + halfLatitude, MaxLatitude),
        longitude + halfLongitude,
        Math.Max(latitude - halfLatitude, -MaxLatitude));

    public string Args => string.Join(",", new[] { West, North, East, South }.Select(c => c.ToString("F7", CultureInfo.InvariantCulture)));
}

internal sealed class OfflineMapsPanel : UserControl
{
    public const string View = "view.offlineMaps";
    private const string Start = "offlineMaps.startDownload";
    private const string Resume = "offlineMaps.resume";
    private const string Cancel = "offlineMaps.cancel";
    private const string Delete = "offlineMaps.delete";
    private const string Rename = "offlineMaps.rename";
    private const string ExportSets = "offlineMaps.export";
    private const string ImportSets = "offlineMaps.import";
    private const string MinZoomSetting = "settings.offlineMapsSettings.minZoomLevelDownload";
    private const string MaxZoomSetting = "settings.offlineMapsSettings.maxZoomLevelDownload";
    private const string Subject = "The tile cache";
    private const string Extension = ".qgctiledb";
    private const int DefaultMinZoom = 13;
    private const int DefaultMaxZoom = 19;
    private const int LowestZoom = 1;
    private const int HighestZoom = 20;
    private const double DefaultAreaKm = 2;

    private readonly StackPanel _sets = new();
    private readonly StackPanel _available = new();
    private readonly TextBlock _unavailable = FormLook.Footnote("");
    private readonly TextBlock _refusal = FormLook.Caption("", "CriticalBrush");
    private readonly TextBlock _transfer = FormLook.Caption("");
    private readonly ProgressBar _working = new() { IsIndeterminate = true, Visibility = Visibility.Collapsed };
    private readonly Button _import;
    private readonly Button _export;
    private readonly Poller _poll;
    private IReadOnlyList<OfflineSet> _read = [];
    private ImmutableDictionary<long, Button> _rows = ImmutableDictionary<long, Button>.Empty;
    private string? _order;
    private Action<OfflineSet>? _refreshShown;

    public OfflineMapsPanel()
    {
        Margin = new Thickness(0, 12, 0, 0);
        _import = FormLook.Button("Import…", ImportTiles);
        _export = FormLook.Button("Export…", ExportTiles);
        FormLook.Fill(_available,
        [
            _sets,
            FormLook.Margined(FormLook.Button("Add new set…", AddSet, ButtonLook.Accent, "\uE710"), new Thickness(0, 12, 0, 12)),
            FormLook.Card(FormLook.Row(FormLook.Text("Import map tiles"), _import), FormLook.Row(FormLook.Text("Export map tiles"), _export)),
            _transfer,
            _working,
        ]);
        Content = FormLook.Stack(4, FormLook.Heading("Tile sets"), _unavailable, _available, _refusal);
        _poll = Live.Changes(this, Panels.Second, () => Bridge.Get(View), Render);
    }

    private void Render(JsonObject view)
    {
        var available = MapUiJson.Flag(view["available"]);
        _unavailable.Visibility = FormLook.Shown(!available);
        _available.Visibility = FormLook.Shown(available);
        FormLook.Say(_unavailable, available ? null : MapUiJson.Text(view["reason"]) is { Length: > 0 } reason ? reason : "There is no map tile cache on this device.");
        _read = MapUiJson.Objects(view["sets"]).Select(OfflineSet.Parse).ToList();
        var order = string.Join("|", _read.Select(set => $"{set.Id}:{set.Name}"));
        if (order == _order)
            _read.ToList().ForEach(set => _rows[set.Id].Content = RowContent(set));
        else
        {
            _order = order;
            _rows = _read.ToImmutableDictionary(set => set.Id, SetRow);
            FormLook.Fill(_sets, _read.Count == 0 ? [] : [FormLook.Card(_read.Select(set => (UIElement)_rows[set.Id]))]);
        }
        _read.ToList().ForEach(set => _refreshShown?.Invoke(set));
    }

    private Button SetRow(OfflineSet set) =>
        FormLook.Stretched(RowContent(set), set.Name, () => Show(_read.FirstOrDefault(read => read.Id == set.Id) ?? set));

    private static FrameworkElement RowContent(OfflineSet set) => FormLook.Row(
        FormLook.Text(set.Name),
        FormLook.Line(8,
            set.Downloading ? new ProgressRing { IsActive = true, Width = 16, Height = 16 } : null,
            FormLook.Text(set.RowText, "FontSizeBody", "TextSecondaryBrush"),
            set.RowText.Length > 0 ? FormLook.Dot(set.Complete ? "SuccessBrush" : "CriticalBrush") : null,
            FormLook.Glyph("\uE76C", 12)));

    private async void Act(string path, params object?[] args)
    {
        FormLook.Say(_refusal, await FormWrites.Off(() => FormWrites.Call(path, Subject, args)));
        _poll.Now();
    }

    private async void Show(OfflineSet picked)
    {
        if (XamlRoot is null) return;
        var name = FormLook.Field(picked.Name, "Name");
        var details = new StackPanel { Spacing = 6 };
        var after = new Holder<Action?>(null);
        var dialog = FormLook.Dialog(XamlRoot, picked.Name, FormLook.Stack(12, picked.DefaultSet ? null : name, details),
            picked.DefaultSet ? null : "OK", picked.DefaultSet ? "Close" : "Cancel");
        void Then(Action action)
        {
            after.Value = action;
            dialog.Hide();
        }
        void Fill(OfflineSet set) => FormLook.Fill(details, Details(set, Then));
        name.TextChanged += (_, _) => dialog.IsPrimaryButtonEnabled = name.Text.Trim().Length > 0;
        _refreshShown = set =>
        {
            if (set.Id == picked.Id) Fill(set);
        };
        Fill(picked);
        var result = await FormLook.Show(dialog);
        _refreshShown = null;
        if (result == ContentDialogResult.Primary && name.Text.Trim() is { Length: > 0 } wanted && wanted != picked.Name) Act(Rename, picked.Id, wanted);
        after.Value?.Invoke();
    }

    private IEnumerable<UIElement?> Details(OfflineSet set, Action<Action> then)
    {
        UIElement Info(string label, string value) => FormLook.Line(8,
            FormLook.Margined(FormLook.Text(label, "FontSizeBody", "TextSecondaryBrush"), new Thickness(0)),
            FormLook.Text(value));
        var resume = !set.DefaultSet && !set.Complete && !set.Downloading ? FormLook.Button("Resume download", () => then(() => Act(Resume, set.Id))) : null;
        var cancel = !set.DefaultSet && set.Downloading ? FormLook.Button("Cancel download", () => then(() => Act(Cancel, set.Id))) : null;
        var delete = FormLook.Button("Delete…", () => then(() => ConfirmDelete(set)));
        delete.IsEnabled = set.CanDelete;
        return set.DefaultSet
            ? [FormLook.Caption(set.Subtitle), Info("Size:", set.SizeText), Info("Tile count:", set.TileCountText), FormLook.Line(8, delete)]
            :
            [
                FormLook.Caption(set.MapType),
                Info("Zoom levels:", set.ZoomText),
                Info("Total:", set.TotalText),
                set.UniqueCount > 0 ? Info("Unique:", set.UniqueText) : null,
                set.Complete ? null : Info("Downloaded:", set.DownloadedText),
                !set.Complete && set.ErrorCount > 0 ? Info("Error count:", set.ErrorCountText) : null,
                FormLook.Margined(FormLook.Line(8, resume, cancel, delete), new Thickness(0, 8, 0, 0)),
            ];
    }

    private async void ConfirmDelete(OfflineSet set)
    {
        if (await FormLook.Confirm(XamlRoot, "Confirm delete",
                set.DefaultSet
                    ? "This will delete all tiles INCLUDING the tile sets you have created yourself.\n\nIs this really what you want?"
                    : $"Delete {set.Name} and all its tiles.\n\nIs this really what you want?",
                "Delete"))
            Act(Delete, set.Id);
    }

    private async void AddSet()
    {
        if (XamlRoot is null) return;
        var seed = await FormWrites.Off(() => (
            View: Bridge.Get(View),
            MapType: MapNative.CurrentType(),
            Min: (int)(MapUiJson.Number(Bridge.Get(MinZoomSetting)["value"]) ?? DefaultMinZoom),
            Max: (int)(MapUiJson.Number(Bridge.Get(MaxZoomSetting)["value"]) ?? DefaultMaxZoom),
            Centre: MapUiJson.VehiclePosition() ?? GcsCentre()));
        var maps = MapUiJson.Texts(seed.View["mapList"]);
        var name = FormLook.Field(MapUiJson.Text(seed.View["uniqueName"]), "Name");
        var mapType = FormLook.Combo(maps, maps.ToList().IndexOf(seed.MapType), 280, seed.MapType);
        var latitude = Number("Centre latitude", seed.Centre?.Latitude ?? double.NaN, -85, 85, 0.001);
        var longitude = Number("Centre longitude", seed.Centre?.Longitude ?? double.NaN, -180, 180, 0.001);
        var area = Number("Area width (km)", DefaultAreaKm, 0.1, 100, 0.5);
        var minZoom = Number("Min zoom", Math.Clamp(seed.Min, LowestZoom, HighestZoom), LowestZoom, HighestZoom, 1);
        var maxZoom = Number("Max zoom", Math.Clamp(seed.Max, LowestZoom, HighestZoom), LowestZoom, HighestZoom, 1);
        var elevation = new CheckBox { Content = "Fetch elevation data", IsChecked = true, FontFamily = PreflightLook.Font };
        var estimate = new StackPanel { Spacing = 4 };
        var error = FormLook.Caption("", "CriticalBrush");
        AutomationProperties.SetName(mapType, "Map type");
        var dialog = FormLook.Dialog(XamlRoot, "Add new set", new ScrollViewer
        {
            MaxHeight = 560,
            Content = FormLook.Stack(10,
                name,
                FormLook.Line(12, FormLook.Margined(FormLook.Text("Map type"), new Thickness(0, 6, 0, 0)), mapType),
                FormLook.Caption("The set covers a square around this point. It starts on the vehicle, or on this computer's position when no vehicle is connected."),
                FormLook.Line(12, latitude, longitude),
                area,
                FormLook.Line(12, minZoom, maxZoom),
                elevation,
                estimate,
                error),
        }, "Download");
        var version = 0;
        var taken = MapUiJson.Texts(seed.View["takenNames"]);
        int Zoom(NumberBox box) => (int)Math.Round(double.IsNaN(box.Value) ? LowestZoom : box.Value);
        string Chosen() => mapType.SelectedIndex >= 0 ? maps[mapType.SelectedIndex] : seed.MapType;
        OfflineRegion? Region() => OfflineRegion.Around(latitude.Value, longitude.Value, area.Value);
        async void Estimate()
        {
            var at = ++version;
            var (type, region, low, high, fetch) = (Chosen(), Region(), Math.Min(Zoom(minZoom), Zoom(maxZoom)), Math.Max(Zoom(minZoom), Zoom(maxZoom)), elevation.IsChecked == true);
            var read = region is null ? null : await FormWrites.Off(() =>
                Bridge.Get($"{View}({type},{region.Args},{low},{high},{(fetch ? "true" : "false")})")["estimate"] as JsonObject);
            if (at != version) return;
            var nameTaken = taken.Contains(name.Text.Trim());
            var tooMany = MapUiJson.Flag(read?["tooMany"]);
            FormLook.Fill(estimate,
            [
                region is null ? FormLook.Caption("Enter the centre of the area to download.", "CautionBrush") : null,
                read is null ? null : FormLook.Pair("Tile count", MapUiJson.Text(read["tileCountText"])),
                read is null ? null : FormLook.Pair("Est size", MapUiJson.Text(read["tileSizeText"])),
                tooMany ? FormLook.Caption("Too many tiles", "CriticalBrush") : null,
                nameTaken ? FormLook.Caption("Tile set with this name already exists", "CriticalBrush") : null,
            ]);
            dialog.IsPrimaryButtonEnabled = region is not null && read is not null && !tooMany && !nameTaken && name.Text.Trim().Length > 0;
        }
        void Zooms()
        {
            var (low, high) = (Zoom(minZoom), Zoom(maxZoom));
            FormWrites.Off(() =>
            {
                FormWrites.Set(MinZoomSetting, Math.Min(low, high));
                FormWrites.Set(MaxZoomSetting, Math.Max(low, high));
            });
            Estimate();
        }
        name.TextChanged += (_, _) => Estimate();
        mapType.SelectionChanged += (_, _) => Estimate();
        latitude.ValueChanged += (_, _) => Estimate();
        longitude.ValueChanged += (_, _) => Estimate();
        area.ValueChanged += (_, _) => Estimate();
        minZoom.ValueChanged += (_, _) => Zooms();
        maxZoom.ValueChanged += (_, _) => Zooms();
        FormLook.Toggled(elevation, _ => Estimate());
        dialog.PrimaryButtonClick += async (_, args) =>
        {
            if (Region() is not { } region)
            {
                args.Cancel = true;
                return;
            }
            var deferral = args.GetDeferral();
            var (typed, type, low, high, fetch) = (name.Text.Trim(), Chosen(), Math.Min(Zoom(minZoom), Zoom(maxZoom)), Math.Max(Zoom(minZoom), Zoom(maxZoom)), elevation.IsChecked == true);
            var refused = await FormWrites.Off(() => FormWrites.Call(Start, Subject,
                typed, type, region.West, region.North, region.East, region.South, low, high, fetch));
            FormLook.Say(error, refused);
            args.Cancel = refused is not null;
            deferral.Complete();
        };
        dialog.IsPrimaryButtonEnabled = false;
        Estimate();
        await FormLook.Show(dialog);
        _poll.Now();
    }

    private async void ExportTiles()
    {
        if (XamlRoot is null || _read.Count == 0) return;
        var chosen = new Holder<ImmutableHashSet<long>>(ImmutableHashSet<long>.Empty);
        var dialog = FormLook.Dialog(XamlRoot, "Export selected tile sets", FormLook.Stack(4, _read.Select(set => (UIElement)FormLook.Check(set.Name, false, on =>
            chosen.Value = on ? chosen.Value.Add(set.Id) : chosen.Value.Remove(set.Id)))), "OK");
        if (await FormLook.Show(dialog) != ContentDialogResult.Primary || chosen.Value.IsEmpty) return;
        if (await FilePicks.Save(XamlRoot, "export", "Tile sets", Extension) is not { } path) return;
        Transfer("Exporting", () => FormWrites.Call(ExportSets, Subject, chosen.Value.Order().Cast<object?>().Prepend(path).ToArray()));
    }

    private async void ImportTiles()
    {
        if (XamlRoot is null) return;
        var mode = new RadioButtons { ItemsSource = new[] { "Append to existing sets", "Replace existing sets" }, SelectedIndex = 0 };
        AutomationProperties.SetName(mode, "Import mode");
        if (await FormLook.Show(FormLook.Dialog(XamlRoot, "Import tile sets", mode, "OK")) != ContentDialogResult.Primary) return;
        var replace = mode.SelectedIndex == 1;
        if (await FilePicks.Open(XamlRoot, [Extension]) is not { } path) return;
        Transfer("Importing", () => FormWrites.Call(ImportSets, Subject, path, replace));
    }

    private async void Transfer(string label, Func<string?> work)
    {
        FormLook.Say(_transfer, label);
        _working.Visibility = Visibility.Visible;
        _import.IsEnabled = false;
        _export.IsEnabled = false;
        FormLook.Say(_refusal, await FormWrites.Off(work));
        FormLook.Say(_transfer, null);
        _working.Visibility = Visibility.Collapsed;
        _import.IsEnabled = true;
        _export.IsEnabled = true;
        _poll.Now();
    }

    private static (double Latitude, double Longitude)? GcsCentre()
    {
        var position = Bridge.Get(GcsPositionPanel.Position);
        var coordinate = position["value"] as JsonObject ?? position;
        return MapUiJson.Flag(coordinate["valid"])
            && MapUiJson.Number(coordinate["latitude"]) is { } latitude
            && MapUiJson.Number(coordinate["longitude"]) is { } longitude
                ? (latitude, longitude)
                : null;
    }

    private static NumberBox Number(string header, double value, double minimum, double maximum, double step)
    {
        var box = new NumberBox
        {
            Header = header,
            Value = value,
            Minimum = minimum,
            Maximum = maximum,
            SmallChange = step,
            LargeChange = step * 10,
            SpinButtonPlacementMode = NumberBoxSpinButtonPlacementMode.Compact,
            ValidationMode = NumberBoxValidationMode.InvalidInputOverwritten,
            Width = 200,
            FontFamily = PreflightLook.Font,
        };
        AutomationProperties.SetName(box, header);
        return box;
    }

    private sealed class Holder<T>(T value)
    {
        public T Value { get; set; } = value;
    }
}
