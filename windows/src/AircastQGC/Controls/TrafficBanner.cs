using System;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Pages.Analyze;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;

namespace AircastQGC.Controls;

public sealed class TrafficBanner : UserControl
{
    private const string TrafficPath = "view.adsbTraffic";
    private const double MaxBannerWidth = 560;

    private readonly string _client = $"trafficBanner.{Guid.NewGuid():N}";
    private readonly Button _banner;
    private readonly FontIcon _icon = Ui.Glyph("", 20);
    private readonly TextBlock _title = Ui.Text("", semibold: true, wrap: false);
    private readonly TextBlock _detail = Ui.Text("", wrap: false);
    private readonly Sheet _list;
    private TrafficReading? _reading;

    public TrafficBanner()
    {
        _list = new Sheet("Traffic", ListBody) { Placement = FlyoutPlacementMode.Bottom };
        _banner = new Button
        {
            MaxWidth = MaxBannerWidth,
            MinHeight = 48,
            HorizontalAlignment = HorizontalAlignment.Stretch,
            HorizontalContentAlignment = HorizontalAlignment.Left,
            Padding = new Thickness(12, 8, 12, 8),
            CornerRadius = new CornerRadius(12),
            BorderThickness = new Thickness(0),
            Content = new StackPanel
            {
                Orientation = Orientation.Horizontal,
                Spacing = 12,
                Children = { _icon, new StackPanel { Children = { _title, _detail } } },
            },
        }.Named("Traffic");
        _banner.Click += (_, _) => _list.ShowAt(_banner);
        Content = _banner;
        Visibility = Visibility.Collapsed;
        Loaded += (_, _) =>
        {
            if (!CoreHost.Started) return;
            BridgeWatch.Watch(_client, [TrafficPath], Apply);
            Apply(Bridge.Get(TrafficPath));
        };
        Unloaded += (_, _) => BridgeWatch.Stop(_client);
    }

    private void Apply(JsonObject view)
    {
        _reading = TrafficReading.Parse(view);
        var alert = _reading?.Alert;
        Visibility = Ui.Shown(alert is not null);
        _list.Refresh();
        if (alert is null || _reading is null) return;
        var urgent = alert.Level >= TrafficLevel.Warning;
        var fill = Ui.Token(urgent ? "CriticalBgBrush" : "CautionBgBrush");
        _banner.Background = fill;
        _banner.Resources["ButtonBackgroundPointerOver"] = fill;
        _banner.Resources["ButtonBackgroundPressed"] = fill;
        _banner.Resources["ButtonBackgroundDisabled"] = fill;
        _banner.IsEnabled = _reading.Contacts.Count > 0;
        _icon.Glyph = urgent ? "" : "";
        _icon.Foreground = Ui.Token(urgent ? "CriticalBrush" : "CautionBrush");
        _title.Text = alert.Title;
        _title.Foreground = Ui.Token("TextPrimaryBrush");
        _detail.Text = alert.Detail;
        _detail.Foreground = Ui.Token("TextPrimaryBrush");
        _detail.Visibility = Ui.Shown(alert.Detail.Trim().Length > 0);
        AutomationProperties.SetHelpText(_banner, Spoken(alert));
    }

    private static string Spoken(TrafficAlert alert) =>
        string.Join(" · ", new[] { alert.Title, alert.Detail }.Where(t => t.Trim().Length > 0));

    private UIElement? ListBody() => _reading is not TrafficReading reading ? null : Ui.Column(
    [
        StatusLook.Line(reading.Summary).With(t => t.FontWeight = Microsoft.UI.Text.FontWeights.SemiBold),
        StatusLook.Line(reading.Caption, StatusTone.Muted, small: true),
        .. reading.Contacts.Select(contact => (UIElement?)Ui.Line(
        [
            StatusLook.Line(contact.Name, contact.Urgent ? StatusTone.Critical : StatusTone.Normal).With(t => t.FontWeight = Microsoft.UI.Text.FontWeights.SemiBold),
            StatusLook.Line(reading.ContactText(contact), contact.Urgent ? StatusTone.Critical : StatusTone.Normal).With(t => t.MaxWidth = 260),
        ], 12)),
    ], 6);
}
