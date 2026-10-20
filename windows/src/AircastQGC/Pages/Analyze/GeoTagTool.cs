using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;
using AircastQGC.Core;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.Windows.Storage.Pickers;

namespace AircastQGC.Pages.Analyze;

internal sealed record GeoTagImage(string FileName, int Status, string StatusString, string Error, (double Latitude, double Longitude)? Coordinate);

internal sealed record GeoTagState(string LogFile, string ImageDirectory, string SaveDirectory, string Error, double Progress,
    bool InProgress, int Tagged, int Skipped, int Failed, double TimeOffsetSecs, bool PreviewMode, IReadOnlyList<GeoTagImage> Images)
{
    internal const int TaggedStatus = 2;

    internal static GeoTagState? Parse(JsonObject json) =>
        MapUiJson.Text(json["class"]) == "GeoTagController"
            ? new GeoTagState(
                MapUiJson.Text(json["logFile"]),
                MapUiJson.Text(json["imageDirectory"]),
                MapUiJson.Text(json["saveDirectory"]),
                MapUiJson.Text(json["errorMessage"]),
                MapUiJson.Number(json["progress"]) ?? 0,
                MapUiJson.Flag(json["inProgress"]),
                Ui.Int(json["taggedCount"]),
                Ui.Int(json["skippedCount"]),
                Ui.Int(json["failedCount"]),
                MapUiJson.Number(json["timeOffsetSecs"]) ?? 0,
                MapUiJson.Flag(json["previewMode"]),
                MapUiJson.Objects(json["imageModel"]).Select(row => new GeoTagImage(
                    MapUiJson.Text(row["fileName"]),
                    Ui.Int(row["status"]),
                    MapUiJson.Text(row["statusString"]),
                    MapUiJson.Text(row["errorMessage"]),
                    row["coordinate"] is JsonObject at && MapUiJson.Number(at["latitude"]) is double latitude && MapUiJson.Number(at["longitude"]) is double longitude
                        ? (latitude, longitude)
                        : null)).ToList())
            : null;
}

internal sealed class GeoTagTool : UserControl
{
    private const string Root = "geoTag";
    private const string DefaultOutput = "TAGGED";
    private const double OffsetLimit = 3600;
    private static readonly string[] ImageExtensions = ["jpg", "jpeg", "tiff", "tif", "dng"];
    private static readonly string[] LogExtensions = ["ulg", "bin"];
    private static readonly (int Status, string Label)[] Legend =
        [(0, "Pending"), (1, "Processing"), (GeoTagState.TaggedStatus, "Tagged"), (3, "Skipped"), (4, "Failed")];

    private readonly StackPanel _progress;
    private readonly ProgressBar _progressBar = new() { Minimum = 0, Maximum = 100 };
    private readonly ItemsControl _messages = Ui.Column([], 4);
    private readonly Button _logStep;
    private readonly ItemsControl _downloaded = Ui.Column([]);
    private readonly Button _imageStep;
    private readonly Button _outputStep;
    private readonly TextBox _offset;
    private readonly CheckBox _preview;
    private readonly Button _start;
    private readonly TextBlock _imagesHeader = Ui.Text("", semibold: true);
    private readonly ItemsControl _legend = Ui.Line([], 12);
    private readonly ItemsControl _images = Ui.Column([], 8);
    private readonly DispatcherTimer _poll = new() { Interval = TimeSpan.FromMilliseconds(250) };

    private GeoTagState? _state;
    private string? _note;
    private bool _busy;
    private bool _offsetShown;
    private IReadOnlyList<string> _logs = [];

