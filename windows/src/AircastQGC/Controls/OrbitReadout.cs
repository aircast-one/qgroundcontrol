using System;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

public sealed partial class OrbitReadout : UserControl
{
    private const string OrbitView = "view.orbit";

    private readonly string _client = $"orbit-readout.{Guid.NewGuid():N}";
    private readonly TextBlock _text = FormLook.Text("", "FontSizeBody", "TextPrimaryBrush", true);

    public OrbitReadout()
    {
        Content = new Border
        {
            Height = 32,
            Padding = new Thickness(14, 0, 14, 0),
            CornerRadius = new CornerRadius(16),
            Background = FormLook.Brush("LayerBrush"),
            BorderBrush = FormLook.Brush("CardStrokeBrush"),
            BorderThickness = new Thickness(1),
            Child = FormLook.Line(8, FormLook.Glyph("", 14, "AccentBrush"), FormLook.Margined(_text, new Thickness(0, 5, 0, 0))),
        };
        Visibility = Visibility.Collapsed;
        Loaded += (_, _) =>
        {
            BridgeWatch.Watch(_client, [OrbitView], Render);
            Render(Bridge.Get(OrbitView));
        };
        Unloaded += (_, _) =>
        {
            if (!IsLoaded) BridgeWatch.Stop(_client);
        };
    }

    private void Render(System.Text.Json.Nodes.JsonObject view)
    {
        var label = PayloadJson.OrbitLabel(view);
        FormLook.Say(_text, label);
        Visibility = VideoJson.Shown(label is not null);
    }
}
