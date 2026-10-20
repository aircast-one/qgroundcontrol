using System;
using System.Collections.Generic;
using System.Collections.Immutable;
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

internal sealed class OfflineMapsPanel : UserControl
{
    public const string View = "view.offlineMaps";
    private const string Resume = "offlineMaps.resume";
    private const string Cancel = "offlineMaps.cancel";
    private const string Delete = "offlineMaps.delete";
    private const string Rename = "offlineMaps.rename";
    private const string ExportSets = "offlineMaps.export";
    private const string ImportSets = "offlineMaps.import";
    private const string Subject = "The tile cache";
    private const string Extension = ".qgctiledb";

    private readonly StackPanel _sets = new();
    private readonly StackPanel _available = new();
    private readonly TextBlock _unavailable = FormLook.Footnote("");
    private readonly TextBlock _refusal = FormLook.Caption("", "CriticalBrush");
    private readonly TextBlock _transfer = FormLook.Caption("");
    private readonly ProgressBar _working = new() { IsIndeterminate = true, Visibility = Visibility.Collapsed };
    private readonly Button _import;
    private readonly Button _export;
    private readonly Poller _poll;
    private readonly SettingsNav _nav;
    private IReadOnlyList<OfflineSet> _read = [];
    private ImmutableDictionary<long, Button> _rows = ImmutableDictionary<long, Button>.Empty;
    private string? _order;
    private Action<OfflineSet>? _refreshShown;

    public OfflineMapsPanel(SettingsNav nav)
    {
        _nav = nav;
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
        var seed = await FormWrites.Off(OfflineSeed.Read);
        _nav.Push("Add new set", new OfflineSetEditor(seed, () =>
        {
            _nav.Pop();
            _poll.Now();
        }));
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

    private sealed class Holder<T>(T value)
    {
        public T Value { get; set; } = value;
    }
}
