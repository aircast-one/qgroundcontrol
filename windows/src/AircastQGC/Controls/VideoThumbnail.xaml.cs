using System;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Forms;
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
    private bool _connected;
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
        Live.Watch(this, NoVehicleState.FlyStatePath, fly =>
        {
            _connected = !NoVehicleState.IsNoVehicle(fly);
            PaintPresence();
        });
        Loaded += (_, _) => Start();
        Unloaded += (_, _) => Stop();
    }

    public event EventHandler? SwapRequested;

    public event EventHandler? PictureChanged;

    public bool IsLive { get; private set; }

    public bool Picture { get; private set; }

    public FrameworkElement AimRail => Synthetic.AimRail;

    public FrameworkElement CameraTag => NoCamera;

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
            PaintPlaceholder();
            PaintPresence();
        }
    }

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
        var live = VideoJson.Flag(video, "decoding");
        var missing = VideoJson.Flag(video, "sourceChosen") ? VideoJson.Text(video, "noVideoText") : "";

        IsLive = live;
        _video = video;
        PaintAspect();
        _title = VideoJson.Or(camera is null ? "" : VideoJson.Text(camera, "title"), "Video");
        Synthetic.Wanted = !live;
        Surface.Visibility = VideoJson.Shown(live);
        _headline = CameraState(video, camera);
        _detail = live ? "" : VideoJson.Or(VideoJson.Text(video, "noVideoReason"), missing);
        PaintPlaceholder();
        PaintPicture();
        PaintPresence();
    }

    private static string CameraState(JsonObject video, JsonObject? camera)
    {
        var status = camera is null ? "" : VideoJson.Text(camera, "status");
        return !VideoJson.Flag(video, "streamEnabled", true) ? "Video off"
            : !VideoJson.Flag(video, "available") ? VideoJson.Or(VideoJson.Text(video, "summary"), "No video")
            : camera is not null && VideoJson.Flag(camera, "connecting") ? "Connecting to the camera…"
            : status.Length > 0 ? $"Camera: {status}"
            : VideoJson.Or(VideoJson.Text(video, "summary"), "No video");
    }

    private void PaintPresence() => Visibility = VideoJson.Shown(_expanded || _connected || IsLive);

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
        var tag = Synthetic.Pictured ? PictureTag.NoCameraMapView : PictureTag.NoCamera;
        var headline = waiting ? SyntheticView.WaitingHeadline : _headline;
        Caption.Text = IsLive ? _title : tag;
        CaptionPill.BorderThickness = new Thickness(IsLive ? 0 : 1);
        Dot.Style = (Style)Resources[IsLive ? "VideoDotLive" : "VideoDotConnecting"];
        Headline.Text = headline;
        Placeholder.Visibility = VideoJson.Shown(!shown && !_expanded);
        if (IsLive || !_expanded) NoCamera.Hide();
        else NoCamera.Say(tag, Synthetic.Showing && !Synthetic.Chosen ? headline : "");
        if (shown || !_expanded) _pill?.Hide();
        else _pill?.Say(headline, waiting ? SyntheticView.WaitingDetail : _detail);
    }

    private void OnTileClick(object sender, RoutedEventArgs e)
    {
        if (!_expanded) SwapRequested?.Invoke(this, EventArgs.Empty);
    }
}
