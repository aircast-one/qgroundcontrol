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
    private const string SwitchSourceAction = "video.switchActiveVideoSource";
    private const string FitClient = "video-thumbnail.fit";
    private const string FitView = "view.control(settings.videoSettings.videoFit)";
    private const string CamerasClient = "video-thumbnail.cameras";
    private const string MultiViewSetting = "settings.videoSettings.multiViewEnabled";

    private readonly CornerRadius _radius;
    private readonly Thickness _border;
    private bool _expanded;
    private bool _streamOn = true;
    private CameraList? _cameras;

    public VideoThumbnail()
    {
        InitializeComponent();
        _radius = Tile.CornerRadius;
        _border = Tile.BorderThickness;
        Loaded += (_, _) => Start();
        Unloaded += (_, _) => Stop();
    }

    public event EventHandler? SwapRequested;

    public bool IsLive { get; private set; }

    public bool Expanded
    {
        get => _expanded;
        set
        {
            _expanded = value;
            Tile.CornerRadius = value ? new CornerRadius(0) : _radius;
            Tile.BorderThickness = value ? new Thickness(0) : _border;
        }
    }

    public void Show() => Visibility = Visibility.Visible;

    public void Hide() => Visibility = Visibility.Collapsed;

    private void Start()
    {
        BridgeWatch.Watch(Client, new[] { VideoView }, Render);
        BridgeWatch.Watch(FitClient, new[] { FitView }, RenderFit);
        BridgeWatch.Watch(CamerasClient, new[] { CameraList.View }, RenderCameras);
        Render(Bridge.Get(VideoView));
        RenderFit(Bridge.Get(FitView));
        RenderCameras(Bridge.Get(CameraList.View));
    }

    private void Stop()
    {
        if (IsLoaded) return;
        BridgeWatch.Stop(Client);
        BridgeWatch.Stop(FitClient);
        BridgeWatch.Stop(CamerasClient);
    }

    private void RenderFit(JsonObject fit) =>
        Surface.Fit = VideoJson.Number(fit["value"]) is double mode ? (int)mode : VideoSurface.FitHeight;

    private void RenderCameras(JsonObject view)
    {
        _cameras = CameraList.Parse(view);
        PaintPip();
    }

    private void PaintPip()
    {
        var on = _cameras?.PipEnabled == true;
        var tip = on ? "Hide the second camera" : "Show the second camera in a small picture";
        PipToggle.Visibility = VideoJson.Shown(_streamOn && _cameras?.PipSlot is not null);
        PipGlyph.Foreground = PreflightLook.Token(on ? "AccentBrush" : "TextPrimaryBrush");
        AutomationProperties.SetItemStatus(PipToggle, on ? "On" : "Off");
        ToolTipService.SetToolTip(PipToggle, tip);
    }

    private void Render(JsonObject video)
    {
        var active = VideoJson.Int(video, "activeSource", 0);
        var camera = (video["cameras"] as JsonArray ?? new JsonArray())
            .OfType<JsonObject>()
            .FirstOrDefault(c => VideoJson.Int(c, "slot", -1) == active);
        var title = VideoJson.Or(camera is null ? "" : VideoJson.Text(camera, "title"), "Video");
        var status = camera is null ? "" : VideoJson.Text(camera, "status");
        var live = VideoJson.Flag(video, "decoding");
        var connecting = VideoJson.Flag(video, "anyConnecting");
        var missing = VideoJson.Flag(video, "sourceChosen") ? VideoJson.Text(video, "noVideoText") : "";

        IsLive = live;
        Surface.Visibility = VideoJson.Shown(live);
        Placeholder.Visibility = VideoJson.Shown(!live);
        Caption.Text = live || status.Length == 0 ? title : $"{title} · {status}";
        Dot.Style = (Style)Resources[live ? "VideoDotLive" : connecting ? "VideoDotConnecting" : "VideoDotIdle"];
        Headline.Text = VideoJson.Flag(video, "streamEnabled", true)
            ? VideoJson.Or(VideoJson.Text(video, "summary"), "No video")
            : "Video off";
        Detail.Text = live ? "" : VideoJson.Or(VideoJson.Text(video, "noVideoReason"), missing);
        Detail.Visibility = VideoJson.Shown(Detail.Text.Length > 0);
        SourceTitle.Text = title;
        SwitchSource.Visibility = VideoJson.Shown(VideoJson.Flag(video, "multipleSources") && camera is not null);
        _streamOn = VideoJson.Flag(video, "streamEnabled", true);
        PaintPip();
    }

    private void OnTileClick(object sender, RoutedEventArgs e) => SwapRequested?.Invoke(this, EventArgs.Empty);

    private void OnPipToggle(object sender, RoutedEventArgs e)
    {
        Bridge.Set(MultiViewSetting, _cameras?.PipEnabled != true);
        RenderCameras(Bridge.Get(CameraList.View));
    }

    private void OnSwitchSource(object sender, RoutedEventArgs e)
    {
        Bridge.Invoke(SwitchSourceAction);
        Render(Bridge.Get(VideoView));
    }
}
