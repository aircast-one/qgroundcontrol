using System;
using AircastQGC.Core;
using AircastQGC.Pages.Analyze;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

public sealed class VideoStatsPill : UserControl
{
    private const string StatsPath = "view.videoStats";
    private const string VideoPath = "view.video";

    private readonly DispatcherTimer _poll = new() { Interval = TimeSpan.FromSeconds(1) };
    private readonly TextBlock _text = Ui.Text("", "FontSizeCaption", semibold: true, wrap: false);
    private bool _expanded;

    public VideoStatsPill()
    {
        IsHitTestVisible = false;
        Visibility = Visibility.Collapsed;
        Content = new Border
        {
            Height = 24,
            Padding = new Thickness(10, 0, 10, 0),
            CornerRadius = new CornerRadius(12),
            Background = Ui.Token("LayerBrush"),
            Opacity = 0.9,
            Child = _text,
        };
        AutomationProperties.SetName(this, "Video stats");
        _poll.Tick += (_, _) => Read();
        Unloaded += (_, _) => _poll.Stop();
    }

    public bool Expanded
    {
        get => _expanded;
        set
        {
            _expanded = value;
            if (value && CoreHost.Started) _poll.Start();
            else _poll.Stop();
            Read();
        }
    }

    private void Read()
    {
        var text = _expanded && CoreHost.Started && MapUiJson.Flag(Bridge.GetFields(VideoPath, "decoding")["decoding"])
            ? MapUiJson.Text(Bridge.Get(StatsPath)["text"]).Trim()
            : "";
        _text.Text = text;
        AutomationProperties.SetHelpText(this, text);
        Visibility = Ui.Shown(text.Length > 0);
    }
}
