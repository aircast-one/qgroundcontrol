using System;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

public sealed partial class VideoThumbnail : UserControl
{
    private const string Client = "video-thumbnail";
    private const string VideoView = "view.video";
    private const string FitClient = "video-thumbnail.fit";
    private const string FitView = "view.control(settings.videoSettings.videoFit)";
    private const string AspectClient = "video-thumbnail.aspect";
    private const string MainName = "Video";

    private readonly CornerRadius _radius;
    private readonly Thickness _border;
    private readonly object _tip;
    private readonly string _swapName;
    private bool _expanded;
    private string _standIn = "";
    private string _title = "Video";
    private string _headline = "No video";
    private string _detail = "";
    private NoVideoPill? _pill;
    private JsonObject _video = new();
    private double? _aspectSetting;

    public VideoThumbnail()
    {
        InitializeComponent();
        _radius = Tile.CornerRadius;
        _border = Tile.BorderThickness;
        _tip = ToolTipService.GetToolTip(Tile);
        _swapName = AutomationProperties.GetName(Tile);
        Synthetic.Changed += (_, _) =>
        {
            PaintPlaceholder();
            PaintPicture();
        };
        Loaded += (_, _) => Start();
        Unloaded += (_, _) => Stop();
    }

    public event EventHandler? SwapRequested;

    public event EventHandler? PictureChanged;

    public bool IsLive { get; private set; }

    public bool Picture { get; private set; }

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
            AutomationProperties.SetName(Tile, value ? MainName : _swapName);
            Tile.IsTabStop = !value;
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
        BridgeWatch.Watch(AspectClient, new[] { VideoSurface.AspectView }, RenderAspect);
        Render(Bridge.Get(VideoView));
        RenderFit(Bridge.Get(FitView));
        RenderAspect(Bridge.Get(VideoSurface.AspectView));
    }

    private void Stop()
    {
        if (IsLoaded) return;
        BridgeWatch.Stop(Client);
        BridgeWatch.Stop(FitClient);
        BridgeWatch.Stop(AspectClient);
    }

    private void RenderFit(JsonObject fit) =>
        Surface.Fit = VideoJson.Number(fit["value"]) is double mode ? (int)mode : VideoSurface.FitHeight;

    private void RenderAspect(JsonObject aspect)
    {
        _aspectSetting = VideoJson.Number(aspect["value"]);
        PaintAspect();
    }

    private void PaintAspect() => Surface.Aspect = VideoSurface.PictureAspect(_video, _aspectSetting);

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
        _video = video;
        PaintAspect();
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
        PaintPicture();
    }

    private void PaintPicture()
    {
        var picture = IsLive || Synthetic.Showing;
        if (picture == Picture) return;
        Picture = picture;
        PictureChanged?.Invoke(this, EventArgs.Empty);
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

    private void OnTileClick(object sender, RoutedEventArgs e)
    {
        if (!_expanded) SwapRequested?.Invoke(this, EventArgs.Empty);
    }
}
