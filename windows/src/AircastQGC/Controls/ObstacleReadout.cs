using System.Text.Json.Nodes;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

public sealed partial class ObstacleReadout : UserControl
{
    private const double BackdropAlpha = 0.9;

    private readonly TextBlock _label = FormLook.Text("", "FontSizeBody", "TextPrimaryBrush", strong: true);
    private readonly Border _pill;

    public ObstacleReadout()
    {
        _pill = new Border
        {
            Padding = new Thickness(10, 6, 10, 6),
            CornerRadius = FormLook.Radius("RadiusControl"),
            Child = _label,
        };
        Content = _pill;
        Visibility = Visibility.Collapsed;
        Live.Watch(this, ObstacleModel.View, Render);
    }

    internal void Render(JsonObject view)
    {
        var warning = ObstacleModel.Warning(view);
        Visibility = FormLook.Shown(warning is not null);
        FormLook.Say(_label, warning?.Label);
        _pill.Background = OverlayInk.Brush(warning?.Close == true ? "CriticalBgBrush" : "LayerBrush", BackdropAlpha);
        AutomationProperties.SetName(this, warning is null ? "" : $"Nearest obstacle {warning.Label}");
    }
}
