using System;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

public sealed class MissionProgressCard : UserControl
{
    private const string ViewPath = "view.missionProgress";

    private readonly TextBlock _line = FormLook.Text("", "FontSizeBody", "TextPrimaryBrush", true);
    private readonly ProgressBar _bar = new() { Minimum = 0, Maximum = 1, HorizontalAlignment = HorizontalAlignment.Stretch };
    private readonly Button _skip;
    private int? _skipTo;

    public event EventHandler<int>? SkipRequested;

    public MissionProgressCard()
    {
        _skip = FormLook.Button("Skip", () =>
        {
            if (_skipTo is { } next) SkipRequested?.Invoke(this, next);
        }, ButtonLook.Subtle);
        AutomationProperties.SetName(_skip, "Skip waypoint");
        _line.VerticalAlignment = VerticalAlignment.Center;
        _bar.Foreground = FormLook.Brush("AccentBrush");
        Visibility = Visibility.Collapsed;
        Content = new Border
        {
            MaxWidth = 360,
            Padding = new Thickness(12, 8, 12, 12),
            Background = FormLook.Brush("CardBrush"),
            BorderBrush = FormLook.Brush("CardStrokeBrush"),
            BorderThickness = new Thickness(1),
            CornerRadius = FormLook.Radius("RadiusOverlay"),
            Child = FormLook.Stack(8, Tight(FormLook.Row(_line, _skip)), _bar),
        };
        Live.Watch(this, ViewPath, Apply);
    }

    private static string Line(JsonObject view) =>
        string.Join(" · ", new[]
        {
            $"To waypoint {Whole(view["current"])} of {Whole(view["last"])}",
            string.Join(" ", new[] { Shown(view["distanceToNext"]), Shown(view["distanceUnits"]) }.Where(part => part.Length > 0)),
        }.Where(part => part.Length > 0));

    internal static Grid Tight(Grid row)
    {
        row.Padding = new Thickness(0);
        row.MinHeight = 0;
        return row;
    }

    private static string Shown(JsonNode? node) => FormJson.Shown(node).Trim();

    private static int Whole(JsonNode? node) => (int)(MapUiJson.Number(node) ?? 0);

    private void Apply(JsonObject view)
    {
        var shown = MapUiJson.Flag(view["shown"]);
        Visibility = FormLook.Shown(shown);
        _skipTo = shown && MapUiJson.Flag(view["canSkip"]) ? Whole(view["skipTo"]) : null;
        _skip.Visibility = FormLook.Shown(_skipTo is not null);
        if (!shown) return;
        FormLook.Say(_line, Line(view));
        _bar.Value = Math.Clamp(MapUiJson.Number(view["fraction"]) ?? 0, 0, 1);
    }
}
