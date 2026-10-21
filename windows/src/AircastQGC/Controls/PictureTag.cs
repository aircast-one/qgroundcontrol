using System.Linq;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;

namespace AircastQGC.Controls;

public sealed partial class PictureTag : UserControl
{
    public const string NoCamera = "No camera";
    public const string NoCameraMapView = "No camera · 3D map view";
    private const string VideoView = "view.video";
    private const string VideoSettings = "Video settings";
    private const string VideoGlyph = "";
    private const string ArrowGlyph = "";
    private const string SettingsGlyph = "";
    private const double MaxText = 320;

    private readonly TextBlock _title = FormLook.Text("", "FontSizeBody", "TextPrimaryBrush", true);
    private readonly TextBlock _detail = FormLook.Caption("");
    private readonly MenuFlyout _menu = new() { Placement = FlyoutPlacementMode.BottomEdgeAlignedLeft };
    private readonly Button _button;

    public PictureTag()
    {
        _title.TextWrapping = TextWrapping.NoWrap;
        _title.TextTrimming = TextTrimming.CharacterEllipsis;
        _title.MaxWidth = MaxText;
        _detail.TextWrapping = TextWrapping.NoWrap;
        _detail.TextTrimming = TextTrimming.CharacterEllipsis;
        _detail.MaxWidth = MaxText;
        var glyph = FormLook.Glyph(VideoGlyph, 18, "CautionBrush");
        glyph.VerticalAlignment = VerticalAlignment.Center;
        var arrow = FormLook.Glyph(ArrowGlyph, 10, "TextPrimaryBrush");
        arrow.VerticalAlignment = VerticalAlignment.Center;
        _button = new Button
        {
            MinHeight = 40,
            Padding = new Thickness(12, 6, 12, 6),
            CornerRadius = FormLook.Radius("RadiusOverlay"),
            BorderThickness = new Thickness(1),
            Content = FormLook.Line(10, glyph, new StackPanel { VerticalAlignment = VerticalAlignment.Center, Children = { _title, _detail } }, arrow),
            Flyout = _menu,
        };
        PreflightLook.Chip(_button);
        new[] { "ButtonBorderBrush", "ButtonBorderBrushPointerOver", "ButtonBorderBrushPressed" }.ToList()
            .ForEach(key => _button.Resources[key] = FormLook.Brush("CautionBrush"));
        ToolTipService.SetToolTip(_button, VideoSettings);
        _menu.Opening += (_, _) => Fill();
        Content = _button;
        Visibility = Visibility.Collapsed;
    }

    public void Say(string title, string detail)
    {
        FormLook.Say(_title, title);
        FormLook.Say(_detail, detail);
        AutomationProperties.SetName(_button, detail.Length > 0 ? $"{title}. {detail}" : title);
        Visibility = Visibility.Visible;
    }

    public void Hide() => Visibility = Visibility.Collapsed;

    private void Fill()
    {
        var cameras = CameraList.Parse(Bridge.Get(CameraList.View));
        var streamOn = VideoJson.Flag(Bridge.Get(VideoView), "streamEnabled", true);
        var listed = CameraMenu.Items(cameras, streamOn, () => { });
        var sources = new MenuFlyoutItem { Text = VideoSettings, Icon = new FontIcon { Glyph = SettingsGlyph } };
        ToolTipService.SetToolTip(sources, "Add or fix a camera in Settings");
        sources.Click += (_, _) => FlyRequests.OpenSettings(Pages.Settings.SettingsCatalog.Video);
        _menu.Items.Clear();
        listed.Concat(listed.Count > 0 ? new MenuFlyoutItemBase[] { new MenuFlyoutSeparator() } : []).Append(sources).ToList().ForEach(_menu.Items.Add);
    }
}