    public GeoTagTool()
    {
        _progress = new StackPanel
        {
            Spacing = 8,
            Children = { Ui.Text("Geotagging in progress..."), _progressBar },
        };
        _logStep = Step(PickLog);
        _imageStep = Step(PickImages);
        _outputStep = Step(PickOutput);
        _offset = new TextBox
        {
            Header = "Time offset (seconds)",
            PlaceholderText = "0.0",
            FontFamily = PreflightLook.Font,
        }.Named("Time offset (seconds)");
        _offset.TextChanged += (_, _) =>
        {
            if (Offset(_offset.Text) is double seconds) Bridge.Set($"{Root}.timeOffsetSecs", JsonValue.Create(seconds));
        };
        _preview = new CheckBox
        {
            Content = new StackPanel
            {
                Children =
                {
                    Ui.Text("Preview mode (don't write files)"),
                    Ui.Caption("Verify time offset before committing"),
                },
            },
        }.Named("Preview mode");
        _preview.Click += (_, _) => Set("previewMode", JsonValue.Create(_preview.IsChecked == true));
        _start = Ui.Button("", StartOrCancel, Look.Accent).With(button => button.HorizontalAlignment = HorizontalAlignment.Stretch);

        Content = new ScrollViewer
        {
            Content = new StackPanel
            {
                Padding = new Thickness(24, 0, 24, 24),
                Spacing = 12,
                Children =
                {
                    Ui.Text("Tag images from a survey mission with GPS coordinates from your flight log.", brush: "TextSecondaryBrush"),
                    _progress,
                    _messages,
                    _logStep,
                    _downloaded.With(list => list.Margin = new Thickness(40, 0, 0, 0)),
                    _imageStep,
                    _outputStep,
                    Ui.Text("Advanced options", semibold: true).With(text => text.Margin = new Thickness(0, 8, 0, 0)),
                    _offset,
                    Ui.Caption("Adjust if camera clock differs from flight log"),
                    _preview,
                    _start,
                    _imagesHeader.With(text => text.Margin = new Thickness(0, 8, 0, 0)),
                    _legend,
                    _images,
                },
            },
        };

        _poll.Tick += (_, _) => Refresh();
        Loaded += (_, _) =>
        {
            Refresh();
            LoadDownloadedLogs();
        };
        Unloaded += (_, _) => _poll.Stop();
    }

    internal static double? Offset(string typed)
    {
        var text = typed.Trim().Replace(',', '.');
        var decimals = text.Contains('.') ? text[(text.IndexOf('.') + 1)..].Length : 0;
        return text.Length == 0 ? 0
            : double.TryParse(text, NumberStyles.Float, CultureInfo.InvariantCulture, out var seconds) && Math.Abs(seconds) <= OffsetLimit && decimals <= 1
                ? seconds
                : null;
    }

    internal static bool IsImage(string name) =>
        ImageExtensions.Contains(Path.GetExtension(name).TrimStart('.').ToLowerInvariant());

    private void Refresh()
    {
        _state = GeoTagState.Parse(Bridge.Group(Root));
        if (_state is { InProgress: true }) _poll.Start();
        else _poll.Stop();
        Render();
    }

    private void Set(string field, JsonNode value)
    {
        Bridge.Set($"{Root}.{field}", value);
        Refresh();
    }

    private void LoadDownloadedLogs()
    {
        var folder = MapUiJson.Text(Bridge.Get("view.control(settings.appSettings.logSavePath)")["value"]);
        Ui.Off(this, () => DownloadedLogs(folder), logs =>
        {
            _logs = logs;
            Render();
        });
    }

    private static IReadOnlyList<string> DownloadedLogs(string folder)
    {
        try
        {
            return folder.Length == 0 || !Directory.Exists(folder)
                ? []
                : new DirectoryInfo(folder).EnumerateFiles()
                    .Where(file => LogExtensions.Contains(file.Extension.TrimStart('.').ToLowerInvariant()))
                    .OrderByDescending(file => file.LastWriteTimeUtc)
                    .Select(file => file.FullName)
                    .ToList();
        }
        catch (IOException) { return []; }
        catch (UnauthorizedAccessException) { return []; }
    }

    private static int ImagesIn(string folder)
    {
        try { return Directory.EnumerateFiles(folder).Count(IsImage); }
        catch (IOException) { return 0; }
        catch (UnauthorizedAccessException) { return 0; }
    }

