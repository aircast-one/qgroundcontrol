using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Pages.Analyze;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

internal sealed record MavlinkLogFile(string Name, long Size, bool Uploaded, bool Writing);

internal sealed record MavlinkLog(bool Px4, bool Running, bool CanStart, bool Persistence, JsonObject Settings,
    IReadOnlyList<MavlinkLogFile> Files, bool Uploading, string UploadingFile, double UploadProgress, string Message)
{
    internal static MavlinkLog Parse(JsonObject view) => new(
        MapUiJson.Flag(view["vehiclePx4"]),
        MapUiJson.Flag(view["logRunning"]),
        MapUiJson.Flag(view["canStartLog"]),
        MapUiJson.Flag(view["persistence"], true),
        view["settings"] as JsonObject ?? new JsonObject(),
        MapUiJson.Objects(view["files"]).Select(file => new MavlinkLogFile(
            MapUiJson.Text(file["name"]),
            (long)(MapUiJson.Number(file["size"]) ?? 0),
            MapUiJson.Flag(file["uploaded"]),
            MapUiJson.Flag(file["writing"]))).ToList(),
        MapUiJson.Flag(view["uploading"]),
        MapUiJson.Text(view["uploadingFile"]),
        Math.Clamp(MapUiJson.Number(view["uploadProgress"]) ?? 0, 0, 1),
        MapUiJson.Text(view["message"]));

    internal string Setting(string field) => MapUiJson.Text(Settings[field]);

    internal bool Flag(string field) => MapUiJson.Flag(Settings[field]);

    internal bool Idle => !Uploading && !Running;

    internal static string SizeText(long size) => size.ToString("N0", CultureInfo.CurrentCulture);

    internal static IReadOnlyList<(string Field, string Typed)> Unsaved(MavlinkLog log, IReadOnlyDictionary<string, string> drafts) =>
        drafts.Where(draft => draft.Value != log.Setting(draft.Key)).Select(draft => (draft.Key, draft.Value)).ToList();

    internal static string UploadEmail(MavlinkLog log, IReadOnlyDictionary<string, string> drafts) =>
        drafts.TryGetValue("emailAddress", out var typed) ? typed : log.Setting("emailAddress");
}

public sealed class Px4LogTransfer : UserControl
{
    private const string ViewPath = "view.mavlinkLog";
    private const string SetPath = "mavlinkLog.set";
    private const string Title = "MAVLink Logging";
    private const string NoPx4 = "Connect a PX4 vehicle to manage logs";
    private const string NoEmail = "Please enter an email address before uploading MAVLink log files.";
    private static readonly TimeSpan SaveDelay = TimeSpan.FromMilliseconds(600);
    private static readonly TimeSpan PollEvery = TimeSpan.FromMilliseconds(1000);

    internal static readonly IReadOnlyList<(string Label, string Raw)> WindSpeeds =
        [("Not Set", "-1"), ("Calm", "0"), ("Breeze", "5"), ("Gale", "8"), ("Storm", "10")];

    internal static readonly IReadOnlyList<(string Label, string Raw)> FlightRatings =
    [
        ("Not Set", "notset"),
        ("Crashed (Pilot Error)", "crash_pilot"),
        ("Crashed (Software or Hardware Issue)", "crash_sw_hw"),
        ("Unsatisfactory", "unsatisfactory"),
        ("Good", "good"),
        ("Great", "great"),
    ];

    private static readonly IReadOnlyList<string> TextFields = ["emailAddress", "description", "uploadURL", "videoURL", "feedback"];

    private readonly TextBlock _noPx4 = Ui.Text(NoPx4, brush: "TextSecondaryBrush");
    private readonly StackPanel _body;
    private readonly Button _start;
    private readonly Button _stop;
    private readonly IReadOnlyDictionary<string, CheckBox> _flags;
    private readonly IReadOnlyDictionary<string, TextBox> _texts;
    private readonly IReadOnlyDictionary<string, ComboBox> _choices;
    private readonly TextBlock _noFiles = Ui.Caption("No log files");
    private readonly ItemsControl _files = Ui.Column([], 4);
    private readonly Button _selectAll;
    private readonly Button _selectNone;
    private readonly Button _delete;
    private readonly Button _cancelUpload;
    private readonly Button _upload;
    private readonly TextBlock _message = Ui.Caption("", "CriticalBrush");
    private readonly DispatcherTimer _poll = new() { Interval = PollEvery };

