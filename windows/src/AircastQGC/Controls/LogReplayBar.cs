using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Pages.Analyze;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Input;
using Microsoft.Windows.Storage.Pickers;

namespace AircastQGC.Controls;

internal sealed record LogReplay(bool Shown, bool Loaded, bool Playing, double Percent, string PlayheadTime, string TotalTime,
    int SpeedIndex, IReadOnlyList<string> Speeds, bool CanLoad, string LoadRefusal)
{
    internal static LogReplay? Parse(JsonObject view) =>
        MapUiJson.Flag(view["available"])
            ? new LogReplay(
                MapUiJson.Flag(view["shown"]),
                MapUiJson.Flag(view["loaded"]),
                MapUiJson.Flag(view["playing"]),
                MapUiJson.Number(view["percent"]) ?? 0,
                MapUiJson.Text(view["playheadTime"]),
                MapUiJson.Text(view["totalTime"]),
                Ui.Int(view["speedIndex"], 3),
                view["speeds"] is JsonArray speeds ? speeds.Select(MapUiJson.Text).ToList() : [],
                MapUiJson.Flag(view["canLoad"]),
                MapUiJson.Text(view["loadRefusal"]))
            : null;

    internal string Progress => string.Join(" of ", new[] { PlayheadTime, TotalTime }.Where(part => part.Length > 0));

    internal static string SpeedLabel(string label) => (label.EndsWith('x') ? label[..^1] : label) + "×";
}

public sealed class LogReplayBar : UserControl
{
    private const string ReplayPath = "view.logReplay";
    private static readonly TimeSpan PlayingPoll = TimeSpan.FromMilliseconds(250);
    private static readonly TimeSpan IdlePoll = TimeSpan.FromMilliseconds(1000);

    private readonly TextBlock _progress = Ui.Text("");
    private readonly TextBlock _percent = Ui.Text("", semibold: true, brush: "AccentBrush");
    private readonly Slider _slider = new() { Minimum = 0, Maximum = 100, StepFrequency = 0.1, IsThumbToolTipEnabled = false };
    private readonly ItemsControl _speeds = Ui.Line([], 8);
    private readonly Button _load;
    private readonly Button _play;
    private readonly DispatcherTimer _poll = new() { Interval = IdlePoll };

    private LogReplay? _replay;
    private bool _applying;
    private bool _pressed;
    private string _speedSignature = "";

    public LogReplayBar()
    {
        Visibility = Visibility.Collapsed;
        _load = Ui.Button("Load telemetry log", Load);
        _play = Ui.Button("Play", () => Act("logReplay.togglePlay"), Look.Accent);
        _slider.Named("Replay position");
        _slider.ValueChanged += (_, _) =>
        {
            if (!_applying && !_pressed) Act("logReplay.seek", JsonValue.Create(_slider.Value));
        };
        _slider.AddHandler(PointerPressedEvent, new PointerEventHandler((_, _) => _pressed = true), true);
        _slider.AddHandler(PointerReleasedEvent, new PointerEventHandler((_, _) => Released()), true);
        _slider.AddHandler(PointerCaptureLostEvent, new PointerEventHandler((_, _) => Released()), true);

        Content = new Border
        {
            Background = Ui.Token("LayerBrush"),
            BorderBrush = Ui.Token("DividerBrush"),
            BorderThickness = new Thickness(0, 1, 0, 0),
            Padding = new Thickness(16, 8, 16, 8),
            Child = new StackPanel
            {
                Spacing = 8,
                Children =
                {
                    new Grid
                    {
                        ColumnDefinitions =
                        {
                            new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) },
                            new ColumnDefinition { Width = GridLength.Auto },
                        },
                        Children = { _progress, _percent.With(text => Grid.SetColumn(text, 1)) },
                    },
                    _slider,
                    new ScrollViewer
                    {
                        HorizontalScrollBarVisibility = ScrollBarVisibility.Auto,
                        VerticalScrollBarVisibility = ScrollBarVisibility.Disabled,
                        HorizontalScrollMode = ScrollMode.Enabled,
                        VerticalScrollMode = ScrollMode.Disabled,
                        Content = _speeds,
                    },
                    new StackPanel
                    {
                        Orientation = Orientation.Horizontal,
                        HorizontalAlignment = HorizontalAlignment.Right,
                        Spacing = 8,
                        Children = { Ui.Button("Close", () => Act("logReplay.close"), Look.Subtle).Named("Close log replay"), _load, _play },
                    },
                },
            },
        };

        _poll.Tick += (_, _) => Poll();
        Loaded += (_, _) =>
        {
            Poll();
            _poll.Start();
        };
        Unloaded += (_, _) => _poll.Stop();
    }

    private void Released()
    {
        if (!_pressed) return;
        _pressed = false;
        Act("logReplay.seek", JsonValue.Create(_slider.Value));
    }

    private void Poll() => Ui.Off(this, () => LogReplay.Parse(Bridge.Get(ReplayPath)), Apply);

    private void Apply(LogReplay? replay)
    {
        _replay = replay;
        _poll.Interval = replay?.Playing == true ? PlayingPoll : IdlePoll;
        Visibility = Ui.Shown(replay is { Shown: true });
        if (replay is not { Shown: true } shown) return;
        _progress.Text = shown.Progress;
        _percent.Text = $"{(int)shown.Percent}%";
        _applying = true;
        _slider.IsEnabled = shown.Loaded;
        _slider.Value = _pressed ? _slider.Value : Math.Clamp(shown.Percent, 0, 100);
        _applying = false;
        _load.Visibility = Ui.Shown(!shown.Loaded);
        _play.IsEnabled = shown.Loaded;
        _play.Content = shown.Playing ? "Pause" : "Play";
        var signature = string.Join("|", shown.Speeds) + $"#{shown.SpeedIndex}";
        if (signature == _speedSignature) return;
        _speedSignature = signature;
        _speeds.Fill(shown.Speeds.Select((label, at) => Speed(LogReplay.SpeedLabel(label), at == shown.SpeedIndex, at)));
    }

    private UIElement Speed(string label, bool on, int index)
    {
        var chip = new ToggleButton
        {
            Content = label,
            IsChecked = on,
            Height = 32,
            Padding = new Thickness(12, 0, 12, 0),
            CornerRadius = new CornerRadius(16),
        }.Named($"Speed {label}");
        chip.Click += (_, _) => Act("logReplay.speed", JsonValue.Create(index));
        return chip;
    }

    private void Act(string path, params JsonNode[] args) =>
        Ui.Off(this, () => Ui.Refusal(Bridge.Invoke(path, new JsonArray(args))), async refusal =>
        {
            Poll();
            if (refusal != null && XamlRoot != null) await Ui.Tell(XamlRoot, "Log replay", refusal);
        });

    private async void Load()
    {
        if (_replay is { CanLoad: false } replay)
        {
            await Ui.Tell(XamlRoot, "Log replay", replay.LoadRefusal);
            return;
        }
        var path = await Ui.PickFile(this, window => new FileOpenPicker(window)
        {
            SuggestedStartLocation = PickerLocationId.DocumentsLibrary,
            FileTypeFilter = { ".tlog", "*" },
        });
        if (path != null) Act("logReplay.start", JsonValue.Create(path));
    }
}
