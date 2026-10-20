using System;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Pages.Analyze;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;

namespace AircastQGC.Controls;

public sealed class VehiclePickerButton : UserControl
{
    private const string Client = "vehiclePicker.vehicles";
    private const double PillHeight = 28;
    private const double PickerWidth = 440;
    private const string FlightGlyph = "\uE709";
    private const string ChevronGlyph = "\uE70D";
    private const string WarningGlyph = "\uE7BA";
    private const string Footnote = "Pick a name to fly that aircraft. Arm, Takeoff and every action on the flight screen go to the one you pick.";

    private readonly Button _pill;
    private readonly TextBlock _name = Ui.Text("", "FontSizeCaption", wrap: false);
    private readonly FontIcon _trailing = Ui.Glyph(ChevronGlyph, 10, "TextSecondaryBrush");
    private readonly Flyout _flyout;
    private readonly TextBlock _title = Ui.Text(VehicleChoices.ChooserTitle, "FontSizeBodyLarge", semibold: true);
    private readonly FleetPanel _panel = new();
    private readonly ToggleSwitch _panelToggle = new() { OnContent = "", OffContent = "", MinWidth = 0 };
    private readonly Grid _panelRow;
    private VehicleChoices? _last;
    private bool _painting;

    public VehiclePickerButton()
    {
        Visibility = Visibility.Collapsed;
        _panelRow = new Grid
        {
            Padding = new Thickness(16, 4, 16, 4),
            ColumnDefinitions = { new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) }, new ColumnDefinition { Width = GridLength.Auto } },
            Children = { Ui.Text("Enable multi-vehicle panel"), _panelToggle.With(toggle => Grid.SetColumn(toggle, 1)) },
        };
        _flyout = new Flyout
        {
            Placement = FlyoutPlacementMode.BottomEdgeAlignedLeft,
            FlyoutPresenterStyle = FleetLook.Presenter(PickerWidth),
            Content = Body(),
        };
        _flyout.Opening += (_, _) => Opened();
        _pill = Ui.Styled(new Button
        {
            Height = PillHeight,
            Padding = new Thickness(10, 0, 10, 0),
            CornerRadius = new CornerRadius(PillHeight / 2),
            BorderThickness = new Thickness(1),
            Flyout = _flyout,
            Content = new StackPanel
            {
                Orientation = Orientation.Horizontal,
                Spacing = 6,
                Children = { Ui.Glyph(FlightGlyph, 12, "AccentBrush"), _name, _trailing },
            },
        }, Look.Standard, () => { }).Named("Choose which vehicle to fly");
        _panelToggle.Named("Enable multi-vehicle panel");
        _panelToggle.Toggled += (_, _) =>
        {
            if (_painting) return;
            _panel.Refuse(FleetBridge.SetPanel(_panelToggle.IsOn));
            SyncToggle();
        };
        _panel.Switched += (_, _) => _flyout.Hide();
        _panel.ConfirmRequested += (_, ask) =>
        {
            _flyout.Hide();
            ConfirmRequested?.Invoke(this, ask);
        };
        Content = _pill;

        Loaded += (_, _) => Start();
        Unloaded += (_, _) => BridgeWatch.Stop(Client);
    }

    public event EventHandler<GuidedAsk>? ConfirmRequested;

    public event EventHandler? ConnectRequested;

    public event EventHandler<string>? HandedOver;

    private UIElement Body() => new ScrollViewer
    {
        Width = PickerWidth - 2,
        MaxHeight = 560,
        Content = new StackPanel
        {
            Padding = new Thickness(0, 16, 0, 16),
            Spacing = 4,
            Children =
            {
                _title.With(text => text.Margin = new Thickness(16, 0, 16, 8)),
                _panel,
                Ui.Styled(new Button
                {
                    HorizontalAlignment = HorizontalAlignment.Stretch,
                    HorizontalContentAlignment = HorizontalAlignment.Left,
                    Margin = new Thickness(8, 4, 8, 0),
                    Padding = new Thickness(8, 6, 8, 6),
                    Content = new StackPanel
                    {
                        Orientation = Orientation.Horizontal,
                        Spacing = 12,
                        Children = { Ui.Glyph("\uE710", 14, "AccentBrush"), Ui.Text("Connect another vehicle", brush: "AccentBrush") },
                    },
                }, Look.Subtle, () =>
                {
                    _flyout.Hide();
                    ConnectRequested?.Invoke(this, EventArgs.Empty);
                }).Named("Connect another vehicle"),
                _panelRow,
                Ui.Caption(Footnote, "TextTertiaryBrush").With(text => text.Margin = new Thickness(16, 8, 16, 0)),
            },
        },
    };

    private void Start()
    {
        if (!CoreHost.Started) return;
        BridgeWatch.Watch(Client, new[] { VehicleChoices.ViewPath }, Apply);
        Apply(Bridge.Get(VehicleChoices.ViewPath));
    }

    private void Apply(JsonObject view)
    {
        var now = VehicleChoices.Parse(view);
        var notice = VehicleChoices.HandoverNotice(_last, now, FleetBridge.LastAsked);
        if (VehicleChoices.ActiveChanged(_last, now)) FleetBridge.Forget();
        _last = VehicleChoices.Remembered(_last, now);

        Visibility = Ui.Shown(now.Ambiguous);
        var lost = now.LostText;
        _name.Text = now.Active?.Name ?? "No vehicle chosen";
        _trailing.Glyph = lost == null ? ChevronGlyph : WarningGlyph;
        _trailing.Foreground = Ui.Token(lost == null ? "TextSecondaryBrush" : "CautionBrush");
        ToolTipService.SetToolTip(_pill, lost ?? "Choose which vehicle to fly");
        _title.Text = lost ?? VehicleChoices.ChooserTitle;
        if (notice != null) HandedOver?.Invoke(this, notice);
    }

    private void Opened()
    {
        _panel.Refuse(null);
        _panelRow.Visibility = Ui.Shown(FleetBridge.PanelToggleShown());
        SyncToggle();
    }

    private void SyncToggle()
    {
        _painting = true;
        _panelToggle.IsOn = FleetBridge.PanelEnabled(Bridge.Get(FleetBridge.PanelValue));
        _painting = false;
    }
}
