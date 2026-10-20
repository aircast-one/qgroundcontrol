using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;
using AircastQGC.Core;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Pages.Analyze;

internal sealed record LogEntry(int Index, int Id, string Time, string Size, bool Received, bool Selected, string Status, bool Downloading, bool Saved);

internal sealed record LogsView(bool Connected, IReadOnlyList<LogEntry> Entries, string EmptyText, bool CanRefresh, bool CanDownload,
    bool CanCancel, bool CanErase, bool EraseSelectedShown, bool CanEraseSelected, bool CanSort, string SortText, bool Busy,
    bool Downloading, bool AnyDownloaded, string SavePath, string SavePathReason)
{
    internal static LogsView Parse(JsonObject view) => new(
        MapUiJson.Flag(view["connected"]),
        MapUiJson.Objects(view["entries"]).Select((entry, at) => new LogEntry(
            Ui.Int(entry["index"], at),
            Ui.Int(entry["id"]),
            LogDownloadTool.TimeText(MapUiJson.Text(entry["time"]), MapUiJson.Text(entry["timeState"]), DateTime.Today),
            MapUiJson.Text(entry["sizeText"]),
            MapUiJson.Flag(entry["received"]),
            MapUiJson.Flag(entry["selected"]),
            MapUiJson.Text(entry["status"]),
            MapUiJson.Text(entry["statusId"]) == "downloading",
            MapUiJson.Text(entry["statusId"]) == "downloaded")).ToList(),
        MapUiJson.Text(view["emptyText"]),
        MapUiJson.Flag(view["canRefresh"]),
        MapUiJson.Flag(view["canDownload"]),
        MapUiJson.Flag(view["canCancel"]),
        MapUiJson.Flag(view["canErase"]),
        MapUiJson.Flag(view["eraseSelectedShown"]),
        MapUiJson.Flag(view["canEraseSelected"]),
        MapUiJson.Flag(view["canSort"]),
        MapUiJson.Text(view["sortText"]),
        MapUiJson.Flag(view["busy"]),
        MapUiJson.Flag(view["downloading"]),
        MapUiJson.Flag(view["anyDownloaded"]),
        MapUiJson.Text(view["savePath"]),
        MapUiJson.Text(view["savePathReason"]));
}

internal sealed class LogDownloadTool : UserControl
{
    internal const string LogsPath = "view.logs";
    internal const string DownloadBlock = "Wait for the log download to complete or cancel it first";
    private const string Root = "logDownload";
    private readonly string _client = $"analyzeTool.logs.{System.Guid.NewGuid():N}";

    private readonly Grid _main;
    private readonly Button _refresh;
    private readonly Button _sort;
    private readonly CheckBox _selectAll = new() { FontFamily = PreflightLook.Font, Margin = new Thickness(24, 0, 24, 0) };
    private readonly Border _busyCard;
    private readonly TextBlock _busyTitle = Ui.Text("", semibold: true);
    private readonly TextBlock _busyProgress = Ui.Caption("");
    private readonly Button _cancel;
    private readonly ItemsControl _sections = Ui.Column([]);
    private readonly ItemsControl _erase = Ui.Line([], 8);
    private readonly ScrollViewer _list;
    private readonly ContentControl _emptyList = new() { HorizontalContentAlignment = HorizontalAlignment.Stretch, IsTabStop = false };
    private readonly TextBlock _saved = Ui.Caption("").With(text => text.Margin = new Thickness(24, 8, 24, 16));
    private readonly TextBlock _downloadLabel = new() { FontFamily = PreflightLook.Font };
    private readonly Button _download;
    private LogsView _logs = LogsView.Parse(new JsonObject());
    private bool? _connectedSeen;
    private string _signature = "";

