using System.Text.Json.Nodes;
using AircastQGC.Core;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;

namespace AircastQGC.Controls;

public sealed partial class PipTile : UserControl
{
    private const string Client = "pip-tile";
    private const string SetPipShown = "video.setPipShown";
    private const string SetActiveSource = "video.setActiveVideoSource";

    private readonly VideoSurface _surface = new() { Channel = VideoNative.Pip };
    private readonly SyntheticView _synthetic = new();
    private readonly TextBlock _waiting = new()
    {
        HorizontalAlignment = HorizontalAlignment.Center,
        VerticalAlignment = VerticalAlignment.Center,
        FontFamily = PreflightLook.Font,
        FontSize = PreflightLook.Size("FontSizeCaption"),
        Foreground = PreflightLook.Token("TextPrimaryBrush"),
    };
    private readonly Grid _cover;
    private readonly Border _dot = new() { Width = 6, Height = 6, CornerRadius = new CornerRadius(3), VerticalAlignment = VerticalAlignment.Center };
    private readonly TextBlock _caption = new()
    {
        FontFamily = PreflightLook.Font,
        FontSize = PreflightLook.Size("FontSizeCaption"),
        Foreground = PreflightLook.Token("TextPrimaryBrush"),
        VerticalAlignment = VerticalAlignment.Center,
        TextTrimming = TextTrimming.CharacterEllipsis,
        MaxLines = 1,
        MaxWidth = 120,
    };
    private readonly Button _tile;
    private int? _slot;
    private bool? _reported;

    public PipTile()
    {
        _cover = new Grid { Background = new SolidColorBrush(Microsoft.UI.Colors.Black), Children = { _waiting } };
        var pill = new Border
        {
            HorizontalAlignment = HorizontalAlignment.Left,
            VerticalAlignment = VerticalAlignment.Bottom,
            Margin = new Thickness(6),
            Height = 18,
            Padding = new Thickness(6, 0, 6, 0),
            CornerRadius = new CornerRadius(9),
            Background = PreflightLook.Token("LayerBrush"),
            Child = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 4, Children = { _dot, _caption } },
        };
        _tile = new Button
        {
            HorizontalAlignment = HorizontalAlignment.Stretch,
            VerticalAlignment = VerticalAlignment.Stretch,
            HorizontalContentAlignment = HorizontalAlignment.Stretch,
            VerticalContentAlignment = VerticalAlignment.Stretch,
            Padding = new Thickness(0),
            Background = new SolidColorBrush(Microsoft.UI.Colors.Black),
            BorderBrush = PreflightLook.Token("TextPrimaryBrush"),
            BorderThickness = new Thickness(2),
            Resources = new ResourceDictionary
            {
                ["ButtonBorderBrushPointerOver"] = PreflightLook.Token("TextPrimaryBrush"),
                ["ButtonBorderBrushPressed"] = PreflightLook.Token("TextPrimaryBrush"),
            },
            CornerRadius = (CornerRadius)Application.Current.Resources["RadiusOverlay"],
            Content = new Grid { Children = { _surface, _synthetic, _cover, pill } },
        };
        _tile.Click += (_, _) => Swap();
        Content = _tile;
        Visibility = Visibility.Collapsed;
        Loaded += (_, _) => Start();
        Unloaded += (_, _) => Stop();
    }

    private void Start()
    {
        BridgeWatch.Watch(Client, new[] { CameraList.View }, Render);
        Render(Bridge.Get(CameraList.View));
    }

    private void Stop()
    {
        if (IsLoaded) return;
        BridgeWatch.Stop(Client);
        Report(false);
    }

    private void Render(JsonObject view)
    {
        var camera = CameraList.Parse(view)?.PipCamera;
        _slot = camera?.Slot;
        var synthetic = camera?.Source == CameraList.SyntheticSource;
        _synthetic.Wanted = synthetic;
        _surface.Visibility = VideoJson.Shown(!synthetic);
        Visibility = VideoJson.Shown(camera is not null);
        Report(camera is not null);
        if (camera is null) return;
        var (label, brush) = CameraList.StatusLook(camera.Status);
        _dot.Background = PreflightLook.Token(brush);
        _caption.Text = VideoJson.Or(camera.Short, camera.Title);
        _waiting.Text = label;
        _cover.Visibility = VideoJson.Shown(!camera.Live);
        AutomationProperties.SetName(_tile, $"Second camera, {camera.Title}");
        ToolTipService.SetToolTip(_tile, $"Show {camera.Title} in the main picture");
    }

    private void Report(bool shown)
    {
        if (_reported == shown) return;
        _reported = shown;
        Bridge.Invoke(SetPipShown, new JsonArray(shown));
    }

    private void Swap()
    {
        if (_slot is not int slot) return;
        Bridge.Invoke(SetActiveSource, new JsonArray(slot));
        Render(Bridge.Get(CameraList.View));
    }
}
