using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Media;

namespace AircastQGC.Controls;

public sealed partial class CameraSwitch : UserControl
{
    private const string VideoView = "view.video";
    private const string SetActiveSource = "video.setActiveVideoSource";
    private const string MultiViewSetting = "settings.videoSettings.multiViewEnabled";
    private const string Group = "camera-switch";
    private const string PipGlyph = "";
    private const string ArrowGlyph = "";
    private const double LabelMax = 120;
    private const double Dot = 8;

    private readonly Border _dot = FormLook.Dot("TextTertiaryBrush", Dot);
    private readonly TextBlock _label = FormLook.Text("", "FontSizeBody");
    private readonly FontIcon _pip = FormLook.Glyph(PipGlyph, 14, "TextPrimaryBrush");
    private readonly Button _chip;
    private readonly MenuFlyout _menu = new() { Placement = FlyoutPlacementMode.LeftEdgeAlignedTop };
    private CameraList? _cameras;
    private bool _streamOn = true;

    public CameraSwitch()
    {
        _label.MaxWidth = LabelMax;
        _label.TextWrapping = TextWrapping.NoWrap;
        _label.Visibility = Visibility.Visible;
        _label.TextTrimming = TextTrimming.CharacterEllipsis;
        _label.VerticalAlignment = VerticalAlignment.Center;
        _chip = new Button
        {
            Height = 36,
            Padding = new Thickness(12, 0, 10, 0),
            CornerRadius = new CornerRadius(18),
            Content = FormLook.Line(6, _dot, _label, _pip, FormLook.Glyph(ArrowGlyph, 10, "TextPrimaryBrush")),
            Flyout = _menu,
        };
        PreflightLook.Chip(_chip);
        ToolTipService.SetToolTip(_chip, "Cameras");
        _menu.Opening += (_, _) => Fill();
        Content = _chip;
        Visibility = Visibility.Collapsed;
        Live.Watch(this, CameraList.View, view =>
        {
            _cameras = CameraList.Parse(view);
            Paint();
        });
        Live.Watch(this, VideoView, video =>
        {
            _streamOn = VideoJson.Flag(video, "streamEnabled", true);
            Paint();
        });
    }

    private IReadOnlyList<CameraEntry> Switchable =>
        _cameras?.Cameras.Where(camera => camera.Active || camera.Problem.Length == 0).ToList() ?? [];

    private CameraEntry? Shown => Switchable.FirstOrDefault(camera => camera.Active) ?? Switchable.FirstOrDefault();

    private bool PipOffered => _streamOn && _cameras?.PipSlot is not null;

    private void Paint()
    {
        var shown = Shown;
        Visibility = FormLook.Shown(shown is not null && (Switchable.Count > 1 || PipOffered));
        if (shown is null) return;
        _dot.Background = FormLook.Brush(CameraList.StatusLook(shown.Status).Brush);
        _label.Text = VideoJson.Or(shown.Short, shown.Title);
        _pip.Visibility = FormLook.Shown(PipOffered && _cameras?.PipEnabled == true);
        AutomationProperties.SetName(_chip, $"Cameras, showing {shown.Title}");
    }

    private void Fill()
    {
        var cameras = Switchable.Count > 1 ? Switchable.Select(CameraItem).ToList() : [];
        var pip = PipOffered ? [PipItem()] : new List<MenuFlyoutItemBase>();
        var divider = cameras.Count > 0 && pip.Count > 0 ? [new MenuFlyoutSeparator()] : new List<MenuFlyoutItemBase>();
        _menu.Items.Clear();
        cameras.Concat(divider).Concat(pip).ToList().ForEach(_menu.Items.Add);
    }

    private MenuFlyoutItemBase CameraItem(CameraEntry camera)
    {
        var (status, brush) = CameraList.StatusLook(camera.Status);
        var item = new RadioMenuFlyoutItem
        {
            Text = camera.Title,
            IsChecked = camera.Active,
            GroupName = Group,
            Icon = new PathIcon
            {
                Data = new EllipseGeometry { Center = new Windows.Foundation.Point(Dot / 2, Dot / 2), RadiusX = Dot / 2, RadiusY = Dot / 2 },
                Foreground = FormLook.Brush(brush),
            },
        };
        AutomationProperties.SetItemStatus(item, status);
        ToolTipService.SetToolTip(item, status);
        item.Click += (_, _) => Show(camera);
        return item;
    }

    private MenuFlyoutItemBase PipItem()
    {
        var on = _cameras?.PipEnabled == true;
        var item = new ToggleMenuFlyoutItem
        {
            Text = "Picture in picture",
            IsChecked = on,
            Icon = new FontIcon { Glyph = PipGlyph },
        };
        ToolTipService.SetToolTip(item, on ? "Hide the second camera" : "Show the second camera in a small picture");
        item.Click += (_, _) =>
        {
            Bridge.Set(MultiViewSetting, !on);
            Refresh();
        };
        return item;
    }

    private void Show(CameraEntry camera)
    {
        if (camera.Active) return;
        Bridge.Invoke(SetActiveSource, new JsonArray(camera.Slot));
        Refresh();
    }

    private void Refresh()
    {
        _cameras = CameraList.Parse(Bridge.Get(CameraList.View));
        Paint();
    }
}
