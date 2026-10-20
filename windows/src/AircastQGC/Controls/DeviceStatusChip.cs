using System;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Core;
using AircastQGC.Forms;
using AircastQGC.Pages.Device;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

public sealed class DeviceStatusChip : UserControl
{
    private const double ChipHeight = 32;

    private readonly string _client = $"devicechip.{Guid.NewGuid():N}";
    private readonly string _path = DeviceLink.ViewPath(DeviceLink.Home);
    private readonly Border _dot = FormLook.Dot("TextSecondaryBrush", 8);
    private readonly TextBlock _text = FormLook.Text("", "FontSizeBody");
    private readonly Button _button;

    public DeviceStatusChip()
    {
        _text.TextWrapping = TextWrapping.NoWrap;
        _text.TextTrimming = TextTrimming.CharacterEllipsis;
        _text.Visibility = Visibility.Visible;
        _button = new Button
        {
            Height = ChipHeight,
            MaxWidth = 320,
            Padding = new Thickness(12, 0, 12, 0),
            CornerRadius = new CornerRadius(ChipHeight / 2),
            Content = FormLook.Line(8, _dot, _text),
        };
        PreflightLook.Standard(_button);
        _button.Click += (_, _) => FlyRequests.OpenSettings(DeviceLink.Page);
        _button.Visibility = Visibility.Collapsed;
        Content = _button;
        Loaded += async (_, _) =>
        {
            BridgeWatch.Watch(_client, [_path], Show);
            Show(await Task.Run(() => Bridge.Get(_path)));
        };
        Unloaded += (_, _) => BridgeWatch.Stop(_client);
    }

    private void Show(JsonObject json)
    {
        var shown = DeviceStatus.Of(DeviceReading.Of(json).View) is { NeedsAttention: true } status ? status : null;
        _button.Visibility = FormLook.Shown(shown is not null);
        if (shown is null) return;
        _dot.Background = FormLook.Brush(Kit.LevelBrush(shown.Level));
        _text.Text = shown.Chip;
        AutomationProperties.SetName(_button, $"{shown.Name}: {shown.Headline}");
        ToolTipService.SetToolTip(_button, shown.Chip);
    }
}
