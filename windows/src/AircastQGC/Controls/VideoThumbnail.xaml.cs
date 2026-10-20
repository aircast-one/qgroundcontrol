using System;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

public sealed partial class VideoThumbnail : UserControl
{
    private const string Client = "video-thumbnail";
    private const string VideoView = "view.video";
    private const string FitClient = "video-thumbnail.fit";
    private const string FitView = "view.control(settings.videoSettings.videoFit)";

    private readonly CornerRadius _radius;
    private readonly Thickness _border;
    private readonly object _tip;
    private bool _expanded;
    private string _standIn = "";
    private string _title = "Video";
    private string _headline = "No video";
    private string _detail = "";
    private NoVideoPill? _pill;

    public VideoThumbnail()
    {
        InitializeComponent();
        _radius = Tile.CornerRadius;
        _border = Tile.BorderThickness;
        _tip = ToolTipService.GetToolTip(Tile);
        Synthetic.Changed += (_, _) => PaintPlaceholder();
        Loaded += (_, _) => Start();
        Unloaded += (_, _) => Stop();
    }

    public event EventHandler? SwapRequested;

    public bool IsLive { get; private set; }

    public FrameworkElement AimRail => Synthetic.AimRail;

    public NoVideoPill? Pill
    {
        get => _pill;
        set
        {
            _pill = value;
            PaintPlaceholder();
        }
    }

    public bool Expanded
    {
        get => _expanded;
        set
        {
            _expanded = value;
            Tile.CornerRadius = value ? new CornerRadius(0) : _radius;
            Tile.BorderThickness = value ? new Thickness(0) : _border;
            CaptionPill.Visibility = VideoJson.Shown(!value);
            ToolTipService.SetToolTip(Tile, value ? null : _tip);
            Synthetic.Aimable = value;
            Synthetic.StandIn = value ? _standIn : null;
            PaintPlaceholder();
        }
    }

    public void Show() => Visibility = Visibility.Visible;

    public void Hide() => Visibility = Visibility.Collapsed;

    private void Start()
    {
        BridgeWatch.Watch(Client, new[] { VideoView }, Render);
        BridgeWatch.Watch(FitClient, new[] { FitView }, RenderFit);
        Render(Bridge.Get(VideoView));
        RenderFit(Bridge.Get(FitView));
    }

    private void Stop()
    {
        if (IsLoaded) return;
        BridgeWatch.Stop(Client);
        BridgeWatch.Stop(FitClient);
    }

    private void RenderFit(JsonObject fit) =>
        Surface.Fit = VideoJson.Number(fit["value"]) is double mode ? (int)mode : VideoSurface.FitHeight;

    private void Render(JsonObject video)
    {
        var active = VideoJson.Int(video, "activeSource", 0);
        var camera = (video["cameras"] as JsonArray ?? new JsonArray())
            .OfType<JsonObject>()
            .FirstOrDefault(c => VideoJson.Int(c, "slot", -1) == active);
        var status = camera is null ? "" : VideoJson.Text(camera, "status");
        var live = VideoJson.Flag(video, "decoding");
        var connecting = VideoJson.Flag(video, "anyConnecting");
        var missing = VideoJson.Flag(video, "sourceChosen") ? VideoJson.Text(video, "noVideoText") : "";

        IsLive = live;
        _title = VideoJson.Or(camera is null ? "" : VideoJson.Text(camera, "title"), "Video");
        _standIn = VideoJson.Text(video, "standIn");
        Synthetic.Wanted = !live;
        Synthetic.StandIn = _expanded ? _standIn : null;
        Surface.Visibility = VideoJson.Shown(live);
        Caption.Text = live || status.Length == 0 ? _title : $"{_title} · {status}";
        Dot.Style = (Style)Resources[live ? "VideoDotLive" : connecting ? "VideoDotConnecting" : "VideoDotIdle"];
        _headline = VideoJson.Flag(video, "streamEnabled", true)
            ? VideoJson.Or(VideoJson.Text(video, "summary"), "No video")
            : "Video off";
        _detail = live ? "" : VideoJson.Or(VideoJson.Text(video, "noVideoReason"), missing);
        PaintPlaceholder();
    }

    private void PaintPlaceholder()
    {
        var shown = IsLive || Synthetic.Showing;
        var waiting = Synthetic.Chosen && !shown;
        if (Synthetic.Chosen)
        {
            Caption.Text = _title;
            Dot.Style = (Style)Resources[shown ? "VideoDotLive" : "VideoDotIdle"];
        }
        var headline = waiting ? SyntheticView.WaitingHeadline : _headline;
        Headline.Text = headline;
        Placeholder.Visibility = VideoJson.Shown(!shown && !_expanded);
        if (shown || !_expanded) _pill?.Hide();
        else _pill?.Say(headline, waiting ? SyntheticView.WaitingDetail : _detail);
    }

    private void OnTileClick(object sender, RoutedEventArgs e) => SwapRequested?.Invoke(this, EventArgs.Empty);
}
