using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Automation.Peers;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

public sealed partial class NoVideoPill : UserControl
{
    private const string VideoGlyph = "";
    private const double MaxText = 420;

    private readonly TextBlock _headline = FormLook.Text("", "FontSizeBody", "TextPrimaryBrush", true);
    private readonly TextBlock _detail = FormLook.Caption("");

    public NoVideoPill()
    {
        _headline.MaxWidth = MaxText;
        _headline.TextWrapping = TextWrapping.NoWrap;
        _headline.TextTrimming = TextTrimming.CharacterEllipsis;
        _detail.MaxWidth = MaxText;
        _detail.MaxLines = 2;
        _detail.TextWrapping = TextWrapping.Wrap;
        _detail.TextTrimming = TextTrimming.CharacterEllipsis;
        var glyph = FormLook.Glyph(VideoGlyph, 20, "TextSecondaryBrush");
        glyph.VerticalAlignment = VerticalAlignment.Center;
        Content = new Border
        {
            Padding = new Thickness(16, 10, 20, 10),
            MinHeight = 44,
            CornerRadius = new CornerRadius(22),
            Background = FormLook.Brush("ChipBrush"),
            BorderBrush = FormLook.Brush("ControlStrokeBrush"),
            BorderThickness = new Thickness(1),
            Child = FormLook.Line(12, glyph, new StackPanel { VerticalAlignment = VerticalAlignment.Center, Children = { _headline, _detail } }),
        };
        IsHitTestVisible = false;
        Visibility = Visibility.Collapsed;
        AutomationProperties.SetLiveSetting(this, AutomationLiveSetting.Polite);
    }

    public void Say(string headline, string detail)
    {
        FormLook.Say(_headline, headline);
        FormLook.Say(_detail, detail);
        AutomationProperties.SetName(this, detail.Length > 0 ? $"{headline}. {detail}" : headline);
        Visibility = Visibility.Visible;
    }

    public void Hide() => Visibility = Visibility.Collapsed;
}