    private async void PickLog()
    {
        _busy = true;
        Render();
        var path = await Ui.PickFile(this, window => new FileOpenPicker(window)
        {
            SuggestedStartLocation = PickerLocationId.DocumentsLibrary,
            FileTypeFilter = { ".ulg", ".bin", "*" },
        });
        _busy = false;
        _note = path == null ? _note : null;
        if (path != null) Set("logFile", JsonValue.Create(path));
        else Render();
    }

    private async void PickImages()
    {
        _busy = true;
        Render();
        var path = await Ui.PickFolder(this);
        _busy = false;
        _note = path == null ? _note : ImagesIn(path) == 0 ? "That folder holds no JPEG, TIFF or DNG images." : null;
        if (path != null) Set("imageDirectory", JsonValue.Create(path));
        else Render();
    }

    private async void PickOutput()
    {
        _busy = true;
        Render();
        var path = await Ui.PickFolder(this);
        _busy = false;
        if (path != null) Set("saveDirectory", JsonValue.Create(path));
        else Render();
    }

    private void PickDownloaded(string path)
    {
        _note = null;
        Set("logFile", JsonValue.Create(path));
    }

    private void StartOrCancel()
    {
        var running = _state?.InProgress == true;
        _note = running ? _note : null;
        Bridge.Invoke(running ? $"{Root}.cancelTagging" : $"{Root}.startTagging");
        Refresh();
    }

    private static string OutputText(GeoTagState state) =>
        state.SaveDirectory.Length > 0 ? state.SaveDirectory
        : state.ImageDirectory.Length > 0 ? Path.Combine(state.ImageDirectory, DefaultOutput)
        : $"Default: {Path.DirectorySeparatorChar}{DefaultOutput} subfolder";

    private static string? Summary(GeoTagState state)
    {
        if (state.InProgress || state.Tagged <= 0) return null;
        var details = new[]
        {
            state.Skipped > 0 ? $"{state.Skipped} skipped" : null,
            state.Failed > 0 ? $"{state.Failed} failed" : null,
        }.OfType<string>().ToList();
        return $"Successfully tagged {state.Tagged} images" + (details.Count == 0 ? "" : $" ({string.Join(", ", details)})");
    }

    private static string ButtonText(GeoTagState state) =>
        state.InProgress ? "Cancel" : state.PreviewMode ? "Preview" : "Start tagging";

    private static string StatusBrush(int status) => status switch
    {
        1 => "AccentBrush",
        GeoTagState.TaggedStatus => "SuccessBrush",
        3 => "CautionBrush",
        4 => "CriticalBrush",
        _ => "TextTertiaryBrush",
    };