    public LogDownloadTool()
    {
        _refresh = Ui.Button("Refresh", () => Act("refresh"));
        _sort = Ui.Button("", () => Act("toggleSortByDate"));
        _cancel = Ui.Button("Cancel", () => Act("cancel"), Look.Subtle);
        _download = Ui.Styled(new Button
        {
            Content = new StackPanel
            {
                Orientation = Orientation.Horizontal,
                Spacing = 8,
                Children = { Ui.Glyph("\uE896", 16, "AccentOnBrush"), _downloadLabel },
            },
            HorizontalAlignment = HorizontalAlignment.Right,
            VerticalAlignment = VerticalAlignment.Bottom,
            Margin = new Thickness(16),
            Padding = new Thickness(16, 10, 16, 10),
            CornerRadius = (CornerRadius)Application.Current.Resources["RadiusOverlay"],
            Visibility = Visibility.Collapsed,
        }, Look.Accent, () => Act("download"));
        _selectAll.Click += (_, _) => SelectAll();
        _busyCard = Ui.Card(new StackPanel
        {
            Spacing = 10,
            Children =
            {
                new Grid
                {
                    ColumnSpacing = 12,
                    ColumnDefinitions =
                    {
                        new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) },
                        new ColumnDefinition { Width = GridLength.Auto },
                        new ColumnDefinition { Width = GridLength.Auto },
                    },
                    Children =
                    {
                        _busyTitle,
                        _busyProgress.With(text => Grid.SetColumn(text, 1)),
                        _cancel.With(button => Grid.SetColumn(button, 2)),
                    },
                },
                new ProgressBar { IsIndeterminate = true },
            },
        }).With(card => card.Margin = new Thickness(24, 8, 24, 8));
        _list = new ScrollViewer
        {
            Content = new StackPanel
            {
                Children =
                {
                    _sections,
                    _erase.With(erase => erase.Margin = new Thickness(24, 8, 24, 8)),
                    new Border { Height = 88 },
                },
            },
        };
        _main = new Grid
        {
            RowDefinitions =
            {
                new RowDefinition { Height = GridLength.Auto },
                new RowDefinition { Height = GridLength.Auto },
                new RowDefinition { Height = GridLength.Auto },
                new RowDefinition { Height = new GridLength(1, GridUnitType.Star) },
                new RowDefinition { Height = GridLength.Auto },
            },
            Children =
            {
                new StackPanel
                {
                    Orientation = Orientation.Horizontal,
                    Spacing = 8,
                    Padding = new Thickness(24, 0, 24, 8),
                    Children = { _refresh, _sort },
                },
                _selectAll.With(box => Grid.SetRow(box, 1)),
                _busyCard.With(card => Grid.SetRow(card, 2)),
                _list.With(list => Grid.SetRow(list, 3)),
                _emptyList.With(empty => Grid.SetRow(empty, 3)),
                _saved.With(text => Grid.SetRow(text, 4)),
                _download.With(button => Grid.SetRowSpan(button, 5)),
            },
        };

        Loaded += (_, _) =>
        {
            _connectedSeen = null;
            _signature = "";
            BridgeWatch.Watch(_client, [LogsPath], Apply);
            Apply(Bridge.Group(LogsPath));
        };
        Unloaded += (_, _) =>
        {
            BridgeWatch.Stop(_client);
            NavigationLock.Release(DownloadBlock);
        };
    }

    internal static string TimeText(string raw, string state, DateTime today) => state switch
    {
        "unreceived" => "",
        "unknown" => "Date unknown",
        _ => DateTimeOffset.TryParse(raw, CultureInfo.InvariantCulture, DateTimeStyles.AssumeLocal, out var parsed)
            ? Compact(parsed.LocalDateTime, today)
            : "Date unknown",
    };

    internal static string Compact(DateTime time, DateTime today) => time.Date switch
    {
        var day when day == today.Date => time.ToString("t", CultureInfo.CurrentCulture),
        var day when day == today.Date.AddDays(-1) => "Yesterday",
        var day when day.Year == today.Year => time.ToString("MMM d", CultureInfo.CurrentCulture),
        _ => time.ToString("MMM d, yyyy", CultureInfo.CurrentCulture),
    };

    private static void Act(string action) => Ui.Off(() => Bridge.Invoke($"{Root}.{action}"));

    private void Apply(JsonObject view)
    {
        var signature = view.ToJsonString();
        if (signature == _signature) return;
        _signature = signature;
        _logs = LogsView.Parse(view);
        if (_logs.Downloading) NavigationLock.Hold(DownloadBlock);
        else NavigationLock.Release(DownloadBlock);
        if (_logs.Connected && _connectedSeen != true && !_logs.Busy) Act("refresh");
        _connectedSeen = _logs.Connected;
        Render();
    }

    private void Render()
    {
        var logs = _logs;
        Content = logs.Connected
            ? _main
            : Ui.EmptyState("\uE8A5", "No vehicle", logs.EmptyText.Length > 0 ? logs.EmptyText : "Connect a vehicle to download its flight logs.");
        if (!logs.Connected) return;

        var selectable = logs.Entries.Where(entry => entry.Received).ToList();
        var selectedCount = logs.Entries.Count(entry => entry.Selected);

        _refresh.IsEnabled = logs.CanRefresh;
        _sort.Visibility = Ui.Shown(logs.SortText.Length > 0);
        _sort.Content = MapUiJson.SentenceCase(logs.SortText);
        _sort.IsEnabled = logs.CanSort;

        _selectAll.Visibility = Ui.Shown(selectable.Count > 0 && !logs.Busy);
        _selectAll.IsChecked = selectable.Count > 0 && selectedCount == selectable.Count;
        _selectAll.Content = selectedCount < selectable.Count ? "Select all" : "Deselect all";

        var downloading = logs.Entries.FirstOrDefault(entry => entry.Downloading);
        _busyCard.Visibility = Ui.Shown(logs.Busy);
        _busyTitle.Text = downloading is null ? "Downloading" : $"Downloading log {downloading.Id}";
        _busyProgress.Text = downloading?.Status ?? "";
        _cancel.Visibility = Ui.Shown(logs.CanCancel);

        _list.Visibility = Ui.Shown(logs.Entries.Count > 0);
        _emptyList.Visibility = Ui.Shown(logs.Entries.Count == 0);
        _emptyList.Content = logs.Entries.Count == 0
            ? Ui.EmptyState("\uE8A5", "No logs yet", logs.EmptyText.Length > 0 ? logs.EmptyText : "This vehicle reports no flight logs.")
            : null;
        _sections.Fill(new[]
            {
                (Title: "On the vehicle", Entries: logs.Entries.Where(entry => !entry.Saved).ToList()),
                (Title: "On this computer", Entries: logs.Entries.Where(entry => entry.Saved).ToList()),
            }
            .Where(section => section.Entries.Count > 0)
            .SelectMany(section => section.Entries.Select(entry => Row(entry, !logs.Busy)).Prepend(Ui.SectionHeader(section.Title))));
        _erase.Visibility = Ui.Shown(!logs.Busy);
        _erase.Fill(new UIElement?[]
        {
            logs.EraseSelectedShown ? Ui.Button("Erase selected", () => Erase(EraseKind.Selected), Look.Critical, logs.CanEraseSelected) : null,
            Ui.Button("Erase all", () => Erase(EraseKind.All), Look.Critical, logs.CanErase),
        });

        var saved = !logs.AnyDownloaded ? null
            : logs.SavePath.Length > 0 ? $"Saved to {logs.SavePath}"
            : logs.SavePathReason.Length > 0 ? logs.SavePathReason
            : null;
        _saved.Text = saved ?? "";
        _saved.Visibility = Ui.Shown(saved != null);

        _download.Visibility = Ui.Shown(logs.CanDownload && selectedCount > 0);
        _downloadLabel.Text = $"Download ({selectedCount})";
        Microsoft.UI.Xaml.Automation.AutomationProperties.SetName(_download, $"Download ({selectedCount})");
    }

    private void SelectAll()
    {
        var selectable = _logs.Entries.Where(entry => entry.Received).ToList();
        var selectAll = _logs.Entries.Count(entry => entry.Selected) < selectable.Count;
        Lane.Run(() => selectable.ForEach(entry => Bridge.Set($"{Root}.model.{entry.Index}.selected", JsonValue.Create(selectAll))));
    }

    private static UIElement Row(LogEntry entry, bool enabled)
    {
        var row = Ui.Styled(new Button
        {
            Content = new Grid
            {
                ColumnSpacing = 16,
                ColumnDefinitions =
                {
                    new ColumnDefinition { Width = GridLength.Auto },
                    new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) },
                    new ColumnDefinition { Width = GridLength.Auto },
                },
                Children =
                {
                    Ui.Circle(40, entry.Selected ? "AccentBrush" : "LayerBrush",
                        Ui.Glyph(entry.Selected ? "\uE73E" : "\uE8A5", 20, entry.Selected ? "AccentOnBrush" : "TextSecondaryBrush")
                            .With(icon => icon.HorizontalAlignment = HorizontalAlignment.Center)),
                    new StackPanel
                    {
                        VerticalAlignment = VerticalAlignment.Center,
                        Children =
                        {
                            Ui.Text($"Log {entry.Id}"),
                            Ui.Caption(string.Join(" · ", new[] { entry.Time, entry.Size }.Where(part => part.Length > 0))),
                        },
                    }.With(panel => Grid.SetColumn(panel, 1)),
                    Ui.Caption(entry.Status, "AccentBrush").With(status => Grid.SetColumn(status, 2)),
                },
            },
            IsEnabled = enabled && entry.Received,
            HorizontalAlignment = HorizontalAlignment.Stretch,
            HorizontalContentAlignment = HorizontalAlignment.Stretch,
            MinHeight = 72,
            Padding = new Thickness(16, 12, 16, 12),
            Margin = new Thickness(8, 0, 8, 0),
        }.Named($"Log {entry.Id}"), Look.Subtle, () =>
            Ui.Off(() => Bridge.Set($"{Root}.model.{entry.Index}.selected", JsonValue.Create(!entry.Selected))));
        if (entry.Selected) row.Background = Ui.Token("SubtleSelectedBrush");
        return row;
    }

    private enum EraseKind { All, Selected }

    private async void Erase(EraseKind kind)
    {
        var (action, title, text, confirm) = kind == EraseKind.All
            ? ("eraseAll", "Delete All Onboard Log Files", "All onboard log files will be erased permanently. Is this really what you want?", "Erase All")
            : ("eraseSelected", "Delete Selected Onboard Log Files", "The selected onboard log files will be erased permanently. Is this really what you want?", "Erase Selected");
        if (await Ui.Confirm(XamlRoot, title, text, confirm)) Act(action);
    }
}
