using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Pages.Analyze;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

internal sealed record FollowMeReading(string Mode, bool WouldSend, string Reason, int Following, int Vehicles)
{
    internal const string ViewPath = "view.followMe";

    internal static FollowMeReading? Parse(JsonObject view)
    {
        if (MapUiJson.Text(view["class"]) != "FollowMe") return null;
        var vehicles = MapUiJson.Objects(view["vehicles"]);
        return new FollowMeReading(
            MapUiJson.Text(view["mode"]),
            MapUiJson.Flag(view["wouldSend"]),
            MapUiJson.Text(view["reason"]),
            vehicles.Count(vehicle => MapUiJson.Flag(vehicle["following"])),
            vehicles.Count);
    }

    internal string? Label =>
        Mode is not ("always" or "followMe") || Vehicles == 0 || Reason == "noVehicleInFollowMode"
            ? null
            : WouldSend
                ? Following > 1 ? $"Following you · {Following} vehicles" : "Following you"
                : $"Not following you — {Trouble(Reason)}";

    internal static string Trouble(string reason) => reason switch
    {
        "modeUnknown" => "the Follow Me setting is not one this app understands",
        "modeNever" => "Follow Me is switched off",
        "noVehicles" => "no vehicle is connected",
        "noVehicleInFollowMode" => "no vehicle is in Follow Me mode",
        "noFix" => "this computer has no position yet",
        "fixInvalid" => "this computer's position is not valid",
        "fixStale" => "this computer's position has stopped updating",
        "fixUnusable" => "this computer's position cannot be used",
        "allVehiclesRefused" => "the vehicle refused the position",
        _ => "the position is not being sent",
    };
}

public sealed class FollowMeReadout : UserControl
{
    private const string Client = "followMeReadout";
    private const double ReadoutWidth = 300;

    private readonly TextBlock _text = Ui.Text("", "FontSizeBody");
    private readonly Border _box;

    public FollowMeReadout()
    {
        Visibility = Visibility.Collapsed;
        _text.MaxLines = 2;
        _text.TextTrimming = TextTrimming.CharacterEllipsis;
        _box = new Border
        {
            MaxWidth = ReadoutWidth,
            HorizontalAlignment = HorizontalAlignment.Left,
            Padding = new Thickness(10, 6, 10, 6),
            BorderThickness = new Thickness(1),
            BorderBrush = Ui.Token("CardStrokeBrush"),
            CornerRadius = (CornerRadius)Application.Current.Resources["RadiusOverlay"],
            Child = _text,
        };
        Content = _box;
        Loaded += (_, _) =>
        {
            if (!CoreHost.Started) return;
            BridgeWatch.Watch(Client, new[] { FollowMeReading.ViewPath }, Apply);
            Apply(Bridge.Get(FollowMeReading.ViewPath));
        };
        Unloaded += (_, _) => BridgeWatch.Stop(Client);
    }

    private void Apply(JsonObject view)
    {
        var reading = FollowMeReading.Parse(view);
        var label = reading?.Label;
        Visibility = Ui.Shown(label != null);
        _text.Text = label ?? "";
        _box.Background = Ui.Token(reading?.WouldSend == true ? "SuccessBgBrush" : "CardBrush");
    }
}