    private void Render()
    {
        Visibility = Ui.Shown(_state != null);
        if (_state is not { } state) return;
        var editable = !state.InProgress && !_busy;

        _progress.Visibility = Ui.Shown(state.InProgress);
        _progressBar.Value = Math.Clamp(state.Progress, 0, 100);

        var error = state.Error.Length > 0 && !state.InProgress ? state.Error : null;
        _messages.Fill(new[] { error, _note }.OfType<string>().Select(message => (UIElement)Ui.Text(message, brush: "CriticalBrush"))
            .Concat(Summary(state) is { } summary ? [Ui.Text(summary, brush: "AccentBrush")] : []));

        Paint(_logStep, state.LogFile.Length > 0, "1", "Flight log", state.LogFile.Length > 0 ? Path.GetFileName(state.LogFile) : "No file selected", editable);
        Paint(_imageStep, state.ImageDirectory.Length > 0, "2", "Photos folder", state.ImageDirectory.Length > 0 ? state.ImageDirectory : "No folder selected", editable);
        Paint(_outputStep, false, "3", "Output folder (optional)", OutputText(state), editable);

        _downloaded.Visibility = Ui.Shown(_logs.Count > 0 && editable);
        _downloaded.Fill(_logs.Select(log => (UIElement)Ui.Button(Path.GetFileName(log), () => PickDownloaded(log), Look.Subtle)
            .With(button => button.HorizontalAlignment = HorizontalAlignment.Left)));

        if (!_offsetShown)
        {
            _offsetShown = true;
            _offset.Text = state.TimeOffsetSecs.ToString("0.0", CultureInfo.InvariantCulture);
        }
        _offset.IsEnabled = editable;
        _preview.IsChecked = state.PreviewMode;
        _preview.IsEnabled = editable;

        _start.Content = ButtonText(state);
        _start.IsEnabled = state.InProgress || (!_busy && state.LogFile.Length > 0 && state.ImageDirectory.Length > 0);

        var any = state.Images.Count > 0;
        _imagesHeader.Visibility = Ui.Shown(any);
        _imagesHeader.Text = $"Images ({state.Images.Count})";
        _legend.Visibility = Ui.Shown(any);
        _legend.Fill(Legend.Select(entry => (UIElement)new StackPanel
        {
            Orientation = Orientation.Horizontal,
            Spacing = 4,
            Children = { Swatch(entry.Status, 10), Ui.Caption(entry.Label) },
        }));
        _images.Fill(state.Images.Select(ImageRow));
    }

    private static Border Swatch(int status, double size) => new()
    {
        Width = size,
        Height = size,
        CornerRadius = new CornerRadius(2),
        Background = Ui.Token(StatusBrush(status)),
        VerticalAlignment = VerticalAlignment.Center,
    };

    private static UIElement ImageRow(GeoTagImage image)
    {
        var coordinate = image.Coordinate is { } at && image.Status == GeoTagState.TaggedStatus
            ? string.Format(CultureInfo.InvariantCulture, "{0:F6}, {1:F6}", at.Latitude, at.Longitude)
            : "";
        return new Grid
        {
            ColumnSpacing = 12,
            ColumnDefinitions =
            {
                new ColumnDefinition { Width = GridLength.Auto },
                new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) },
                new ColumnDefinition { Width = GridLength.Auto },
            },
            Children =
            {
                Swatch(image.Status, 12),
                new StackPanel
                {
                    Children =
                    {
                        Ui.Text(image.FileName, wrap: false),
                        Ui.Caption(coordinate).With(text => text.Visibility = Ui.Shown(coordinate.Length > 0)),
                    },
                }.With(panel => Grid.SetColumn(panel, 1)),
                Ui.Caption(image.Error.Length > 0 ? image.Error : image.StatusString,
                    image.Status is 3 or 4 ? StatusBrush(image.Status) : "TextPrimaryBrush").With(text => Grid.SetColumn(text, 2)),
            },
        };
    }

    private static Button Step(Action browse) => Ui.Styled(new Button
    {
        HorizontalAlignment = HorizontalAlignment.Stretch,
        HorizontalContentAlignment = HorizontalAlignment.Stretch,
        Padding = new Thickness(16),
        CornerRadius = (CornerRadius)Application.Current.Resources["RadiusOverlay"],
    }, Look.Standard, browse);

    private static void Paint(Button step, bool done, string number, string title, string detail, bool enabled)
    {
        step.IsEnabled = enabled;
        Microsoft.UI.Xaml.Automation.AutomationProperties.SetName(step, title);
        step.Content = new Grid
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
                Ui.Circle(32, done ? "AccentBrush" : "LayerBrush",
                    Ui.Centered(Ui.Text(done ? "✓" : number, semibold: true, brush: done ? "AccentOnBrush" : "TextSecondaryBrush"))),
                new StackPanel
                {
                    Children = { Ui.Text(title, semibold: true), Ui.Caption(detail) },
                }.With(panel => Grid.SetColumn(panel, 1)),
                Ui.Glyph("\uE8B7", 16, "TextSecondaryBrush").With(icon => Grid.SetColumn(icon, 2)),
            },
        };
    }
}