    private MavlinkLog? _log;
    private ImmutableDictionary<string, string> _drafts = ImmutableDictionary<string, string>.Empty;
    private ImmutableDictionary<string, string> _served = ImmutableDictionary<string, string>.Empty;
    private ImmutableHashSet<string> _selected = ImmutableHashSet<string>.Empty;
    private string _fileSignature = "";

    public Px4LogTransfer()
    {
        _start = Ui.Button("Start", () => Act([Call("mavlinkLog.start")]));
        _stop = Ui.Button("Stop", () => Act([Call("mavlinkLog.stop")]));
        _flags = new[]
        {
            ("enableAutoStart", "Start logging automatically", false),
            ("publicLog", "Make logs public", false),
            ("enableAutoUpload", "Upload logs automatically", true),
            ("deleteAfterUpload", "Delete logs after upload", false),
        }.ToDictionary(flag => flag.Item1, flag => FlagField(flag.Item1, flag.Item2, flag.Item3));
        _texts = new[]
        {
            ("emailAddress", "Email Address"),
            ("description", "Default Description"),
            ("uploadURL", "Upload URL"),
            ("videoURL", "Video URL"),
            ("feedback", "Additional Feedback"),
        }.ToDictionary(text => text.Item1, text => TextField(text.Item1, text.Item2));
        _choices = new[]
        {
            ("windSpeed", "Wind Speed", WindSpeeds),
            ("rating", "Flight Rating", FlightRatings),
        }.ToDictionary(choice => choice.Item1, choice => ChoiceField(choice.Item1, choice.Item2, choice.Item3));
        _selectAll = Ui.Button("Select All", () => Select(_log?.Files.Where(file => !file.Writing).Select(file => file.Name) ?? []), Look.Subtle);
        _selectNone = Ui.Button("Select None", () => Select([]), Look.Subtle);
        _delete = Ui.Button("Delete…", Delete, Look.Subtle);
        _cancelUpload = Ui.Button("Cancel Upload…", CancelUpload, Look.Subtle);
        _upload = Ui.Button("Upload…", Upload, Look.Subtle);

        _body = new StackPanel
        {
            Spacing = 8,
            Children =
            {
                Header("MAVLink 2.0 Logging"),
                Ui.Caption("PX4 Pro only"),
                new Grid
                {
                    ColumnSpacing = 8,
                    ColumnDefinitions =
                    {
                        new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) },
                        new ColumnDefinition { Width = GridLength.Auto },
                        new ColumnDefinition { Width = GridLength.Auto },
                    },
                    Children =
                    {
                        Ui.Text("Logging"),
                        _start.With(button => Grid.SetColumn(button, 1)),
                        _stop.With(button => Grid.SetColumn(button, 2)),
                    },
                },
                _flags["enableAutoStart"],
                Header("Log Upload"),
                _texts["emailAddress"],
                _texts["description"],
                _texts["uploadURL"],
                _texts["videoURL"],
                _choices["windSpeed"],
                _choices["rating"],
                _texts["feedback"],
                _flags["publicLog"],
                _flags["enableAutoUpload"],
                _flags["deleteAfterUpload"],
                Header("Saved Log Files"),
                _noFiles,
                _files,
                new StackPanel
                {
                    Orientation = Orientation.Horizontal,
                    Spacing = 4,
                    Children = { _selectAll, _selectNone, _delete, _cancelUpload, _upload },
                },
                _message,
            },
        };
        Content = new ScrollViewer
        {
            Content = new StackPanel
            {
                MaxWidth = 720,
                HorizontalAlignment = HorizontalAlignment.Left,
                Padding = new Thickness(16, 0, 16, 24),
                Children = { _noPx4, _body },
            },
        };
        _noPx4.Visibility = Visibility.Collapsed;
        _body.Visibility = Visibility.Collapsed;

        _poll.Tick += (_, _) => Poll();
        Loaded += (_, _) =>
        {
            Poll();
            _poll.Start();
        };
        Unloaded += (_, _) => _poll.Stop();
    }

    private static TextBlock Header(string text) => Ui.SectionHeader(text).With(header => header.Margin = new Thickness(0, 16, 0, 0));

    private static (string Path, JsonArray Args) Call(string path, params JsonNode?[] args) => (path, new JsonArray(args));

    private static (string Path, JsonArray Args) SetCall(string field, JsonNode? value) => Call(SetPath, JsonValue.Create(field), value);

    private IReadOnlyList<(string Path, JsonArray Args)> UnsavedCalls() =>
        _log is { } log ? MavlinkLog.Unsaved(log, _drafts).Select(draft => SetCall(draft.Field, JsonValue.Create(draft.Typed))).ToList() : [];

    private void Poll() => Ui.Off(this, () => MavlinkLog.Parse(Bridge.Get(ViewPath)), Apply);

    private void Act(IReadOnlyList<(string Path, JsonArray Args)> calls)
    {
        var ui = DispatcherQueue;
        Lane.Run(() =>
        {
            var refused = calls.Select(call => Ui.Refusal(Bridge.Invoke(call.Path, call.Args))).ToList().FirstOrDefault(refusal => refusal != null);
            ui.TryEnqueue(() =>
            {
                if (refused != null) Tell(refused);
                Poll();
            });
        });
    }

    private void SaveThen(string field, JsonNode? value) => Act([.. UnsavedCalls(), SetCall(field, value)]);

    private async void Tell(string text)
    {
        if (XamlRoot != null) await Ui.Tell(XamlRoot, Title, text);
    }

    private async void Confirm(string title, string question, Action run)
    {
        if (XamlRoot != null && await Ui.Confirm(XamlRoot, title, question, "Ok")) run();
    }

    private CheckBox FlagField(string field, string label, bool savesTexts)
    {
        var box = new CheckBox { Content = label, FontFamily = PreflightLook.Font }.Named(label);
        box.Click += (_, _) =>
        {
            var on = JsonValue.Create(box.IsChecked == true);
            if (savesTexts) SaveThen(field, on);
            else Act([SetCall(field, on)]);
        };
        return box;
    }

    private TextBox TextField(string field, string label)
    {
        var box = new TextBox { Header = label, FontFamily = PreflightLook.Font }.Named(label);
        var save = new DispatcherTimer { Interval = SaveDelay };
        save.Tick += (_, _) =>
        {
            save.Stop();
            Act([SetCall(field, JsonValue.Create(box.Text))]);
        };
        box.TextChanged += (_, _) =>
        {
            _drafts = _drafts.SetItem(field, box.Text);
            save.Stop();
            if (_log != null && box.Text != _log.Setting(field)) save.Start();
        };
        return box;
    }

    private ComboBox ChoiceField(string field, string label, IReadOnlyList<(string Label, string Raw)> choices)
    {
        var box = new ComboBox
        {
            Header = label,
            ItemsSource = choices.Select(choice => choice.Label).ToList(),
            HorizontalAlignment = HorizontalAlignment.Stretch,
            FontFamily = PreflightLook.Font,
        }.Named(label);
        box.SelectionChanged += (_, _) =>
        {
            var picked = box.SelectedIndex >= 0 ? choices[box.SelectedIndex].Raw : null;
            if (picked != null && _log != null && picked != Served(_log, field, choices)) SaveThen(field, JsonValue.Create(picked));
        };
        return box;
    }

    private static string Served(MavlinkLog log, string field, IReadOnlyList<(string Label, string Raw)> choices) =>
        choices.Any(choice => choice.Raw == log.Setting(field)) ? log.Setting(field) : choices[0].Raw;

    private void Select(IEnumerable<string> names)
    {
        _selected = names.ToImmutableHashSet();
        if (_log != null) Apply(_log);
    }

    private void Delete()
    {
        var chosen = _selected;
        Confirm("Delete Selected Log Files", "Delete the selected log files?", () =>
        {
            Act([Call("mavlinkLog.delete", chosen.Select(name => (JsonNode?)JsonValue.Create(name)).ToArray())]);
            Select([]);
        });
    }

    private void CancelUpload() =>
        Confirm("Cancel Upload", "Cancel the upload in progress?", () => Act([Call("mavlinkLog.cancelUpload")]));

    private void Upload()
    {
        if (_log is not { } log) return;
        var chosen = _selected;
        Act(UnsavedCalls());
        if (string.IsNullOrWhiteSpace(MavlinkLog.UploadEmail(log, _drafts))) Tell(NoEmail);
        else Confirm("Upload Selected Log Files", "Upload the selected log files?", () =>
        {
            Act([Call("mavlinkLog.upload", chosen.Select(name => (JsonNode?)JsonValue.Create(name)).ToArray())]);
            Select([]);
        });
    }

    private void Apply(MavlinkLog log)
    {
        _log = log;
        _noPx4.Visibility = Ui.Shown(!log.Px4);
        _body.Visibility = Ui.Shown(log.Px4);
        if (!log.Px4) return;
        var editable = log.Persistence;

        _start.IsEnabled = !log.Running && log.CanStart && editable;
        _stop.IsEnabled = log.Running && editable;

        _flags.ToList().ForEach(flag =>
        {
            flag.Value.IsChecked = log.Flag(flag.Key);
            flag.Value.IsEnabled = editable && (flag.Key != "deleteAfterUpload" || log.Flag("enableAutoUpload"));
        });

        TextFields
            .Where(field => _served.GetValueOrDefault(field) != log.Setting(field))
            .ToList()
            .ForEach(field => _texts[field].Text = log.Setting(field));
        _served = TextFields.ToImmutableDictionary(field => field, log.Setting);
        _texts.Values.ToList().ForEach(box => box.IsEnabled = editable);

        new[] { ("windSpeed", WindSpeeds), ("rating", FlightRatings) }.ToList().ForEach(choice =>
        {
            var box = _choices[choice.Item1];
            var index = choice.Item2.Select(option => option.Raw).ToList().IndexOf(Served(log, choice.Item1, choice.Item2));
            if (box.SelectedIndex != index) box.SelectedIndex = index;
            box.IsEnabled = editable;
        });

        _selected = _selected.Intersect(log.Files.Select(file => file.Name));
        RenderFiles(log);

        var uploadedSelected = log.Files.Any(file => _selected.Contains(file.Name) && file.Uploaded);
        _selectAll.IsEnabled = log.Idle;
        _selectNone.IsEnabled = log.Idle;
        _delete.IsEnabled = _selected.Count > 0 && log.Idle;
        _cancelUpload.Visibility = Ui.Shown(log.Uploading);
        _upload.Visibility = Ui.Shown(!log.Uploading);
        _upload.IsEnabled = _selected.Count > 0 && log.Idle && !uploadedSelected;

        _message.Text = log.Message;
        _message.Visibility = Ui.Shown(log.Message.Trim().Length > 0);
    }

    private void RenderFiles(MavlinkLog log)
    {
        _noFiles.Visibility = Ui.Shown(log.Files.Count == 0);
        var signature = string.Join("|", log.Files) + $"#{string.Join(",", _selected.Order())}#{log.Uploading}:{log.UploadingFile}:{log.UploadProgress}";
        if (signature == _fileSignature) return;
        _fileSignature = signature;
        _files.Fill(log.Files.Select(file => FileRow(log, file)));
    }

    private UIElement FileRow(MavlinkLog log, MavlinkLogFile file)
    {
        var uploadingThis = log.Uploading && log.UploadingFile == file.Name;
        var pick = new CheckBox
        {
            IsChecked = _selected.Contains(file.Name),
            IsEnabled = !uploadingThis && !file.Writing,
            MinWidth = 0,
            Padding = new Thickness(0),
        }.Named(file.Name);
        pick.Click += (_, _) => Select(pick.IsChecked == true ? _selected.Add(file.Name) : _selected.Remove(file.Name));
        FrameworkElement status = uploadingThis && !file.Uploaded
            ? new ProgressBar { Minimum = 0, Maximum = 1, Value = log.UploadProgress, Width = 96, VerticalAlignment = VerticalAlignment.Center }
            : Ui.Caption(file.Uploaded ? "Uploaded" : MavlinkLog.SizeText(file.Size));
        return new Grid
        {
            ColumnSpacing = 8,
            ColumnDefinitions =
            {
                new ColumnDefinition { Width = GridLength.Auto },
                new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) },
                new ColumnDefinition { Width = GridLength.Auto },
            },
            Children =
            {
                pick,
                Ui.Caption(file.Name, file.Writing ? "CautionBrush" : "TextPrimaryBrush").With(name => Grid.SetColumn(name, 1)),
                status.With(shown => Grid.SetColumn(shown, 2)),
            },
        };
    }
}
