using System;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

public sealed class ConnectingCard : UserControl
{
    private const string CompletePath = "vehicle.initialConnectComplete";
    private const string ProgressPath = "vehicle.loadProgress";
    private const string VehiclesPath = "view.vehicles";
    private const string SignalLost = "Signal lost";
    private const string LostHint = "Point the antenna at the aircraft or move closer.";
    private const string LinkGlyph = "";
    private const string WarningGlyph = "";

    private readonly FontIcon _icon = new() { FontFamily = FormLook.Icons, FontSize = 32, HorizontalAlignment = HorizontalAlignment.Center, VerticalAlignment = VerticalAlignment.Center };
    private readonly Border _badge = new() { Width = 72, Height = 72, CornerRadius = new CornerRadius(36), HorizontalAlignment = HorizontalAlignment.Center };
    private readonly TextBlock _title = Centered(FormLook.Text("", "FontSizeSubtitle", "TextPrimaryBrush", true));
    private readonly TextBlock _detail = Centered(FormLook.Text("", "FontSizeBody", "TextSecondaryBrush"));
    private readonly TextBlock _percent = FormLook.Text("", "FontSizeBody", "TextSecondaryBrush");
    private readonly ProgressBar _bar = new() { Minimum = 0, Maximum = 100, HorizontalAlignment = HorizontalAlignment.Stretch };
    private readonly TextBlock _dismissLabel = new() { FontFamily = PreflightLook.Font };
    private readonly Button _dismiss;
    private bool? _complete;
    private double _progress;
    private string _name = "";
    private bool _lost;
    private bool _dismissed;

    public ConnectingCard()
    {
        _badge.Child = _icon;
        _dismissLabel.FontSize = FormLook.Size("FontSizeBody");
        _dismiss = new Button { Content = _dismissLabel, HorizontalAlignment = HorizontalAlignment.Center, CornerRadius = FormLook.Radius("RadiusControl") };
        FormLook.Paint(_dismiss, ButtonLook.Subtle);
        _dismiss.Click += (_, _) =>
        {
            _dismissed = true;
            Render();
        };
        HorizontalAlignment = HorizontalAlignment.Center;
        VerticalAlignment = VerticalAlignment.Center;
        Visibility = Visibility.Collapsed;
        Content = new Border
        {
            MaxWidth = 400,
            Margin = new Thickness(16),
            Padding = new Thickness(24),
            Background = FormLook.Brush("CardBrush"),
            BorderBrush = FormLook.Brush("CardStrokeBrush"),
            BorderThickness = new Thickness(1),
            CornerRadius = FormLook.Radius("RadiusOverlay"),
            Child = FormLook.Stack(12,
                _badge,
                _title,
                _detail,
                MissionProgressCard.Tight(FormLook.Row(FormLook.Text("Parameters", "FontSizeBody", "TextPrimaryBrush", true), _percent)),
                _bar,
                _dismiss),
        };
        Live.Watch(this, CompletePath, view =>
        {
            _complete = view["value"] is JsonValue value && value.TryGetValue<bool>(out var done) ? done : null;
            Render();
        });
        Live.Watch(this, ProgressPath, view =>
        {
            _progress = MapUiJson.Number(view["value"]) ?? 0;
            Render();
        });
        Live.Watch(this, VehiclesPath, view =>
        {
            _name = MapUiJson.Objects(view["vehicles"]).Where(v => MapUiJson.Flag(v["active"])).Select(v => MapUiJson.Text(v["name"])).FirstOrDefault() ?? "";
            Render();
        });
        Live.Watch(this, NoVehicleState.FlyStatePath, view =>
        {
            _lost = MapUiJson.Flag(view["contactLost"]);
            Render();
        });
    }

    private static double? LoadedFraction(bool? complete, double progress) =>
        complete == false ? Math.Clamp(progress, 0, 1) : null;

    private static (string Title, string Detail, string Dismiss) Words(string name, bool lost) =>
        lost
            ? (SignalLost, $"Loading stopped. It picks up where it left off when the aircraft answers. {LostHint}", "Hide")
            : (name.Trim().Length > 0 ? $"Connecting to {name.Trim()}" : "Connecting", "Loading its settings.", "Fly now, finish loading in background");

    private void Render()
    {
        var loading = LoadedFraction(_complete, _progress);
        _dismissed = _dismissed && loading is not null;
        Visibility = FormLook.Shown(loading is not null && !_dismissed);
        if (loading is not { } fraction) return;
        var (title, detail, dismiss) = Words(_name, _lost);
        _icon.Glyph = _lost ? WarningGlyph : LinkGlyph;
        _icon.Foreground = FormLook.Brush(_lost ? "CriticalBrush" : "AccentTextBrush");
        _badge.Background = FormLook.Brush(_lost ? "CriticalBgBrush" : "InfoBgBrush");
        FormLook.Say(_title, title);
        _title.Foreground = FormLook.Brush(_lost ? "CriticalBrush" : "TextPrimaryBrush");
        FormLook.Say(_detail, detail);
        FormLook.Say(_percent, ((int)(fraction * 100)).ToString(CultureInfo.InvariantCulture) + "%");
        _bar.Value = fraction * 100;
        _bar.Foreground = FormLook.Brush(_lost ? "TextTertiaryBrush" : "AccentBrush");
        _dismissLabel.Text = dismiss;
        AutomationProperties.SetName(_dismiss, dismiss);
    }

    private static TextBlock Centered(TextBlock text)
    {
        text.HorizontalAlignment = HorizontalAlignment.Center;
        text.TextAlignment = TextAlignment.Center;
        return text;
    }
}
